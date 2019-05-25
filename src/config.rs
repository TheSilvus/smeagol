use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
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

    pub fn parse_bind(&self) -> Result<SocketAddr, ConfigError> {
        Ok(self
            .bind
            .to_socket_addrs()
            .map_err(|_| ConfigError::InvalidSocketAddress)?
            .next()
            .ok_or(ConfigError::InvalidSocketAddress)?)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    InvalidSocketAddress,
}
impl std::error::Error for ConfigError {}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ConfigError::Io(ref err) => write!(f, "IO error: {}", err),
            &ConfigError::Toml(ref err) => write!(f, "TOML error: {}", err),
            &ConfigError::InvalidSocketAddress => write!(f, "Invalid socket address"),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> ConfigError {
        ConfigError::Io(err)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> ConfigError {
        ConfigError::Toml(err)
    }
}
