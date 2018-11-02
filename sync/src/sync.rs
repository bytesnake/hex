use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};
use std::io::{self, Read, Write, ErrorKind, Error};
use std::net::SocketAddr;
use futures::{Future, Stream, sync::{mpsc, oneshot}};
use super::{Beacon, Discover, Gossip};
use bincode::{deserialize, serialize};
use hex_database::{Collection, Track, Playlist, TrackKey, PlaylistKey};

#[derive(Serialize, Deserialize, Debug)]
enum Protocol {
    Syncing(Option<(Vec<Track>, Vec<Playlist>)>),
    GetTrack(TrackKey, Option<Vec<u8>>)
}

pub struct Peer {
    sender: mpsc::Sender<(TrackKey, oneshot::Sender<TrackKey>)>
}

impl Peer {
    pub fn new(db_path: PathBuf, data_path: PathBuf, addr: SocketAddr, name: String, sync_all: bool) -> (Peer, impl Future<Item=(), Error=()> ){
        if !data_path.exists() {
            println!("Data path does not exists .. creating");
            fs::create_dir_all(&data_path).unwrap();
        }

        let (sender, receiver) = mpsc::channel(1024);

        let chain = Self::probe_for_peers(addr, name).and_then(move |(discover, gossip)| {
            let wait_map = Arc::new(RwLock::new(HashMap::new()));
            let wait_map_clone = wait_map.clone();

            let writer = gossip.writer();
            let receiver = receiver
                .map_err(|_| Error::new(ErrorKind::BrokenPipe, "Receiver broken"))
                .for_each(move |(x, y): (TrackKey, oneshot::Sender<TrackKey>)| {
                    serialize(&Protocol::GetTrack(x.clone(), None))
                        .map_err(|x| Error::new(ErrorKind::InvalidData, x))
                        .and_then(|x| writer.push(x))?;

                    wait_map_clone.write().unwrap().insert(x, y);

                    Ok(())
                });

            let collection = Collection::from_file(&db_path);
            let writer = gossip.writer();

            let gossip = gossip.for_each(move |(id, buf)| {
                let packets = match buf.len() {
                    0 => vec![Protocol::Syncing(None)],
                    _ => Self::process_packet(id.clone(), &collection, &data_path, buf, &wait_map, sync_all)?
                };

                for packet in packets {
                    serialize(&packet)
                        .map_err(|x| Error::new(ErrorKind::InvalidData, x))
                        .and_then(|buf| writer.write(&id, buf))?;
                }

                Ok(())
            });

            let discover = discover.for_each(move |_| Ok(()));

            Future::join(discover, Future::join(gossip, receiver)).map(|_| ())
        }).map_err(|err| println!("Got error in sync: {}", err));

        (Peer { sender: sender }, chain)
    }

    fn probe_for_peers(addr: SocketAddr, name: String) -> impl Future<Item=(Discover, Gossip), Error=io::Error> {
        Beacon::new(1, 500).and_then(move |x| {
            match x {
                Some(addr) => println!(" discovered at {:?}", addr),
                None => println!(" nobody at all!")
            }
        
            let discover = Discover::new(1);
            let gossip = Gossip::new(addr, x, name);
    
            Ok((discover, gossip))
        })
    }

