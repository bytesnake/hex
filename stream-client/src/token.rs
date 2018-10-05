use std::slice;
use uuid::Uuid;
use rand::{thread_rng, Rng};
use client::{Client, Track, Incoming, Outgoing};

pub struct Token {
    token: u32,
    pos: usize,
    sample: u64,
    tracks: Vec<Track>,
    id: Uuid
}

impl Token {
    pub fn new(client: &mut Client, token: u32) -> Option<Token> {
        client.send_once(Outgoing::GetToken { token });
        match client.recv() {
            Ok(Incoming::GetToken((token, Some((playlist, tracks))))) => {
                if token.played.is_empty() {
                    return Some(Token {
                        token: token.token,
                        pos: 0,
                        sample: 0,
                        tracks: tracks,
                        id: Uuid::new_v4()
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

                let id = Uuid::new_v4();
                client.send(id, Outgoing::StreamNext { key: Some(tracks[pos].key.clone()) });
                client.recv();
                //client.send(id, Outgoing::StreamSeek { sample: (token.pos * 48000.0) as u32 });
                //client.recv();

                Some(Token {
                    token: token.token,
                    pos: pos,
                    tracks: tracks,
                    id: id,
                    sample: (token.pos * 48000.0) as u64
                })
            },
            Ok(Incoming::GetToken((token, None))) => {
                return Some(Token {
                    token: token.token,
                    pos: 0,
                    sample: 0,
                    tracks: Vec::new(),
                    id: Uuid::new_v4()
                });
            },
            _ => None
        }
    }

    pub fn create(client: &mut Client) -> u32 {
        client.send_once(Outgoing::CreateToken);
        if let Ok(Incoming::CreateToken(id)) = client.recv() {
            id
        } else {
            panic!("Got wrong return!");
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

        client.send(self.id, Outgoing::UpdateToken { 
            token: self.token,
            played: played.join(","), 
            pos: pos 
        });
        client.recv();
    }

    pub fn has_tracks(&self) -> bool {
        self.tracks.len() > 0
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

                self.sample += buf.len() as u64 / 2;
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

