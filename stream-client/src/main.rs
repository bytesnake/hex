extern crate mfrc522;
extern crate sysfs_gpio;
extern crate spidev;
extern crate cpal;
extern crate rb;
extern crate websocket;
extern crate rand;

extern crate hex_database;
extern crate hex_server_protocol;

mod audio;
mod events;
mod client;
mod token;

use client::Client;
use events::Event;        

fn main() {
    let (events, push_new) = events::events();
    let mut client = Client::new();
    let mut audio = audio::AudioDevice::new();

    let mut token: Option<token::Token> = None;
    let mut create_counter = 0;
    loop {
        if let Ok(events) = events.try_recv() {
            for event in events {
                match event {
                    Event::ButtonPressed(x) => {
                        if let Some(ref mut token) = token {
                            match x {
                                3 => {audio.clear(); token.next_track()},
                                1 => {audio.clear(); token.prev_track()},
                                0 => {create_counter += 1; token.shuffle()},
                                2 => token.upvote(&mut client),
                                _ => println!("Not supported yet!")
                            }
                        } else {
                            create_counter = 0;
                        }
                    },
                    Event::NewCard(num) => {
                        println!("Got card with number {}", num);

                        if let Some(new_token) = token::Token::new(&mut client, num) {
                            token = Some(new_token);
                        } else {
                            push_new.send(token::Token::create(&mut client)).unwrap();
                        }
                    },
                    Event::CardLost => {
                        if let Some(ref mut token) = token {
                            token.removed(&mut client);
                        }
                        
                        token = None;
                        
                        audio.clear();
                    }
                }
            }
        }

        /*if create_counter == 10 {
            1
        }*/

        if let Some(ref mut token) = token {
            if token.has_tracks() {
                if let Some(packet) = token.next_packet(&mut client) {
                    audio.buffer(packet);
                }
            }
        }
    }
}

