use std::result;
use bincode;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Bincode(bincode::Error)
}
