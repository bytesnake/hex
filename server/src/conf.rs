use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::net::{IpAddr, Ipv4Addr};

use toml;
use std::default::Default;

use error::{Error, Result};

#[derive(Deserialize, Debug)]
pub struct Server {
    #[serde(default = "default_port")]
    pub port: u16
}

fn default_host() -> IpAddr { IpAddr::V4(Ipv4Addr::LOCALHOST) }
fn default_port() -> u16 { 2798 }
fn default_port_web() -> u16 { 80 }
fn default_port_sync() -> u16 { 8004 }

impl Default for Server {
    fn default() -> Self {
        Server {
            port: 2798
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Music {
    pub db_path: PathBuf,
    pub data_path: PathBuf
}

impl Default for Music {
    fn default() -> Self {
        let home = env::home_dir().expect("Could not found a home directory!");

        Music {
            data_path: home.join(".music"),
            db_path: home.join(".music.db")
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct WebServer {
    pub path: PathBuf,
    #[serde(default = "default_port_web")]
    pub port: u16
}

#[derive(Deserialize, Debug, Clone)]
pub struct Syncc {
    #[serde(default)]
    pub sync_all: bool,
    pub name: String,
    #[serde(default = "default_port_sync")]
    pub port: u16
}

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
    pub fn from_file(path: &str) -> Result<Conf> {
        let mut file = File::open(path).map_err(|_| Error::Configuration)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| Error::Configuration)?;

        toml::from_str(&contents)
            .map_err(|_| Error::Configuration)
    }
}
