use std::fs::File;
use std::collections::HashMap;
use std::slice;

use serde_json;

use tokio_core::reactor::Handle;

use websocket::message::OwnedMessage;

use proto;

use conf;
use error::{Result, Error};
use proto::{Track, Token, Event};

use convert::{UploadState, UploadProgress, download::{DownloadState, DownloadProgress}};

use hex_database::{self, events::Action};
use hex_music_container::{self, Configuration, Container};

use acousticid;

enum RequestState {
    Search {
        query: String,
        seek: usize
    },

    Stream {
        track: hex_database::Track,
        container: Container<File>
    }
}

pub struct State {
    handle: Handle,
    reqs: HashMap<String, RequestState>,
    pub collection: hex_database::Collection,
    data_path: String,
    buffer: Vec<u8>,
    uploads: Vec<UploadState>,
    downloads: Vec<DownloadState>,
}

impl State {
    pub fn new(handle: Handle, conf: conf::Music) -> State {
        State {
            handle: handle,
            reqs: HashMap::new(),
            collection: hex_database::Collection::from_file(&conf.db_path),
            data_path: conf.data_path,
            buffer: Vec::new(),
            uploads: Vec::new(),
            downloads: Vec::new()
        }
    }

    pub fn process(&mut self, origin: String, msg: String) -> Result<OwnedMessage> {
        println!("Got: {}", &msg);

        let packet: proto::IncomingWrapper = serde_json::from_str(&msg)
            .map_err(|_| Error::Parsing)?;
    
        let mut remove = false;
        let mut binary_data: Option<Vec<u8>> = None;

        let payload: (&str, Result<proto::Outgoing>) = match packet.msg {
            proto::Incoming::GetTrack { key } => { 
                ("get_track", self.collection.get_track(&key)
                    .map(|x| Track::from_db_obj(x))
                    .map(|x| proto::Outgoing::Track(x))
                    .map_err(|err| Error::Database(err))
                )
            },
            proto::Incoming::Search { query } => {
                let prior_state = self.reqs.entry(packet.id.clone())
                    .or_insert(RequestState::Search { 
                        query: query,
                        seek: 0
                    });

                let (query, seek) = match prior_state {
                    &mut RequestState::Search{ ref mut query, ref mut seek } => (query, seek),
                    _ => panic!("blub")
                };

                let res = self.collection.search_limited(&query, *seek)
                    .map(|x| {
                        x.into_iter()
                            .map(|x| Track::from_db_obj(x))
                            .collect::<Vec<Track>>()
                    })
                    .map(|x| {
                        // update information about position in stream
                        let more = x.len() >= 50;
                        remove = !more;
                        *seek += x.len() + 1;

                        // create a struct containing all results
                        proto::Outgoing::SearchResult {
                            query: query.clone(),
                            answ: x,
                            more: more
                        }
                    })
                    .map_err(|err| Error::Database(err));

                ("search", res)
            },
            proto::Incoming::StreamNext { key } => {
                let State {
                    ref data_path,
                    ref collection,
                    ref mut reqs,
                    ..
                } = *self;

                let entry = reqs.entry(packet.id.clone());

                let prior_state = entry
                    .or_insert_with(|| {
                        collection.add_event(Action::PlaySong(key.clone().unwrap()).with_origin(origin)).unwrap();

                        RequestState::Stream {
                            container: Container::<File>::with_key(&data_path, &key.clone().unwrap()).unwrap(),
                            track: collection.get_track(&key.unwrap()).unwrap()
                        }
                    });
                
                let mut container = match prior_state {
                    &mut RequestState::Stream { ref mut container, .. } => container,
                    _ => panic!("blub")
                };


                let mut pcm = Ok(Vec::new());
                for i in 0..10 {
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
                            ("stream_next", Err(Error::MusicContainer(hex_music_container::error::Error::ReachedEnd)))
                        } else {
                            binary_data = Some(pcm);
                            ("stream_next", Ok(proto::Outgoing::StreamNext))
                        }
                    },
                    Err(err) => {
                        ("stream_next", Err(err))
                    }
                }
            },

