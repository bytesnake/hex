//! Database elements for all tables

#[cfg(feature="rusqlite")]
use rusqlite::Row;
#[cfg(feature="rusqlite")]
use rusqlite::Result;
use std::{mem, fmt};
use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// Track identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct TrackKey([u8; 16]);

impl TrackKey {
    pub fn from_vec(buf: &[u8]) -> TrackKey {
        assert_eq!(buf.len(), 16);

        let mut key = TrackKey([0u8; 16]);
        key.0.copy_from_slice(buf);

        key
    }

    pub fn from_str(buf: &str) -> TrackKey {
        assert_eq!(buf.len(), 32);

        let mut key = TrackKey([0u8; 16]);

        for i in 0..16 {
            key.0[i] = u8::from_str_radix(&buf[i*2..i*2+1], 16).unwrap();
        }

        key
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_string(&self) -> String {
        let mut tmp = String::new();
        for i in 0..16 {
            tmp.push_str(&format!("{:02X}", (self.0)[i]));
        }

        tmp
    }

    pub fn to_path(&self) -> PathBuf {
        PathBuf::from(&self.to_string())
    }
}

impl fmt::Display for TrackKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub type Fingerprint = Vec<i32>;


/// A single track with metadata
///
/// In case of no interpret and an original composition the interpret is the same as the composer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct Track {
    /// A unique key used to access the track (hash of the fingerprint)
    pub key: TrackKey,
    /// A unique fingerprint describing the content of the track
    pub fingerprint: Fingerprint,
    /// The title of the track
    pub title: Option<String>,
    /// The album containing the track
    pub album: Option<String>,
    /// The interpret
    pub interpret: Option<String>,
    /// All people who helped to perform the track
    pub people: Option<String>,
    /// The original composer
    pub composer: Option<String>,
    /// Duration in milliseconds
    pub duration: f64,
    /// Number of favs
    pub favs_count: u32
}

impl Track {
    /// Create an empty track with no metadata
    pub fn empty(fingerprint: Fingerprint, duration: f64) -> Track {
        let mut hasher = Sha256::new();                                                           
        let v_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts( 
                fingerprint.as_ptr() as *const u8,
                fingerprint.len() * std::mem::size_of::<i32>(),
            )       
        };      
        
        hasher.input(v_bytes);
        let a = hasher.result();
        let key = TrackKey::from_vec(&a[0..16]);

        Track {
            key: key,
            fingerprint: fingerprint,
            title: None,
            album: None,
            interpret: None,
            people: None,
            composer: None,
            duration: duration,
            favs_count: 0
        }
    }

    /// Create a track from database row
    #[cfg(feature = "rusqlite")]
    pub fn from_row(row: &Row) -> Result<Track> {
        let key: Vec<u8> = row.get_checked(0)?;

        Ok(Track {
            key:        TrackKey::from_vec(&key),
            fingerprint:u8_into_i32(row.get_checked(1)?),
            title:      row.get_checked(2)?,
            album:      row.get_checked(3)?,
            interpret:  row.get_checked(4)?,
            people:     row.get_checked(5)?,
            composer:   row.get_checked(6)?,
            duration:   row.get_checked(7)?,
            favs_count: row.get_checked(8)?
        })
    }
}

/// Playlist identification
pub type PlaylistKey = i64;

/// A single playlist containing many tracks
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct Playlist {
    /// A unique key used to access the playlist
    pub key: PlaylistKey,
    /// The playlist's title
    pub title: String,
    /// A description of the playlist, can be a longer text
    pub desc: Option<String>,
    /// Vector of all track keys
    pub tracks: Vec<TrackKey>,
    /// In case the playlist originates from an outside server and should be updated by the `sync`
    /// crate
    pub origin: Option<String>
}

#[cfg(feature = "rusqlite")]
impl Playlist {
    pub fn from_row(row: &Row) -> Result<Playlist> {
        let keys: Vec<u8> = row.get_checked(3)?;
        let keys: Vec<TrackKey> = keys.chunks(16)
            .map(|x| TrackKey::from_vec(x)).collect();

        Ok(Playlist {
            key:    row.get_checked(0)?,
            title:  row.get_checked(1)?,
            desc:   row.get_checked(2)?,
            tracks: keys,
            origin: row.get_checked(4)?
        })
    }
}
/// Token identification
pub type TokenId = i64;

/// A single token connecting a token to a playlist
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct Token {
    /// Token number saved on the cardridge
    pub token: TokenId,
    /// Key of the playlist
    pub key: Option<PlaylistKey>,
    /// All played song (ignored by shuffle)
    pub played: Vec<TrackKey>,
    /// Position of the actual song
    pub pos: Option<f64>,
    /// Change counter (shared between all peers)
    pub counter: u32,
    /// Last time the token was updates (local version)
    pub last_update: String
}

#[cfg(feature = "rusqlite")]
impl Token {
    pub fn from_row(row: &Row) -> Result<Token> {
        let keys: Vec<u8> = row.get_checked(2)?;
        let keys: Vec<TrackKey> = keys.chunks(16)
            .map(|x| TrackKey::from_vec(x)).collect();

        Ok(Token {
            token:          row.get_checked(0)?,
            key:            row.get_checked(1)?,
            played:         keys,
            pos:            row.get_checked(3)?,
            counter:        row.get_checked(4)?,
            last_update:    row.get_checked(5)?
        })
    }
}
pub fn i32_into_u8(mut buf: Vec<i32>) -> Vec<u8> {
    unsafe {
        let ratio = 4;

        let length = buf.len() * ratio;
        let capacity = buf.capacity() * ratio;
        let ptr = buf.as_mut_ptr() as *mut u8;

        // Don't run the destructor for vec32
        mem::forget(buf);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    }
}
pub fn u8_into_i32(mut buf: Vec<u8>) -> Vec<i32> {
    unsafe {
        let ratio = 4;

        let length = buf.len() / ratio;
        let capacity = buf.capacity() / ratio;
        let ptr = buf.as_mut_ptr() as *mut i32;

        // Don't run the destructor for vec32
        mem::forget(buf);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    }
}
pub fn i64_into_u8(mut buf: Vec<i64>) -> Vec<u8> {
    unsafe {
        let ratio = 8;

        let length = buf.len() * ratio;
        let capacity = buf.capacity() * ratio;
        let ptr = buf.as_mut_ptr() as *mut u8;

        // Don't run the destructor for vec32
        mem::forget(buf);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    }
}
pub fn u8_into_i64(mut buf: Vec<u8>) -> Vec<i64> {
    unsafe {
        let ratio = 8;

        let length = buf.len() / ratio;
        let capacity = buf.capacity() / ratio;
        let ptr = buf.as_mut_ptr() as *mut i64;

        // Don't run the destructor for vec32
        mem::forget(buf);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    }
}

