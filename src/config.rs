use std::fmt;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub bind: String,

    pub index: String,
    pub max_upload_size: u64,

    pub repo: String,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    TomlError(toml::de::Error),
}
impl std::error::Error for ConfigError {}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ConfigError::IoError(ref err) => write!(f, "IO error: {}", err),
            &ConfigError::TomlError(ref err) => write!(f, "TOML error: {}", err),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> ConfigError {
        ConfigError::IoError(err)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> ConfigError {
        ConfigError::TomlError(err)
    }
}
