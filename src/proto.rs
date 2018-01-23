use serde_json::{self, Value};
use hex_music::database::{Track, Playlist};

#[derive(Deserialize)]
#[serde(tag = "fn")]
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
    #[serde(rename="stream_next")]
    StreamNext {
        key: String
    },
    #[serde(rename="stream_end")]
    StreamEnd,
    #[serde(rename="stream_seek")]
    StreamSeek {
        pos: f64
    },
    #[serde(rename="update_track")]
    UpdateTrack {
        key: String,
        title: Option<String>,
        album: Option<String>,
        interpret: Option<String>,
        conductor: Option<String>,
        composer: Option<String>
    },
    #[serde(rename="get_suggestion")]
    GetSuggestion {
        key: String
    },
    #[serde(rename="add_playlist")]
    AddPlaylist {
        name: String
    },
    #[serde(rename="set_playlist_image")]
    SetPlaylistImage {
        key: String
    },
    #[serde(rename="add_to_playlist")]
    AddToPlaylist {
        key: String
    },
    #[serde(rename="get_playlists")]
    GetPlaylists,
    #[serde(rename="get_playlist")]
    GetPlaylist {
        key: String
    },
    #[serde(rename="get_playlists_of_track")]
    GetPlaylistsOfTrack {
        key: String
    },
    #[serde(rename="delete_track")]
    DeleteTrack {
        key: String
    }
}

#[derive(Deserialize)]
pub struct IncomingWrapper {
    pub id: String,
    pub msg: Incoming
}

#[derive(Serialize, Debug)]
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
        key: String
    },
    StreamNext,
    StreamSeek {
        pos: f64
    },
    StreamEnd,
    UpdateTrack(Result<String,()>),
    GetSuggestion {
        key: String,
        data: Result<String, ()>
    },
    AddPlaylist(Playlist),
    SetPlaylistImage,
    AddToPlaylist,
    GetPlaylists(Vec<Playlist>),
    GetPlaylist((Playlist,Vec<Track>)),
    GetPlaylistsOfTrack(Vec<Playlist>),
    DeleteTrack
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
