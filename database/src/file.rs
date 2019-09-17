use std::fs::File;
use std::path::PathBuf;
use std::io::Read;
use std::collections::HashMap;
use futures::Complete;

use crate::error::*;
use crate::objects::TrackKey;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Packet {
    pub id: TrackKey,
    pub body: PacketBody
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PacketBody {
    AskForFile,
    HasFile(bool),
    GetFile(Option<Vec<u8>>)
}

pub struct State {
    data_path: PathBuf,
    awaiting: HashMap<TrackKey, (usize, Complete<Result<(TrackKey, Vec<u8>)>>)>
}

impl State {
    pub fn new(data_path: PathBuf) -> State {
        State {
            data_path,
            awaiting: HashMap::new()
        }
    }

    fn get_file(&self, id: TrackKey) -> Option<Vec<u8>> {
        let mut file = File::open(self.data_path.join(&id.to_string())).ok()?;
        let mut content = vec![];
        file.read_to_end(&mut content).unwrap();

        Some(content)
    }

    fn has_file(&self, id: TrackKey) -> bool {
        self.data_path.join(&id.to_string()).exists()
    }

    pub fn process(&mut self, packet: Packet) -> Option<Packet> {
        let Packet { id, body } = packet;

        let inner = match body {
            PacketBody::AskForFile => {
                Some(PacketBody::HasFile(self.has_file(id)))
            },
            PacketBody::HasFile(has_file) => {

                if has_file {
                    if !self.has_file(id) {
                        Some(PacketBody::GetFile(None))
                    } else {
                        None
                    }
                } else {
                    let mut ct = 0;
                    if let Some(ref mut elm) = self.awaiting.get_mut(&id) {
                        elm.0 -= 1;
                        ct = elm.0;
                    }

                    if ct == 0 {
                        if let Some((_, shot)) = self.awaiting.remove(&id) {
                            if let Err(err) = shot.send(Err(Error::SyncFailed("No peer had file available".into()))) {
                                eprintln!("Oneshot err = {:?}", err);
                            }

                        }
                    }

                    None
                }
            },
            PacketBody::GetFile(file) => {
                match file {
                    None => {
                        Some(PacketBody::GetFile(self.get_file(id)))
                    },
                    Some(data) => {
                        if let Some((_, shot)) = self.awaiting.remove(&id) {
                            if let Err(err) = shot.send(Ok((id, data))) {
                                eprintln!("Oneshot err = {:?}", err);
                            }

                        }

                        None
                    }
                }
            }
        };

        inner.map(|x| Packet { id: id, body: x })
    }

    pub fn add(&mut self, track_key: TrackKey, num_peers: usize, c: Complete<Result<(TrackKey, Vec<u8>)>>) {
        self.awaiting.insert(track_key, (num_peers, c));
    }

    pub fn remove(&mut self, track_key: TrackKey) {
        self.awaiting.remove(&track_key);
    }
}
