//! Connection based state
//!
//! Every client has an own state which contains a byte buffer, uploads, downloads, pending
//! requests and the database connection. The state exists as long as the connection and for
//! example allows the client to create an iterator of search results.

use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::HashMap;
use std::slice;

use tokio_core::reactor::Handle;

use crate::error::{Result, Error};

use crate::convert::{UploadState, download::{DownloadState}};

use hex_database::{self, Track, Token, View, Playlist};
use hex_music_container::{self, Configuration, Container};
use hex_server_protocol::{Request, Answer, RequestAction, AnswerAction, PacketId, objects::UploadProgress};

use crate::acousticid;

/// A pending request
///
/// There are requests which are not finished after a single call. They are rembered with the `id`
/// supplemented in the first call and can be further executed with the same `id`.
enum RequestState {
    /// A search query should remember which results were already transmitted
    Search {
        query: String,
        seek: usize
    },

    /// A running stream
    Stream {
        track: hex_database::Track,
        container: Container<File>
    }
}

/// State containing useful items
pub struct State {
    /// Handle to the event queue
    handle: Handle,
    /// All pending requests
    reqs: HashMap<PacketId, RequestState>,
    /// Open connection to the database
    pub collection: View,
    /// Path to the data section
    data_path: PathBuf,
    /// All uploads
    uploads: Vec<UploadState>,
    /// All downloads
    downloads: Vec<DownloadState>,
    /// Have we inserted a token last time?
    token_avail: bool
}

impl State {
    /// Create a new `State` from a configuration
    pub fn new(handle: Handle, path: &Path, view: View) -> State {
        State {
            handle: handle,
            reqs: HashMap::new(),
            collection: view,
            data_path: path.join("data"),
            uploads: Vec::new(),
            downloads: Vec::new(),
            token_avail: false
        }
    }

