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
extern crate hex_sync;

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
use std::path::{Path, PathBuf};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use tokio_core::reactor::Core;

fn main() {
    // check if we got the configuration, otherwise just load the default settings
    let conf = match env::args().skip(1).next() {
        Some(x) => conf::Conf::from_file(&x).unwrap(),
        None => conf::Conf::default()
    };

    println!("Configuration: {:#?}", conf);

    if let Some(webserver) = conf.webserver.clone() {
        let data_path = conf.music.data_path.clone();
        let addr = SocketAddr::new(conf.host.clone(), webserver.port);
        thread::spawn(move || {
            webserver::create_webserver(addr, &webserver.path, &data_path);
        });
    }

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

    server::start(conf)
}
