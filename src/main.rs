extern crate websocket;
extern crate futures;
extern crate tokio_core;
extern crate serde_json;

mod server;
mod protocol;

pub fn main() {
    server::start();
}
