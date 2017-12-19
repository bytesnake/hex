use serde_json::{self, Value};
use hex_music::database::Track;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Incoming {
    Search {
        query: String
    },
    GetTrack {
        key: String
    }
}

#[derive(Deserialize)]
pub struct IncomingWrapper {
    pub id: String,
    #[serde(rename="fn")]
    pub fnc: String,
    pub payload: Incoming
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Outgoing {
    SearchResult {
        query: String,
        answ: Vec<Track>,
        more: bool
    },
    Track {
        data: Track
    }
}

#[derive(Serialize)]
struct OutgoingWrapper {
    id: String,
    fnc: String,
    payload: Value
}

impl Outgoing {
    pub fn to_string(&self, id: &str, fnc: &str) -> Result<String, ()> {
        let wrapper = OutgoingWrapper {
            id: id.into(),
            fnc: fnc.into(),
            payload: serde_json::to_value(self).unwrap()
        };

        serde_json::to_string(&wrapper).map_err(|_| ())
    }
}
