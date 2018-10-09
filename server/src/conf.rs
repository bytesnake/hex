//! Parse configuration or use default values
//!
//! This module parses the configuration in TOML style and replaces any missing value with
//! defaults. The configuration can later be used in the webserver, sync server and websocket
//! server to behave in the expected way.

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::net::{IpAddr, Ipv4Addr};

use toml;
use std::default::Default;

use error::{Error, Result};

/// The websocket server configuration
#[derive(Deserialize, Debug)]
pub struct Server {
    #[serde(default = "default_port")]
    pub port: u16
}

/// Default host is localhost
fn default_host() -> IpAddr { IpAddr::V4(Ipv4Addr::LOCALHOST) }
/// Default port of the websocket server is 2798
fn default_port() -> u16 { 2798 }
/// Default port of the webserver is 80
fn default_port_web() -> u16 { 80 }
/// Default port of the sync server is 8004
fn default_port_sync() -> u16 { 8004 }

impl Default for Server {
    fn default() -> Self {
        Server {
            port: 2798
        }
    }
}

/// Path to the music database and data section
#[derive(Deserialize, Debug, Clone)]
pub struct Music {
    pub db_path: PathBuf,
    pub data_path: PathBuf
}

impl Default for Music {
    /// By default in the home folder
    fn default() -> Self {
        let home = env::home_dir().expect("Could not found a home directory!");

        Music {
            data_path: home.join(".music"),
            db_path: home.join(".music.db")
        }
    }
}

/// Webserver configuration
#[derive(Deserialize, Debug, Clone)]
pub struct WebServer {
    /// Path to the frontend is required
    pub path: PathBuf,
    /// The webserver port is optional and defaults to 80
    #[serde(default = "default_port_web")]
    pub port: u16
}

/// Sync server configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Syncc {
    /// The sync process can take all available audio data, only useful in server with alot of free
    /// dataspace
    #[serde(default)]
    pub sync_all: bool,
    /// Unique name of the peer as known to all other peers, is required
    pub name: String,
    /// The sync server port is optional and defaults to 8004
    #[serde(default = "default_port_sync")]
    pub port: u16
}

/// Global configuration
#[derive(Deserialize,Debug)]
pub struct Conf {
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub music: Music,
    pub webserver: Option<WebServer>,
    pub sync: Option<Syncc>

}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            host: default_host(),
            server: Server::default(),
            music: Music::default(),
            webserver: None,
            sync: None
        }
    }
}

impl Conf {
    /// Load the configuration from a file and converts it into the `Conf` struct.
    pub fn from_file(path: &Path) -> Result<Conf> {
        let mut file = File::open(path).map_err(|_| Error::Configuration)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| Error::Configuration)?;

        toml::from_str(&contents)
            .map_err(|_| Error::Configuration)
    }
}
