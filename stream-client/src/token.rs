use std::slice;
use rand::{thread_rng, Rng};
use client::{Client, gen_id};

use hex_database::Track;
use hex_server_protocol::{PacketId, RequestAction, AnswerAction, Answer};

pub struct Token {
    token: u32,
    pos: usize,
    sample: u64,
    tracks: Vec<Track>,
    id: PacketId
}

impl Token {
    pub fn new(client: &mut Client, token: u32) -> Option<Token> {
        // ask for the new token
        client.send_once(RequestAction::GetToken { token });
        let Answer {msg, ..} = client.recv();

        match msg {
            Ok(AnswerAction::GetToken((token, Some((_playlist, tracks))))) => {
                if token.played.is_empty() {
                    return Some(Token {
                        token: token.token,
                        pos: 0,
                        sample: 0,
                        tracks: tracks,
                        id: gen_id()
                    });
                }

                let mut played: Vec<Track> = token.played.split(",").filter_map(|x| {
                    tracks.iter().cloned().filter(|y| y.key == x).next()
                }).collect();

                let mut shuffle = false;
                for (a,b) in played.iter().zip(tracks.iter()) {
                    shuffle |= b.key != a.key;
                }

                let pos = played.len() - 1;
                let tracks = match shuffle {
                    false => tracks,
                    true => {
                        let mut rem_tracks: Vec<Track> = tracks.iter().filter(|x| {
                            !played.contains(x)
                        }).cloned().collect();

                        thread_rng().shuffle(&mut rem_tracks);
                        played.append(&mut rem_tracks);
                        
                        played
                    }
                };

                let id = gen_id();
                client.send(id, RequestAction::StreamNext { key: Some(tracks[pos].key.clone()) });
                client.recv();
                //client.send(id, RequestAction::StreamSeek { sample: (token.pos * 48000.0) as u32 });
                //client.recv();

                Some(Token {
                    token: token.token,
                    pos: pos,
                    tracks: tracks,
                    id: id,
                    sample: (token.pos * 48000.0) as u64
                })
            },
            Ok(AnswerAction::GetToken((token, None))) => {
                return Some(Token {
                    token: token.token,
                    pos: 0,
                    sample: 0,
                    tracks: Vec::new(),
                    id: gen_id()
                });
            },
            _ => None
        }
    }

    pub fn create(client: &mut Client) -> u32 {
        client.send_once(RequestAction::CreateToken);
        let Answer {msg, ..} = client.recv();

        match msg {
            Ok(AnswerAction::CreateToken(id)) => id,
            _ => panic!("Could not create a new token")
        }
    }

    pub fn removed(&mut self, client: &mut Client) {
        if self.tracks.len() == self.pos {
            self.tracks.clear();
        } else {
            self.tracks.split_off(self.pos+1);
        }
        let played: Vec<String> = self.tracks.iter().map(|x| x.key.clone()).collect();
        let pos = (self.sample as f64) / 48000.0;

        client.send(self.id, RequestAction::UpdateToken { 
            token: self.token,
            key: None,
            played: Some(played.join(",")),
            pos: Some(pos)
        });
        client.recv();
    }

    pub fn has_tracks(&self) -> bool {
        self.tracks.len() > 0
    }

    pub fn next_packet(&mut self, client: &mut Client) -> Option<&[i16]> {
        let key = self.tracks[self.pos].key.clone();
        client.send(self.id.clone(), RequestAction::StreamNext { key: Some(key) });
        let Answer {msg, ..} = client.recv();

        match msg {
            Ok(AnswerAction::StreamNext(buf)) => {
                let buf = unsafe { 
                    slice::from_raw_parts(
                        buf.as_ptr() as *const i16, 
                        buf.len() / 2
                    )
                };

                self.sample += buf.len() as u64 / 2;
                println!("Acquired new buffer {}", buf.len());
                return Some(buf);
            },
            Err(_) => {
                if self.pos < self.tracks.len() - 1 {
                    println!("End of song!");
                    self.pos += 1;
                    self.id = gen_id();
                } else {
                    println!("End of playlist!");
                    self.pos = 0;
                }
            },
            _ => println!("Got something else")
        }

        None
    }

    pub fn next_track(&mut self) {
        if self.pos < self.tracks.len() - 1 {
            self.pos += 1;
            self.id = gen_id();
        }
    }

    pub fn prev_track(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.id = gen_id();
        }
    }

    pub fn shuffle(&mut self) {
        let mut tracks: Vec<Track> = self.tracks.split_off(self.pos + 1);
        thread_rng().shuffle(&mut tracks);
        self.tracks.append(&mut tracks);
    }

    pub fn upvote(&mut self, client: &mut Client) {
        client.send_once(RequestAction::VoteForTrack {
            key: self.tracks[self.pos].key.clone()
        });
        client.recv();
    }
}

