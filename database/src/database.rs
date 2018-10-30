use std::path::Path;
use rusqlite::{self, Result, Statement, Error};
use search::SearchQuery;
use objects::{self, *};
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
    
        // create the necessary tables (if not already existing)
        socket.execute_batch(include_str!("create_db.sql")).unwrap();

        Collection { socket }
    }

    pub fn in_memory() -> Collection {
        let socket = rusqlite::Connection::open_in_memory().unwrap();

        socket.execute_batch(include_str!("create_db.sql")).unwrap();

        Collection { socket }
    }

    /// Prepare a search with a provided query and translate it to SQL. This method fails in case
    /// of an invalid query.
    pub fn search_prep(&self, query: SearchQuery) -> Result<Statement> {
        let query = query.to_sql_query();

        self.socket.prepare(&query)
    }

    /// Execute the prepared search and return an iterator over all results
    pub fn search<'a>(&self, stmt: &'a mut Statement) -> impl Iterator<Item = Track> + 'a {
        stmt.query_map(&[], |row| Track::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok())
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

    /// Get a playlist with a certain key and return the metadata and tracks
    pub fn get_playlist(&self, key: PlaylistKey) -> Result<(Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare(
            "SELECT * FROM Playlists WHERE Key=?;")?;

        let mut query = stmt.query(&[&key])?;
        let playlist = query.next().ok_or(Error::QueryReturnedNoRows)?.map(|row| Playlist::from_row(&row))??;

        let query = format!("SELECT * FROM Tracks WHERE key in ({});", playlist.tracks.iter().map(|key| key.to_string()).collect::<Vec<String>>().join(","));
        let mut stmt = self.socket.prepare(&query)?;
        let res = self.search(&mut stmt).collect();

        Ok((playlist, res))
    }

    /// Look all playlists up belonging to a certain track
    pub fn get_playlists_of_track(&self, key: TrackKey) -> Result<Vec<Playlist>> {
        let mut stmt = self.socket.prepare(&format!("SELECT * FROM Playlists WHERE Tracks like '%{}%'", key))?;

        let res = stmt.query_map(&[], |row| Playlist::from_row(row))?.filter_map(|x| x.ok()).filter_map(|x| x.ok()).collect();

        Ok(res)
    }


    /// Create a empty playlist with a `title` and `origin`
    ///
    /// The `origin` field is only used when the playlist originates from a different server and
    /// should therefore be updated after a new version appears.
    pub fn add_playlist(&self, title: &str, origin: Option<String>) -> Result<Playlist> {
        self.socket.execute("INSERT INTO playlists (title, origin) VALUES (?1, ?2, ?3)", &[&title, &origin])?;
        let rowid = self.socket.last_insert_rowid();

        Ok(Playlist {
            key: rowid,
            title: title.into(),
            desc: None,
            tracks: Vec::new(),
            origin: origin
        })
    }

    /// Deletes a playlist with key `key`
    pub fn delete_playlist(&self, key: PlaylistKey) -> Result<()> {
        self.socket.execute("DELETE FROM Playlists WHERE Key = ?", &[&key])
            .map(|_| ())
    }

    pub fn update_playlist(&self, key: PlaylistKey, title: Option<String>, desc: Option<String>, tracks: Option<Vec<i32>>, origin: Option<String>) -> Result<()> {
        /*if let Some(title) = title {
            self.socket.execute("UPDATE playlists SET title = ?1 WHERE Key = ?2", &[&title, &key])?;
        }
        if let Some(desc) = desc {
            self.socket.execute("UPDATE playlists SET desc = ?1 WHERE Key = ?2", &[&desc, &key])?;
        }
        if let Some(tracks) = tracks {
            self.socket.execute("UPDATE playlists SET tracks = ?1 WHERE Key = ?2", &[&tracks, &key])?;
        }
        if let Some(origin) = origin {
            self.socket.execute("UPDATE playlists SET origin = ?1 WHERE Key = ?2", &[&origin, &key])?;
        }*/
        self.socket.execute("UPDATE Playlists SET title = ?1, desc = ?2, tracks = ?3, origin = ?4 WHERE Key = ?5", &[&title, &desc, &tracks.map(|x| objects::i32_into_u8(x)), &origin, &key])
            .map(|_| ())
    }

    /// Add a track to a certain playlist
    ///
    /// It is important that `playlist` is the title of the playlist and not the key. This method
    /// returns the updated playlist.
    pub fn add_to_playlist(&self, key: TrackKey, playlist: PlaylistKey) -> Result<()> {
        let mut stmt = self.socket.prepare(
            "SELECT tracks FROM Playlists WHERE Key=?;")?;
        
        let mut query = stmt.query(&[&playlist])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let mut tracks = objects::u8_into_i64(row.get(0));
        tracks.push(key);
        let buf = objects::i64_into_u8(tracks);

        self.socket.execute("UPDATE Playlists SET tracks = ?1 WHERE Key = ?2", &[&buf, &playlist])?;

        Ok(())
    }

    /// Remove a track to a certain playlist
    pub fn delete_from_playlist(&self, key: TrackKey, playlist: PlaylistKey) -> Result<()> {
        let mut stmt = self.socket.prepare(
            "SELECT tracks FROM Playlists WHERE Key=?;")?;
        
        let mut query = stmt.query(&[&playlist])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let mut tracks = objects::u8_into_i64(row.get(0));
        let index = tracks.iter().position(|x| *x == key)
            .ok_or(Error::QueryReturnedNoRows)?;

        tracks.remove(index);
        let buf = objects::i64_into_u8(tracks);

        self.socket.execute("UPDATE Playlists SET tracks = ?1 WHERE Key = ?2", &[&buf, &playlist])?;

        Ok(())
    }

    /// Insert a new track into the database
    pub fn insert_track(&self, track: Track) -> Result<()> {
        let buf = objects::i32_into_u8(track.fingerprint.clone());

        self.socket.execute("INSERT INTO Tracks
                                    (Key, Fingerprint, Title, Album, Interpret, People, Composer, Duration, FavsCount)
                                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                    &[&track.key, &buf, &track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.duration, &track.favs_count]).map(|_| ())
    }

    /// Insert a new playlist into the database
    pub fn insert_playlist(&self, p: Playlist) -> Result<()> {
        let tracks = objects::i64_into_u8(p.tracks.clone());

        self.socket.execute("INSERT INTO Playlists
                                (Key, Title, Desc, Tracks, Origin)
                                VALUES (?1, ?2, ?3, ?4, ?5)",
                                &[&p.key, &p.title, &p.desc, &tracks, &p.origin]).map(|_| ())
    }

    /// Delete a track with key `key`
    pub fn delete_track(&self, key: TrackKey) -> Result<()> {
        self.socket.execute("DELETE FROM Tracks WHERE Key = ?", &[&key]).map(|_| ())
    }

    /// Get a track with key `key`
    pub fn get_track(&self, key: TrackKey) -> Result<Track> {
        let mut stmt = self.socket.prepare(&format!("SELECT * FROM Tracks WHERE Key = '{}'", key))?;

        let mut result = self.search(&mut stmt);
        
        result.next().ok_or(Error::QueryReturnedNoRows)
    }

    /// Update the metadata of tracks
    ///
    /// In case none of the parameters is Option::Some, then no field is updated.
    pub fn update_track(&self, key: TrackKey, title: Option<String>, album: Option<String>, interpret: Option<String>, people: Option<String>, composer: Option<String>) -> Result<TrackKey> {
        self.socket.execute("UPDATE Tracks SET title = ?1, album = ?2, interpret = ?3, people = ?4, composer = ?5 WHERE Key = ?6", &[&title, &album, &interpret, &people, &composer, &key])?;
    /*
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
        }*/

        return Ok(key);
    }

    /// Increment the favourite count for a track
    pub fn vote_for_track(&self, key: TrackKey) -> Result<()> {
        self.socket.execute("UPDATE Tracks SET FavsCount = FavsCount + 1 WHERE Key = ?1", &[&key]).map(|_| ())
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_token(&self, token: TokenId) -> Result<(Token, Option<(Playlist, Vec<Track>)>)> {
        let mut stmt = self.socket.prepare(
            "SELECT * FROM Tokens WHERE Token=?;")?;
            
        let mut query = stmt.query(&[&token])?;
        let token = query.next().ok_or(Error::QueryReturnedNoRows)?.map(|row| Token::from_row(&row))??;

        if let Some(playlist) = token.key {
            let (playlist, tracks) = self.get_playlist(playlist)?;

            Ok((token, Some((playlist, tracks))))
        } else {
            Ok((token, None))
        }
    }

    /// Create a new token with a valid id
    pub fn create_token(&self) -> Result<TokenId> {
        let empty = Vec::new();

        self.socket.execute(
            "INSERT INTO Tokens(played, pos, counter) VALUES (?1, ?2, ?3)",
                &[&empty, &0.0, &0])?;

        Ok(self.socket.last_insert_rowid())
    }

    /// Update the metadata of a token
    ///
    /// When no parameter is Option::Some no metadata will be updated.
    pub fn update_token(&self, token: TokenId, key: Option<String>, played: Option<Vec<i64>>, pos: Option<f64>) -> Result<()> {
        self.socket.execute("UPDATE Tokens SET Key = ?1, Played = ?2, Pos = ?3, counter = counter+1 WHERE token = ?4", &[&key, &played.map(|x| objects::i64_into_u8(x)), &pos, &token]).map(|_| ())?;
        /*
        if let Some(key) = key {
            self.socket.execute("UPDATE tokens SET key = ?1 WHERE token = ?2", &[&key, &token]).map(|_| ())?;
        }
        if let Some(played) = played {
            self.socket.execute("UPDATE tokens SET played = ?1 WHERE token = ?2", &[&played, &token]).map(|_| ())?;
        }
        if let Some(pos) = pos {
            self.socket.execute("UPDATE tokens SET pos = ?1 WHERE token = ?2", &[&pos, &token]).map(|_| ())?;
        }*/

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
