use std::io::Write;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use futures::{Sink, Stream, Future, IntoFuture, oneshot, Complete};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio::prelude::FutureExt;
use rusqlite::{self, Statement, OpenFlags};
use tokio;
use std::time::Duration;
use std::thread;

use crate::error::{Error, Result};
use crate::search::SearchQuery;
use crate::objects::*;

use hex_gossip::{Gossip, PeerId, GossipConf, Spread, Transition, Inspector, Discover, Packet, SpreadTo};
use crate::transition::{Storage, TransitionAction, transition_from_sql};

type Awaiting = Arc<Mutex<HashMap<TrackKey, Complete<()>>>>;
/// Instance of the database
pub struct Instance {
    gossip: Option<(Spread<Storage>, PeerId, Sender<TransitionAction>)>,
    storage: Option<(Arc<Mutex<Storage>>, PeerId, Sender<TransitionAction>)>,
    path: PathBuf,
    receiver: Option<Receiver<TransitionAction>>,
    awaiting: Awaiting
}

impl Instance {
    pub fn from_file<T: AsRef<Path>>(path: T, conf: GossipConf) -> Instance {
        let path = path.as_ref();
        let data_path = PathBuf::from(path.parent().unwrap());

        // try to create the database, if not existing
        {
            let socket = rusqlite::Connection::open(path).unwrap();
    
            // create the necessary tables (if not already existing)
            socket.execute_batch(include_str!("create_db.sql")).unwrap();
        }

        let awaiting: Awaiting = Arc::new(Mutex::new(HashMap::new()));
        if let Some(id) = conf.id.clone() {
            let (sender, receiver) = channel(1024);

            if conf.addr.is_some() {
                let storage = Storage::new(path);
                let gossip = Gossip::new(conf, storage);
                let writer = gossip.writer();
                let my_sender = sender.clone();
                let tmp_awaiting = awaiting.clone();
                let (network, addr) = (gossip.network(), gossip.addr());

                let gossip = gossip
                    .map_err(|e| eprintln!("Gossip err: {}", e))
                    .and_then(move |x| {
                        trace!("Got a new transition!");

                        match x {
                            Packet::Push(x) => {
                                let action = TransitionAction::from_vec(&x.body.unwrap());

                                let tmp = sender.clone();
                                if let Err(err) = tmp.send(action).wait() {
                                    eprintln!("Sender err = {:?}", err);
                                }
                            },
                            Packet::File(id, data) => {
                                if let Some(data) = data {
                                    let id = TrackKey::from_vec(&id);

                                    if let Some(shot) = tmp_awaiting.lock().unwrap().remove(&id) {
                                        let path = data_path.join(id.to_path());

                                        if !path.exists() {
                                            let mut file = File::create(path).unwrap();

                                            if let Err(err) = file.write(&data) {
                                                eprintln!("File write err = {:?}", err);
                                            }
                                        }

                                        if let Err(err) = shot.send(()) {
                                            eprintln!("Oneshot err = {:?}", err);
                                        }

                                    }
                                }
                            },
                            _ => {
                            }
                        }

                        Ok(())

                    })
                    .for_each(|_| Ok(())).into_future();

                let discover = Discover::new(1, network, addr.port())
                    .map_err(|e| eprintln!("Discover err = {:?}", e))
                    .for_each(|_| Ok(())).into_future();

                let spread = writer.get();

                thread::spawn(move || {
                    tokio::run(Future::join3(gossip, discover, spread).map(|_| ()));
                });

                Instance { gossip: Some((writer, id, my_sender)), storage: None, path: path.to_path_buf(), receiver: Some(receiver), awaiting }
            } else {
                let storage = Storage::new(path);
                Instance { gossip: None, storage: Some((Arc::new(Mutex::new(storage)), id, sender)), path: path.to_path_buf(), receiver: Some(receiver), awaiting }
            }
        } else {
            Instance { gossip: None, storage: None, path: path.to_path_buf(), receiver: None, awaiting }
        }
    }

    pub fn recv(&mut self) -> Receiver<TransitionAction> {
        self.receiver.take().unwrap()
    }


