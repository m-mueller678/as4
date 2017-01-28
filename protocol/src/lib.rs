extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

use std::io::{Write, Read, ErrorKind};
use std::cmp::Ordering;
use std::str;
use std::fmt::Debug;
use serde_json::*;
use serde::{Serialize, Deserialize};
use serde::de::Error as ErrorTrait;

pub type Result<T> = serde_json::Result<T>;
pub type Error = serde_json::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    ConnectionLost,
    ProtocolError,
    ServerError,

    Created(u32),
    JoinFail,

    Start(StartData),
    TurnResult(i8),
    EndOfGame,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StartData {
    pub number_turns: u32,
    pub total_points: u32,
}

impl StartData {
    pub fn new(turns: u32, points: u32) -> Self {
        StartData {
            number_turns: turns,
            total_points: points,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    Create,
    Join(u32),

    Move(u32),
}


pub fn ord_to_i8(o: Ordering) -> i8 {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn protocol_error() -> serde_json::Error {
    serde_json::Error::custom("protocol error")
}

pub fn i8_to_ord(o: i8) -> Option<Ordering> {
    match o {
        - 1 => Some(Ordering::Less),
        0 => Some(Ordering::Equal),
        1 => Some(Ordering::Greater),
        _ => None,
    }
}

#[derive(Debug)]
pub struct BufStream<Stream: Write + Read> {
    read_buf: Vec<u8>,
    len: usize,
    stream: Stream,
}

impl<Stream: Write + Read> BufStream<Stream> {
    pub fn new(stream: Stream, capacity: usize) -> Self {
        BufStream {
            read_buf: vec![0; capacity],
            len: 0,
            stream: stream,
        }
    }
    pub fn send<T: Serialize + Debug>(&mut self, msg: &T) -> Result<()> {
        to_writer(&mut self.stream, msg)?;
        if let Err(e) = self.stream.write_all(&[0]) {
            return Err(serde_json::Error::Io(e));
        }
        Ok(())
    }
    pub fn receive<T: Deserialize + Debug>(&mut self) -> Option<Result<T>> {
        let read_res = self.stream.read(&mut self.read_buf[self.len..]);
        match read_res {
            Ok(read_len) => {
                self.len += read_len;
                self.read_from_buf()
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.read_from_buf()
                } else {
                    Some(Err(e.into()))
                }
            }
        }
    }
    fn read_from_buf<T: Deserialize + Debug>(&mut self) -> Option<Result<T>> {
        let pos_opt = self.read_buf[..self.len].iter().position(|x| *x == 0u8);
        if let Some(pos) = pos_opt {
            let ret = from_slice(&self.read_buf[..pos]);
            self.set_read_start(pos + 1);
            Some(ret)
        } else {
            if self.len == self.read_buf.len() {
                Some(Err(Error::custom("read buffer full")))
            } else {
                None
            }
        }
    }
    pub fn raw(&self) -> &Stream {
        &self.stream
    }
    fn set_read_start(&mut self, shift: usize) {
        assert!(self.len >= shift);
        self.len -= shift;
        for i in 0..self.len {
            self.read_buf[i] = self.read_buf[i + shift];
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn send_receive() {
        let messages = [
            ServerMessage::Created(1234567),
            ServerMessage::ProtocolError,
            ServerMessage::JoinFail,
            ServerMessage::TurnResult(-76),
        ];
        let mut stream = BufStream::new(Cursor::new(Vec::<u8>::new()), 2048);
        for msg in messages.iter() {
            stream.send(msg).unwrap();
        }
        stream.stream.set_position(0);
        for msg in messages.iter() {
            assert_eq!(stream.receive::<ServerMessage>().unwrap().unwrap(), *msg);
        }
    }
}