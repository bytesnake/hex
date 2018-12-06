use std::result;
use std::io;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NotEnoughBytes,
    Cryptography,
    Deserialize,
    WrongVersion,
    Io(io::Error)
}
