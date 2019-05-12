extern crate log;
extern crate pretty_env_logger;
extern crate warp;

mod smeagol;
use smeagol::Smeagol;

fn main() {
    pretty_env_logger::init_custom_env("SMEAGOL_LOG");

    let smeagol = Smeagol::new();
    smeagol.start();
}
