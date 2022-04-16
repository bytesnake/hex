use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use std::fs::File;
use serde::{Serialize, Deserialize};

mod error;

pub use error::{Result, StoreError};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlaylistStatus {
    card_id: u32,
    #[serde(default)]
    current_pos: Option<(usize, f32)>,
    #[serde(default)]
    random_seed: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Playlist {
    #[serde(default)]
    pub allow_random: bool,
    #[serde(default)]
    pub radio_url: Option<String>,
    #[serde(skip)]
    pub files: Vec<PathBuf>,
    #[serde(skip)]
    status: Option<PlaylistStatus>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Store {
    #[serde(skip)]
    root_path: PathBuf,
    playlists: HashMap<String, Playlist>,
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
        let mut store: HashMap<String,Playlist> = toml::from_str(&source)?;
        let root_path = path.to_path_buf();

        // open music position file
        let mut f = File::open(path.join("Zyklop.toml"))
            .map_err(|e| StoreError::ConfMissing(path.to_path_buf(), e))?;

        // load file into string
        let mut source = String::new();
        f.read_to_string(&mut source)?;

        // parse and deserialize string to a vector of playlists
        let stati: HashMap<String, PlaylistStatus> = toml::from_str(&source)?;

        dbg!(&stati);

        for (key, status) in stati {
            let mut playlist = match store.get_mut(&key) {
                Some(x) => x,
                None => continue
            };

            playlist.status = Some(status);
        }
        dbg!(&store);

        for (name, playlist) in &mut store {
            if playlist.radio_url.is_none() {
                playlist.files = std::fs::read_dir(&root_path.join("files").join(&name)).unwrap()
                    .filter_map(|x| x.ok())
                    .map(|x| x.path())
                    .collect();
            }
        }

        Ok(Store {
            playlists: store,
            root_path,
        })
    }

    /// Save the playlists configuration to a file
    ///
    /// This converts `self.playlists` to string by serializing it with TOML and then writes the
    /// string to the `Music.toml` file. An error may occure when the file can't be open or written
    /// to
    pub fn save(&mut self) -> Result<()> {
        let stati = self.playlists.iter_mut()
            .filter_map(|(name, x)| x.status.take().map(|x| (name.clone(), x)))
            .collect::<HashMap<_, _>>();

        let self_str = toml::to_string(&self.playlists)?;

        let mut f = File::create(self.root_path.join("Music.toml"))
            .map_err(|e| StoreError::ConfMissing(self.root_path.to_path_buf(), e))?;

        f.write_all(self_str.as_bytes())?;

        let stati = toml::to_string(&stati)?;

        let mut f = File::create(self.root_path.join("Zyklop.toml"))
            .map_err(|e| StoreError::ConfMissing(self.root_path.to_path_buf(), e))?;

        f.write_all(stati.as_bytes())?;

        Ok(())
    }
}
/*

    /// Return a vector of all playlists
    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists 
    }

    /// Return all playlists which do not have a card
    pub fn playlists_without_card(&self) -> Vec<Playlist> {
        self.playlists.iter().filter(|x| x.card_id.is_none())
            .cloned()
            .collect()
    }

    /// Return next card id, not used by anyone
    pub fn next_card_id(&self) -> u32 {
        let mut ids = self.playlists.iter().filter_map(|x| x.card_id).collect::<Vec<_>>();
        ids.sort();

        for sl in ids.windows(2) {
            let (a,b) = (sl[0], sl[1]);

            if a+1 != b {
                return a + 1;
            }
        }

        if ids.len() == 0 {
            0
        } else {
            return ids[ids.len()-1] + 1;
        }
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

    /// Get files from folder
    pub fn get_files(&self, name: &str) -> Vec<PathBuf> {
        self.playlists.iter()
            .filter(|x| x.name == name)
            .next()
            .ok_or(StoreError::PlaylistNotFound(name.into()))
            .map(|x| x.files.clone())
            .unwrap_or(vec![])
    }

    /// Set playlist card id
    pub fn set_playlist_card_id(&mut self, name: &str, id: u32) -> Result<()> {
        self.playlist_by_name(name)?.card_id = Some(id);

        Ok(())
    }

    /// Set playlist card id
    pub fn set_position(&mut self, name: &str, pos: usize) -> Result<()> {
        self.playlist_by_name(name)?.position = Some((pos, 0));

        Ok(())
    }
}*/

impl Drop for Store {
    fn drop(&mut self) {
        if let Err(err) = self.save() {
            eprintln!("{:?}", err);
        }
    }
}
