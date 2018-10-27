use rusqlite::{self, Result, Statement, Error};
use search::SearchQuery;
use uuid::Uuid;
use std::path::Path;
use objects::*;
use events::Event;

/// Represents an open connection to a database
pub struct Collection {
    socket: rusqlite::Connection
}

impl Collection {
    /// Open a SQLite database from a file and create the neccessary tables in case they don't
    /// exist.
    pub fn from_file(path: &Path) -> Collection {
        let socket = rusqlite::Connection::open(path).unwrap();
    
        socket.execute_batch(
            "BEGIN;
                CREATE TABLE IF NOT EXISTS music (Title TEXT, Album TEXT, Interpret TEXT, Fingerprint TEXT NOT NULL, People TEXT, Composer TEXT, Key TEXT NOT NULL, Duration REAL NOT NULL, FavsCount INTEGER, Channels INTEGER);
                CREATE TABLE IF NOT EXISTS Playlists (Key TEXT NOT NULL, Title TEXT, Desc TEXT, Tracks TEXT, Count INTEGER NOT NULL, Origin TEXT);
                CREATE TABLE IF NOT EXISTS Events (Date Text, Origin Text, Event Text, Data TEXT);
                CREATE TABLE IF NOT EXISTS Summarise (Day TEXT, Connects INTEGER, Plays INTEGER, Adds INTEGER, Removes INTEGER);
                CREATE TABLE IF NOT EXISTS Tokens (Token INTEGER, Key TEXT, Played TEXT, Pos NUMERIC);
            COMMIT;"
        ).unwrap();

        Collection { socket }
    }

    /// Prepare a search with a provided query and translate it to SQL. This method fails in case
    /// of an invalid query.
    pub fn search_prep(&self, query: SearchQuery) -> Result<Statement> {
        let query = query.to_sql_query();

        println!("Query: {}", query);
        self.socket.prepare(&query)
    }

