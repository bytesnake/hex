use std::io::Read;
use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(feature="rusqlite")]
use rusqlite::Row;
#[cfg(feature="rusqlite")]
use bincode::{serialize, deserialize};
#[cfg(feature="rusqlite")]
use hex_gossip::{Inspector, Transition, TransitionKey};

use objects::{self, Track, Playlist, Token, TrackKey, PlaylistKey, TokenId};

#[cfg(feature="rusqlite")]
static UPSERT_TRACK: &str = r#"
    INSERT INTO Tracks(Key, Fingerprint, Title, Album, Interpret, People, Composer, Duration, FavsCount, Created)
        VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, date('now'))
        ON CONFLICT(Key) DO UPDATE SET
            Title = excluded.Title,
            Album = excluded.Album,
            Interpret = excluded.Interpret,
            People = excluded.People,
            Composer = excluded.Composer,
            FavsCount = excluded.FavsCount;
"#;

#[cfg(feature="rusqlite")]
static UPSERT_PLAYLIST: &str = r#"
    INSERT INTO Playlists(Key, Title, Desc, Tracks, Author)
        VALUES(?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(Key) DO UPDATE SET
            Title = excluded.Title,
            Desc = excluded.Desc,
            Tracks = excluded.Tracks;
"#;

#[cfg(feature="rusqlite")]
static UPSERT_TOKEN: &str = r#"
    INSERT INTO Tokens(Token, Key, Played, Pos, Lastuse) 
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(Token) DO UPDATE SET
            Key = excluded.Key,
            Played = excluded.Played,
            Pos = excluded.Pos,
            Lastuse = excluded.Lastuse;
"#;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TransitionAction {
    // create either a Token, Playlist or Track
    UpsertTrack(Track),
    UpsertPlaylist(Playlist),
    UpsertToken(Token),

    // delete either a Token, Playlist or Track by its key
    DeleteTrack(TrackKey),
    DeletePlaylist(PlaylistKey),
    DeleteToken(TokenId),
}

#[cfg(feature="rusqlite")]
impl TransitionAction {
    pub fn from_vec(buf: &[u8]) -> TransitionAction {
        deserialize(buf).unwrap()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        serialize(&self).unwrap()
    }
}

#[cfg(feature="rusqlite")]
pub fn transition_from_sql(row: &Row) -> Transition {
    let a: Vec<u8> = row.get(0);
    let b: Vec<u8> = row.get(1);
    let c: Vec<u8> = row.get(3);

    let key = TransitionKey::from_vec(&a);
    let pk = b;
    let refs = c.chunks(32).map(|x| TransitionKey::from_vec(x)).collect();

    Transition {
        key, pk, refs, 
        body: row.get(5),
        sign: [0; 32],
        state: row.get(4)
    }
}

/// The inspector will open a write/read connection to the database and fill it with foreign and
/// domestic changes. Transitions issued from ourselves are also forwarded to the inspector.
#[cfg(feature="rusqlite")]
pub struct Storage {
    socket: rusqlite::Connection,
    data_path: PathBuf
}

#[cfg(feature="rusqlite")]
impl Storage {
    pub fn new<T: AsRef<Path>>(path: T) -> Storage {
        Storage {
            data_path: path.as_ref().parent().unwrap().join("data").to_path_buf(),
            socket: rusqlite::Connection::open(path).unwrap()
        }
    }

