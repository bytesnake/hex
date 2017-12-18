use error::{Error, Result};

use rusqlite;
use rusqlite::Statement;
use music_search::SearchQuery;

use acousticid::Tracks;

use std::str;

use audio_file::AudioFile;

use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Serialize, Clone, Debug)]
pub struct Track {
    title: Option<String>,
    album: Option<String>,
    interpret: Option<String>,
    conductor: Option<String>,
    composer: Option<String>,
    fingerprint: String,
    key: String
}

impl Track {
    pub fn empty(key: &str, fingerprint: &str) -> Track {
        Track {
            key: key.into(),
            fingerprint: fingerprint.into(),
            title: None,
            album: None,
            interpret: None,
            conductor: None,
            composer: None
        }
    }

    pub fn new(key: &str, fingerprint: &str, title: Option<String>, album: Option<String>, interpret: Option<String>, conductor: Option<String>, composer: Option<String>) -> Track {
        Track {
            key: key.into(),
            fingerprint: fingerprint.into(),
            title: title,
            album: album,
            interpret: interpret,
            conductor: conductor,
            composer: composer
        }
    }
}

pub struct Connection {
    socket: rusqlite::Connection
}

impl Connection {
    pub fn new() -> Connection {
        let home_dir = env::home_dir().expect("Could not find the home directory!");

        Connection { socket: rusqlite::Connection::open(&format!("{}/.music.db", home_dir)).unwrap() }
    }

    pub fn search_prep(&self, query: SearchQuery) -> Statement {
        if query.is_empty() {
            self.socket.prepare("SELECT Title, Album, Interpret, Conductor, Composer, Key FROM music").unwrap()
        } else {
            let query = query.to_sql_query();

            self.socket.prepare(&format!("SELECT Title, Album, Interpret, Conductor, Composer, Key FROM music WHERE {};", query)).unwrap()
        }
    }

    pub fn search<'a>(&self, stmt: &'a mut Statement) -> impl Iterator<Item = Track> + 'a {
        stmt.query_map(&[], |row| {
            Track {
                title: row.get(0),
                album: row.get(1),
                interpret: row.get(2),
                conductor: row.get(4),
                composer: row.get(5),
                key: row.get(6)
            }
        }).unwrap().filter_map(|x| x.ok()).map(|x| x.clone())
    }

    pub fn insert_track(&self, track: Track) {
        self.socket.execute("INSERT INTO music (Title, Album, Interpret, Conductor, Composer) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", &[&track.title, &track.album, &track.interpret, &track.conductor, &track.composer]).unwrap();
    }
}
