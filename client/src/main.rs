extern crate protocol;

mod session;

use session::*;
use std::env::args;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let session1 = new_session(&args().nth(1).unwrap().as_str()).unwrap();
    let session1=session1.create().unwrap();
    sleep(Duration::from_secs(1));
    let session2 = new_session(&args().nth(1).unwrap().as_str()).unwrap();
    if let JoinResult::Playing(session2)=session2.join(session1.id()){
        let session1=session1.wait().unwrap();
        println!("1:{:?}",session1);
        println!("2:{:?}",session2);
    }
}