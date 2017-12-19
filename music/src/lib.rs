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
}
