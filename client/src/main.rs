extern crate protocol;

mod session;

use session::Session;
use std::env::args;

fn main() {
    let mut session=Session::new(&args().nth(1).unwrap().as_str()).unwrap();
    println!("{:?}",session.create());
}