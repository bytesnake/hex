//! Collecting error types

use std::{io, result};
use hex_database;
use hex_music_container;
use hex_server_protocol;

/// Our custom `Result` using the `Error` struct
pub type Result<T> = result::Result<T, Error>;

/// Error crate composing errors from music_container, database, io, acousticID
#[derive(Debug)]
pub enum Error {
    /// Error originating from the music_container
    MusicContainer(hex_music_container::error::Error),
    /// Error originating from the database
    Database(hex_database::Error),
    /// Protocol error
    Protocol(hex_server_protocol::Error),
    /// Input/Output error in Rust std
    Io(io::Error),
    /// Configuration error, most of the time wrong format
    Configuration,
    /// AcousticID error, e.g. could not generate fingerprint
    AcousticID,
    /// Wrong response wrong the acousticID server
    AcousticIDResponse(String),
    /// Wrong metadata section in the answer
    AcousticIDMetadata,
    /// Could not convert with FFMPEG
    ConvertFFMPEG,
    /// Could not download with youtube-dl
    ConvertYoutube,
    /// Parsing failed
    Parsing,
    /// Channel failed
    ChannelFailed
}