    pub fn view(&self) -> View {
        let socket = rusqlite::Connection::open_with_flags(&self.path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();

        match (&self.gossip, &self.storage) {
            (Some((ref writer, ref peer_id, ref sender)), None) => {
                View { 
                    socket, 
                    writer: Some(writer.clone()), 
                    peer_id: Some(peer_id.clone()), 
                    storage: None,
                    sender: Some(sender.clone()),
                    awaiting: self.awaiting.clone()
                }
            },
            (None, Some((ref storage, ref peer_id, ref sender))) => {
                View { socket, writer: None, peer_id: Some(peer_id.clone()), storage: Some(storage.clone()), sender: Some(sender.clone()), awaiting: self.awaiting.clone() }
            },
            _ => {
                View { socket, writer: None, peer_id: None, storage: None, sender: None, awaiting: self.awaiting.clone() }
            }
        }
    }
}

/// Represents an open connection to a database
pub struct View {
    socket: rusqlite::Connection,
    peer_id: Option<PeerId>,
    writer: Option<Spread<Storage>>,
    storage: Option<Arc<Mutex<Storage>>>,
    sender: Option<Sender<TransitionAction>>,
    awaiting: Awaiting
}

impl View {
    pub fn commit(&self, transition: TransitionAction) -> Result<()> {
        trace!("Commit new transition {:?}", transition);

        match (&self.writer, &self.storage, &self.peer_id, &self.sender) {
            (Some(ref writer), _, _, Some(ref sender)) => { 
                sender.clone().send(transition.clone()).wait().unwrap();
                writer.push(transition.to_vec()); 
                Ok(()) 
            },
            (None, Some(ref storage), Some(ref id), Some(ref sender)) => {
                sender.clone().send(transition.clone()).wait().unwrap();
                let tips = storage.lock().unwrap().tips();
                let transition = Transition::new(id.clone(), tips, transition.to_vec());
                storage.lock().unwrap().store(transition);

                Ok(())
            },
            _ => {
                Err(Error::ReadOnly)
            }
        }
    }

    pub fn id(&self) -> PeerId {
        self.peer_id.clone().unwrap()
    }

    pub fn ask_for_file(&self, track_id: TrackKey) -> impl Future<Item = (), Error = Error> {
        let (c, p) = oneshot();

        match &self.writer {
            Some(spread) => {
                if spread.num_peers() == 0 {
                    drop(c);
                } else {
                    self.awaiting.lock().unwrap().insert(track_id.clone(), c);
                    spread.spread(Packet::File(track_id.to_vec(), None), SpreadTo::Everyone);
                }
            },
            None => {
                //c.send(Err(Error::NotFound));
                drop(c)
            }
        }

        let awaiting = self.awaiting.clone();
        p.timeout(Duration::from_millis(3000)).then(move |x| {
            if let Ok(ref mut map) = awaiting.lock() {
                (*map).remove(&track_id.clone());
            }

            x
        }).map_err(|_| Error::NotFound)
    }
    /// Prepare a search with a provided query and translate it to SQL. This method fails in case
    /// of an invalid query.
    pub fn search_prep(&self, query: SearchQuery) -> Result<Statement> {
        let query = query.to_sql_query();

        self.socket.prepare(&query).map_err(|e| Error::Sqlite(e))
    }

