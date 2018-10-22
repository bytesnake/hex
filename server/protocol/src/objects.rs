//! Protocol of the websocket server
//!
//! The protocol uses JSON as coding and a request/answer id for every packet to know where to put
//! the answer. 

use std::result;
use error::{Error, Result};

use bincode::{serialize, deserialize};

use hex_database::{Track, Playlist, Token, Event};

/// Identification of a packet
///
/// A request should contain a random number associating it with the pending answer.
pub type PacketId = [u32; 4];
/// Incoming message
///
/// The incoming message is wrapped in a packet struct containing the `id` field. Any `fn` field is
/// in snake_case formatting and can contain more parameters. 
#[derive(Debug)]
#[cfg_attr(any(feature="server", target_arch = "wasm32"), derive(Deserialize))]
#[cfg_attr(feature="client", derive(Serialize))]
pub enum RequestAction {
    /// Search for tracks with a query
    Search {
        query: String
    },
    /// Get a single track with a key
    GetTrack {
        key: String
    },
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
    /// Delete a track from a playlist
    DeleteFromPlaylist {
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
        format: String,
        data: Vec<u8>
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
#[derive(Debug)]
#[cfg_attr(feature="server", derive(Deserialize))]
#[cfg_attr(feature="client", derive(Serialize))]
pub struct Request {
    pub id: PacketId,
    pub msg: RequestAction
}

impl Request {
    pub fn new(id: PacketId, msg: RequestAction) -> Request {
        Request { id, msg }
    }

    #[cfg(feature="server")]
    pub fn try_from(buf: &[u8]) -> Result<Request> {
        deserialize(buf).map_err(|err| Error::Bincode(err))
    }
    
    #[cfg(feature="client")]
    pub fn to_buf(&self) -> Result<Vec<u8>> {
        serialize(self).map_err(|err| Error::Bincode(err))
    }
}

/// Outgoing packets
#[derive(Debug)]
#[cfg_attr(feature="client", derive(Deserialize))]
#[cfg_attr(any(feature="server", target_arch = "wasm32"), derive(Serialize))]
pub enum AnswerAction {
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
    StreamNext(Vec<u8>),
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
    DeleteFromPlaylist,
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

#[derive(Debug)]
#[cfg_attr(feature="client", derive(Deserialize))]
#[cfg_attr(feature="server", derive(Serialize))]
pub struct Answer {
    pub id: PacketId,
    pub msg: result::Result<AnswerAction, String>
}

impl Answer {
    pub fn new(id: PacketId, msg: result::Result<AnswerAction, String>) -> Answer {
        Answer {
            id, 
            msg
        }
    }

    #[cfg(feature="client")]
    pub fn try_from(buf: &[u8]) -> Result<Answer> {
        deserialize(buf).map_err(|err| Error::Bincode(err))
    }
    
    #[cfg(feature="server")]
    pub fn to_buf(&self) -> Result<Vec<u8>> {
        serialize(self).map_err(|err| Error::Bincode(err))
    }
}

#[derive(Debug)]
#[cfg_attr(feature="client", derive(Deserialize))]
#[cfg_attr(any(feature="server", target_arch = "wasm32"), derive(Serialize))]
pub struct UploadProgress {
    pub desc: String,
    pub kind: String,
    pub progress: f32,
    pub id: PacketId,
    pub key: Option<String>
}

#[derive(Debug, Clone)]
#[cfg_attr(feature="client", derive(Deserialize))]
#[cfg_attr(any(feature="server", target_arch = "wasm32"), derive(Serialize))]
pub struct DownloadProgress {
    pub id: PacketId,
    pub format: String,
    pub progress: f32,
    pub download: Option<String>
}

impl DownloadProgress {
    pub fn empty() -> DownloadProgress {
        DownloadProgress {
            id: [0,0,0,0],
            format: String::new(),
            progress: 0.0,
            download: None
        }
    }
}