    /// Execute the prepared search and return an iterator over all results
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
        }).unwrap().filter_map(|x| x.ok())
    }

    /// Search for a query and returns 50 tracks starting at `start`
    pub fn search_limited(&self, query: &str, start: usize) -> Result<Vec<Track>> {
        let query = SearchQuery::new(query).ok_or(Error::QueryReturnedNoRows)?;

        let mut stmt = self.search_prep(query)?;
        let res = self.search(&mut stmt).skip(start).take(50).collect();

        Ok(res)
    }

    /// Get all available playlists and return their metadata
    pub fn get_playlists(&self) -> Vec<Playlist> {
        let mut stmt = self.socket.prepare("SELECT Key, Title, Desc, Tracks, Count, Origin FROM Playlists").unwrap();

        let vec = stmt.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                tracks: row.get(3),
                count: row.get(4),
                origin: row.get(5)
            }
        }).unwrap().filter_map(|x| x.ok()).collect();

        vec
    }

    /// Get all available tracks and return their metadata
    pub fn get_tracks(&self) -> Vec<Track> {
        let mut stmt = self.socket.prepare("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music").unwrap();

        let vec = stmt.query_map(&[], |row| {
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
        }).unwrap().filter_map(|x| x.ok()).collect();

        vec
    }

    /// Get a playlist with a certain key and return the metadata and tracks
    pub fn get_playlist(&self, key: &str) -> Result<(Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare(
            "SELECT Key, Title, Desc, Tracks, Count, Origin
                FROM Playlists WHERE key=?;")?;

        let mut query = stmt.query(&[&key])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            tracks: row.get(3),
            count: row.get(4),
            origin: row.get(5)
        };

        let keys: Option<String> = playlist.tracks.clone();

        if let Some(keys) = keys {
            let query = format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE key in ({});", keys.split(",").map(|row| { format!("'{}'", row) }).collect::<Vec<String>>().join(","));

            let mut stmt = self.socket.prepare(&query)?;

            let res = self.search(&mut stmt).collect();

            Ok((playlist, res))
        } else {
            Ok((playlist, Vec::new()))
        }
    }

    /// Look all playlists up belonging to a certain track
    pub fn get_playlists_of_track(&self, key: &str) -> Result<Vec<Playlist>> {
        let mut stmt = self.socket.prepare(&format!("SELECT Key, Title, Desc, Tracks, Count, Origin FROM Playlists WHERE tracks like '%{}%'", key))?;

        let res = stmt.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                tracks: row.get(3),
                count: row.get(4),
                origin: row.get(5)
            }
        })?.filter_map(|x| x.ok()).collect();

        Ok(res)
    }


    /// Create a empty playlist with a `title` and `origin`
    ///
    /// The `origin` field is only used when the playlist originates from a different server and
    /// should therefore be updated after a new version appears.
    pub fn add_playlist(&self, title: &str, origin: Option<String>) -> Result<Playlist> {
        let key = Uuid::new_v4().simple().to_string();

        self.socket.execute("INSERT INTO playlists (key, title, count, origin) VALUES (?1, ?2, ?3, ?4)", &[&key, &title, &0, &origin])?;

        Ok(Playlist {
            key: key,
            title: title.into(),
            desc: None,
            tracks: None,
            count: 0,
            origin: origin
        })
    }

    /// Deletes a playlist with key `key`
    pub fn delete_playlist(&self, key: &str) -> Result<()> {
        self.socket.execute("DELETE FROM playlists WHERE key = ?", &[&key])
            .map(|_| ())
    }

    pub fn update_playlist(&self, key: &str, title: Option<String>, desc: Option<String>, tracks: Option<String>, count: Option<u32>, origin: Option<String>) -> Result<()> {
        if let Some(title) = title {
            self.socket.execute("UPDATE playlists SET title = ?1 WHERE Key = ?2", &[&title, &key])?;
        }
        if let Some(desc) = desc {
            self.socket.execute("UPDATE playlists SET desc = ?1 WHERE Key = ?2", &[&desc, &key])?;
        }
        if let Some(tracks) = tracks {
            self.socket.execute("UPDATE playlists SET tracks = ?1 WHERE Key = ?2", &[&tracks, &key])?;
        }
        if let Some(count) = count {
            self.socket.execute("UPDATE playlists SET count = ?1 WHERE Key = ?2", &[&count, &key])?;
        }
        if let Some(origin) = origin {
            self.socket.execute("UPDATE playlists SET origin = ?1 WHERE Key = ?2", &[&origin, &key])?;
        }

        Ok(())
    }

    /// Add a track to a certain playlist
    ///
    /// It is important that `playlist` is the title of the playlist and not the key. This method
    /// returns the updated playlist.
    pub fn add_to_playlist(&self, key: &str, playlist: &str) -> Result<Playlist> {
        let mut stmt = self.socket.prepare(
            "SELECT Key, Title, Desc, Tracks, Count, Origin
                FROM Playlists WHERE Title=?;")?;
        
        let mut query = stmt.query(&[&playlist])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let mut _playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            tracks: row.get(3),
            count: row.get(4),
            origin: row.get(5)
        };

        _playlist.count += 1;

        let keys: Option<String> = row.get(3);
        let keys: String = keys.map(|x| format!("{},{}", x, key)).unwrap_or(key.into());

        println!("Track: {}", keys);

        self.socket.execute("UPDATE playlists SET Count = ?1, tracks = ?2 WHERE Key = ?3", &[&_playlist.count, &keys, &_playlist.key])?;

        Ok(_playlist)
    }

    /// Remove a track to a certain playlist
    pub fn delete_from_playlist(&self, key: &str, playlist_key: &str) -> Result<()> {
        let mut stmt = self.socket.prepare(
            "SELECT Key, Title, Desc, Tracks, Count, Origin
                FROM Playlists WHERE Key=?;")?;
        
        let mut query = stmt.query(&[&playlist_key])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let mut _playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            tracks: row.get(3),
            count: row.get(4),
            origin: row.get(5)
        };

        _playlist.count -= 1;

        let keys: Option<String> = row.get(3);
        //let keys: String = keys.map(|x| format!("{},{}", x, key)).unwrap_or(key.into());
        let keys = keys.map(|x| { 
            let mut x = x.split(",").collect::<Vec<&str>>(); 
            x.retain(|x| x != &key);
            x.join(",")
        }).unwrap_or("".into());

        self.socket.execute("UPDATE playlists SET Count = ?1, tracks = ?2 WHERE Key = ?3", &[&_playlist.count, &keys, &_playlist.key])?;

        Ok(())
    }

    /// Insert a new track into the database
    pub fn insert_track(&self, track: Track) -> Result<()> {
        self.socket.execute("INSERT INTO music
                                    (Title, Album, Interpret, People, Composer, Key, Fingerprint, Duration, FavsCount, Channels)
                                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                    &[&track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.key, &track.fingerprint, &track.duration, &track.favs_count, &track.channels]).map(|_| ())
    }

    /// Insert a new playlist into the database
    pub fn insert_playlist(&self, p: Playlist) -> Result<()> {
        self.socket.execute("INSERT INTO playlists
                                (Key, Title, Desc, Tracks, Count, Origin)
                                VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                                &[&p.key, &p.title, &p.desc, &p.tracks, &p.count, &p.origin]).map(|_| ())
    }

    /// Delete a track with key `key`
    pub fn delete_track(&self, key: &str) -> Result<()> {
        self.socket.execute("DELETE FROM music WHERE key = ?", &[&key]).map(|_| ())
    }

    /// Get a track with key `key`
    pub fn get_track(&self, key: &str) -> Result<Track> {
        let mut stmt = self.socket.prepare(&format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE Key = '{}'", key))?;

        let mut result = self.search(&mut stmt);
        
        result.next().ok_or(Error::QueryReturnedNoRows)
    }

    /// Update the metadata of tracks
    ///
    /// In case none of the parameters is Option::Some, then no field is updated.
    pub fn update_track(&self, key: &str, title: Option<String>, album: Option<String>, interpret: Option<String>, people: Option<String>, composer: Option<String>) -> Result<String> {
        if let Some(title) = title {
            self.socket.execute("UPDATE music SET Title = ? WHERE Key = ?", &[&title, &key])?;
        }
        if let Some(album) = album {
            self.socket.execute("UPDATE music SET Album = ? WHERE Key = ?", &[&album, &key])?;
        }
        if let Some(interpret) = interpret {
            self.socket.execute("UPDATE music SET Interpret = ? WHERE Key = ?", &[&interpret, &key])?;
        }
        if let Some(people) = people {
            self.socket.execute("UPDATE music SET People = ? WHERE Key = ?", &[&people, &key])?;
        }
        if let Some(composer) = composer {
            self.socket.execute("UPDATE music SET Composer = ? WHERE Key = ?", &[&composer, &key])?;
        }

        return Ok(key.into());
    }

    /// Increment the favourite count for a track
    pub fn vote_for_track(&self, key: &str) -> Result<()> {
        self.socket.execute("UPDATE music SET FavsCount = FavsCount + 1 WHERE Key = ?1", &[&key]).map(|_| ())
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_token(&self, token: u32) -> Result<(Token, Option<(Playlist, Vec<Track>)>)> {
        let mut stmt = self.socket.prepare(
            "SELECT Token, Key, Played, Pos
                FROM Tokens WHERE Token=?;")?;
            
        let mut query = stmt.query(&[&token])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let token = Token {
            token: row.get(0),
            key: row.get(1),
            played: row.get(2),
            pos: row.get(3)
        };

        if token.key.is_empty() {
            Ok((token, None))
        } else {
            let (playlist, tracks) = self.get_playlist(&token.key)?;

            Ok((token, Some((playlist, tracks))))
        }
    }

    /// Create a new token with a valid id
    pub fn create_token(&self) -> Result<u32> {
        let id: u32 = self.socket.query_row(
            "SELECT MAX(token) FROM Tokens", &[], |row| row.get(0))?;

        // the next token id is one bigger than the largest
        let id = id + 1;

        self.socket.execute(
            "INSERT INTO Tokens(token, key, played, pos) VALUES (?1, ?2, ?3, ?4)",
                &[&id, &"", &"", &0.0]).map(|_| id)
    }

    /// Update the metadata of a token
    ///
    /// When no parameter is Option::Some no metadata will be updated.
    pub fn update_token(&self, token: u32, key: Option<String>, played: Option<String>, pos: Option<f64>) -> Result<()> {
        if let Some(key) = key {
            self.socket.execute("UPDATE tokens SET key = ?1 WHERE token = ?2", &[&key, &token]).map(|_| ())?;
        }
        if let Some(played) = played {
            self.socket.execute("UPDATE tokens SET played = ?1 WHERE token = ?2", &[&played, &token]).map(|_| ())?;
        }
        if let Some(pos) = pos {
            self.socket.execute("UPDATE tokens SET pos = ?1 WHERE token = ?2", &[&pos, &token]).map(|_| ())?;
        }

        Ok(())
    }

    /// Add a new event to the database with a timestamp
    pub fn add_event(&self, event: Event) -> Result<()> {
        self.socket.execute(
            "INSERT INTO Events (Date, Origin, Event, Data) VALUES (datetime('now'), ?1, ?2, ?3)",
                &[&event.origin(), &event.tag(), &event.data_to_string()]).map(|_| ())
    }

    /// Return all registered events
    pub fn get_events(&self) -> Vec<(String, Event)> {
        let mut stmt = self.socket.prepare(
            "SELECT Date, Origin, Event, Data FROM Events;").unwrap();

        let rows = stmt.query_map(&[], |x| {
            (x.get(0), Event::from(x.get(1), x.get(2), x.get(3)))
        }).unwrap().filter_map(|x| x.ok()).filter_map(|x| {
            if let Some(y) = x.1.ok() {
                Some((x.0, y))
            } else {
                None
            }
        }).collect();

        rows
    }

    /// Summarise a day (used by `nightly-worker`)
    pub fn summarise_day(&self, day: String, connects: u32, plays: u32, adds: u32, removes: u32) -> Result<()> {
        self.socket.execute(
            "INSERT INTO Summarise (day, connects, plays, adds, removes) VALUES (?1, ?2, ?3, ?4, ?5)",
                &[&day, &connects, &plays, &adds, &removes]).map(|_| ())
    }

    /// Get a summarise of all days since beginning of use
    pub fn get_summarisation(&self) -> Vec<(String, u32, u32, u32, u32)> {
        let mut stmt = self.socket.prepare(
            "SELECT day, connects, plays, adds, removes FROM Summarise;").unwrap();

        let rows = stmt.query_map(&[], |x| {
            (x.get(0), x.get(1), x.get(2), x.get(3), x.get(4))
        }).unwrap().filter_map(|x| x.ok()).collect();

        rows
    }

    /// Find the latest summarise day in the database
    pub fn get_newest_summarise_day(&self) -> Result<String> {
        let mut stmt = self.socket.prepare(
            "SELECT day FROM Summarise order by day desc limit 1;").unwrap();

        let mut query = stmt.query(&[]).unwrap();
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        Ok(row.get(0))
    }
}

