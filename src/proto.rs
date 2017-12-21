use serde_json::{self, Value};
use hex_music::database::Track;

#[derive(Deserialize)]
//#[serde(tag = "fn", content="payload")]
#[serde(untagged)]
pub enum Incoming {
    #[serde(rename="search")]
    Search {
        query: String
    },
    #[serde(rename="get_track")]
    GetTrack {
        key: String
    },
    #[serde(rename="clear_buffer")]
    ClearBuffer,
    #[serde(rename="add_track")]
    AddTrack {
        format: String
    },
    #[serde(rename="get_track_data")]
    GetTrackData {
        key: String
    },
    #[serde(rename="update_track")]
    UpdateTrack {
        key: String,
        title: Option<String>,
        album: Option<String>,
        interpret: Option<String>,
        conductor: Option<String>,
        composer: Option<String>
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
    Track(Result<Track, ()>),
    ClearBuffer,
    AddTrack {
        data: Track
    },
    GetTrackData,
    UpdateTrack(Result<String,()>)
}

#[derive(Serialize)]
struct OutgoingWrapper {
    id: String,
    #[serde(rename="fn")]
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
