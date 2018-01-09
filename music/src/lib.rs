#![feature(conservative_impl_trait)]

extern crate rusqlite;
extern crate chromaprint;
extern crate curl;
extern crate opus;
extern crate hound;
#[macro_use] extern crate log;
extern crate simple_logger;
extern crate uuid;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;

pub mod music_search;
pub mod database;
pub mod acousticid;
pub mod audio_file;
pub mod error;

use std::env;
use std::io::Read;
use std::fs::File;

use database::Track;

pub struct Collection {
    socket: database::Connection
}

impl Collection {
    pub fn new() -> Collection {
        Collection {
            socket: database::Connection::new()
        }
    }

    pub fn search(&self, query: &str, start: usize) -> Vec<database::Track> {
        let query = music_search::SearchQuery::new(query).unwrap();

        let mut stmt = self.socket.search_prep(query);
        let res = self.socket.search(&mut stmt).skip(start).take(50).collect();

        res
    }

    pub fn add_track(&self, format: &str, data: &[u8]) -> database::Track {
        let track = audio_file::AudioFile::new(data, format).unwrap().to_db().unwrap();

        self.socket.insert_track(track.clone());

        track
    }

    pub fn get_track(&self, key: &str) -> Result<database::Track, ()> {
        self.socket.get_track(key).map_err(|_| ())
    }

    pub fn update_track(&self, key: &str, title: Option<String>, album: Option<String>, interpret: Option<String>, conductor: Option<String>, composer: Option<String>) -> Result<String, ()> {
        self.socket.update_track(key, title, album, interpret, conductor, composer).map_err(|_| ())
    }

    pub fn get_suggestion(&self, key: &str) -> Result<String, ()> {
        let track = self.socket.get_track(key).map_err(|_| ())?;

        track.suggestion().map_err(|_| ())
    }

    /// Create a new stream with track
    pub fn stream_start(&self, key: &str) -> Result<File, ()> {
        let mut path = env::home_dir().ok_or(())?;
        path.push(".music");
        path.push(key);

        File::open(path).map_err(|_| ())
    }

    /// Get the next opus package
    pub fn stream_next(&self, file: &mut File) -> Vec<u8> {
        let mut data = vec![0u8; 1024];

        let nread = file.read(&mut data).unwrap();
        data.truncate(nread);

        data
        
    }

    /// Goto in a certain position in the file
    pub fn stream_seek(&self, pos: f64, track: &Track, file: &mut File) -> f64 {
        0.0
    }
}
