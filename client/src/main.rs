extern crate protocol;

use protocol::*;
use std::net::TcpStream;

fn main() {
    let stream=TcpStream::connect("127.0.0.1:12345").unwrap();
    let stream=BufStream::new(stream);
}
