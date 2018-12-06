use std::result;
use toml;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Deserialize(toml::de::Error),
    ConfigurationNotFound,
    MissingEnv
}