    /// Execute the prepared search and return an iterator over all results
    pub fn search<'a>(&self, stmt: &'a mut Statement) -> impl Iterator<Item = Track> + 'a {
        stmt.query_map(&[], |row| Track::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok())
    }

    /// Search for a query and returns 50 tracks starting at `start`
    pub fn search_limited(&self, query: &str, start: usize) -> Result<Vec<Track>> {
        let query = SearchQuery::new(query);

        let mut stmt = self.search_prep(query)?;
        let res = self.search(&mut stmt).skip(start).take(50).collect();

        Ok(res)
    }

    /// Get all available playlists and return their metadata
    pub fn get_playlists(&self) -> Vec<Playlist> {
        let mut stmt = self.socket.prepare("SELECT * FROM Playlists").unwrap();

        let vec = stmt.query_map(&[], |row| Playlist::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok()).collect();

        vec
    }

    /// Get all available tracks and return their metadata
    pub fn get_tracks(&self) -> Vec<Track> {
        let mut stmt = self.socket.prepare("SELECT * FROM Tracks").unwrap();

        let vec = stmt.query_map(&[], |row| Track::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok()).collect();

        vec
    }

    pub fn get_num_tracks(&self) -> i64 {
        let mut stmt = self.socket.prepare("SELECT COUNT(*) FROM Tracks").unwrap();
        let count: i64 = stmt.query_map(&[], |row| row.get(0)).unwrap().next().unwrap().unwrap();

        count
    }

    /// Get a track with key `key`
    pub fn get_track(&self, key: TrackKey) -> Result<Track> {
        let mut stmt = self.socket.prepare("SELECT * FROM Tracks WHERE Key = ?").unwrap();

        let mut stream = stmt.query_map(&[&key.to_vec()], |row| Track::from_row(row)).unwrap()
            .filter_map(|x| x.ok()).filter_map(|x| x.ok());

        stream.next().ok_or(Error::NotFound)
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_token(&self, token: TokenId) -> Result<(Token, Option<(Playlist, Vec<Track>)>)> {
        let mut stmt = self.socket.prepare("SELECT * FROM Tokens WHERE Token=?;").unwrap();
            
        let mut query = stmt.query(&[&token]).unwrap();

        let token = query.next()
            .ok_or(Error::NotFound)
            .and_then(|x| x.map_err(|e| Error::Sqlite(e)))
            .and_then(|row| Token::from_row(&row).map_err(|e| Error::Sqlite(e)))?;

        if let Some(playlist) = token.key {
            let (playlist, tracks) = self.get_playlist(playlist)?;

            Ok((token, Some((playlist, tracks))))
        } else {
            Ok((token, None))
        }
    }

    /// Get a playlist with a certain key and return the metadata and tracks
    pub fn get_playlist(&self, key: PlaylistKey) -> Result<(Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare("SELECT * FROM Playlists WHERE Key=?;").unwrap();

        let mut query = stmt.query(&[&key]).unwrap();
        let playlist = query.next()
            .ok_or(Error::NotFound)
            .and_then(|x| x.map_err(|e| Error::Sqlite(e)))
            .and_then(|row| Playlist::from_row(&row).map_err(|e| Error::Sqlite(e)))?;

        let idx_map : HashMap<TrackKey, usize>= playlist.tracks.iter().enumerate().map(|(a,b)| (b.clone(),a)).collect();

        let query = format!("SELECT * FROM Tracks WHERE hex(key) in ({});", 
                            playlist.tracks.iter().map(|key| format!("\"{}\"", key.to_string())).collect::<Vec<String>>().join(",")
                        );

        let mut stmt = self.socket.prepare(&query).unwrap();
        let mut res: Vec<Track> = self.search(&mut stmt).collect();

        res.sort_by(|a,b| {
            let idx_a = idx_map.get(&a.key).unwrap();
            let idx_b = idx_map.get(&b.key).unwrap();

            idx_a.cmp(idx_b)
        });

        Ok((playlist, res))
    }

    /// Get the last used token
    pub fn get_last_used_token(&self) -> Result<(Token, Option<(Playlist, Vec<Track>)>)> {
        let mut stmt = self.socket.prepare("SELECT * FROM Tokens ORDER BY lastuse DESC Limit 1").unwrap();

        let mut query = stmt.query(&[]).unwrap();
        let token = query.next()
            .ok_or(Error::NotFound)
            .and_then(|x| x.map_err(|e| Error::Sqlite(e)))
            .and_then(|row| Token::from_row(&row).map_err(|e| Error::Sqlite(e)))?;

        if let Some(playlist) = token.key {
            let (playlist, tracks) = self.get_playlist(playlist)?;

            Ok((token, Some((playlist, tracks))))
        } else {
            Ok((token, None))
        }
    }

    /// Look all playlists up belonging to a certain track
    pub fn get_playlists_of_track(&self, key: TrackKey) -> Result<Vec<Playlist>> {
        let tmp = key.to_vec();
        let mut stmt = self.socket.prepare("SELECT * FROM Playlists WHERE INSTR(Tracks, ?) > 0").unwrap();

        let res = stmt.query_map(&[&tmp], |row| Playlist::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok()).collect();

        Ok(res)
    }

    /// Return all database transitions
    pub fn get_transitions(&self) -> Vec<Transition> {
        let mut stmt = self.socket.prepare("SELECT * FROM Transitions;").unwrap();


        let rows = stmt.query_map(&[], |x| transition_from_sql(x)).unwrap().filter_map(|x| x.ok()).collect();

        rows
    }

    pub fn get_num_transitions(&self, _days: u32) -> u64 {
        0
    }

    pub fn last_playlist_key(&self) -> Result<PlaylistKey> {
        let mut stmt = self.socket.prepare("SELECT Key FROM Playlists ORDER BY Key DESC LIMIT 1").unwrap();
        let res: i64 = stmt.query_map(&[], |row| row.get(0)).unwrap().filter_map(|x| x.ok()).next().unwrap_or(0);

        Ok(res)
    }

    pub fn last_token_id(&self) -> Result<TokenId> {
        let mut stmt = self.socket.prepare("SELECT token FROM Tokens ORDER BY Key DESC LIMIT 1").unwrap();
        let res: i64 = stmt.query_map(&[], |row| row.get(0)).unwrap().filter_map(|x| x.ok()).next().unwrap_or(0);

        Ok(res)
    }

    /// Create a empty playlist with a `title` and `origin`
    ///
    /// The `origin` field is only used when the playlist originates from a different server and
    /// should therefore be updated after a new version appears.
    pub fn add_playlist(&self, mut playlist: Playlist) -> Result<()> {
        let has_playlist = self.get_playlist(playlist.key).is_ok();

        if !has_playlist {
            // get highest id

            if !self.peer_id.is_some() {
                return Err(Error::ReadOnly);
            }

            playlist.origin = self.peer_id.clone().unwrap();

            self.commit(TransitionAction::UpsertPlaylist(playlist))
        } else {
            Err(Error::AlreadyExists)
        }
    }

    /// Deletes a playlist with key `key`
    pub fn delete_playlist(&self, key: PlaylistKey) -> Result<()> {
        let has_playlist = self.get_playlist(key).is_ok();

        if has_playlist {
            self.commit(TransitionAction::DeletePlaylist(key))
        } else {
            Err(Error::NotFound)
        }
    }

    pub fn update_playlist(&self, key: PlaylistKey, title: Option<String>, desc: Option<String>) -> Result<()> {
        let (mut playlist, _) = self.get_playlist(key).unwrap();

        if let Some(title) = title {
            playlist.title = title;
        }

        if let Some(desc) = desc {
            playlist.desc = Some(desc);
        }

        self.commit(TransitionAction::UpsertPlaylist(playlist))
    }

    /// Add a track to a certain playlist
    ///
    /// It is important that `playlist` is the title of the playlist and not the key. This method
    /// returns the updated playlist.
    pub fn add_to_playlist(&self, key: TrackKey, playlist: PlaylistKey) -> Result<()> {
        let (mut playlist, _) = self.get_playlist(playlist).unwrap();

        playlist.tracks.push(key);

        self.commit(TransitionAction::UpsertPlaylist(playlist))
    }

    /// Remove a track to a certain playlist
    pub fn delete_from_playlist(&self, key: TrackKey, playlist: PlaylistKey) -> Result<()> {
        let (mut playlist, _) = self.get_playlist(playlist).unwrap();

        let index = playlist.tracks.iter().position(|x| x.to_vec() == key.to_vec())
            .ok_or(Error::NotFound)?;

        playlist.tracks.remove(index);

        self.commit(TransitionAction::UpsertPlaylist(playlist))
    }

    /// Insert a new track into the database
    pub fn add_track(&self, track: Track) -> Result<()> {
        let has_track = self.get_track(track.key).is_ok();

        if !has_track {
            self.commit(TransitionAction::UpsertTrack(track))
        } else {
            Err(Error::AlreadyExists)
        }
    }

    /// Delete a track with key `key`
    pub fn delete_track(&self, key: TrackKey) -> Result<()> {
        let has_track = self.get_track(key).is_ok();

        if has_track {
            self.commit(TransitionAction::DeleteTrack(key))
        } else {
            Err(Error::NotFound)
        }
    }

    /// Update the metadata of tracks
    ///
    /// In case none of the parameters is Option::Some, then no field is updated.
    pub fn update_track(&self, key: TrackKey, title: Option<&str>, album: Option<&str>, interpret: Option<&str>, people: Option<&str>, composer: Option<&str>) -> Result<TrackKey> {
        let mut track = self.get_track(key).unwrap();

        if let Some(title) = title {
            track.title = Some(title.into());
        }

        if let Some(album) = album {
            track.album = Some(album.into());
        }

        if let Some(interpret) = interpret {
            track.interpret = Some(interpret.into());
        }

        if let Some(people) = people {
            track.people = Some(people.into());
        }

        if let Some(composer) = composer {
            track.composer = Some(composer.into());
        }

        self.commit(TransitionAction::UpsertTrack(track))
            .map(|_| key)
    }

    /// Increment the favourite count for a track
    pub fn vote_for_track(&self, key: TrackKey) -> Result<()> {
        let mut track = self.get_track(key).unwrap();

        track.favs_count += 1;

        self.commit(TransitionAction::UpsertTrack(track))
    }

    pub fn use_token(&self, token: TokenId) -> Result<()> {
        let (mut token, _) = self.get_token(token).unwrap();

        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        token.last_use = since_the_epoch.as_secs() as i64;

        self.commit(TransitionAction::UpsertToken(token))
    }

    /// Create a new token with a valid id
    pub fn add_token(&self, token: Token) -> Result<TokenId> {
        let id = token.token;

        self.commit(TransitionAction::UpsertToken(token))
            .map(|_| id)
    }

    /// Update the metadata of a token
    ///
    /// When no parameter is Option::Some no metadata will be updated.
    pub fn update_token(&self, token: TokenId, key: Option<PlaylistKey>, played: Option<Vec<TrackKey>>, pos: Option<f64>) -> Result<()> {
        let (mut token, _) = self.get_token(token).unwrap();

        if let Some(key) = key {
            token.key = Some(key);
        }

        if let Some(played) = played {
            token.played = played;
        }

        if let Some(pos) = pos {
            token.pos = Some(pos);
        }

        self.commit(TransitionAction::UpsertToken(token))
    }

    /// Summarise a day (used by `nightly-worker`)
    pub fn summarise_day(&self, day: String, transitions: u32, tracks: u32) -> Result<()> {
        self.socket.execute(
            "INSERT INTO Summary (Day, Transitions, Tracks) VALUES (?1, ?2, ?3)",
                &[&day, &transitions, &tracks]).map(|_| ()).map_err(|e| Error::Sqlite(e))

    }

    /// Get a summarise of all days since beginning of use
    pub fn get_complete_summary(&self) -> Vec<(String, u32, u32)> {
        let mut stmt = self.socket.prepare(
            "SELECT Day, Transitions, Tracks FROM Summary;").unwrap();

        let rows = stmt.query_map(&[], |x| {
            (x.get(0), x.get(1), x.get(2))
        }).unwrap().filter_map(|x| x.ok()).collect();

        rows
    }

    /// Find the latest summarise day in the database
    pub fn get_latest_summary_day(&self) -> Result<String> {
        let mut stmt = self.socket.prepare(
            "SELECT day FROM Summary order by Day desc limit 1;").unwrap();

        let mut query = stmt.query(&[]).unwrap();
        let row = query.next().ok_or(Error::NotFound)?
            .map_err(|e| Error::Sqlite(e))?;

        Ok(row.get(0))
    }
}

