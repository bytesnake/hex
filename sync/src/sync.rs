use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};
use std::io::{self, Read, Write, ErrorKind, Error};
use std::net::SocketAddr;
use futures::{Future, Stream, sync::{mpsc, oneshot}};
use super::{Beacon, Discover, Gossip};
use bincode::{deserialize, serialize};
use hex_database::{Collection, Track as DTrack, Playlist as DPlaylist};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub title: Option<String>,
    pub album: Option<String>,
    pub interpret: Option<String>,
    pub people: Option<String>,
    pub composer: Option<String>,
    pub fingerprint: String,
    pub key: String,
    pub duration: f64,
    pub favs_count: u32,
    pub channels: u32 
}

impl Track {
    pub fn from_db(obj: DTrack) -> Track {
        let DTrack { title, album, interpret, people, composer, fingerprint, key, duration, favs_count, channels } = obj;
        
        Track { title, album, interpret, people, composer, fingerprint, key, duration, favs_count, channels }
    }

    pub fn to_db(self) -> DTrack {
        let Track { title, album, interpret, people, composer, fingerprint, key, duration, favs_count, channels } = self;

        DTrack { title, album, interpret, people, composer, fingerprint, key, duration, favs_count, channels }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub key: String,
    pub title: String,
    pub desc: Option<String>,
    pub tracks: Option<String>,
    pub count: u32,
    pub origin: Option<String>
}

impl Playlist {
    pub fn from_db(obj: DPlaylist) -> Playlist {
        let DPlaylist { key, title, desc, tracks, count, origin } = obj;

        Playlist { key, title, desc, tracks, count, origin }
    }

    pub fn to_db(self) -> DPlaylist {
        let Playlist { key, title, desc, tracks, count, origin } = self;

        DPlaylist { key, title, desc, tracks, count, origin }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Protocol {
    Syncing(Option<(Vec<Track>, Vec<Playlist>)>),
    GetTrack(String, Option<Vec<u8>>)
}

pub struct Peer {
    sender: mpsc::Sender<(String, oneshot::Sender<String>)>
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
                .for_each(move |(x, y): (String, oneshot::Sender<String>)| {
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

    fn process_packet(name: String, collection: &Collection, data_path: &PathBuf, buf: Vec<u8>, wait_map: &Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>, sync_all: bool) -> Result<Vec<Protocol>, io::Error> {
        match deserialize::<Protocol>(&buf) {
            Ok(Protocol::Syncing(Some((tracks, playlists)))) => {
                println!("Got sync with {} tracks and {} playlists", tracks.len(), playlists.len());

                let tracks = Self::update_tracks(collection, tracks);
                let nplaylists = Self::update_playlists(name, collection, playlists);

                println!("Updated {} tracks and {} playlists", tracks.len(), nplaylists);

                if sync_all {
                    //println!("SYNC ALL!");

                    Ok(tracks.into_iter().map(|x| Protocol::GetTrack(x, None)).collect())
                } else {
                    Ok(Vec::new())
                }
            },
            Ok(Protocol::Syncing(None)) => {
                let playlists = collection.get_playlists().into_iter().map(|x| Playlist::from_db(x)).collect();

                // weed out non existent tracks
                let existing_tracks = Self::existing_tracks(data_path)?;

                let tracks = collection.get_tracks().into_iter().filter_map(|x| {
                    if existing_tracks.contains_key(&x.key) {
                        Some(Track::from_db(x))
                    } else {
                        None
                    }
                }).collect();
        
                Ok(vec![Protocol::Syncing(Some((tracks, playlists)))])
            },
            Ok(Protocol::GetTrack(key, None)) => {
                if collection.get_track(&key).is_ok() {
                    if let Ok(mut f) = File::open(data_path.join(&key)) {
                        let mut buf = Vec::new();
                        f.read_to_end(&mut buf).unwrap();
        
                        return Ok(vec![Protocol::GetTrack(key, Some(buf))]);
                    }
                }

                Ok(Vec::new())
            },
            Ok(Protocol::GetTrack(key, Some(buf))) => {
                let path = data_path.join(&key);
        
                if !path.exists() && collection.get_track(&key).is_ok() {
                    let mut f = File::create(path).unwrap();
                    f.write(&buf).unwrap();
                }
        
                if let Some(shot) = wait_map.write().unwrap().remove(&key) {
                    shot.send(key.clone())
                        .map_err(|x| Error::new(ErrorKind::BrokenPipe, x))?;
                }

                Ok(Vec::new())
            },
            Err(err) => {
                println!("Could not parse: {:?}", err);

                Ok(Vec::new())
            }
        }

    }

    fn update_tracks(collection: &Collection, tracks: Vec<Track>) -> Vec<String> {
        let map: HashSet<String> = collection.get_tracks()
            .into_iter()
            .map(|x| x.key.clone())
            .collect();

        let mut out = Vec::new();
        for track in tracks {
            if !map.contains(&track.key) {
                out.push(track.key.clone());
                collection.insert_track(track.to_db()).unwrap();
            }
        }

        out
    }

    fn update_playlists(name: String, collection: &Collection, playlists: Vec<Playlist>) -> usize {
        // get all playlists which are from this peer
        let (map_update, map_other): (HashMap<String, Option<String>>, HashMap<String, Option<String>>) = collection.get_playlists().into_iter().map(|x| (x.key, x.origin)).partition(|(_, origin)| {
            if let Some(origin) = origin {
                origin == &name
            } else {
                false
            }
        });

        let mut i = 0;
        for mut playlist in playlists {
            if map_update.contains_key(&playlist.key) {
                let Playlist { key, title, desc, tracks, count, .. } = playlist;

                collection.update_playlist(&key, Some(title), desc, tracks, Some(count), None).unwrap();

                i += 1;
            } else if !map_other.contains_key(&playlist.key) {
                playlist.origin = Some(name.clone());

                collection.insert_playlist(playlist.to_db()).unwrap();

                i += 1;
            }
        }

        i
    }

    fn existing_tracks(data_path: &PathBuf) -> Result<HashMap<String, ()>, io::Error> {
        let res = fs::read_dir(data_path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();

                if path.is_file() {
                    Some((path.file_name().unwrap().to_str().unwrap().into(), ()))
                } else {
                    None
                }
            }).collect();

        Ok(res)
    }

    pub fn ask_for_track(&mut self, key: String) -> oneshot::Receiver<String> {
        let (p, c) = oneshot::channel();

        self.sender.try_send((key, p)).unwrap();

        c
    }
}
