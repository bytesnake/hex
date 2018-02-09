use serde_json::{self, Value};
use hex_music::database::{Track, Playlist};
use error::{ErrorKind, Result};
use failure::Fail;
use std::result;
use youtube;


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
        people: Option<String>,
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
    #[serde(rename="delete_playlist")]
    DeletePlaylist {
        key: String
    },
    #[serde(rename="set_playlist_image")]
    SetPlaylistImage {
        key: String
    },
    #[serde(rename="add_to_playlist")]
    AddToPlaylist {
        key: String,
        playlist: String
    },
    #[serde(rename="update_playlist")]
    UpdatePlaylist {
        key: String,
        title: Option<String>,
        desc: Option<String>
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
    },
    #[serde(rename="from_youtube")]
    FromYoutube {
        path: String
    },
    #[serde(rename="finish_youtube")]
    FinishYoutube,
    #[serde(rename="set_card_key")]
    SetCardKey {
        key: String
    },
    #[serde(rename="get_card_key")]
    GetCardKey,
    #[serde(rename="vote_for_track")]
    VoteForTrack {
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
    Track(Track),
    ClearBuffer,
    AddTrack(String),
    StreamNext,
    StreamSeek {
        pos: f64
    },
    StreamEnd,
    UpdateTrack(String),
    GetSuggestion {
        key: String,
        data: String
    },
    AddPlaylist(Playlist),
    DeletePlaylist,
    UpdatePlaylist,
    SetPlaylistImage,
    AddToPlaylist(Playlist),
    GetPlaylists(Vec<Playlist>),
    GetPlaylist((Playlist,Vec<Track>)),
    GetPlaylistsOfTrack(Vec<Playlist>),
    DeleteTrack(()),
    FromYoutube(youtube::State),
    FinishYoutube(Track),
    SetCardKey,
    GetCardKey(Option<String>),
    VoteForTrack
}

#[derive(Serialize)]
pub struct OutgoingResult(pub result::Result<Outgoing, String>);

#[derive(Serialize)]
struct OutgoingWrapper {
    id: String,
    #[serde(rename="fn")]
    fnc: String,
    payload: Value
}

impl OutgoingResult {
    pub fn to_string(&self, id: &str, fnc: &str) -> Result<String> {
        let wrapper = OutgoingWrapper {
            id: id.into(),
            fnc: fnc.into(),
            payload: serde_json::to_value(self).unwrap()
        };

        serde_json::to_string(&wrapper)
            .map_err(|err| err.context(ErrorKind::Parsing).into())
    }
}
