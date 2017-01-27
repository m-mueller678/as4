extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate rand;

mod server;
mod protocol;

use std::env::args;

fn main() {
    env_logger::init().unwrap();
    server::run(&args().nth(1).expect("get socket address").as_str());
}