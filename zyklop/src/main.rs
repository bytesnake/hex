extern crate mfrc522;
extern crate sysfs_gpio;
extern crate spidev;
extern crate cpal;
extern crate rb;
extern crate rand;

extern crate hex_database;
extern crate hex_music_container;

mod error;
mod audio;
mod events;
mod token;

use std::env;
use std::path::PathBuf;
use events::Event;        

use error::Error;

use hex_database::{Instance, Token, GossipConf};

fn main() {
    let path = env::args().skip(1).next()
        .map(|x| PathBuf::from(&x))
        .filter(|x| x.exists())
        .ok_or(Error::InvalidPath).expect("Could not find path!");

    let data_path = path.join("data");
    let instance = Instance::from_file(&path.join("music.db"), GossipConf::new());
    let view = instance.view();

    let (events, push_new) = events::events();
    let mut audio = audio::AudioDevice::new();

    let mut token: Option<token::Current> = None;
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
                                2 => {
                                    if let Err(err) = view.vote_for_track(token.track_key().unwrap()) {
                                        eprintln!("Error: Could not vote for track {:?}: {:?}", token.track_key(), err);
                                    }
                                },
                                x => eprintln!("Error: Input {} not supported yet", x)
                            }
                        } else {
                            create_counter = 0;
                        }
                    },
                    Event::NewCard(num) => {
                        println!("Got card with number {}", num);

                        match view.get_token(num as i64) {
                            Ok((a, Some((_, b)))) => token = Some(token::Current::new(a, b, data_path.clone())),
                            Ok((a, None)) => token = Some(token::Current::new(a, Vec::new(), data_path.clone())),
                            Err(hex_database::Error::NotFound) => {
                                println!("Not found!");
                                let id = view.last_token_id().unwrap() + 1;
                                let token = Token {
                                    token: id,
                                    key: None,
                                    played: Vec::new(),
                                    pos: None,
                                    last_use: 0
                                };

                                view.add_token(token).expect("Error: Could not create a new token!");

                                push_new.send(id as u32).unwrap();
                            },
                            Err(err) => eprintln!("Error: Could not get token with error: {:?}", err)
                        }
                    },
                    Event::CardLost => {
                        if let Some(ref mut token) = token {
                            let Token { token, played, pos, .. } = token.data();

                            if let Err(err) = view.update_token(token, None, Some(played), pos) {
                                eprintln!("Error: Could not update token {:?} because {:?}", token, err);
                            }
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
                println!("Next buffer!");
                if let Some(packet) = token.next_packet() {
                    audio.buffer(&packet);
                }
            }
        }
    }
}

