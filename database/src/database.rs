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

        let query = format!("SELECT * FROM Tracks WHERE hex(key) in ({});", playlist.tracks.iter().map(|key| format!("\"{}\"", key.to_string())).collect::<Vec<String>>().join(","));
        let mut stmt = self.socket.prepare(&query)?;
        let res = self.search(&mut stmt).collect();
        //let mut stmt = self.socket.prepare("SELECT * FROM Tracks WHERE hex(key) in ()");


        Ok((playlist, res))
    }

    /// Look all playlists up belonging to a certain track
    pub fn get_playlists_of_track(&self, key: TrackKey) -> Result<Vec<Playlist>> {
        let tmp = key.to_vec();
        let mut stmt = self.socket.prepare("SELECT * FROM Playlists WHERE INSTR(Tracks, ?) > 0")?;

        let res = stmt.query_map(&[&tmp], |row| Playlist::from_row(row))?.filter_map(|x| x.ok()).filter_map(|x| x.ok()).collect();

        Ok(res)
    }


    /// Create a empty playlist with a `title` and `origin`
    ///
    /// The `origin` field is only used when the playlist originates from a different server and
    /// should therefore be updated after a new version appears.
    pub fn add_playlist(&self, title: &str, origin: Option<String>) -> Result<Playlist> {
        self.socket.execute("INSERT INTO playlists (title, origin, tracks) VALUES (?1, ?2, ?3)", &[&title, &origin, &Vec::new()])?;
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

    pub fn update_playlist(&self, key: PlaylistKey, title: Option<String>, desc: Option<String>, origin: Option<String>) -> Result<()> {
        let mut query = String::from("UPDATE Playlists SET ");

        let mut parts = Vec::new();
        if title.is_some() { parts.push("title = ?1"); }
        if desc.is_some() { parts.push("desc = ?2"); }
        if origin.is_some() { parts.push("origin = ?3"); }

        query.push_str(&parts.join(", "));
        query.push_str(" WHERE Key = ?4;");

        self.socket.execute(&query, &[&title, &desc, &origin, &key]).map(|_| ())
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

        let tracks: Vec<u8> = row.get(0);
        let mut tracks: Vec<TrackKey> = tracks.chunks(16)
            .map(|x| TrackKey::from_vec(x)).collect();

        tracks.push(key);
        let tracks: Vec<u8> = tracks.into_iter().flat_map(|x| x.to_vec()).collect();

        self.socket.execute("UPDATE Playlists SET tracks = ?1 WHERE Key = ?2", &[&tracks, &playlist])?;

        Ok(())
    }

    /// Remove a track to a certain playlist
    pub fn delete_from_playlist(&self, key: TrackKey, playlist: PlaylistKey) -> Result<()> {
        let mut stmt = self.socket.prepare(
            "SELECT tracks FROM Playlists WHERE Key=?;")?;
        
        let mut query = stmt.query(&[&playlist])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let tracks: Vec<u8> = row.get(0);
        let mut tracks: Vec<TrackKey> = tracks.chunks(16)
            .map(|x| TrackKey::from_vec(x)).collect();

        let index = tracks.iter().position(|x| x.to_vec() == key.to_vec())
            .ok_or(Error::QueryReturnedNoRows)?;

        tracks.remove(index);
        let tracks: Vec<u8> = tracks.into_iter().flat_map(|x| x.to_vec()).collect();

        self.socket.execute("UPDATE Playlists SET tracks = ?1 WHERE Key = ?2", &[&tracks, &playlist])?;

        Ok(())
    }

    /// Insert a new track into the database
    pub fn insert_track(&self, track: Track) -> Result<()> {
        let buf = objects::u32_into_u8(track.fingerprint.clone());

        self.socket.execute("INSERT INTO Tracks
                                    (Key, Fingerprint, Title, Album, Interpret, People, Composer, Duration, FavsCount, Created)
                                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, DATETIME('NOW'))",
                                    &[&track.key.to_vec(), &buf, &track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.duration, &track.favs_count]).map(|_| ())
    }

    /// Insert a new playlist into the database
    pub fn insert_playlist(&self, p: Playlist) -> Result<()> {
        let tracks: Vec<u8> = p.tracks.iter().flat_map(|x| x.to_vec()).collect();

        self.socket.execute("INSERT INTO Playlists
                                (Key, Title, Desc, Tracks, Origin)
                                VALUES (?1, ?2, ?3, ?4, ?5)",
                                &[&p.key, &p.title, &p.desc, &tracks, &p.origin]).map(|_| ())
    }

    /// Delete a track with key `key`
    pub fn delete_track(&self, key: TrackKey) -> Result<()> {
        self.socket.execute("DELETE FROM Tracks WHERE Key = ?", &[&key.to_vec()]).map(|_| ())
    }

    /// Get a track with key `key`
    pub fn get_track(&self, key: TrackKey) -> Result<Track> {
        let mut stmt = self.socket.prepare("SELECT * FROM Tracks WHERE Key = ?")?;
        let mut stream = stmt.query_map(&[&key.to_vec()], |row| Track::from_row(row)).unwrap().filter_map(|x| x.ok()).filter_map(|x| x.ok());

        stream.next().ok_or(Error::QueryReturnedNoRows)
    }

    /// Update the metadata of tracks
    ///
    /// In case none of the parameters is Option::Some, then no field is updated.
    pub fn update_track(&self, key: TrackKey, title: Option<&str>, album: Option<&str>, interpret: Option<&str>, people: Option<&str>, composer: Option<&str>) -> Result<TrackKey> {
        let mut query = String::from("UPDATE Tracks SET ");

        let mut parts = Vec::new();
        if title.is_some() { parts.push("title = ?1"); }
        if album.is_some() { parts.push("album = ?2"); }
        if interpret.is_some() { parts.push("interpret = ?3"); }
        if people.is_some() { parts.push("people = ?4"); }
        if composer.is_some() { parts.push("composer = ?5"); }

        query.push_str(&parts.join(", "));
        query.push_str(" WHERE Key = ?6;");

        self.socket.execute(&query, &[&title, &album, &interpret, &people, &composer, &key.to_vec()]).map(|_| key)
    }

    /// Increment the favourite count for a track
    pub fn vote_for_track(&self, key: TrackKey) -> Result<()> {
        self.socket.execute("UPDATE Tracks SET FavsCount = FavsCount + 1 WHERE Key = ?1", &[&key.to_vec()]).map(|_| ())
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

    /// Get the last used token
    pub fn get_last_used_token(&self) -> Result<(Token, Option<(Playlist, Vec<Track>)>)> {
        let mut stmt = self.socket.prepare(
            "SELECT * FROM Tokens ORDER BY datetime(lastuse) DESC Limit 1")?;

        let mut query = stmt.query(&[])?;
        let token = query.next().ok_or(Error::QueryReturnedNoRows)?.map(|row| Token::from_row(&row))??;

        if let Some(playlist) = token.key {
            let (playlist, tracks) = self.get_playlist(playlist)?;

            Ok((token, Some((playlist, tracks))))
        } else {
            Ok((token, None))
        }
    }

    pub fn use_token(&self, token: TokenId) -> Result<()> {
        self.socket.execute(
            "UPDATE Tokens SET lastuse = DATETIME('now') WHERE token = ?",&[&token])
            .map(|_| ())

    }

    /// Create a new token with a valid id
    pub fn create_token(&self) -> Result<TokenId> {
        let empty = Vec::new();

        self.socket.execute(
            "INSERT INTO Tokens(played, pos, counter, lastuse) VALUES (?1, ?2, ?3, DATETIME('now'))",
                &[&empty, &0.0, &0])?;

        Ok(self.socket.last_insert_rowid())
    }

    /// Update the metadata of a token
    ///
    /// When no parameter is Option::Some no metadata will be updated.
    pub fn update_token(&self, token: TokenId, key: Option<PlaylistKey>, played: Option<Vec<TrackKey>>, pos: Option<f64>) -> Result<()> {
        let mut query = String::from("UPDATE Tokens SET ");

        let mut parts = vec!["counter = counter+1"];
        if key.is_some() { parts.push("key = ?1"); }
        if played.is_some() { parts.push("played = ?2"); }
        if pos.is_some() { parts.push("pos = ?3"); }

        query.push_str(&parts.join(", "));
        query.push_str(" WHERE token = ?4;");

        let played: Option<Vec<u8>> = played.map(|x|x.into_iter().flat_map(|x| x.to_vec()).collect());

        self.socket.execute(&query, &[&key, &played, &pos, &token]).map(|_| ())
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

#[cfg(test)]
mod tests {
    use super::Collection;
    use objects::{Playlist, Track};
    use search::SearchQuery;

    fn gen_track() -> Track {
        let mut track = Track::empty(vec![1i32; 10], 100.0);
        track.title = Some("Blue like something".into());
        track.composer = Some("Random Guy".into());

        track
    }

    #[test]
    pub fn test_search() {
        let db = Collection::in_memory();

        let track = gen_track();

        db.insert_track(track.clone()).unwrap();

        // create a new search query
        let query = SearchQuery::new("title:Blue").unwrap();

        // initiate the search
        let mut stmt = db.search_prep(query).unwrap();
        assert_eq!(track, db.search(&mut stmt).next().unwrap());
    }

    #[test]
    pub fn test_playlist() {
        let db = Collection::in_memory();

        let track = gen_track();
        let playlist = Playlist {
            key: 30,
            title: "My very own playlist".into(),
            desc: Some("".into()),
            tracks: vec![],
            origin: None
        };

        // check if there are no playlists in the database
        assert_eq!(db.get_playlists().len(), 0);

        // add a new playlist to the database
        db.insert_playlist(playlist.clone()).unwrap();
        assert_eq!(db.get_playlists(), vec![playlist.clone()]);
        assert_eq!(db.get_playlist(playlist.key).unwrap().0, playlist);

        // update the playlist, add a desc
        let new_desc = Some("Even with a description".into());
        db.update_playlist(playlist.key, None, new_desc.clone(), None).unwrap();
        assert_eq!(db.get_playlist(playlist.key).unwrap().0.desc, new_desc);
        db.update_playlist(playlist.key, None, Some("".into()), None).unwrap();

        // add a track to the playlist
        db.insert_track(track.clone()).unwrap();
        db.add_to_playlist(track.key, playlist.key).unwrap();
        assert_eq!(db.get_playlists_of_track(track.key).unwrap()[0].title, playlist.title);
        
        // remove the track from the playlist
        db.delete_from_playlist(track.key, playlist.key).unwrap();
        assert_eq!(db.get_playlists_of_track(track.key).unwrap().len(), 0);

    }

    #[test]
    pub fn test_tracks() {
        let db = Collection::in_memory();

        // create a new track
        let track = gen_track();
        db.insert_track(track.clone()).unwrap();

        // vote ten times for this track
        for _ in 0..10 {
            db.vote_for_track(track.key).unwrap();
        }

        assert_eq!(db.get_track(track.key).unwrap().favs_count, track.favs_count + 10);

        // update the track metadata
        let (title, album, interpret, people, composer) = (
            "Eye in the Sky", "Live", "Alan Parsons", "Alan Parsons", "Alan Parsons");
        
        db.update_track(track.key, Some(title), Some(album), Some(interpret), None, Some(composer)).unwrap();

        let tmp = db.get_track(track.key).unwrap();
        assert!(
            tmp.title == Some(title.into()) && 
            tmp.album == Some(album.into()) && 
            tmp.interpret == Some(interpret.into()) &&
            tmp.people == None,
            tmp.composer == Some(composer.into())
        );

        db.delete_track(track.key).unwrap();

        assert_eq!(db.get_tracks().len(), 0);
    }

    #[test]
    pub fn test_tokens() {
        let db = Collection::in_memory();

        //create a track and playlist
        let track = gen_track();
        let playlist = Playlist {
            key: 30,
            title: "My very own playlist".into(),
            desc: Some("".into()),
            tracks: vec![],
            origin: None
        };

        // setup up track and plalist
        db.insert_track(track.clone()).unwrap();
        db.insert_playlist(playlist.clone()).unwrap();
        db.add_to_playlist(track.key, playlist.key).unwrap();

        assert_eq!(db.create_token().unwrap(), 1);

        // set the playlist as a token and compare everything
        db.update_token(1, Some(playlist.key), None, Some(5.0)).unwrap();

        let (tmp, rem) = db.get_token(1).unwrap();
        let rem = rem.unwrap();
        assert!(tmp.key.unwrap() == playlist.key && tmp.pos == Some(5.0));
        assert_eq!(tmp.counter, 1);
        assert_eq!(rem.0.title, playlist.title);
        assert_eq!(rem.1[0].key, track.key);
        
        // create ten tokens
        for _ in 0..10 {
            db.create_token().unwrap();
        }

        // the next one should have the id 12
        assert_eq!(db.create_token().unwrap(), 12);
        
        // update the 6th token
        db.use_token(6).unwrap();

        let (token, _) = db.get_last_used_token().unwrap();
    }

}
