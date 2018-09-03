extern crate websocket;
extern crate termion;
extern crate tui;
extern crate rb;
extern crate cpal;

extern crate serde;
#[macro_use]
extern crate serde_derive;

mod audio;
mod control;
mod client;

fn main() {
    let mut audio_device = audio::AudioDevice::new();
    let format = audio_device.format();

    println!("{:?}", audio_device.format());

    let mut tui = control::TextInterface::new();

    let client = client::Client::new(tui.sender(), audio_device);

    tui.run(client.sender());

    //control::display();

    /*loop {
        audio_device.buffer(&vec![0i16; format.channels as usize]);
    }*/
    println!("Hello, world!");
}
