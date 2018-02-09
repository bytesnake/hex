use error::{ErrorKind, Result};
use failure::ResultExt;

use rusqlite;
use rusqlite::Statement;

use music_search::SearchQuery;
use audio_file::AudioFile;
use acousticid;

use std::{env, str};
use std::fs::File;
use std::io::Write;
use std::process::Command;

use failure::Fail;

use uuid::Uuid;

#[derive(Serialize, Clone, Debug)]
pub struct Track {
    title: Option<String>,
    album: Option<String>,
    interpret: Option<String>,
    people: Option<String>,
    composer: Option<String>,
    fingerprint: String,
    pub key: String,
    pub duration: f64,
    favs_count: u32,
    channels: u32
}

#[derive(Serialize, Clone, Debug)]
pub struct Playlist {
    pub key: String,
    pub title: String,
    desc: Option<String>,
    count: u32
}

impl Track {
    pub fn empty(key: &str, fingerprint: &str, duration: f64) -> Track {
        Track {
            key: key.into(),
            fingerprint: fingerprint.into(),
            duration: duration,
            title: None,
            album: None,
            interpret: None,
            people: None,
            composer: None,
            favs_count: 0,
            channels: 2
        }
    }

    pub fn new(key: &str, fingerprint: &str, duration: f64, title: Option<String>, album: Option<String>, interpret: Option<String>, people: Option<String>, composer: Option<String>, favs_count: u32, channels: u32) -> Track {
        Track {
            key: key.into(),
            fingerprint: fingerprint.into(),
            duration: duration,
            title: title,
            album: album,
            interpret: interpret,
            people: people,
            composer: composer,
            favs_count: favs_count,
            channels: channels
        }
    }

    pub fn suggestion(&self) -> Result<String> {
        acousticid::get_metadata(&self.fingerprint, self.duration as u32)
    }
}

pub struct Connection {
    socket: rusqlite::Connection
}

impl Connection {
    pub fn open_file(path: &str) -> Connection {
        //let mut dir = env::home_dir().expect("Could not find the home directory!");
        //dir.push(".music.db");

        Connection { socket: rusqlite::Connection::open(path).unwrap() }
    }

    /// Search for a certain track. The SearchQuery ensures that a valid string is generated.
    /// TODO: search for tracks in a certain playlist

    pub fn search_prep(&self, query: SearchQuery) -> Result<Statement> {
        let tmp = {
            let query = query.to_sql_query();

            println!("Query: {}", query);
            self.socket.prepare(&query).context(ErrorKind::Database)
        };

        Ok(tmp?)
    }

