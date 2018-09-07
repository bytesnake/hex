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
extern crate rand;

mod audio;
mod events;
mod client;
mod token;

use std::slice;
use std::thread;
use std::time::Duration;
use client::{Client, Outgoing, Token, Incoming};
use events::Event;        

fn main() {
    let events = events::events();
    let mut client = Client::new();
    let mut audio = audio::AudioDevice::new();

    let mut token: Option<token::Token> = None;
    loop {
        if let Ok(events) = events.try_recv() {
            for event in events {
                match event {
                    Event::ButtonPressed(x) => {
                        if let Some(ref mut token) = token {
                            match x {
                                3 => token.next_track(),
                                1 => token.prev_track(),
                                0 => token.shuffle(),
                                2 => token.upvote(&mut client),
                                _ => println!("Not supported yet!")
                            }
                        }
                    },
                    Event::NewCard(num) => token = Some(token::Token::new(&mut client, 0)),
                    Event::CardLost => {
                        if let Some(ref mut token) = token {
                            token.removed(&mut client);
                        }
                        
                        token = None
                    }
                }
            }
        }

        if let Some(ref mut token) = token {
            if let Some(packet) = token.next_packet(&mut client) {
                audio.buffer(packet);
            }
        }
    }
}

