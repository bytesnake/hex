extern crate websocket;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;
extern crate bytes;
extern crate hyper;
extern crate hyper_staticfile;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate toml;
extern crate curl;
extern crate chromaprint;
extern crate uuid;

extern crate hex_database;
extern crate hex_music_container;

mod error;
mod conf;
mod webserver;
mod acousticid;
mod convert;
mod server;
mod proto;
mod state;

use std::env;
use std::thread;

fn main() {
    // check if we got the configuration, otherwise just load the default settings
    let conf = match env::args().skip(1).next() {
        Some(x) => conf::Conf::from_file(&x).unwrap(),
        None => conf::Conf::default()
    };

    println!("Configuration: {:#?}", conf);

    if let Some(webserver) = conf.webserver.clone() {
        thread::spawn(move || {
            webserver::create_webserver(&webserver.host, webserver.port);
        });
    }

    server::start(conf)
}
