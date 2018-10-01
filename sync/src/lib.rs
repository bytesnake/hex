extern crate tokio;                                                                           
#[macro_use]
extern crate futures;
extern crate bytes;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

pub mod gossip;
pub mod discover;
pub mod local_ip;

pub use discover::{Discover, Beacon};
pub use gossip::Gossip;
