//! Database elements for all tables

/// A single track with metadata
///
/// In case of no interpret and an original composition the interpret is the same as the composer.
#[derive(Debug, Clone)]
pub struct Track {
    /// The title of the track
    pub title: Option<String>,
    /// The album containing the track
    pub album: Option<String>,
    /// The interpreter of an original composition
    pub interpret: Option<String>,
    /// All people who helped to perform the track
    pub people: Option<String>,
    /// The original composer
    pub composer: Option<String>,
    /// A unique fingerprint describing the content of the track
    pub fingerprint: String,
    /// A unique key used to access the track
    pub key: String,
    /// Duration in milliseconds
    pub duration: f64,
    /// Number of favs
    pub favs_count: u32,
    /// Number of channels in the track
    pub channels: u32 
}

impl Track {
    /// Create an empty track with no metadata
    pub fn empty(fingerprint: String, key: String, duration: f64, channels: u32) -> Track {
        Track {
            title: None,
            album: None,
            interpret: None,
            people: None,
            composer: None,
            fingerprint: fingerprint,
            key: key,
            duration: duration,
            favs_count: 0,
            channels: channels
        }
    }
}

/// A single playlist containing many tracks
#[derive(Debug)]
pub struct Playlist {
    /// A unique key used to access the playlist
    pub key: String,
    /// The playlist's title
    pub title: String,
    /// A description of the playlist, can be a longer text
    pub desc: Option<String>,
    /// All containing tracks in 'a,b,c,...' format
    pub tracks: Option<String>,
    /// Number of tracks in `tracks` field
    pub count: u32,
    /// In case the playlist originates from an outside server and should be updated by the `sync`
    /// crate
    pub origin: Option<String>
}

/// A single token connecting a token to a playlist
#[derive(Debug)]
pub struct Token {
    /// Token number saved on the cardridge
    pub token: u32,
    /// Key of the playlist
    pub key: String,
    /// All played song (ignored by shuffle)
    pub played: String,
    /// Position of the actual song
    pub pos: f64
}
