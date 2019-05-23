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

    // TODO graceful error handling on configerror
    let smeagol = Smeagol::new().expect("Unable to initialize Smeagol");
    smeagol.start();
}
