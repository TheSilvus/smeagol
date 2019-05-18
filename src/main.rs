extern crate comrak;
extern crate git2;
extern crate handlebars;
extern crate log;
extern crate percent_encoding;
extern crate pretty_env_logger;
extern crate serde;
extern crate serde_json;
#[cfg(test)]
extern crate tempdir;
extern crate toml;
extern crate warp;

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
