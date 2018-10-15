//! Manage the music database and provide `Track`, `Playlist` and `Token` structs
//!
//! This crate can be used to search, get all playlists, find a certain token and do a lot of other
//! useful stuff. The underlying implementation uses a SQLite database and manages all information
//! with some tables. It is used in the `server` binary, `sync` library and other libraries which
//! have to alter the database in a stable way.
//!
//! ## Example:
//! ```rust
//! let collection = Collection::new(Path::new("/opt/music/music.db"));
//! for playlist in collection.get_playlists() {
//!     println!("{:#?}", playlist);
//! }
//! ```

#[cfg(feature="rusqlite")]
extern crate rusqlite;
extern crate uuid;
#[cfg(feature="serde")]
#[macro_use]
extern crate serde;

pub mod objects;
pub mod search;
pub mod events;
#[cfg(feature="rusqlite")]
mod database;

pub use events::{Action, Event};
pub use objects::{Track, Playlist, Token};
#[cfg(feature="rusqlite")]
pub use rusqlite::{Result, Statement, Error};
#[cfg(feature="rusqlite")]
pub use database::*;
