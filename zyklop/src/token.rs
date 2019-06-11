use std::fs::File;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;
use std::path::{Path, PathBuf};
use rand::{thread_rng, Rng};

use hex_database::{Track, Token, TrackKey};
use hex_music_container::{Container, Configuration};

use crate::error::{Error, Result};

pub struct Stream {
    pub track: Track,
    container: Container<File>
}

impl Stream {
    pub fn new(track: Track, data_path: &Path) -> Result<Stream> {
        let path = data_path.join(track.key.to_path());
        
        while !path.exists() {
            println!("Wait for file ..");
            thread::sleep(Duration::from_millis(500));
        }

        let file = File::open(data_path.join(track.key.to_path()))
            .map_err(|_| Error::NotAvailable)?;
        
        let container = Container::load(file)
            .map_err(|err| Error::MusicContainer(err))?;

        Ok(Stream {
            track, container
        })
    }

    pub fn next(&mut self) -> Result<Vec<i16>> {
        self.container.next_packet(Configuration::Stereo)
            .map_err(|err| Error::MusicContainer(err))
    }

    pub fn track(&self) -> Track {
        self.track.clone()
    }

    pub fn goto(&mut self, pos: f64) {
        self.container.seek_to_sample((48000.0 * pos) as u32);
    }
}

pub struct Current {
    pub stream: Option<Stream>,
    pub token: Token,
    not_played: Vec<Track>,
    played: Vec<Track>,
    data_path: PathBuf,
    sender: Sender<TrackKey>
}

impl Current {
    pub fn new(mut token: Token, mut tracks: Vec<Track>, data_path: PathBuf, sender: Sender<TrackKey>) -> Current {
        let current_track_key = token.played.pop();
        let current_track = current_track_key.and_then(|track_key| {
            tracks.iter().position(|x| x.key == track_key)
            .map(|index| tracks.remove(index))
        });

        let (played, not_played): (Vec<Track>, Vec<Track>) = tracks.iter().cloned().partition(|x| {
            token.played.contains(&x.key)
        });
        
        let stream = match current_track {
            Some(track) => {
                let mut stream = Stream::new(track, &data_path).unwrap();

                println!("Load current track: {:?}", token.pos);

                if let Some(pos) = token.pos {
                    stream.goto(pos);
                }

                Some(stream)
            },
            None => None
        };

        Current {
            stream,
            token,
            played,
            not_played,
            data_path,
            sender
        }
    }

    pub fn data(&self) -> Token {
        self.token.clone()
    }

    pub fn track(&self) -> Option<Track> {
        match self.stream {
            Some(ref stream) => Some(stream.track()),
            None => None
        }
    }

    pub fn track_key(&self) -> Option<TrackKey> {
        self.stream.as_ref().map(|x| x.track.key)
    }

    pub fn has_tracks(&self) -> bool {
        !self.not_played.is_empty() || !self.played.is_empty() || self.stream.is_some()
    }

    pub fn next_packet(&mut self) -> Option<Vec<i16>> {
        if self.stream.is_none() {
            self.next_track();
        }

        let mut remove = false;
        if let Some(ref mut stream) = self.stream {
            match stream.next() {
                Ok(buf) => {
                    if let Some(ref mut pos) = self.token.pos {
                        *pos += buf.len() as f64 / 2.0 / 48000.0;
                    } else {
                        self.token.pos = Some(buf.len() as f64 / 2.0 / 48000.0);
                    }
            
                    return Some(buf);
                },
                Err(Error::MusicContainer(hex_music_container::error::Error::ReachedEnd)) => {
                    println!("Reached end!");

                    remove = true;
                },
                Err(err) => { eprintln!("Error: {:?}", err); }
            }
        }

        if remove {
            self.stream = None;
        }

        return self.next_packet();
    }

    pub fn create_stream(&self, elm: Track, path: &Path) -> Option<Stream> {
        // TODO acquire file if not existing
        Some(Stream::new(elm, path).unwrap())
    }

    pub fn next_track(&mut self) {
        // if all tracks are played, begin again
        if self.not_played.is_empty() {
            self.not_played = self.played.clone();
        }


        // push the last track to played
        if let Some(ref stream) = self.stream {
            self.played.push(stream.track());
        }

        // if there is still a track left start to stream it
        if self.not_played.len() > 0 {
            // ask for the next three tracks
            for key in self.not_played.iter().take(2).map(|x| x.key.clone()) {
                self.sender.send(key).unwrap();
            }

            let elm = self.not_played.remove(0);
            self.stream = self.create_stream(elm, &self.data_path);
            self.token.pos = Some(0.0);
        }
    }

    pub fn prev_track(&mut self) {
        // save the current track as not played
        if let Some(ref stream) = self.stream {
            self.not_played.insert(0, stream.track());
        }

        // if there are no tracks left, play the first not_played
        if self.played.is_empty() {
            let elm = self.not_played.remove(0);
            self.stream = self.create_stream(elm, &self.data_path);
            self.token.pos = Some(0.0);
        } else {
            let elm = self.played.pop().unwrap();
            self.stream = self.create_stream(elm, &self.data_path);
            self.token.pos = Some(0.0);
        }
    }

    pub fn shuffle(&mut self) {
        thread_rng().shuffle(&mut self.not_played);
    }
}

