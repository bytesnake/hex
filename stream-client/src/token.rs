use std::slice;
use uuid::Uuid;
use rand::{thread_rng, Rng};
use client::{Client, Track, Incoming, Outgoing};

pub struct Token {
    pos: usize,
    tracks: Vec<Track>,
    id: Uuid
}

impl Token {
    pub fn new(client: &mut Client, token: u32) -> Token {
        client.send_once(Outgoing::GetToken { token: 0 });
        match client.recv() {
            Ok(Incoming::GetToken((token, playlist, tracks))) => {
                return Token {
                    pos: token.pos,
                    tracks: tracks,
                    id: Uuid::new_v4()
                };
            },
            _ => panic!("Invalid token!")
        }
    }

    pub fn next_packet(&mut self, client: &mut Client) -> Option<&[i16]> {
        let key = self.tracks[self.pos].key.clone();
        client.send(self.id.clone(), Outgoing::StreamNext { key: Some(key) });
        match client.recv() {
            Ok(Incoming::Buffer(buf)) => {
                let buf = unsafe { 
                    slice::from_raw_parts(
                        buf.as_ptr() as *const i16, 
                        buf.len() / 2
                    )
                };

                println!("Acquired new buffer {}", buf.len());
                return Some(buf);
            },
            Err(EndOfStream) => {
                if self.pos < self.tracks.len() - 1 {
                    println!("End of song!");
                    self.pos += 1;
                    self.id = Uuid::new_v4();
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
            self.id = Uuid::new_v4();
        }
    }

    pub fn prev_track(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.id = Uuid::new_v4();
        }
    }

    pub fn shuffle(&mut self) {
        let mut tracks = self.tracks.split_off(self.pos + 1);
        thread_rng().shuffle(&mut tracks);
        self.tracks.append(&mut tracks);
    }

    pub fn upvote(&mut self, client: &mut Client) {
        client.send_once(Outgoing::VoteForTrack {
            key: self.tracks[self.pos].key.clone()
        });
        client.recv();
    }
}

