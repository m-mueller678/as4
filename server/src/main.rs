extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate serde;
extern crate rand;
extern crate protocol;

mod server;

use std::env::args;

fn main() {
    env_logger::init().unwrap();
    server::run(&args().nth(1).expect("get socket address").as_str());
}