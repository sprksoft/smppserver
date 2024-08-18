use serde::Deserialize;
use std::fs;
use thiserror::Error;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub metrics_addr: String,
    pub listen_addr: String,
    pub max_stored_messages: usize,
    pub name_reserve_time: u64,
    pub max_users: u16,
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
        let path = std::env::var("SMPPGC_CONFIG").unwrap_or("/etc/smppgc.toml".to_string());
        let conf_str = fs::read_to_string(path)?;
        Ok(toml::from_str(&conf_str)?)
    }
}
