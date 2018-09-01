extern crate rusqlite;
extern crate uuid;

pub mod objects;
pub mod search;

pub use objects::{Track, Playlist, Token};
pub use rusqlite::{Result, Statement, Error};

use uuid::Uuid;

use search::SearchQuery;

pub struct Collection {
    socket: rusqlite::Connection
}

impl Collection {
    pub fn from_file(path: &str) -> Collection {
        Collection { socket: rusqlite::Connection::open(path).unwrap() }
    }

    /// Search for a certain track. The SearchQuery ensures that a valid string is generated.
    /// TODO: search for tracks in a certain playlist

    pub fn search_prep(&self, query: SearchQuery) -> Result<Statement> {
        let query = query.to_sql_query();

        println!("Query: {}", query);
        self.socket.prepare(&query)
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
        }).unwrap().filter_map(|x| x.ok())
    }

    pub fn search_limited(&self, query: &str, start: usize) -> Result<Vec<Track>> {
        let query = SearchQuery::new(query).ok_or(Error::QueryReturnedNoRows)?;

        let mut stmt = self.search_prep(query)?;
        let res = self.search(&mut stmt).skip(start).take(50).collect();

        Ok(res)
    }

    /// Returns the metadata of all available playlists
    pub fn get_playlists(&self) -> Vec<Playlist> {
        let mut stmt = self.socket.prepare("SELECT Key, Title, Desc, Count FROM Playlists").unwrap();

        let vec = stmt.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                count: row.get(3)
            }
        }).unwrap().filter_map(|x| x.ok()).collect();

        vec
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_playlist(&self, key: &str) -> Result<(Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare(
            "SELECT Key, Title, Desc, Count, tracks 
                FROM Playlists WHERE key=?;")?;

        let mut query = stmt.query(&[&key])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let playlist = Playlist {
            key: row.get(0),
            title: row.get(1),
            desc: row.get(2),
            count: row.get(3)
        };

        let keys: Option<String> = row.get(4);

        if let Some(keys) = keys {
            let query = format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE key in ({});", keys.split(",").map(|row| { format!("'{}'", row) }).collect::<Vec<String>>().join(","));

            let mut stmt = self.socket.prepare(&query)?;

            let res = self.search(&mut stmt).collect();

            Ok((playlist, res))
        } else {
            Ok((playlist, Vec::new()))
        }
    }

    pub fn get_playlists_of_track(&self, key: &str) -> Result<Vec<Playlist>> {
        let mut stmt = self.socket.prepare(&format!("SELECT Key, Title, Desc, Count FROM Playlists WHERE tracks like '%{}%'", key))?;

        let res = stmt.query_map(&[], |row| {
            Playlist {
                key: row.get(0),
                title: row.get(1),
                desc: row.get(2),
                count: row.get(3)
            }
        })?.filter_map(|x| x.ok()).collect();

        Ok(res)
    }


    pub fn add_playlist(&self, title: &str) -> Result<Playlist> {
        let key = Uuid::new_v4().simple().to_string();

        self.socket.execute("INSERT INTO playlists (key, title, count) VALUES (?1, ?2, ?3)", &[&key, &title, &0])?;

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
    }

    pub fn update_playlist(&self, key: &str, title: Option<String>, desc: Option<String>) -> Result<()> {
        if let Some(title) = title {
            self.socket.execute("UPDATE playlists SET title = ?1 WHERE Key = ?2", &[&title, &key])?;
        }
        if let Some(desc) = desc {
            self.socket.execute("UPDATE playlists SET desc = ?1 WHERE Key = ?2", &[&desc, &key])?;
        }

        Ok(())
    }

    pub fn add_to_playlist(&self, key: &str, playlist: &str) -> Result<Playlist> {
        let mut stmt = self.socket.prepare(
            "SELECT Key, Title, Desc, Count, tracks 
                FROM Playlists WHERE Title=?;")?;
        
        let mut query = stmt.query(&[&playlist])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

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

        self.socket.execute("UPDATE playlists SET Count = ?1, tracks = ?2 WHERE Key = ?3", &[&_playlist.count, &keys, &_playlist.key])?;

        Ok(_playlist)
    }

    pub fn insert_track(&self, track: Track) -> Result<()> {
        self.socket.execute("INSERT INTO music
                                    (Title, Album, Interpret, People, Composer, Key, Fingerprint, Duration, FavsCount, Channels)
                                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                    &[&track.title, &track.album, &track.interpret, &track.people, &track.composer, &track.key, &track.fingerprint, &track.duration, &track.favs_count, &track.channels]).map(|_| ())
    }

    pub fn delete_track(&self, key: &str) -> Result<()> {
        self.socket.execute("DELETE FROM music WHERE key = ?", &[&key]).map(|_| ())
    }

    pub fn get_track(&self, key: &str) -> Result<Track> {
        let mut stmt = self.socket.prepare(&format!("SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music WHERE Key = '{}'", key))?;

        let mut result = self.search(&mut stmt);
        
        result.next().ok_or(Error::QueryReturnedNoRows)
    }
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

    pub fn vote_for_track(&self, key: &str) -> Result<()> {
        self.socket.execute("UPDATE music SET FavsCount = FavsCount + 1 WHERE Key = ?1", &[&key]).map(|_| ())
    }

    /// Get the metadata and tracks for a certain playlist
    pub fn get_token(&self, token: &str) -> Result<(Token, Playlist, Vec<Track>)> {
        let mut stmt = self.socket.prepare(
            "SELECT Token, Key, Pos, Completion
                FROM Tokens WHERE Token=?;")?;
            
        let mut query = stmt.query(&[&token])?;
        let row = query.next().ok_or(Error::QueryReturnedNoRows)??;

        let token = Token {
            token: row.get(0),
            key: row.get(1),
            pos: row.get(2),
            completion: row.get(3)
        };

        let (playlist, tracks) = self.get_playlist(&token.key)?;

        Ok((token, playlist, tracks))
    }

    pub fn insert_token(&self, token: Token) -> Result<()> {
        self.socket.execute(
            "INSERT INTO Tokens(token, key, pos, completion) VALUES (?1, ?2, ?3, ?4)",
                &[&token.token, &token.key, &token.pos, &token.completion]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
