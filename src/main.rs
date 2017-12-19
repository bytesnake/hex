extern crate websocket;
extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate hex_music;

mod server;
mod proto;
mod state;

pub fn main() {
    server::start();
}
