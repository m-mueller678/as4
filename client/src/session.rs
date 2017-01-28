use protocol::*;
use std::net::{ToSocketAddrs, TcpStream};
use std;
use std::cmp::Ordering;

pub struct NewSession {
    stream: Stream,
}

#[derive(Debug)]
pub struct WaitingSession {
    id: u32,
    stream: Stream,
}

#[derive(Debug)]
pub struct PlayingSession {
    left: u32,
    max_len: usize,
    guesses: Vec<u32>,
    results: Vec<Ordering>,
    stream: Stream,
}

pub fn new_session<A: ToSocketAddrs>(address: &A) -> std::io::Result<NewSession> {
    let stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;
    Ok(NewSession {
        stream: BufStream::new(stream, 2048),
    })
}

pub enum JoinResult {
    Playing(PlayingSession),
    JoinFail(NewSession),
    IoError(Error),
}

impl NewSession {
    pub fn create(mut self) -> Result<WaitingSession> {
        self.stream.send(&ClientMessage::Create)?;
        match receive(&mut self.stream) {
            Ok(ServerMessage::Created(id)) => Ok(WaitingSession { stream: self.stream, id: id }),
            Ok(_) => Err(protocol_error()),
            Err(e) => Err(e),
        }
    }
    pub fn join(mut self, id: u32) -> JoinResult {
        if let Err(e) = self.stream.send(&ClientMessage::Join(id)) {
            JoinResult::IoError(e)
        } else {
            match receive(&mut self.stream) {
                Ok(ServerMessage::Start(data)) => JoinResult::Playing(PlayingSession::new(self.stream, data)),
                Ok(ServerMessage::JoinFail) => JoinResult::JoinFail(self),
                Ok(_) => JoinResult::IoError(protocol_error()),
                Err(e) => JoinResult::IoError(e),
            }
        }
    }
}

impl WaitingSession {
    pub fn wait(mut self) -> Result<PlayingSession> {
        match receive(&mut self.stream) {
            Ok(ServerMessage::Start(data)) => Ok(PlayingSession::new(self.stream, data)),
            Ok(_) => Err(protocol_error()),
            Err(e) => Err(e),
        }
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug)]
pub enum PlayingError {
    ProtocolErrorClaim,
    PartnerDisconnect,
    IoError(Error)
}

impl PlayingSession {
    fn new(stream: Stream, data: StartData) -> Self {
        PlayingSession {
            left: data.total_points,
            max_len: data.number_turns as usize,
            guesses: Vec::with_capacity(data.number_turns as usize),
            results: Vec::with_capacity(data.number_turns as usize),
            stream: stream,
        }
    }
    pub fn make_guess(&mut self, n: u32) -> Result<()> {
        assert!(self.guesses.len() < self.max_len);
        assert!(self.left >= n);
        self.guesses.push(n);
        self.left -= n;
        if let Err(e) = self.stream.send(&ClientMessage::Move(n)) {
            Err(e)
        } else {
            Ok(())
        }
    }
    pub fn wait_result(&mut self) -> std::result::Result<(), PlayingError> {
        assert!(self.guesses.len() > self.results.len());
        match receive(&mut self.stream) {
            Ok(ServerMessage::TurnResult(res)) => {
                if let Some(ord) = i8_to_ord(res) {
                    self.results.push(ord);
                    Ok(())
                } else {
                    Err(PlayingError::IoError(protocol_error()))
                }
            }
            Ok(ServerMessage::ProtocolError) => Err(PlayingError::ProtocolErrorClaim),
            Ok(ServerMessage::ConnectionLost) => Err(PlayingError::PartnerDisconnect),
            Ok(_) => Err(PlayingError::IoError(protocol_error())),
            Err(e) => Err(PlayingError::IoError(e)),
        }
    }
    pub fn points_left(&self) -> u32 { self.left }
    pub fn max_turns(&self) -> usize { self.max_len }
    pub fn guesses(&self) -> &Vec<u32> { &self.guesses }
    pub fn results(&self) -> &Vec<Ordering> { &self.results }
}

type Stream = BufStream<TcpStream>;

fn receive(stream: &mut Stream) -> Result<ServerMessage> {
    loop {
        if let Some(rec_res) = stream.receive() {
            return rec_res
        }
    }
}
