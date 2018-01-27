extern crate websocket;
extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_staticfile;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate hex_music;
extern crate failure;
#[macro_use] extern crate failure_derive;
extern crate toml;

mod error;
mod server;
mod proto;
mod state;
mod conf;
mod webserver;

use std::env;
use std::thread;

pub fn main() {
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

    server::start(conf);
}
