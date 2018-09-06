extern crate mfrc522;
extern crate sysfs_gpio;
extern crate spidev;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate cpal;
extern crate rb;
extern crate websocket;

mod audio;
mod events;
mod client;

fn main() {
    let events = events::events();
}