    pub fn process_request(&mut self, req: Request) -> Answer {
        let Request { id, msg } = req;
        let mut remove = false;

        let answ = match msg {
            RequestAction::GetTrack { key } => {
                self.collection.get_track(key)
                    .map(|x| AnswerAction::Track(x))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::UpdateTrack { key, title, album, interpret, people, composer } => {
                self.collection.update_track(key, 
                    title.as_ref().map(String::as_str), 
                    album.as_ref().map(String::as_str), 
                    interpret.as_ref().map(String::as_str), 
                    people.as_ref().map(String::as_str), 
                    composer.as_ref().map(String::as_str)
                )
                    .map(|x| AnswerAction::UpdateTrack(x))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::Search { query } => {
                let prior_state = self.reqs.entry(id.clone())
                    .or_insert(RequestState::Search { 
                        query: query,
                        seek: 0
                    });

                let (query, seek) = match prior_state {
                    &mut RequestState::Search{ ref mut query, ref mut seek } => (query, seek),
                    _ => panic!("blub")
                };

                self.collection.search_limited(&query, *seek)
                    .map(|x| {
                        // update information about position in stream
                        let more = x.len() >= 50;
                        remove = !more;
                        *seek += x.len() + 1;

                        // create a struct containing all results
                        AnswerAction::SearchResult {
                            query: query.clone(),
                            answ: x,
                            more: more
                        }
                    })
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::StreamNext { key } => {
                let State {
                    ref data_path,
                    ref collection,
                    ref mut reqs,
                    ..
                } = *self;

                let entry = reqs.entry(id.clone());

                let prior_state = entry
                    .or_insert_with(|| {
                        //collection.add_event(Action::PlaySong(key.unwrap()).with_origin(origin)).unwrap();
                        let file = File::open(data_path.join(key.unwrap().to_path())).unwrap();

                        RequestState::Stream {
                            container: Container::<File>::load(file).unwrap(),
                            track: collection.get_track(key.unwrap()).unwrap()
                        }
                    });
                
                let mut container = match prior_state {
                    &mut RequestState::Stream { ref mut container, .. } => container,
                    _ => panic!("blub")
                };


                let mut pcm = Ok(Vec::new());
                for _ in 0..10 {
                    let data = container.next_packet(Configuration::Stereo)
                        .map(|x| {
                            unsafe {
                                slice::from_raw_parts(
                                    x.as_ptr() as *const u8,
                                    x.len() * 2
                                )
                            }
                        });

                    match data {
                        Ok(data) => {
                            match pcm {
                                Ok(ref mut pcm) => pcm.extend(data.into_iter()),
                                _ => {}
                            }
                        },
                        Err(err) => {
                            match err {
                                hex_music_container::error::Error::ReachedEnd => {},
                                err => pcm = Err(Error::MusicContainer(err))
                            }

                            break;
                        }
                    }
                }

                match pcm {
                    Ok(pcm) => {
                        if pcm.len() == 0 {
                            Err(Error::MusicContainer(hex_music_container::error::Error::ReachedEnd))
                        } else {
                            Ok(AnswerAction::StreamNext(pcm))
                        }
                    },
                    Err(err) => Err(err)
                }
            },

            RequestAction::StreamSeek { sample } => {
                let (mut container, track) = match self.reqs.get_mut(&id).unwrap() {
                    &mut RequestState::Stream { ref mut container, ref mut track } => (container, track),
                    _ => panic!("blub")
                };

                if sample as f64 > track.duration * 48000.0 {
                    panic!("blub");
                }
                
                //let pos = self.collection.stream_seek(pos, &track, &mut file);
                container.seek_to_sample(sample as u32);

                Ok(AnswerAction::StreamSeek { sample })
            },

            RequestAction::StreamEnd => {
                remove = true;

                Ok(AnswerAction::StreamEnd)
            },

            RequestAction::GetSuggestion { key } => {
                let suggestion = self.collection.get_track(key)
                    .map_err(|x| Error::Database(x))
                    .and_then(|x| acousticid::get_metadata(&x.fingerprint, x.duration as u32));

                suggestion.map(|x| AnswerAction::GetSuggestion {
                        key: key.clone(),
                        data: x
                })
            },

            RequestAction::AddPlaylist { name } => {
                let key = self.collection.last_playlist_key().unwrap() + 1;
                let playlist = Playlist {
                    key: key.clone(),
                    title: name,
                    desc: None,
                    tracks: Vec::new(),
                    origin: self.collection.id()
                };

                self.collection.add_playlist(playlist.clone())
                    .map(|_| AnswerAction::AddPlaylist(playlist))
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::DeletePlaylist { key } => {
                self.collection.delete_playlist(key)
                    .map(|_| AnswerAction::DeletePlaylist)
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::UpdatePlaylist { key, title, desc } => {
                self.collection.update_playlist(key, title, desc)
                    .map(|_| AnswerAction::UpdatePlaylist)
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::SetPlaylistImage { key, image } => {
                Ok(AnswerAction::SetPlaylistImage)
            },

            RequestAction::AddToPlaylist { key, playlist } => {
                self.collection.add_to_playlist(key, playlist)
                    .map(|_| AnswerAction::AddToPlaylist)
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::DeleteFromPlaylist { key, playlist } => {
                self.collection.delete_from_playlist(key, playlist)
                    .map(|_| AnswerAction::DeleteFromPlaylist)
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::GetPlaylists => {
                Ok(AnswerAction::GetPlaylists(self.collection.get_playlists()))
            },

            RequestAction::GetPlaylist { key }=> {
                self.collection.get_playlist(key)
                    .map(|mut x| {
                        for track in &mut x.1 {
                            track.fingerprint = vec![];
                        }

                        AnswerAction::GetPlaylist(x)
                    })
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::GetPlaylistsOfTrack { key } => {
                self.collection.get_playlists_of_track(key)
                    .map(|x| AnswerAction::GetPlaylistsOfTrack(x))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::DeleteTrack { key } => {
                println!("Delete track with key: {}", key);
                //self.collection.add_event(Action::DeleteSong(key.clone()).with_origin(origin.clone())).unwrap();

                self.collection.delete_track(key)
                    .map(|x| AnswerAction::DeleteTrack(x))
                    .map_err(|err| Error::Database(err))
            },

            RequestAction::UploadYoutube { path } => {
                let handle = self.handle.clone();

                self.uploads.push(UploadState::youtube(id.clone(), &path, handle));

                Ok(AnswerAction::UploadYoutube)
            },

            RequestAction::UploadTrack { name, format, data } => {
                println!("Got track buffer with: {}", data.len());
                let handle = self.handle.clone();

                self.uploads.push(UploadState::converting_ffmpeg(handle, name, id.clone(), &data, &format));

                Ok(AnswerAction::UploadTrack)
            },

            RequestAction::AskUploadProgress => {
                // tick each item
                for item in &mut self.uploads {
                    if let Some(track) = item.tick(self.data_path.clone()) {
                        self.collection.add_track(track).unwrap();
                    }
                }

                // collect update informations
                let infos = self.uploads.iter().filter_map(|item| {
                    let id = item.id()?;

                    Some(UploadProgress {
                        desc: item.desc().clone(),
                        kind: item.kind().into(),
                        progress: item.progress(),
                        id: id,
                        key: item.track_key()
                    })
                }).collect();

                // delete finished uploads
                self.uploads.retain(|x| x.should_retain());

                Ok(AnswerAction::AskUploadProgress(infos))
            },

            RequestAction::VoteForTrack { key } => {
                self.collection.vote_for_track(key)
                    .map(|_| AnswerAction::VoteForTrack)
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::GetToken { token } => {
                self.token_avail = true;

                self.collection.get_token(token)
                    .map(|(token, x)| {
                        if let Some((playlist, tracks)) = x {
                            (
                                token,
                                Some((playlist, tracks))
                            )
                        } else {
                            (
                                token,
                                None
                            )
                        }
                    })
                    .map(|x| AnswerAction::GetToken(x))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::CreateToken => {
                let id = self.collection.last_token_id().unwrap() + 1;
                let token = Token {
                    token: id,
                    key: None,
                    played: Vec::new(),
                    pos: None,
                    last_use: 0
                };

                self.collection.add_token(token)
                    .map(|id| AnswerAction::CreateToken(id))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::UpdateToken { token, key, played, pos } => {
                if self.token_avail {
                    self.token_avail = false;
                }

                self.collection.update_token(token, key, played, pos)
                     .map(|_| AnswerAction::UpdateToken)
                     .map_err(|err| Error::Database(err))
            },
            RequestAction::LastToken => {
                self.collection.get_last_used_token()
                    .map(|x| AnswerAction::LastToken(Some(x.0.token)))
                    .map_err(|err| Error::Database(err))
            },
            RequestAction::GetSummary => {
                Ok(AnswerAction::GetSummary(self.collection.get_complete_summary()))
            },
            RequestAction::GetTransitions => {
                Ok(AnswerAction::GetTransitions(self.collection.get_transitions()))
            },
            RequestAction::Download { format, tracks } => {
                let id = id.clone();
                tracks.into_iter()
                    .map(|x| self.collection.get_track(x)
                        .map_err(|err| Error::Database(err))
                    )
                    .collect::<Result<Vec<Track>>>()
                    .map(|tracks| {
                        self.downloads.push(DownloadState::new(self.handle.clone(), id, format, tracks, 2, self.data_path.clone()));

                        AnswerAction::Download
                    })
            },
            RequestAction::AskDownloadProgress => {
                let res = self.downloads.iter_mut()
                    .map(|x| x.progress())
                    .collect();

                Ok(AnswerAction::AskDownloadProgress(res))
            }
        };

        // remove if no longer needed
        if remove {
            self.reqs.remove(&id);
        }

        //println!("Outgoing: {:?}", answ);

        Answer::new(id, answ.map_err(|err| format!("{:?}", err)))
    }

    /// Process a single packet
    ///
    /// * `origin` - where does the request originates from
    /// * `msg` - what is the content of the message
    /// * `gtoken` - globally shared token, used to change the token in frontend
    pub fn process(&mut self, buf: Vec<u8>) -> Option<Vec<u8>> {
        //println!("Process buf {}", buf.len());
        Request::try_from(&buf)
            .map(|req| self.process_request(req))
            .and_then(|answer| answer.to_buf())
            .map_err(|err| { println!("Parse error: {:?}", err); err})
            .ok()
    }
}