#[cfg(test)]
mod tests {

    use super::Instance;
    use hex_gossip::{GossipConf, PeerId};
    use crate::objects::{Playlist, Track, Token};
    use crate::search::SearchQuery;
    use crate::transition::TransitionAction;
    use futures::{Stream, IntoFuture, Future, Async};

    fn gen_track() -> Track {
        let mut track = Track::empty(vec![1u32; 10], 100.0);
        track.title = Some("Blue like something".into());
        track.composer = Some("Random Guy".into());

        track
    }

    fn gossip() -> GossipConf {
        GossipConf::new().id(vec![0; 16])
    }

    #[test]
    pub fn test_search() {
        let instance = Instance::from_file("/tmp/test.db", gossip());
        let view = instance.view();

        let track = gen_track();

        view.add_track(track.clone()).unwrap();

        // create a new search query
        let query = SearchQuery::new("title:Blue't");

        // initiate the search
        let mut stmt = view.search_prep(query).unwrap();
        assert_eq!(track, view.search(&mut stmt).next().unwrap());
    }

    #[test]
    pub fn test_playlist() {
        let mut instance = Instance::from_file("/tmp/test2.db", gossip());
        let view = instance.view();

        let track = gen_track();
        let playlist = Playlist {
            key: 30,
            title: "My very own playlist".into(),
            desc: Some("".into()),
            tracks: vec![],
            origin: vec![0; 16]
        };

        // check if there are no playlists in the database
        assert_eq!(view.get_playlists().len(), 0);

        // add a new playlist to the database
        view.add_playlist(playlist.clone()).unwrap();
        assert_eq!(view.get_playlists(), vec![playlist.clone()]);
        assert_eq!(view.get_playlist(playlist.key).unwrap().0, playlist);

        // update the playlist, add a desc
        let new_desc = Some("Even with a description".into());
        view.update_playlist(playlist.key, None, new_desc.clone()).unwrap();
        assert_eq!(view.get_playlist(playlist.key).unwrap().0.desc, new_desc);
        view.update_playlist(playlist.key, None, Some("".into())).unwrap();

        // add a track to the playlist
        view.add_track(track.clone()).unwrap();
        view.add_to_playlist(track.key, playlist.key).unwrap();
        assert_eq!(view.get_playlists_of_track(track.key).unwrap()[0].title, playlist.title);
        
        // remove the track from the playlist
        view.delete_from_playlist(track.key, playlist.key).unwrap();
        assert_eq!(view.get_playlists_of_track(track.key).unwrap().len(), 0);
    }

