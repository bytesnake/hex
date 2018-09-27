use std::{io, result};
use hex_database;
use hex_music_container;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MusicContainer(hex_music_container::error::Error),
    Database(hex_database::Error),
    Io(io::Error),
    Configuration,
    AcousticID,
    AcousticIDResponse(String),
    AcousticIDMetadata,
    ConvertFFMPEG,
    ConvertYoutube,
    Parsing,
    ChannelFailed
}