    /// Execute the search and returns an iterator over tracks.
    pub fn search<'a>(&self, stmt: &'a mut Statement) -> impl Iterator<Item = Track> + 'a {
        stmt.query_map(&[], |row| {
            Track {
                title: row.get(0),
                album: row.get(1),
                interpret: row.get(2),
                fingerprint: row.get(3),
                people: row.get(4),
                composer: row.get(5),
                key: row.get(6),
                duration: row.get(7),
                favs_count: row.get(8),
                channels: row.get(9)
            }
        }).unwrap().filter_map(|x| x.ok()).map(|x| x.clone())
    }

    /// Returns the metadata of all available playlists
    pub fn get_playlists(&self) -> Vec<Playlist> {
        let mut tmp = self.socket.prepare("SELECT Key, Title, Desc, Count FROM Playlists").unwrap();

        let res = tmp.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                count: row.get(3)
            }
        }).unwrap().filter_map(|x| x.ok()).collect();

        res
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_playlist(&self, key: &str) -> Result<(Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare("SELECT Key, Title, Desc, Count, tracks FROM Playlists WHERE key=?;").context(ErrorKind::Database)?;

        let mut rows = stmt.query(&[&key]).context(ErrorKind::Database)?;
        let row = match rows.next() {
            Some(x) => x,
            None => return Err(format_err!("No element found with key {}", key).context(ErrorKind::Database).into())
        }.unwrap();

        let playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            count: row.get(3)
        };

        let keys: Option<String> = row.get(4);

        println!("Got keys: {:?}", keys);

        if let Some(keys) = keys {
            let query = format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE key in ({});", keys.split(",").map(|row| { format!("'{}'", row) }).collect::<Vec<String>>().join(","));

            let mut stmt = self.socket.prepare(&query).context(ErrorKind::Database)?;

            let res = self.search(&mut stmt).collect();

            Ok((playlist, res))
        } else {
            Ok((playlist, Vec::new()))
        }
    }

    pub fn get_playlists_of_track(&self, key: &str) -> Result<Vec<Playlist>> {
        let mut stmt = self.socket.prepare(&format!("SELECT Key, Title, Desc, Count FROM Playlists WHERE tracks like '%{}%'", key)).context(ErrorKind::Database)?;
        let res = stmt.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                count: row.get(3)
            }
        }).unwrap().filter_map(|x| x.ok()).collect();

        Ok(res)
    }


    pub fn add_playlist(&self, title: &str) -> Result<Playlist> {
        let key = Uuid::new_v4().simple().to_string();

        self.socket.execute("INSERT INTO playlists (key, title, count) VALUES (?1, ?2, ?3)", &[&key, &title, &0]).context(ErrorKind::Database)?;

        Ok(Playlist {
            key: key,
            title: title.into(),
            desc: None,
            count: 0
        })
    }

    pub fn delete_playlist(&self, key: &str) -> Result<()> {
        self.socket.execute("DELETE FROM playlists WHERE key = ?", &[&key])
            .map(|_| ())
            .map_err(|err| err.context(ErrorKind::Database).into())
    }

    pub fn update_playlist(&self, key: &str, title: Option<String>, desc: Option<String>) -> Result<()> {
        if let Some(title) = title {
            self.socket.execute("UPDATE playlists SET title = ?1 WHERE Key = ?2", &[&title, &key]).context(ErrorKind::Database)?;
        }
        if let Some(desc) = desc {
            self.socket.execute("UPDATE playlists SET desc = ?1 WHERE Key = ?2", &[&desc, &key]).context(ErrorKind::Database)?;
        }

        Ok(())
    }

    pub fn add_to_playlist(&self, key: &str, playlist: &str) -> Result<Playlist> {
        let mut stmt = self.socket.prepare("SELECT Key, Title, Desc, Count, tracks FROM Playlists WHERE Title=?;").context(ErrorKind::Database)?;
        let mut rows = stmt.query(&[&playlist]).context(ErrorKind::Database)?;
        let row = rows.next().ok_or(ErrorKind::Database)?.context(ErrorKind::Database)?;

        let mut _playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            count: row.get(3)
        };

        _playlist.count += 1;

        let keys: Option<String> = row.get(4);
        let keys: String = keys.map(|x| format!("{},{}", x, key)).unwrap_or(key.into());

        println!("Track: {}", keys);

        
        self.socket.execute("UPDATE playlists SET Count = ?1, tracks = ?2 WHERE Key = ?3", &[&_playlist.count, &keys, &_playlist.key]).context(ErrorKind::Database)?;

        Ok(_playlist)
    }

    pub fn insert_track(&self, track: Track) -> Result<()> {
        self.socket.execute("INSERT INTO music 
                                    (Title, Album, Interpret, People, Composer, Key, Fingerprint, Duration, FavsCount, Channels) 
                                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", 
                                    &[&track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.key, &track.fingerprint, &track.duration, &track.favs_count, &track.channels])
            .context(ErrorKind::Database)?;

        Ok(())
    }

    pub fn delete_track(&self, key: &str) -> Result<()> {
        self.socket.execute("DELETE FROM music WHERE key = ?", &[&key]).context(ErrorKind::Database)?;

        Ok(())
    }

    pub fn get_track(&self, key: &str) -> Result<Track> {
        let mut stmt = self.socket.prepare(&format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE Key = '{}'", key)).context(ErrorKind::Database)?;
        
        let res = self.search(&mut stmt).next().ok_or(ErrorKind::Database);

        Ok(res?)
    }
    pub fn update_track(&self, key: &str, title: Option<String>, album: Option<String>, interpret: Option<String>, people: Option<String>, composer: Option<String>) -> Result<String> {
        if let Some(title) = title {
            self.socket.execute("UPDATE music SET Title = ? WHERE Key = ?", &[&title, &key]).context(ErrorKind::Database)?;
        }
        if let Some(album) = album {
            self.socket.execute("UPDATE music SET Album = ? WHERE Key = ?", &[&album, &key]).context(ErrorKind::Database)?;
        }
        if let Some(interpret) = interpret {
            self.socket.execute("UPDATE music SET Interpret = ? WHERE Key = ?", &[&interpret, &key]).context(ErrorKind::Database)?;
        }
        if let Some(people) = people {
            self.socket.execute("UPDATE music SET People = ? WHERE Key = ?", &[&people, &key]).context(ErrorKind::Database)?;
        }
        if let Some(composer) = composer {
            self.socket.execute("UPDATE music SET Composer = ? WHERE Key = ?", &[&composer, &key]).context(ErrorKind::Database)?;
        }

        return Ok(key.into());
    }

    pub fn vote_for_track(&self, key: &str) -> Result<()> {
        self.socket.execute("UPDATE music SET FavsCount = FavsCount + 1 WHERE Key = ?1", &[&key]).context(ErrorKind::Database)?;

        Ok(())
    }
}