            proto::Incoming::StreamSeek { sample } => {
                let (mut container, track) = match self.reqs.get_mut(&packet.id).unwrap() {
                    &mut RequestState::Stream { ref mut container, ref mut track } => (container, track),
                    _ => panic!("blub")
                };

                if sample as f64 > track.duration * 48000.0 {
                    panic!("blub");
                }
                
                //let pos = self.collection.stream_seek(pos, &track, &mut file);
                container.seek_to_sample(sample as u32);

                ("stream_seek", Ok(proto::Outgoing::StreamSeek {
                    sample: sample
                }))
            },

            proto::Incoming::StreamEnd => {
                remove = true;

                ("stream_end", Ok(proto::Outgoing::StreamEnd))
            },
            proto::Incoming::ClearBuffer => {
                self.buffer.clear();

                ("clear_buffer", Ok(proto::Outgoing::ClearBuffer))
            },

            proto::Incoming::UpdateTrack { key, title, album, interpret, people, composer } => {
                ("update_track", 
                    self.collection.update_track(&key, title, album, interpret, people, composer)
                        .map(|x| proto::Outgoing::UpdateTrack(x))
                        .map_err(|err| Error::Database(err))

                )
            },

            proto::Incoming::GetSuggestion { key } => {
                let suggestion = self.collection.get_track(&key)
                    .map_err(|x| Error::Database(x))
                    .and_then(|x| acousticid::get_metadata(&x.fingerprint, x.duration as u32));

                ("get_suggestion", suggestion.map(|x| proto::Outgoing::GetSuggestion {
                        key: key.clone(),
                        data: x
                }))
            },

