//! Protocol of the websocket server
//!
//! The protocol uses JSON as coding and a request/answer id for every packet to know where to put
//! the answer. 

use serde_json::{self, Value};
use error::{Error, Result};
use std::result;
use convert::{DownloadProgress, UploadProgress};

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

/// Incoming message
///
/// The incoming message is wrapped in a packet struct containing the `id` field. Any `fn` field is
/// in snake_case formatting and can contain more parameters. 
#[derive(Deserialize)]
#[serde(tag = "fn")]
#[serde(rename_all = "snake_case")]
pub enum Incoming {
    /// Search for tracks with a query
    Search {
        query: String
    },
    /// Get a single track with a key
    GetTrack {
        key: String
    },
    /// Clear internal byte buffer
    ClearBuffer,
    /// Get the next packet in a stream (`key` has to be available in first call)
    StreamNext {
        key: Option<String>
    },
    /// End a stream
    StreamEnd,
    /// Seek in a stream forward
    StreamSeek {
        sample: u32
    },
    /// Update possible fields in a track
    UpdateTrack {
        key: String,
        title: Option<String>,
        album: Option<String>,
        interpret: Option<String>,
        people: Option<String>,
        composer: Option<String>
    },
    /// Get suggestions for a track from acousticid
    GetSuggestion {
        key: String
    },
    /// Create a new playlist
    AddPlaylist {
        name: String
    },
    /// Delete a playlist
    DeletePlaylist {
        key: String
    },
    /// Set a playlist image
    SetPlaylistImage {
        key: String
    },
    /// Add a track to a playlist
    AddToPlaylist {
        key: String,
        playlist: String
    },
    /// Update metadata of a playlist
    UpdatePlaylist {
        key: String,
        title: Option<String>,
        desc: Option<String>
    },
    /// Get all playlists
    GetPlaylists,
    /// Get a single playlist with key
    GetPlaylist {
        key: String
    },
    /// Get all playlists of a track
    GetPlaylistsOfTrack {
        key: String
    },
    /// Delete a track
    DeleteTrack {
        key: String
    },
    /// Start upload from a youtube music video
    UploadYoutube {
        path: String
    },
    /// Start upload of a track saved in the internal buffer
    UploadTrack {
        name: String,
        format: String
    },
    /// Vote for a track
    VoteForTrack {
        key: String
    },
    /// Ask the upload progress
    AskUploadProgress,
    /// Get a token
    GetToken {
        token: u32
    },
    /// Update the metadata of a token
    UpdateToken {
        token: u32,
        key: Option<String>,
        played: Option<String>,
        pos: Option<f64>
    },
    /// Create a new token
    CreateToken,
    /// Get the last inserted token
    LastToken,
    /// Get the summarise for all days
    GetSummarise,
    /// Get all events
    GetEvents,
    /// Start download a bunch of tracks
    Download {
        format: String,
        tracks: Vec<String>
    },
    /// Ask for the download progress
    AskDownloadProgress
}

/// Wrapper for the Incoming message
///
/// This struct supplements the protocol with an identification. It can be useful to match the
/// answer to the request, or to have stateful calls. For example the `search` query should return
/// just a bunch of tracks each time executed, but has to remember which were already transmitted.
#[derive(Deserialize)]
pub struct IncomingWrapper {
    pub id: String,
    pub msg: Incoming
}

/// Outgoing packets
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Outgoing {
    /// The result of a single search
    SearchResult {
        /// Searched query
        query: String,
        /// Slice of answers
        answ: Vec<Track>,
        /// Are there more tracks available? (repeated call)
        more: bool
    },
    /// Response to a `GetTrack` call
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
    GetToken((Token, Option<(Playlist, Vec<Track>)>)),
    UpdateToken,
    CreateToken(u32),
    LastToken(Option<u32>),
    GetSummarise(Vec<(String, u32, u32, u32, u32)>),
    GetEvents(Vec<(String, Event)>),
    Download,
    AskDownloadProgress(Vec<DownloadProgress>)
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
