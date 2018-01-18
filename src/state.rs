use std::fs::File;
use std::collections::HashMap;
use serde_json::{self, Value};

use websocket::message::OwnedMessage;

use hex_music::{self, database};
use hex_music::database::Track;

use proto;

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

    pub fn process(&mut self, msg: String) -> Result<OwnedMessage,()> {
        println!("Got: {}", &msg);

        let packet: proto::IncomingWrapper = serde_json::from_str(&msg).map_err(|_| ())?;
    
        let mut remove = false;
        let mut binary_data: Option<Vec<u8>> = None;

        let payload = match packet.msg {
            proto::Incoming::GetTrack { key } => { 
                ("get_track", proto::Outgoing::Track(self.collection.get_track(&key)))
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

                let res = self.collection.search(&query, *seek);

                // update information about position in stream
                let more = res.len() >= 50;
                remove = !more;
                *seek += res.len() + 1;

                // create a struct containing all results
                ("search", proto::Outgoing::SearchResult {
                    query: query.clone(),
                    answ: res,
                    more: more
                })
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

                ("stream_next", proto::Outgoing::StreamNext)
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

                ("stream_seek", proto::Outgoing::StreamSeek {
                    pos: pos
                })
            },

            proto::Incoming::StreamEnd => {
                remove = true;

                ("stream_end", proto::Outgoing::StreamEnd)
            },
            proto::Incoming::ClearBuffer => {
                self.buffer.clear();

                ("clear_buffer", proto::Outgoing::ClearBuffer)
            },

            proto::Incoming::AddTrack { format } => {
                let res = self.collection.add_track(&format, &self.buffer);

                ("add_track", proto::Outgoing::AddTrack {
                    key: res.key
                })
            },

            proto::Incoming::UpdateTrack { key, title, album, interpret, conductor, composer } => {
                ("update_track", 
                    proto::Outgoing::UpdateTrack(self.collection.update_track(&key, title, album, interpret, conductor, composer))
                )
            },

            proto::Incoming::GetSuggestion { key } => {
                ("get_suggestion", proto::Outgoing::GetSuggestion {
                    key: key.clone(),
                    data: self.collection.get_suggestion(&key)
                })
            },

            proto::Incoming::AddPlaylist { name } => {
                ("add_playlist", proto::Outgoing::AddPlaylist(self.collection.add_playlist(&name)))
            },

            proto::Incoming::SetPlaylistImage { key } => {
                ("set_playlist_image", proto::Outgoing::SetPlaylistImage)
            },

            proto::Incoming::AddToPlaylist { key } => {
                ("add_to_playlist", proto::Outgoing::AddToPlaylist)
            },

            proto::Incoming::GetPlaylists => {
                ("get_playlists", proto::Outgoing::GetPlaylists(self.collection.get_playlists()))
            },

            proto::Incoming::GetPlaylist { key }=> {
                ("get_playlist", proto::Outgoing::GetPlaylist(self.collection.get_playlist(&key)))
            },

            proto::Incoming::GetPlaylistsOfTrack { key } => {
                ("get_playlists_of_track", proto::Outgoing::GetPlaylistsOfTrack(self.collection.get_playlists_of_track(&key)))
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
            payload.1.to_string(&packet.id, payload.0).map(|x| OwnedMessage::Text(x))
        }
    }

    pub fn process_binary(&mut self, data: &[u8]) {
        println!("Got binary with length: {}", data.len());

        self.buffer.extend_from_slice(data);
    }
}
