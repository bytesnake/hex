use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("configuration file not found in {0}")]
    ConfMissing(PathBuf, #[source] io::Error),
    #[error("generic IO error")]
    Io(#[from] io::Error),
    #[error("parsing TOML file failed")]
    TomlParsing(#[from] toml::de::Error),
    #[error("generating TOML file failed")]
    TomlGen(#[from] toml::ser::Error),
    #[error("could not find playlist with name {0}")]
    PlaylistNotFound(String),
    #[error("music file not found with name {0}")]
    SongNotFound(String),
    #[error("binary {0} missing")]
    BinaryMissing(String),
    #[error("mplayer exited with stderr={0}")]
    MplayerFailed(String),
    #[error("reached end of playlist")]
    ReachedEndOfPlaylist,
    #[error("reached beginning of playlist")]
    ReachedBeginningOfPlaylist,
    #[error("randomized playing is disallowed")]
    RandomNotAllowed,
}
