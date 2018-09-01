use std::{result, io};
use opus;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    File(io::Error),
    CorruptedFile,
    Opus(opus::Error),
    InvalidSize,
    InvalidRange,
    NotSupported,
    SendFailed,
    ReachedEnd
}
