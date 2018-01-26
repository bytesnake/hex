use std::env;
use std::fs::File;

use toml;
use std::default::Default;

use error::{ErrorKind, Result};

#[derive(Deserialize)]
pub struct Server {
    #[serde(default = "default_addr")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16
}

fn default_addr() -> &'static str { "127.0.0.1" }
fn default_port() -> u16 { 2798 }

impl Default for Server {
    fn default() -> Self {
        Server {
            host: "127.0.0.1".into(),
            port: 2798
        }
    }
}

#[derive(Deserialize)]
pub struct Music {
    pub data_path: String,
    pub db_path: String
}

impl Default for Music {
    fn default() -> Self {
        let home = env::home_dir().expect("Could not found a home directory!").to_str().unwrap();

        Music {
            data_path: format!("{}/.music/", home),
            db_path: format!("{}/.music.db", home)
        }
    }
}

#[derive(Deserialize, Default)]
pub struct Conf {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub music: Music
}

impl Conf {
    pub fn from_file(path: &str) -> Result<Conf> {
        let mut file = File::open(path).context(ErrorKind::Configuration)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::Configuration)?;

        toml::from_str(contents).context(ErrorKind::Configuration)
    }
}