    pub fn apply(&self, trans: Transition) {
        // check wether all referenced transitions are already applied
        let all_applied = match self.restore(trans.refs.clone()) {
            Some(x) => x.iter().all(|x| x.state != 2),
            None => false
        };

        // don't apply if at least one reference is not yet applied
        if !all_applied {
            return;
        }

        // otherweise set refs to non-tip
        let tips: Vec<TransitionKey> = self.tips().into_iter()
            .filter(|x| trans.refs.contains(x))
            .collect();

        for key in tips {
            self.socket.execute("UPDATE Transitions SET State=0 WHERE Key=?", &[&key.0.as_ref()]).unwrap();
        }

        // parse the body to a transition action
        let res: TransitionAction = deserialize(&trans.body.unwrap()).unwrap();
        trace!("Apply {:?}", res);

        // update database according to the change
        match res {
            TransitionAction::UpsertTrack(track) => self.socket.execute(UPSERT_TRACK, 
                &[
                    &track.key.to_vec(), 
                    &objects::u32_into_u8(track.fingerprint.clone()), 
                    &track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.duration, &track.favs_count
                ]).unwrap(),

            TransitionAction::UpsertPlaylist(playlist) => self.socket.execute(UPSERT_PLAYLIST, 
                &[
                    &playlist.key, &playlist.title, &playlist.desc, 
                    &playlist.tracks.into_iter().map(|x| x.to_vec()).flatten().collect::<Vec<u8>>(), 
                    &playlist.origin
                ]).unwrap(),

            TransitionAction::UpsertToken(token) => self.socket.execute(UPSERT_TOKEN, 
                &[
                    &token.token,
                    &token.key, 
                    &token.played.into_iter().map(|x| x.to_vec()).flatten().collect::<Vec<u8>>(), 
                    &token.pos, &token.last_use
                ]).unwrap(),

            TransitionAction::DeleteTrack(track_key) => self.socket.execute("DELETE FROM Tracks WHERE Key=?", &[&track_key.to_vec()]).unwrap(),
            TransitionAction::DeletePlaylist(playlist_key) => self.socket.execute("DELETE FROM Playlists WHERE Key=?", &[&playlist_key]).unwrap(),
            TransitionAction::DeleteToken(token) => self.socket.execute("DELETE FROM Tokens WHERE token=?", &[&token]).unwrap()
        };

        // find references to this transitions and try to apply them too
        let mut stmt = self.socket.prepare("SELECT * FROM Transitions WHERE INSTR(Refs, ?)").unwrap();

        let key = trans.key.0.to_vec();
        let vec: Vec<Transition> = stmt.query_map(&[&key], |row| transition_from_sql(&row)).unwrap().filter_map(|x| x.ok()).collect();

        // if there is no reference to us, we are a tip, otherwise we're fully integrated
        if vec.len() == 0 {
            self.socket.execute("UPDATE Transitions SET State=1 WHERE Key=?", &[&trans.key.0.as_ref()]).unwrap();
        } else {
            self.socket.execute("UPDATE Transitions SET State=0 WHERE Key=?", &[&trans.key.0.as_ref()]).unwrap();
        }

        for t in vec {
            if t.state == 2 {
                self.apply(t);
            }
        }
    }

}

#[cfg(feature="rusqlite")]
impl Inspector for Storage {
    fn approve(&self, trans: &Transition) -> bool {
        deserialize::<TransitionAction>(&trans.body.clone().unwrap()).is_ok()
    }

    fn store(&self, trans: Transition) {
        let Transition { key, pk, sign, refs, body, .. } = trans.clone();

        self.socket.execute("INSERT INTO Transitions (Key, PublicKey, Signature, Refs, State, Data, Created) VALUES (?1, ?2, ?3, ?4, ?5, ?6, DATETIME('NOW'))",
            &[
                &key.0.as_ref(), 
                &pk, 
                &sign.as_ref(),
                &refs.into_iter().map(|x| x.0.to_vec()).flatten().collect::<Vec<u8>>(), 
                &2,
                &body.clone().unwrap()
            ]).unwrap();

        self.apply(trans);
    }

    fn restore(&self, keys: Vec<TransitionKey>) -> Option<Vec<Transition>> {
        let key_len = keys.len();

        let stmt = format!("SELECT * FROM Transitions WHERE hex(key) IN ({});", keys.into_iter().map(|x| format!("\"{}\"", x.to_string())).collect::<Vec<String>>().join(","));

        let mut stmt = self.socket.prepare(&stmt).unwrap();

        let vec: Vec<Transition> = stmt.query_map(&[], |row| transition_from_sql(&row)).unwrap().filter_map(|x| x.ok()).collect();

        if vec.len() != key_len {
            None
        } else {
            Some(vec)
        }
    }

    fn tips(&self) -> Vec<TransitionKey> {
        let mut stmt = self.socket.prepare("SELECT Key FROM Transitions WHERE State=1").unwrap();

        let vec = stmt.query_map(&[], |row| {
            let key: Vec<u8> = row.get(0);

            TransitionKey::from_vec(&key)
        }).unwrap()
            .filter_map(|x| x.ok()).collect();

        vec
    }

    fn has(&self, key: &TransitionKey) -> bool {
        let mut stmt = self.socket.prepare("SELECT * FROM Transitions WHERE Key = ?").unwrap();
        let mut stream = stmt.query_map(&[&key.0.as_ref()], |_| true).unwrap()
            .filter_map(|x| x.ok());

        stream.next().is_some()
    }

    fn get_file(&self, id: &[u8]) -> Option<Vec<u8>> {
        if id.len() != 16 {
            return None;
        }

        let mut tmp = String::new();
        for i in 0..16 {
            tmp.push_str(&format!("{:02X}", id[i]));
        }

        let mut file = File::open(self.data_path.join(&tmp)).ok()?;
        let mut content = vec![];
        file.read_to_end(&mut content).unwrap();

        Some(content)
    }
}

