use std::result;
use hex_music_container;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    //Database(hex_database::Error),
    MusicContainer(hex_music_container::error::Error),
    NotAvailable
}
