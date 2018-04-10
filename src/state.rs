use std::fs::File;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::borrow::BorrowMut;

use serde_json;
use failure::ResultExt;

use futures::{Sink, Stream, IntoFuture, Future};

use tokio_core::reactor::Handle;
use websocket;
use tokio_io;

use websocket::message::OwnedMessage;

use hex_music;
use hex_music::database::Track;

use proto;

use conf;
use error::{Result, ErrorKind};
use youtube;
use hex_music::ffmpeg;
use hex_music::opus_conv;

enum RequestState {
    Search {
        query: String,
        seek: usize
    },

    Stream {
        file: File,
        track: Track
    }
}

#[derive(Serialize, Debug)]
pub struct UploadProgress {
    kind: String,
    progress: f32,
    key: String,
    track_key: Option<String>
}

enum UploadState {
    YoutubeDownload {
        downloader: youtube::Downloader,
        state: Rc<RefCell<youtube::State>>,
        key: String
    },
    ConvertingFFMPEG {
        converter: ffmpeg::Converter,
        state: Rc<RefCell<ffmpeg::State>>,
        key: String
    },
    ConvertingOpus {
        converter: opus_conv::Converter,
        state: Rc<RefCell<opus_conv::State>>,
        key: String,
        track_key: Option<String>
    }
}

impl UploadState {
    pub fn youtube(key: String, path: &str, handle: Handle) -> UploadState {
        let mut dwnd = youtube::Downloader::new(handle.clone(), path);

        let state = Rc::new(RefCell::new(youtube::State::empty()));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::YoutubeDownload {
            downloader: dwnd,
            state: state,
            key: key
        }
    }

    pub fn converting_ffmpeg(handle: Handle, key: String, data: &[u8], format: &str) -> UploadState {
        let mut dwnd = ffmpeg::Converter::new(handle.clone(), data, format).unwrap();

        let state = Rc::new(RefCell::new(ffmpeg::State::empty("")));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::ConvertingFFMPEG {
            converter: dwnd,
            state: state,
            key: key
        }
    }

    pub fn converting_opus(handle: Handle, key: String, samples: &[i16], duration: f32, num_channel: u32) -> UploadState {
        let mut dwnd = opus_conv::Converter::new(handle.clone(), Vec::from(samples), duration, num_channel);

        let state = Rc::new(RefCell::new(opus_conv::State::empty()));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        //handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::ConvertingOpus {
            converter: dwnd,
            state: state,
            key: key,
            track_key: None
        }
        
    }

    pub fn kind(&self) -> &str {
        match *self {
            UploadState::YoutubeDownload { .. } => "youtube_download",
            UploadState::ConvertingFFMPEG { .. } => "converting_ffmpeg",
            UploadState::ConvertingOpus { .. } => "converting_opus"
        }
    }
    pub fn progress(&self) -> f32 {
        match *self {
            UploadState::YoutubeDownload { ref state, .. } => state.borrow().progress,
            UploadState::ConvertingFFMPEG { ref state, .. } => state.borrow().progress,
            UploadState::ConvertingOpus { ref state, .. } => state.borrow().progress
        }
    }
    pub fn key(&self) -> String {
        match *self {
            UploadState::YoutubeDownload { ref key, .. } => key.clone(),
            UploadState::ConvertingFFMPEG { ref key, .. } => key.clone(),
            UploadState::ConvertingOpus { ref key, .. } => key.clone()
        }
    }

    pub fn track_key(&self) -> Option<String> {
        match *self {
            UploadState::YoutubeDownload { .. } => None,
            UploadState::ConvertingFFMPEG { .. } => None,
            UploadState::ConvertingOpus { ref track_key, .. } => track_key.clone()
        }
    }
}

pub struct State {
    handle: Handle,
    reqs: HashMap<String, RequestState>,
    collection: hex_music::Collection,
    buffer: Vec<u8>,
    uploads: Vec<UploadState>
}

impl State {
    pub fn new(handle: Handle, conf: conf::Music) -> State {
        State {
            handle: handle,
            reqs: HashMap::new(),
            collection: hex_music::Collection::new(conf.db_path, conf.data_path),
            buffer: Vec::new(),
            uploads: Vec::new()
        }
    }

