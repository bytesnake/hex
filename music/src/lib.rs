#![feature(conservative_impl_trait)]

extern crate rusqlite;
extern crate chromaprint;
extern crate curl;
extern crate opus;
extern crate hound;
#[macro_use] extern crate log;
extern crate simple_logger;
extern crate uuid;
#[macro_use] extern crate failure;
#[macro_use] extern crate failure_derive;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;
extern crate futures;
extern crate bytes;

pub mod music_search;
pub mod database;
pub mod acousticid;
pub mod audio_file;
pub mod error;
pub mod ffmpeg;
pub mod opus_conv;

use std::env;
use std::io::Read;
use std::fs::File;
use std::fs;
use std::mem;
use std::path::PathBuf;

use database::{Playlist, Track};
use error::{Result, ErrorKind};
use failure::ResultExt;

pub struct Collection {
    socket: database::Connection,
    data_path: String
}

impl Collection {
    pub fn new(db_path: String, data_path: String) -> Collection {
        Collection {
            socket: database::Connection::open_file(&db_path),
            data_path: data_path
        }
    }

    pub fn search(&self, query: &str, start: usize) -> Result<Vec<database::Track>> {
        let query = music_search::SearchQuery::new(query).ok_or(format_err!("Invalid query: {}", query))?;

        let mut stmt = self.socket.search_prep(query)?;
        let res = self.socket.search(&mut stmt).skip(start).take(50).collect();

        Ok(res)
    }

    pub fn add_track(&self, format: &str, data: &[u8]) -> Result<database::Track> {
        let track = audio_file::AudioFile::new(data, format)?.to_db()?;

        self.socket.insert_track(track.clone())?;

        Ok(track)
    }

    pub fn get_track(&self, key: &str) -> Result<database::Track> {
        self.socket.get_track(key)
    }

    pub fn update_track(&self, key: &str, title: Option<String>, album: Option<String>, interpret: Option<String>, people: Option<String>, composer: Option<String>) -> Result<String> {
        self.socket.update_track(key, title, album, interpret, people, composer)
    }

    pub fn get_suggestion(&self, key: &str) -> Result<String> {
        let track = self.socket.get_track(key)?;

        track.suggestion()
    }

    /// Create a new stream with track
    pub fn stream_start(&self, key: &str) -> Result<File> {
        /*let mut path = env::home_dir().ok_or(format_err!("Invalid home path!"))?;
        path.push(".music");
        path.push(key);*/
        let mut path = PathBuf::new();
        path.push(&self.data_path);
        path.push(key);

        Ok(File::open(path).context(ErrorKind::Database)?)
    }

    /// Get the next opus package
    pub fn stream_next(&self, file: &mut File) -> Vec<u8> {
        let mut data = Vec::new();

        loop {
            //let mut data = vec![0u8; 2400];
            let mut len_buf = [0u8; 4];
            let nlength = file.read(&mut len_buf).unwrap();

            if nlength < 4 {
                return data;
            }

            let len = unsafe { mem::transmute::<[u8; 4], u32>(len_buf).to_be() };

            //println!("Read packet with length {}", len);

            let mut tmp = vec![0; len as usize];


            let nread = file.read(&mut tmp).unwrap();
            //data.truncate(nread);

            data.extend_from_slice(&len_buf);
            data.extend_from_slice(&tmp[0..nread]);

            //println!("Length: {}", data.len());

            if data.len() > 2048 {
                break;
            }
        }

        data
        
    }

    /// Goto in a certain position in the file
    pub fn stream_seek(&self, pos: f64, track: &Track, file: &mut File) -> f64 {
        0.0
    }

    pub fn get_playlists(&self) -> Vec<Playlist> {
        self.socket.get_playlists()
    }

    pub fn add_playlist(&self, name: &str) -> Result<Playlist> {
        self.socket.add_playlist(name)
    }

    pub fn delete_playlist(&self, key: &str) -> Result<()> {
        self.socket.delete_playlist(key)
    }

    pub fn update_playlist(&self, key: &str, title: Option<String>, desc: Option<String>) -> Result<()> {
        self.socket.update_playlist(key, title, desc)
    }

    pub fn add_to_playlist(&self, key: &str, playlist: &str) -> Result<Playlist> {
        self.socket.add_to_playlist(key, playlist)
    }

    pub fn get_playlist(&self, key: &str) -> Result<(Playlist, Vec<Track>)> {
        self.socket.get_playlist(key)
    }

    pub fn get_playlists_of_track(&self, key: &str) -> Result<Vec<Playlist>> {

        self.socket.get_playlists_of_track(key)
    }

    pub fn delete_track(&self, key: &str) -> Result<()> {
        let mut path = PathBuf::new();
        path.push(&self.data_path);
        path.push(key);
        /*let mut path = env::home_dir().ok_or(()).unwrap();
        path.push(".music");
        path.push(key);*/

        fs::remove_file(path);

        self.socket.delete_track(key)
    }

    pub fn vote_for_track(&self, key: &str) -> Result<()> {
        self.socket.vote_for_track(key)
    }
}
