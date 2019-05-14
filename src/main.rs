extern crate git2;
extern crate handlebars;
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
#[cfg(test)]
extern crate tempdir;
extern crate warp;

mod smeagol;
use smeagol::Smeagol;
mod git;
use git::GitRepository;
mod error;
use error::SmeagolError;
mod warp_helper;

fn main() {
    pretty_env_logger::init_custom_env("SMEAGOL_LOG");

    let smeagol = Smeagol::new().expect("Unable to initialize Smeagol");
    smeagol.start();
}
