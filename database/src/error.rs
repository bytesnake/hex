use std::result;
#[cfg(feature = "rusqlite")]
use hex_gossip;
#[cfg(feature = "rusqlite")]
use rusqlite;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "rusqlite")]
    Sqlite(rusqlite::Error),
    #[cfg(feature = "rusqlite")]
    Gossip(hex_gossip::Error),
    AlreadyExists,
    NotFound,
    ReadOnly,
    AcousticId
}
