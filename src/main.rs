extern crate git2;
extern crate log;
extern crate pretty_env_logger;
extern crate warp;

mod smeagol;
pub use smeagol::Smeagol;
mod git;
pub use git::GitRepository;

fn main() {
    pretty_env_logger::init_custom_env("SMEAGOL_LOG");

    let path = "index.md";
    let path = path.bytes().collect::<Vec<_>>();

    let smeagol = Smeagol::new();
    smeagol.start();
}
