use std::collections::HashMap;
use serde_json::{self, Value};

use hex_music::{self, database};
use proto;

enum RequestState {
    Search {
        query: String,
        seek: usize
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

    pub fn process(&mut self, msg: String) -> Result<String,()> {
        let packet: proto::IncomingWrapper = serde_json::from_str(&msg).expect("Couldnt parse!");
    
        let mut remove = false;

        println!("Got: {}", &msg);
        let payload = match packet.payload {
            proto::Incoming::GetTrack { key } => { 
                proto::Outgoing::Track(self.collection.get_track(&key))
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
                proto::Outgoing::SearchResult {
                    query: query.clone(),
                    answ: res,
                    more: more
                }
            },
            proto::Incoming::ClearBuffer => {
                self.buffer.clear();

                proto::Outgoing::ClearBuffer
            },

            proto::Incoming::AddTrack { format } => {
                let res = self.collection.add_track(&format, &self.buffer);

                proto::Outgoing::AddTrack {
                    data: res
                }
            },

            proto::Incoming::GetTrackData { key } => {
                //let data = self.collection.get_track_data(&key);

                proto::Outgoing::GetTrackData
            },
            
            proto::Incoming::UpdateTrack { key, title, album, interpret, conductor, composer } => {
                proto::Outgoing::UpdateTrack(self.collection.update_track(&key, title, album, interpret, conductor, composer))
            }
        };

        // remove if no longer needed
        if remove {
            self.reqs.remove(&packet.id);
        }

        // wrap the payload to a full packet and convert to a string
        payload.to_string(&packet.id, &packet.fnc)
    }

    pub fn process_binary(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
}
