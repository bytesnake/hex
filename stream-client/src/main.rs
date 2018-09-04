extern crate mfrc522;
extern crate sysfs_gpio;
extern crate spidev;

mod audio;
mod events;
mod client;

fn main() {
    let events = events::events();
}

