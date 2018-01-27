use std::fs::File;
use std::collections::HashMap;
use serde_json;
use failure::ResultExt;

use websocket::message::OwnedMessage;

use hex_music;
use hex_music::database::Track;

use proto;

use error::{Result, ErrorKind};

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

pub struct State {
    reqs: HashMap<String, RequestState>,
    collection: hex_music::Collection,
    buffer: Vec<u8>
}

impl State {
    pub fn new() -> State {
        State {
            reqs: HashMap::new(),
            collection: hex_music::Collection::new(),
            buffer: Vec::new()
        }
    }

    pub fn process(&mut self, msg: String) -> Result<OwnedMessage> {
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

            proto::Incoming::AddTrack { format } => {
                ("add_track", self.collection.add_track(&format, &self.buffer)
                    .map(|x| proto::Outgoing::AddTrack(x.key))
                    .map_err(|err| err.context(ErrorKind::Music).into())
                )
            },

            proto::Incoming::UpdateTrack { key, title, album, interpret, conductor, composer } => {
                ("update_track", 
                    self.collection.update_track(&key, title, album, interpret, conductor, composer)
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
