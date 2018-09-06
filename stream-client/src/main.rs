extern crate mfrc522;
extern crate sysfs_gpio;
extern crate spidev;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate cpal;
extern crate rb;
extern crate websocket;
extern crate uuid;

mod audio;
mod events;
mod client;

use client::{Client, Outgoing};
        
fn main() {
    let events = events::events();
    let mut client = Client::new();

    let a = Outgoing::StreamNext { key: None };

    client.send(uuid::Uuid::new_v4(), a);

    //println!("{:?}", Incoming::deserialize("(\"stream_next\", Ok(StreamNext))".into()));
}

