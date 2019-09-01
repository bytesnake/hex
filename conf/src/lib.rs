
//! Parse configuration or use default values
//!
//! This module parses the configuration in TOML style and replaces any missing value with
//! defaults. The configuration can later be used in the webserver, sync server and websocket
//! server to behave in the expected way.

#[macro_use]
extern crate serde;
extern crate toml;

pub mod error;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub use self::error::*;

use std::default::Default;

/// The websocket server configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Server {
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Default host is localhost
fn default_host() -> IpAddr { IpAddr::V4(Ipv4Addr::LOCALHOST) }
/// Default port of the websocket server is 2798
fn default_port() -> u16 { 2798 }
/// Default port of the webserver is 80
fn default_port_web() -> u16 { 80 }
/// Default port of the database peer is 8004
fn default_port_dbpeer() -> u16 { 8004 }
fn default_discover() -> bool { true }

impl Default for Server {
    fn default() -> Self {
        Server {
            port: 2798,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpotifyAPI {
    pub id: String,
    pub secret: String
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
pub struct DatabasePeer {
    /// The sync process can take all available audio data, only useful in server with alot of free
    /// dataspace
    #[serde(default)]
    pub sync_all: bool,
    /// Unique identification to all other peers (256bits)
    pub id: String,
    /// Network key (256bits)
    pub network: String,
    /// The sync server port is optional and defaults to 8004
    #[serde(default = "default_port_dbpeer")]
    pub port: u16,
    #[serde(default)]
    pub contacts: Vec<SocketAddr>,
    #[serde(default = "default_discover")]
    pub discover: bool
}

impl DatabasePeer {
    pub fn id(&self) -> Vec<u8> {
        if self.id.len() != 64 {
            panic!("Error: Invalid peer id length - {} != 64", self.id.len());
        }

        let mut key = vec![0u8; 32];

        for i in 0..32 {
            key[i] = u8::from_str_radix(&self.id[i*2..i*2+2], 16).unwrap();
        }   

        key
    }

    pub fn network_key(&self) -> [u8; 32] {
        if self.id.len() != 64 {
            panic!("Error: Invalid peer id length - {} != 64", self.id.len());
        }

        let mut key = [0u8; 32];

        for i in 0..32 {
            key[i] = u8::from_str_radix(&self.network[i*2..i*2+2], 16).unwrap();
        }   

        key
    }
}

/// Global configuration
#[derive(Deserialize,Debug, Clone)]
pub struct Conf {
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default)]
    pub server: Server,
    pub webserver: Option<WebServer>,
    pub peer: Option<DatabasePeer>,
    pub spotify: Option<SpotifyAPI>
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            host: default_host(),
            server: Server::default(),
            webserver: None,
            peer: None,
            spotify: None
        }
    }
}

impl Conf {
    /// Load the configuration from a file and converts it into the `Conf` struct.
    pub fn from_file(path: &Path) -> Result<Conf> {
        let mut file = File::open(path)
            .map_err(|_| Error::ConfigurationNotFound)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        toml::from_str(&contents)
            .map_err(|e| Error::Deserialize(e))
    }

    /// Load the configuration from the environment
    pub fn new() -> Result<(Conf, PathBuf)> {
            // check if we got the configuration, otherwise just load the default settings
        let path = env::vars()
            .filter(|(key, _)| key == "HEX_PATH").map(|(_, a)| PathBuf::from(&a)).next()
            .ok_or(Error::MissingEnv)?;

        Conf::from_file(&path.join("conf.toml")).map(|x| (x, path))
    }
}
