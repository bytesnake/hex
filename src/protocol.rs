use std::collections::HashMap;
use serde_json::{self, Value};

use hex_music;

enum Request {
    GetTrack(String),
    Search((String, u32))
}

impl Request {
    pub fn from_value(name: &str, payload: Value) -> Result<Request,()> {
        match name {
            "get_track" => Ok(Request::GetTrack(payload["hash"].as_str().ok_or(())?.into())),
            "search" => {
                Ok(Request::Search((
                        payload["query"].as_str().ok_or(())?.into(),
                        0
                )))
            },
            _ => Err(())
        }
    }
}

pub struct State {
    reqs: HashMap<String, Request>,
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
        let pack: Value = serde_json::from_str(&msg).map_err(|_| ())?;
        let id = pack["id"].as_str().ok_or(())?;
        let fnc = pack["fn"].as_str().ok_or(())?;

        // check if there is a chain of packages with the same ID
        let prior_state = self.reqs.get(id).ok_or(());
        // convert the payload to an enum
        //let data = Request::from_value(fnc, pack["payload"].clone())?;

        match data {
            Request::GetTrack(hash) => {},
            Request::Search((query,_)) => {
                if let Ok(state) = prior_state {
                } else {
                    let query = pack["payload"]["query"].as_str().ok_or(())?;

                    let res = collection.search(query);
                
                    if res.len() < 51 {
                        
                    }

                    self.reqs.insert(fnc, (query, res, 0));

                }
            }
        }

        Ok("{}".into())
    }
}
