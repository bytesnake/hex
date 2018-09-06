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

use client::{Client, Outgoing, Token};
        
fn main() {
    let events = events::events();
    let mut client = Client::new();

    let token = Token::with_playlist(0, "437f4559d21445e99a238daa217c3448");
    let a = Outgoing::InsertToken { token: token };

    client.send(uuid::Uuid::new_v4(), a);

    //println!("{:?}", Incoming::deserialize("(\"stream_next\", Ok(StreamNext))".into()));
}

