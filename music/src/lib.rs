#![feature(conservative_impl_trait)]

extern crate websocket;
extern crate futures;
extern crate tokio_core;
extern crate rusqlite;
extern crate chromaprint;
extern crate curl;
extern crate opus;
extern crate hound;
#[macro_use] extern crate log;
extern crate simple_logger;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;

pub mod music_search;
pub mod database;
pub mod acousticid;
pub mod audio_file;
pub mod error;

pub struct Music {
    conn: database::Connection
}
