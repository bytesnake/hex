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

#[macro_use]
extern crate log;
extern crate websocket;
#[macro_use]
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_codec;
extern crate tokio_process;
extern crate bytes;
extern crate hyper;
extern crate hyper_staticfile;
extern crate http;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate curl;
extern crate base64;
extern crate tempfile;

extern crate hex_conf;
extern crate hex_database;
extern crate hex_music_container;
extern crate hex_server_protocol;

mod error;
mod webserver;
mod acousticid;
mod convert;
mod server;
mod state;

use std::thread;
use std::path::PathBuf;
use std::net::SocketAddr;

/// Main function spinning up all server
fn main() {
    let (conf, path) = match hex_conf::Conf::new() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error: Could not load configuration {:?}", err);
            (hex_conf::Conf::default(), PathBuf::from("/opt/music/"))
        }
    };

    println!("Configuration: {:#?}", conf);

    // start the webserver in a seperate thread if it is mentioned in the configuration
    if let Some(webserver) = conf.webserver.clone() {
        let data_path = path.join("data");
        let addr = SocketAddr::new(conf.host.clone(), webserver.port);
        thread::spawn(move || {
            webserver::create_webserver(addr, webserver.path.clone(), data_path.clone());
        });
    }

    // start the websocket server in the main thread
    server::start(conf, path)
}