            proto::Incoming::AddPlaylist { name } => {
                ("add_playlist", self.collection.add_playlist(&name, None)
                    .map(|x| proto::Playlist::from_db_obj(x))
                    .map(|x| proto::Outgoing::AddPlaylist(x))
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::DeletePlaylist { key } => {
                ("delete_playlist", self.collection.delete_playlist(&key)
                    .map(|_| proto::Outgoing::DeletePlaylist)
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::UpdatePlaylist { key, title, desc } => {
                ("update_playlist", self.collection.update_playlist(&key, title, desc, None)
                    .map(|_| proto::Outgoing::UpdatePlaylist)
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::SetPlaylistImage { key } => {
                ("set_playlist_image", Ok(proto::Outgoing::SetPlaylistImage))
            },

            proto::Incoming::AddToPlaylist { key, playlist } => {
                ("add_to_playlist", self.collection.add_to_playlist(&key, &playlist)
                    .map(|x|proto::Playlist::from_db_obj(x))
                    .map(|x| proto::Outgoing::AddToPlaylist(x))
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::GetPlaylists => {
                let pls = self.collection.get_playlists().into_iter()
                    .map(|x| proto::Playlist::from_db_obj(x)).collect();

                ("get_playlists", Ok(proto::Outgoing::GetPlaylists(pls)))
            },

            proto::Incoming::GetPlaylist { key }=> {
                ("get_playlist", self.collection.get_playlist(&key)
                    .map(|x| (
                        proto::Playlist::from_db_obj(x.0), 
                        x.1.into_iter().map(proto::Track::from_db_obj).collect()
                    ))
                    .map(|x| proto::Outgoing::GetPlaylist(x))
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::GetPlaylistsOfTrack { key } => {
                ("get_playlists_of_track", self.collection.get_playlists_of_track(&key)
                    .map(|x| {
                        x.into_iter().map(proto::Playlist::from_db_obj).collect()
                    })
                    .map(|x| proto::Outgoing::GetPlaylistsOfTrack(x))
                    .map_err(|err| Error::Database(err))
                )
            },
            proto::Incoming::DeleteTrack { key } => {
                self.collection.add_event(Action::DeleteSong(key.clone()).with_origin(origin.clone())).unwrap();

                ("delete_track", self.collection.delete_track(&key)
                    .map(|x| proto::Outgoing::DeleteTrack(x))
                    .map_err(|err| Error::Database(err))
                )
            },

            proto::Incoming::UploadYoutube { path } => {
                println!("YOUTUBE ID {}", packet.id);

                let handle = self.handle.clone();

                self.uploads.push(UploadState::youtube(packet.id.clone(), &path, handle));

                ("upload_youtube", Ok(proto::Outgoing::UploadYoutube))
            },

            proto::Incoming::UploadTrack { name, format } => {
                let handle = self.handle.clone();

                self.uploads.push(UploadState::converting_ffmpeg(handle, name, packet.id.clone(), &self.buffer, &format));

                ("upload_track", Ok(proto::Outgoing::UploadTrack))
            },

            proto::Incoming::AskUploadProgress => {
                println!("Ask upload progress");

                // tick each item
                for item in &mut self.uploads {
                    if let Some(track) = item.tick(packet.id.clone(), self.data_path.clone()) {
                        self.collection.add_event(Action::AddSong(track.key.clone()).with_origin(origin.clone())).unwrap();
                        self.collection.insert_track(track).unwrap();
                    }
                }

                // collect update informations
                let infos = self.uploads.iter().map(|item| {
                    UploadProgress {
                        desc: item.desc().clone(),
                        kind: item.kind().into(),
                        progress: item.progress(),
                        key: item.key(),
                        track_key: item.track_key()
                    }
                }).collect();

                // delete finished uploads
                self.uploads.retain(|x| x.should_retain());

                ("ask_upload_progress", Ok(proto::Outgoing::AskUploadProgress(infos)))
            },

            proto::Incoming::VoteForTrack { key } => {
                ("vote_for_track", self.collection.vote_for_track(&key)
                    .map(|_| proto::Outgoing::VoteForTrack)
                    .map_err(|err| Error::Database(err))
                )
            },
            proto::Incoming::GetToken { token } => {
                ("get_token", self.collection.get_token(token)
                    .map(|x| {
                        let tracks: Vec<proto::Track> = x.2.into_iter()
                            .map(|x| proto::Track::from_db_obj(x))
                            .collect();
                        (
                            proto::Token::from_db_obj(x.0),
                            proto::Playlist::from_db_obj(x.1),
                            tracks
                        )
                    })
                    .map(|x| proto::Outgoing::GetToken(x))
                    .map_err(|err| Error::Database(err))
                )
            
            },
            proto::Incoming::InsertToken { token } => {
                ("insert_token", self.collection.insert_token(Token::to_db_obj(token))
                    .map(|_| proto::Outgoing::InsertToken)
                    .map_err(|err| Error::Database(err))
                )
            },
            proto::Incoming::UpdateToken { token, played, pos } => {
                ("update_token", self.collection.update_token(token, played, pos)
                     .map(|_| proto::Outgoing::UpdateToken)
                     .map_err(|err| Error::Database(err))
                )
            },
            proto::Incoming::GetSummarise => {
                ("get_summarise", Ok(proto::Outgoing::GetSummarise(self.collection.get_summarisation())))
            },
            proto::Incoming::GetEvents => {
                ("get_events", Ok(proto::Outgoing::GetEvents(
                    self.collection.get_events().into_iter()
                        .map(|x| (x.0, Event::from_db_obj(x.1)))
                        .collect()
                )))
            },
            proto::Incoming::Download { format, tracks } => {
                let id = packet.id.clone();
                let res = tracks.into_iter()
                    .map(|x| self.collection.get_track(&x)
                        .map(|x| Track::from_db_obj(x))
                        .map_err(|err| Error::Database(err))
                    )
                    .collect::<Result<Vec<Track>>>()
                    .map(|tracks| {
                        self.downloads.push(DownloadState::new(self.handle.clone(), id, format, tracks, 2, self.data_path.clone()));

                        proto::Outgoing::Download
                    });

                ("download", res)
            },
            proto::Incoming::AskDownloadProgress => {
                let res = self.downloads.iter_mut()
                    .map(|x| x.progress())
                    .collect();

                ("ask_download_progress", Ok(proto::Outgoing::AskDownloadProgress(res)))
            }
        };

        // remove if no longer needed
        if remove {
            self.reqs.remove(&packet.id);
        }

        println!("Outgoing: {:?}", payload);

        if let Some(data) = binary_data {
            Ok(OwnedMessage::Binary(data))
        } else {
            // wrap the payload to a full packet and convert to a string
            proto::OutgoingResult(payload.1.map_err(|err| format!("{:?}", err))).to_string(&packet.id, payload.0).map(|x| OwnedMessage::Text(x))
        }
    }

    pub fn process_binary(&mut self, data: &[u8]) {
        println!("Got binary with length: {}", data.len());

        self.buffer.extend_from_slice(data);
    }
}