    fn process_packet(name: String, collection: &Collection, data_path: &PathBuf, buf: Vec<u8>, wait_map: &Arc<RwLock<HashMap<TrackKey, oneshot::Sender<TrackKey>>>>, sync_all: bool) -> Result<Vec<Protocol>, io::Error> {
        match deserialize::<Protocol>(&buf) {
            Ok(Protocol::Syncing(Some((tracks, playlists)))) => {
                let ntracks = Self::update_tracks(collection, tracks.clone());
                let nplaylists = Self::update_playlists(name.clone(), collection, playlists);

                println!("Updated {} tracks and {} playlists from '{}'", ntracks, nplaylists, name);

                if sync_all {
                    let map = Self::existing_tracks(data_path).unwrap();

                    Ok(tracks.into_iter().filter_map(|x| {
                        if !map.contains_key(&x.key) {
                            Some(Protocol::GetTrack(x.key, None))
                        } else {
                            None
                        }
                    }).collect())
                } else {
                    Ok(Vec::new())
                }
            },
            Ok(Protocol::Syncing(None)) => {
                let playlists = collection.get_playlists();

                // weed out non existent tracks
                let existing_tracks = Self::existing_tracks(data_path)?;

                let tracks = collection.get_tracks().into_iter().filter_map(|x| {
                    if existing_tracks.contains_key(&x.key) {
                        Some(x)
                    } else {
                        None
                    }
                }).collect();
        
                Ok(vec![Protocol::Syncing(Some((tracks, playlists)))])
            },
            Ok(Protocol::GetTrack(key, None)) => {
                if collection.get_track(key).is_ok() {
                    if let Ok(mut f) = File::open(data_path.join(key.to_path())) {
                        let mut buf = Vec::new();
                        f.read_to_end(&mut buf).unwrap();
        
                        return Ok(vec![Protocol::GetTrack(key, Some(buf))]);
                    }
                }

                Ok(Vec::new())
            },
            Ok(Protocol::GetTrack(key, Some(buf))) => {
                let path = data_path.join(key.to_path());
        
                if !path.exists() && collection.get_track(key).is_ok() {
                    let mut f = File::create(path).unwrap();
                    f.write(&buf).unwrap();
                }
        
                if let Some(shot) = wait_map.write().unwrap().remove(&key) {
                    shot.send(key.clone())
                        .map_err(|x| Error::new(ErrorKind::BrokenPipe, format!("Could not get track: {}", x)))?;
                }

                Ok(Vec::new())
            },
            Err(err) => {
                println!("Could not parse: {:?}", err);

                Ok(Vec::new())
            }
        }

    }

    fn update_tracks(collection: &Collection, tracks: Vec<Track>) -> usize {
        let map: HashSet<TrackKey> = collection.get_tracks()
            .into_iter()
            .map(|x| x.key)
            .collect();

        let mut i = 0;
        for track in tracks {
            if !map.contains(&track.key) {
                collection.insert_track(track).unwrap();
                i += 1;
            }
        }

        i
    }

    fn update_playlists(name: String, collection: &Collection, playlists: Vec<Playlist>) -> usize {
        // get all playlists which are from this peer
        let (map_update, map_other): (HashMap<PlaylistKey, Option<String>>, HashMap<PlaylistKey, Option<String>>) = collection.get_playlists().into_iter().map(|x| (x.key, x.origin)).partition(|(_, origin)| {
            if let Some(origin) = origin {
                origin == &name
            } else {
                false
            }
        });

        let mut i = 0;
        for mut playlist in playlists {
            if map_update.contains_key(&playlist.key) {
                let Playlist { key, title, desc, tracks, .. } = playlist;

                collection.update_playlist(key, Some(title), desc, None).unwrap();

                i += 1;
            } else if !map_other.contains_key(&playlist.key) {
                playlist.origin = Some(name.clone());

                collection.insert_playlist(playlist).unwrap();

                i += 1;
            }
        }

        i
    }

    fn existing_tracks(data_path: &PathBuf) -> Result<HashMap<TrackKey, ()>, io::Error> {
        let res = fs::read_dir(data_path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();

                if path.is_file() {
                    let key = TrackKey::from_str(path.file_name().unwrap().to_str().unwrap());
                    Some((key, ()))
                } else {
                    None
                }
            }).collect();

        Ok(res)
    }

    pub fn ask_for_track(&mut self, key: TrackKey) -> oneshot::Receiver<TrackKey> {
        let (p, c) = oneshot::channel();

        self.sender.try_send((key, p)).unwrap();

        c
    }
}
