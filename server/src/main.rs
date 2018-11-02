//! HTTP and websocket server providing RPC calls to clients
//!
//! This is the main server application. It uses the `database`, `music_container` and `sync` crate
//! to manage the music and provides further routines to upload or download music from the server.
//! The server actually consists of three different server. A HTTP server provides the frontend to
//! clients, the websocket server wraps function calls to the database and parses them and the sync
//! server synchronizes the database between peers. Each has its own port, as set in the
//! configuration, and the HTTP server as well as the sync server are disabled by default. To
//! enable them, they have to be in the configuration file:
//!
//! ```toml
//! host = "127.0.0.1"
//!
//! [webserver]
//! path = "../frontend/build/"
//! port = 8081
//! 
//! [sync]
//! port = 8004
//! name = "Peer"
//! sync_all = true
//! ```
//!
//! and can then be passed as an argument. (e.g. `./target/release/hex_server conf.toml`)

extern crate websocket;
#[macro_use]
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;
extern crate bytes;
extern crate hyper;
extern crate hyper_staticfile;
extern crate http;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate toml;
extern crate curl;
extern crate chromaprint;
extern crate base64;
extern crate tempfile;
extern crate sha2;

extern crate hex_database;
extern crate hex_music_container;
extern crate hex_sync;
extern crate hex_server_protocol;

mod error;
mod conf;
mod webserver;
mod acousticid;
mod convert;
mod server;
mod state;

use std::env;
use std::thread;
use std::path::{Path, PathBuf};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use tokio_core::reactor::Core;

/// Main function spinning up all server
fn main() {
    // check if we got the configuration, otherwise just load the default settings
    let conf = match env::args().skip(1).next().map(|x| PathBuf::from(x)) {
        Some(x) => conf::Conf::from_file(&x).unwrap(),
        None => conf::Conf::default()
    };

    println!("Configuration: {:#?}", conf);

    // start the webserver in a seperate thread if it is mentioned in the configuration
    if let Some(webserver) = conf.webserver.clone() {
        let data_path = conf.music.data_path.clone();
        let addr = SocketAddr::new(conf.host.clone(), webserver.port);
        thread::spawn(move || {
            webserver::create_webserver(addr, webserver.path.clone(), data_path.clone());
        });
    }

    // start the sync server in a seperate thread if it mentioned in the configuration
    if let Some(sync) = conf.sync.clone() {
        let (peer, chain) = hex_sync::Peer::new(
            conf.music.db_path.clone(),
            conf.music.data_path.clone(), 
            SocketAddr::new(conf.host.clone(), sync.port),
            sync.name,
            sync.sync_all
        );

        thread::spawn(|| {
            thread::sleep(Duration::from_millis(300));

            let mut core = Core::new().unwrap();
            core.run(chain);
        });
    }

    // start the websocket server in the main thread
    server::start(conf)
}
