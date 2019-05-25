use log::error;

mod smeagol;
use smeagol::Smeagol;

mod config;
use config::Config;
mod filetype;
use filetype::Filetype;
mod git;
use git::GitRepository;
mod path;
use path::{Path, PathStringBuilder};
mod error;
use error::SmeagolError;
mod warp_helper;

fn main() {
    pretty_env_logger::init_custom_env("SMEAGOL_LOG");

    match Smeagol::new() {
        Ok(smeagol) => smeagol.start(),
        Err(SmeagolError::Config(ref err)) => error!("Could not load config: {}", err),
        Err(ref err) => panic!("{}", err),
    }
}
