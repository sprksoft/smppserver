use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {}
impl Config {
    pub fn load() -> Result<Self, toml::de::Error> {
        let conf_str = fs::read_to_string("/etc/smppgc.toml").unwrap();
        let conf: Config = toml::from_str(&conf_str)?;

        let conf_str = fs::read_to_string("smppgc.toml").unwrap();
        Config::deserialize_in_place(&conf_str)?;
        Ok(conf)
    }
}
