use std::path::Path;

use rusqlite::Row;
use bincode::{serialize, deserialize};
use hex_gossip::{Inspector, Transition, TransitionKey, PeerId};

use objects::{self, Track, Playlist, Token, TrackKey, PlaylistKey, TokenId};

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

static UPSERT_PLAYLIST: &str = r#"
    INSERT INTO Playlists(Key, Title, Desc, Tracks, Author)
        VALUES(?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(Key) DO UPDATE SET
            Title = excluded.Title,
            Desc = excluded.Desc,
            Tracks = excluded.Tracks;
"#;

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

impl TransitionAction {
    pub fn from_vec(buf: &[u8]) -> TransitionAction {
        deserialize(buf).unwrap()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        serialize(&self).unwrap()
    }
}

pub fn transition_from_sql(row: &Row) -> Transition {
    let a: Vec<u8> = row.get(0);
    let b: Vec<u8> = row.get(1);
    //let c: Vec<u8> = row.get(3);

    let pk = PeerId(a);
    let refs = b.chunks(32).map(|x| TransitionKey::from_vec(x)).collect();

    Transition {
        pk, refs, 
        body: row.get(2),
        sign: [0; 32],
        is_tip: row.get(4)
    }
}

/// The inspector will open a write/read connection to the database and fill it with foreign and
/// domestic changes. Transitions issued from ourselves are also forwarded to the inspector.
pub struct Storage {
    socket: rusqlite::Connection
}

impl Storage {
    pub fn new<T: AsRef<Path>>(path: T) -> Storage {
        Storage {
            socket: rusqlite::Connection::open(path).unwrap()
        }
    }
}

impl Inspector for Storage {
    fn approve(&self, trans: &Transition) -> bool {
        deserialize::<TransitionAction>(&trans.body.clone().unwrap()).is_ok()
    }

    fn store(&self, trans: Transition) {
        //println!("Store: {:?}", deserialize::<TransitionAction>(&trans.body.clone().unwrap()));
        let key = trans.key().0;
        let key_ref = key.as_ref();
        let pk = trans.pk.0.clone();

        self.socket.execute("INSERT INTO Transitions (Key, PublicKey, Signature, Refs, IsTip, Data, Created) VALUES (?1, ?2, ?3, ?4, 1, ?5, DATETIME('NOW'))",
            &[
                &key_ref, 
                &pk, 
                &trans.sign.as_ref(), 
                &trans.refs.clone().into_iter().map(|x| x.0.to_vec()).flatten().collect::<Vec<u8>>(), 
                &trans.body
            ]).unwrap();

        // set refs to non-tip
        let tips: Vec<TransitionKey> = self.tips().into_iter()
            .filter(|x| trans.refs.contains(x))
            .collect();

        for key in tips {
            self.socket.execute("UPDATE Transitions SET IsTip=0 WHERE Key=?", &[&key.0.as_ref()]).unwrap();
        }

        // update database according to the change
        match deserialize(&trans.body.unwrap()).unwrap() {
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
                    &playlist.origin.0
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

    }

    fn restore(&self, keys: Vec<TransitionKey>) -> Vec<Transition> {
        let stmt = format!("SELECT * FROM Transitions WHERE Key IN ({})", keys.into_iter().map(|x| format!("x'{}'", x.to_string())).collect::<Vec<String>>().join(","));

        let mut stmt = self.socket.prepare(&stmt).unwrap();

        let vec = stmt.query_map(&[], |row| transition_from_sql(&row)).unwrap().filter_map(|x| x.ok()).collect();

        vec

    }

    fn tips(&self) -> Vec<TransitionKey> {
        let mut stmt = self.socket.prepare("SELECT Key FROM Transitions WHERE IsTip=1").unwrap();

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
}

