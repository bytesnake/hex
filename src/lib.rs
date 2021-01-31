use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use std::fs::File;
use serde::{Serialize, Deserialize};

mod error;

pub use error::{Result, StoreError};

#[derive(Debug, Deserialize, Serialize)]
pub struct Playlist {
    name: String,
    #[serde(default)]
    card_id: Option<u32>,
    #[serde(default)]
    allow_random: bool,
    #[serde(default)]
    radio_url: Option<String>,
}

pub struct Store {
    root_path: PathBuf,
    playlists: Vec<Playlist>
}

impl Store {
    /// Load a music store from a path
    ///
    /// All music is stored inside a single folder. The file `/Music.toml` describes playlists and
    /// their propertiers. The actual music files are stored inside `/files/*/*.flac`. 
    ///
    /// # Examples
    /// ```
    /// use base::Store;
    /// let store = Store::from_path("/home/lorenz/music/").unwrap();
    /// ```
    pub fn from_path<T: AsRef<Path>>(path: T) -> Result<Store> {
        // convert parameter (may be a string) to path reference
        let path = path.as_ref();

        // open configuration file
        let mut f = File::open(path.join("Music.toml"))
            .map_err(|e| StoreError::ConfMissing(path.to_path_buf(), e))?;

        // load file into string
        let mut source = String::new();
        f.read_to_string(&mut source)?;

        // parse and deserialize string to a vector of playlists
        let playlists: Vec<Playlist> = toml::from_str(&source)?;

        Ok(Store {
            root_path: path.to_path_buf(),
            playlists
        })
    }

    /// Save the playlists configuration to a file
    ///
    /// This converts `self.playlists` to string by serializing it with TOML and then writes the
    /// string to the `Music.toml` file. An error may occure when the file can't be open or written
    /// to
    pub fn save(&self) -> Result<()> {
        let self_str = toml::to_string(&self.playlists)?;

        let mut f = File::open(self.root_path.join("Music.toml"))
            .map_err(|e| StoreError::ConfMissing(self.root_path.to_path_buf(), e))?;

        f.write(self_str.as_bytes())?;

        Ok(())
    }

    /// Return a vector of all playlists
    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists 
    }

    /// Search for a playlist with a name
    pub fn playlist_by_name(&mut self, name: &str) -> Result<&mut Playlist> {
        self.playlists.iter_mut()
            .filter(|x| x.name == name)
            .next()
            .ok_or(StoreError::PlaylistNotFound(name.into()))
    }
    ///
    /// Search for a playlist by the playlist ID
    pub fn playlist_by_card(&mut self, id: u32) -> Result<&mut Playlist> {
        self.playlists.iter_mut()
            .filter(|x| x.card_id.map(|x| x == id).unwrap_or(false))
            .next()
            .ok_or(StoreError::PlaylistNotFound(format!("card {}", id)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
