use serde::Deserialize;
use std::fs;
use thiserror::Error;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub listen_addr: String,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let path = "smppgc.toml";
        let conf_str = fs::read_to_string(path)?;
        Ok(toml::from_str(&conf_str)?)
    }
}
