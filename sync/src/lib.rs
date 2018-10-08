//! Synchronize a music database with other peers in a local network. This crate implements 
//! peer discovery in an unknown network as well as peer-to-peer communication. On top of 
//! that a synchronize protocol replicates the database and manages all data. In the _partial_ mode
//! only the database is fully replicated and single audio files has to be requested in order to be
//! playable. This is for example useful in an Android application which allows to carry certain
//! playlists with you. In the _full_ mode everything is pulled in, useful in server applications.
//!
//! # Example
//! ```
//! // create a new peer with database path, data path, peer address, sync_everything
//! let (peer, chain) = Peer::new(
//!     Path::new("/opt/music/music.db"),
//!     Path::new("/opt/music/data/"),
//!     "127.0.0.1:8000".parse::<SocketAddr>(),
//!     false
//! );
//!
//! // start the peer in a seperate thread
//! thread::spawn(|| tokio::run(chain));
//!
//! // ask for a certain track to be available
//! peer.ask_for_track("<track_id>").wait();
//! ```
extern crate tokio;                                                                           
#[macro_use]
extern crate futures;
extern crate bytes;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate hex_database;

pub mod gossip;
pub mod discover;
pub mod local_ip;
pub mod sync;

pub use discover::{Discover, Beacon};
pub use gossip::Gossip;
pub use sync::Peer;
