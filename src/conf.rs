use std::env;
use std::fs::File;
use std::io::Read;

use toml;
use std::default::Default;

use error::{ErrorKind, Result};
use failure::ResultExt;
use failure::Fail;

#[derive(Deserialize, Debug)]
pub struct Server {
    #[serde(default = "default_addr")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16
}

fn default_addr() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 2798 }
fn default_port_web() -> u16 { 80 }

impl Default for Server {
    fn default() -> Self {
        Server {
            host: "127.0.0.1".into(),
            port: 2798
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Music {
    pub data_path: String,
    pub db_path: String
}

impl Default for Music {
    fn default() -> Self {
        let home = env::home_dir().expect("Could not found a home directory!");
        let home_str = home.to_str().unwrap();

        Music {
            data_path: format!("{}/.music/", home_str),
            db_path: format!("{}/.music.db", home_str)
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct WebServer {
    pub path: String,
    #[serde(default = "default_addr")]
    pub host: String,
    #[serde(default = "default_port_web")]
    pub port: u16
}

#[derive(Deserialize, Default, Debug)]
pub struct Conf {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub music: Music,
    pub webserver: Option<WebServer>
}

impl Conf {
    pub fn from_file(path: &str) -> Result<Conf> {
        let mut file = File::open(path).context(ErrorKind::Configuration)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::Configuration)?;

        toml::from_str(&contents)
            .map_err(|err| err.context(ErrorKind::Configuration).into())
    }
}
