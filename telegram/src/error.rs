use std::result;
use std::io;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MusicContainer(hex_music_container::error::Error),
    Database(hex_database::Error),
    Io(io::Error),
    ChannelFailed,
    AcousticID
}
