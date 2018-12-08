//! Manage the music database and provide `Track`, `Playlist` and `Token` structs
//!
//! This crate can be used to search, get all playlists, find a certain token and do a lot of other
//! useful stuff. The underlying implementation uses a SQLite database and manages all information
//! with some tables. It is used in the `server` binary, `sync` library and other libraries which
//! have to alter the database in a stable way.
//!
//! ## Example:
//! ```rust,no_run
//! extern crate hex_database;
//! extern crate hex_gossip;
//!
//! use std::path::Path;
//! use hex_database::{Instance, View};
//! use hex_gossip::GossipConf;
//!
//! pub fn main() {
//!     let instance = Instance::from_file("/opt/music/music.db", GossipConf::new());
//!     let view = instance.view();
//!     for playlist in view.get_playlists() {
//!         println!("{:#?}", playlist);
//!     }
//! }
//! ```

#[cfg(feature="rusqlite")]
extern crate rusqlite;
#[cfg(feature="serde")]
#[macro_use]
extern crate serde;
#[cfg(feature="sha2")]
extern crate sha2;
#[cfg(feature="rusqlite")]
extern crate hex_gossip;
#[cfg(feature="rusqlite")]
extern crate bincode;
#[cfg(feature="rusqlite")]
extern crate futures;
#[cfg(feature="rusqlite")]
extern crate tokio;
#[macro_use]
extern crate log;

pub mod error;
pub mod objects;
pub mod search;
pub mod events;
#[cfg(feature="rusqlite")]
mod database;
#[cfg(feature="rusqlite")]
mod transition;

pub use error::{Result, Error};
pub use events::{Action, Event};
pub use objects::{Track, Playlist, Token, TrackKey, PlaylistKey, TokenId};
#[cfg(feature="rusqlite")]
pub use database::*;
#[cfg(feature="hex-gossip")]
pub use hex_gossip::GossipConf;
