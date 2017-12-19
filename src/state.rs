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
    collection: hex_music::Collection
}

impl State {
    pub fn new() -> State {
        State {
            reqs: HashMap::new(),
            collection: hex_music::Collection::new()
        }
    }

    pub fn process(&mut self, msg: String) -> Result<String,()> {
        let packet: proto::IncomingWrapper = serde_json::from_str(&msg).map_err(|_| ())?;

        let mut remove = false;

        println!("Got: {}", &msg);
        let payload = match packet.payload {
            proto::Incoming::GetTrack { key } => { 
                proto::Outgoing::Track {
                    data: database::Track::empty("", "")
                }
            },
            proto::Incoming::Search { query } => {
                println!("Got search");

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
            }
        };

        // remove if no longer needed
        if remove {
            self.reqs.remove(&packet.id);
        }

        // wrap the payload to a full packet and convert to a string
        payload.to_string(&packet.id, &packet.fnc)
    }
}