    pub fn process(&mut self, msg: String, card_key: &mut Option<String>) -> Result<OwnedMessage> {
        println!("Got: {}", &msg);

        let packet: proto::IncomingWrapper = serde_json::from_str(&msg).context(ErrorKind::Parsing)?;
    
        let mut remove = false;
        let mut binary_data: Option<Vec<u8>> = None;

        let payload: (&str, Result<proto::Outgoing>) = match packet.msg {
            proto::Incoming::GetTrack { key } => { 
                ("get_track", self.collection.get_track(&key).map(|x| proto::Outgoing::Track(x)))
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

                let res = self.collection.search(&query, *seek)
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
                    .map_err(|err| err.context(ErrorKind::Music).into());

                ("search", res)
            },
            proto::Incoming::StreamNext { key } => {
                let prior_state = self.reqs.entry(packet.id.clone())
                    .or_insert(RequestState::Stream {
                        file: self.collection.stream_start(&key).unwrap(),
                        track: self.collection.get_track(&key).unwrap()
                    });

                let mut file = match prior_state {
                    &mut RequestState::Stream { ref mut file, .. } => file,
                    _ => panic!("blub")
                };

                let data = self.collection.stream_next(&mut file);

                binary_data = Some(data);

                ("stream_next", Ok(proto::Outgoing::StreamNext))
            },

            proto::Incoming::StreamSeek { pos } => {
                let (mut file, track) = match self.reqs.get_mut(&packet.id).unwrap() {
                    &mut RequestState::Stream { ref mut file, ref mut track } => (file, track),
                    _ => panic!("blub")
                };

                if pos < 0.0 || pos > track.duration {
                    panic!("blub");
                }
                
                let pos = self.collection.stream_seek(pos, &track, &mut file);

                ("stream_seek", Ok(proto::Outgoing::StreamSeek {
                    pos: pos
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
                        .map_err(|err| err.context(ErrorKind::Music).into())

                )
            },

            proto::Incoming::GetSuggestion { key } => {
                ("get_suggestion", self.collection.get_suggestion(&key)
                    .map(|x| proto::Outgoing::GetSuggestion {
                        key: key.clone(),
                        data: x
                    })
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::AddPlaylist { name } => {
                ("add_playlist", self.collection.add_playlist(&name)
                    .map(|x| proto::Outgoing::AddPlaylist(x))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::DeletePlaylist { key } => {
                ("delete_playlist", self.collection.delete_playlist(&key)
                    .map(|x| proto::Outgoing::DeletePlaylist)
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::UpdatePlaylist { key, title, desc } => {
                ("update_playlist", self.collection.update_playlist(&key, title, desc)
                    .map(|x| proto::Outgoing::UpdatePlaylist)
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::SetPlaylistImage { key } => {
                ("set_playlist_image", Ok(proto::Outgoing::SetPlaylistImage))
            },

            proto::Incoming::AddToPlaylist { key, playlist } => {
                ("add_to_playlist", self.collection.add_to_playlist(&key, &playlist)
                    .map(|x| proto::Outgoing::AddToPlaylist(x))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::GetPlaylists => {
                ("get_playlists", Ok(proto::Outgoing::GetPlaylists(self.collection.get_playlists())))
            },

            proto::Incoming::GetPlaylist { key }=> {
                ("get_playlist", self.collection.get_playlist(&key)
                    .map(|x| proto::Outgoing::GetPlaylist(x))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::GetPlaylistsOfTrack { key } => {
                ("get_playlists_of_track", self.collection.get_playlists_of_track(&key)
                    .map(|x| proto::Outgoing::GetPlaylistsOfTrack(x))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },
            proto::Incoming::DeleteTrack { key } => {
                ("delete_track", self.collection.delete_track(&key)
                    .map(|x| proto::Outgoing::DeleteTrack(x))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::UploadYoutube { path } => {
                println!("YOUTUBE ID {}", packet.id);

                let handle = self.handle.clone();

                self.uploads.push(UploadState::youtube(packet.id.clone(), &path, handle));

                ("upload_youtube", Ok(proto::Outgoing::UploadYoutube))
            },

            proto::Incoming::UploadTrack { format } => {
                let handle = self.handle.clone();

                self.uploads.push(UploadState::converting_ffmpeg(handle, packet.id.clone(), &self.buffer, &format));

                ("upload_track", Ok(proto::Outgoing::UploadTrack))
            },

            proto::Incoming::AskUploadProgress => {
                println!("Ask upload progress");

                let mut tmp = self.uploads.split_off(0);
                
                for item in tmp.iter_mut() {
                    let mut tmp2 = None;

                    match *item {
                        UploadState::YoutubeDownload { ref state, ref key, ref downloader } => {
                            let state = state.borrow();
                            if state.progress == 1.0 {
                                //TODO
                                tmp2 = Some(UploadState::converting_ffmpeg(downloader.handle.clone(), packet.id.clone(), &state.get_content().unwrap(), state.format()));
                            }
                        },
                        UploadState::ConvertingFFMPEG { ref key, ref state, ref converter } => {
                            let state = state.borrow();

                            let (data, num_channel, duration) = state.read();

                            if state.progress == 1.0 {

                                tmp2 = Some(UploadState::converting_opus(converter.handle.clone(), packet.id.clone(), &data, duration as f32, num_channel));
                            }
                        },
                        UploadState::ConvertingOpus { ref state, ref mut track_key, .. } => {
                            let state = state.borrow();

                            if state.progress == 1.0 {
                                if let Some(Ok((ref track, ref data))) = state.data {
                                    let key = self.collection.add_track(&data, track.clone()).unwrap();

                                    *track_key = Some(key);
                                }
                            }
                        }
                    }

                    if let Some(tmp2) = tmp2 {
                        *item = tmp2;
                    }
                }

                // replace old version
                self.uploads = tmp;

                // collect update informations
                let infos = self.uploads.iter().map(|item| {
                    UploadProgress {
                        kind: item.kind().into(),
                        progress: item.progress(),
                        key: item.key(),
                        track_key: item.track_key()
                    }
                }).collect();

                ("ask_upload_progress", Ok(proto::Outgoing::AskUploadProgress(infos)))
            },

            proto::Incoming::SetCardKey { key } => {
                *card_key = Some(key);

                ("set_card_key", Ok(proto::Outgoing::SetCardKey))
            },

            proto::Incoming::GetCardKey => {
                ("get_card_key", Ok(proto::Outgoing::GetCardKey(card_key.clone())))
            },

            proto::Incoming::VoteForTrack { key } => {
                ("vote_for_track", self.collection.vote_for_track(&key)
                    .map(|_| proto::Outgoing::VoteForTrack)
                    .map_err(|err| err.context(ErrorKind::Music).into()))
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
            proto::OutgoingResult(payload.1.map_err(|err| format!("{}", err))).to_string(&packet.id, payload.0).map(|x| OwnedMessage::Text(x))
        }
    }

    pub fn process_binary(&mut self, data: &[u8]) {
        println!("Got binary with length: {}", data.len());

        self.buffer.extend_from_slice(data);
    }
}
