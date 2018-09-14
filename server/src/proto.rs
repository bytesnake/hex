use serde_json::{self, Value};
use error::{Error, Result};
use std::result;
use convert::UploadProgress;

use hex_database;

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub title: Option<String>,
    pub album: Option<String>,
    pub interpret: Option<String>,
    pub people: Option<String>,
    pub composer: Option<String>,
    pub key: String,
    pub duration: f64,
    pub favs_count: u32,
}

impl Track {
    pub fn from_db_obj(obj: hex_database::Track) -> Track {
        Track {
            title: obj.title,
            album: obj.album,
            interpret: obj.interpret,
            people: obj.people,
            composer: obj.composer,
            key: obj.key,
            duration: obj.duration,
            favs_count: obj.favs_count,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub key: String,
    pub title: String,
    pub desc: Option<String>,
    pub count: u32
}

impl Playlist {
    pub fn from_db_obj(obj: hex_database::Playlist) -> Playlist {
        Playlist {
            key: obj.key,
            title: obj.title,
            desc: obj.desc,
            count: obj.count
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub token: u32,
    pub key: String,
    pub played: String,
    pub pos: f64
}

impl Token {
    pub fn from_db_obj(obj: hex_database::Token) -> Token {
        Token {
            token: obj.token,
            key: obj.key,
            played: obj.played,
            pos: obj.pos
        }
    }

    pub fn to_db_obj(self) -> hex_database::Token {
        hex_database::Token {
            token: self.token,
            key: self.key,
            played: self.played,
            pos: self.pos
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Event {
    origin: String,
    action: Action
}

#[derive(Debug, Serialize)]
pub enum Action {
    Connect(f32),
    PlaySong(String),
    AddSong(String),
    DeleteSong(String)
}

impl Action {
    pub fn from_db_obj(obj: hex_database::Action) -> Action {
        match obj {
            hex_database::Action::Connect(x) => Action::Connect(x),
            hex_database::Action::PlaySong(x) => Action::PlaySong(x),
            hex_database::Action::AddSong(x) => Action::AddSong(x),
            hex_database::Action::DeleteSong(x) => Action::DeleteSong(x)
        }
    }
}

impl Event {
    pub fn from_db_obj(obj: hex_database::Event) -> Event {
        Event {
            origin: obj.origin(),
            action: Action::from_db_obj(obj.action())
        }
    }
}

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
    #[serde(rename="stream_next")]
    StreamNext {
        key: Option<String>
    },
    #[serde(rename="stream_end")]
    StreamEnd,
    #[serde(rename="stream_seek")]
    StreamSeek {
        sample: u32
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
    #[serde(rename="upload_youtube")]
    UploadYoutube {
        path: String
    },
    #[serde(rename="upload_track")]
    UploadTrack {
        name: String,
        format: String
    },
    #[serde(rename="vote_for_track")]
    VoteForTrack {
        key: String
    },
    #[serde(rename="ask_upload_progress")]
    AskUploadProgress,
    #[serde(rename="get_token")]
    GetToken {
        token: u32
    },
    #[serde(rename="insert_token")]
    InsertToken {
        token: Token
    },
    #[serde(rename="update_token")]
    UpdateToken {
        token: u32,
        played: String,
        pos: f64
    },
    #[serde(rename="get_summarise")]
    GetSummarise,
    #[serde(rename="get_events")]
    GetEvents
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
    StreamNext,
    StreamSeek {
        sample: u32
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
    UploadYoutube,
    UploadTrack,
    VoteForTrack,
    AskUploadProgress(Vec<UploadProgress>),
    GetToken((Token, Playlist, Vec<Track>)),
    InsertToken,
    UpdateToken,
    GetSummarise(Vec<(String, u32, u32, u32, u32)>),
    GetEvents(Vec<(String, Event)>)
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
            .map_err(|_| Error::Parsing)
    }
}