    #[test]
    pub fn test_tracks() {
        let instance = Instance::from_file("/tmp/test3.db", gossip());
        let view = instance.view();

        // create a new track
        let track = gen_track();
        view.add_track(track.clone()).unwrap();

        // vote ten times for this track
        for _ in 0..10 {
            view.vote_for_track(track.key).unwrap();
        }

        assert_eq!(view.get_track(track.key).unwrap().favs_count, track.favs_count + 10);

        // update the track metadata
        let (title, album, interpret, _, composer) = (
            "Eye in the Sky", "Live", "Alan Parsons", "Alan Parsons", "Alan Parsons");
        
        view.update_track(track.key, Some(title), Some(album), Some(interpret), None, Some(composer)).unwrap();

        let tmp = view.get_track(track.key).unwrap();
        assert!(
            tmp.title == Some(title.into()) && 
            tmp.album == Some(album.into()) && 
            tmp.interpret == Some(interpret.into()) &&
            tmp.people == None,
            tmp.composer == Some(composer.into())
        );

        view.delete_track(track.key).unwrap();

        assert_eq!(view.get_tracks().len(), 0);
    }

    #[test]
    pub fn test_tokens() {
        let instance = Instance::from_file("/tmp/test4.db", gossip());
        let view = instance.view();

        //create a track and playlist
        let track = gen_track();
        let playlist = Playlist {
            key: 30,
            title: "My very own playlist".into(),
            desc: Some("".into()),
            tracks: vec![],
            origin: vec![0u8; 16]
        };

        // setup up track and plalist
        view.add_track(track.clone()).unwrap();
        view.add_playlist(playlist.clone()).unwrap();
        view.add_to_playlist(track.key, playlist.key).unwrap();

        let token = Token {
            token: view.last_token_id().unwrap() + 1,
            key: None,
            played: Vec::new(),
            pos: None,
            last_use: 0
        };

        assert_eq!(view.add_token(token).unwrap(), 1);

        // set the playlist as a token and compare everything
        view.update_token(1, Some(playlist.key), None, Some(5.0)).unwrap();

        let (tmp, rem) = view.get_token(1).unwrap();
        let rem = rem.unwrap();
        assert!(tmp.key.unwrap() == playlist.key && tmp.pos == Some(5.0));
        assert_eq!(rem.0.title, playlist.title);
        assert_eq!(rem.1[0].key, track.key);
        
        // update the 0th token
        view.use_token(1).unwrap();

        let (last_token, _) = view.get_last_used_token().unwrap();

        assert_eq!(view.get_token(1).unwrap().0, last_token);
    }

    #[test]
    pub fn recv_stream() {
        let mut instance = Instance::from_file("/tmp/test5.db", gossip());
        let view = instance.view();

        let track = gen_track();

        view.add_track(track.clone()).unwrap();

        match instance.recv().poll() {
            Ok(Async::Ready(Some(x))) => {
                assert_eq!(x, TransitionAction::UpsertTrack(track));
            },
            _ => { panic!("Wrong result!"); }
        }
    }
}
