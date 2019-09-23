use std::io::{Read, Write};
use std::fs::File;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::{Future, oneshot, Complete};
use bincode::serialize;
use hex_gossip::{SpreadTo, Spread};

use crate::error::*;
use crate::objects::TrackKey;
use crate::transition::Storage;

pub type Files = Arc<State>;

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
    awaiting: Mutex<HashMap<TrackKey, (usize, Complete<Result<(TrackKey, Vec<u8>)>>)>>,
    spread: Spread<Storage>
}

impl State {
    pub fn new(data_path: PathBuf, spread: Spread<Storage>) -> State {
        State {
            data_path,
            awaiting: Mutex::new(HashMap::new()),
            spread
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

    pub fn process(&self, packet: Packet) -> Option<Packet> {
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
                    if let Some(ref mut elm) = self.awaiting.lock().unwrap().get_mut(&id) {
                        elm.0 -= 1;
                        ct = elm.0;
                    }

                    if ct == 0 {
                        if let Some((_, shot)) = self.awaiting.lock().unwrap().remove(&id) {
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
                        if let Some((_, shot)) = self.awaiting.lock().unwrap().remove(&id) {
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

    pub fn ask_for_file(&self, track_id: TrackKey) -> impl Future<Item = (), Error = Error> {
        let (c, p) = oneshot();

        if self.spread.num_peers() == 0 {
            if let Err(err) = c.send(Err(Error::SyncFailed("No peers available to ask for file".into()))) {
                eprintln!("Send error = {:?}", err);
            }
        } else {
            self.awaiting.lock().unwrap().insert(track_id.clone(), (self.spread.num_peers(), c));

            let buf = serialize(&Packet { id: track_id, body: PacketBody::AskForFile }).unwrap();

            self.spread.spread(hex_gossip::Packet::Other(buf), SpreadTo::Everyone);
            self.spread.flush_all();
        }

        let data_path = self.data_path.clone();
        p
            .then(|res| {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(err)) => Err(err),
                    Err(_) => Err(Error::SyncFailed("Internal channel canceled".into()))
                }
            })
            .and_then(move |(id, buf)| {
                let path = data_path.join(id.to_path());
                if !path.exists() {
                    let mut file = File::create(path).unwrap();

                    if let Err(err) = file.write(&buf) {
                        eprintln!("File write err = {:?}", err);
                    }
                }

                Ok(())
            })
    }
}
