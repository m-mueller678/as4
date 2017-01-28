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
    if let JoinResult::Playing(mut session2) = session2.join(session1.id()) {
        let mut session1 = session1.wait().unwrap();
        println!("1:{:?}",session1);
        println!("2:{:?}",session2);
        for _ in 0..session1.max_turns() {
            let put1 = session1.points_left() / 2;
            let put2 = session2.points_left() / 4;
            session1.make_guess(put1).unwrap();
            session2.make_guess(put2).unwrap();
            session1.wait_result().unwrap();
            session2.wait_result().unwrap();
        }
        println!("guesses1: {:?}", session1.guesses());
        println!("guesses2: {:?}", session2.guesses());
        println!("results1: {:?}", session1.results());
    }
}