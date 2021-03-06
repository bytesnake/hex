mod error;
mod audio;
mod events;
//mod events_term;
mod token;

use std::path::PathBuf;

use events::Event;
use hex_database::{Instance, Token, GossipConf};

fn main() {
    env_logger::init();

    let (conf, path) = match hex_conf::Conf::new() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error: Could not load configuration {:?}", err);
            (hex_conf::Conf::default(), PathBuf::from("/opt/music/"))
        }
    };
    let data_path = path.join("data");
    let db_path = path.join("music.db");

    let mut gossip = GossipConf::new();
    
    if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port));
        gossip = gossip.id(peer.id());
        gossip = gossip.network_key(peer.network_key());
    }

    let instance = Instance::from_file(&db_path, gossip);
    let (read, write) = (instance.reader(), instance.writer());

    //let (sender, receiver): (Sender<TrackKey>, Receiver<TrackKey>) = channel();

    let (events, push_new) = events::events();
    let mut audio = audio::AudioDevice::new();

    let mut token: Option<token::Current> = None;
    let mut create_counter = 0;
    loop {
        if let Ok(events) = events.try_recv() {
            println!("Got events {:?}", events);

            for event in events {
                match event {
                    Event::ButtonPressed(x) => {
                        if let Some(ref mut token) = token {
                            match x {
                                3 => {audio.clear(); token.next_track()},
                                1 => {audio.clear(); token.prev_track()},
                                0 => {create_counter += 1; token.shuffle()},
                                2 => {
                                    if let Some(ref stream) = token.stream {
                                        if let Err(err) = write.vote_for_track(stream.track.key) {
                                            eprintln!("Error: Could not vote for track {:?}: {:?}", token.track_key(), err);
                                        }
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

                        match read.get_token(num as i64) {
                            Ok((a, Some((_, b)))) => {
                                token = Some(token::Current::new(a, b, instance.files(), data_path.clone()));
                            },
                            Ok((a, None)) => {
                                token = Some(token::Current::new(a, Vec::new(), instance.files(), data_path.clone()));
                            },
                            Err(hex_database::Error::NotFound) => {
                                println!("Not found!");
                                let id = read.last_token_id().unwrap() + 1;
                                let token = Token {
                                    token: id,
                                    key: None,
                                    played: Vec::new(),
                                    pos: None,
                                    last_use: 0
                                };

                                write.add_token(token).expect("Error: Could not create a new token!");

                                push_new.send(id as u32).unwrap();
                            },
                            Err(err) => eprintln!("Error: Could not get token with error: {:?}", err)
                        }
                        if let Some(ref token) = token {
                            if let Err(err) = write.use_token(token.token.token) {
                                eprintln!("Error: Could not user token {:?} because {:?}", token.token.token, err);
                            }
                        }
                    },
                    Event::CardLost => {
                        if let Some(ref mut token) = token {
                            let current_track = token.track();
                            let Token { token, mut played, pos, .. } = token.data();

                            // push current track as last element of played tracks
                            if let Some(current_track) = current_track {
                                if !played.contains(&current_track.key) {
                                    played.push(current_track.key);
                                }
                            }

                            if let Err(err) = write.update_token(token, None, Some(played), pos) {
                                eprintln!("Error: Could not update token {:?} because {:?}", token, err);
                            }

                        }
                        
                        token = None;
                        
                        audio.clear();
                    }
                }
            }
        }

        if create_counter == 3 {
            println!("Reset token to new id ..");
            let id = read.last_token_id().unwrap() + 1;
            let token = Token {
                token: id,
                key: None,
                played: Vec::new(),
                pos: None,
                last_use: 0
            };

            write.add_token(token).expect("Error: Could not create a new token!");

            push_new.send(id as u32).unwrap();

            create_counter = 0;
        }

        if let Some(ref mut token) = token {
            if token.has_tracks() {
                if let Some(packet) = token.next_packet() {
                    audio.buffer(&packet);
                }
            } else {
            }
        }
    }
}

