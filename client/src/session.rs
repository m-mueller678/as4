use protocol::*;
use std::net::{ToSocketAddrs, TcpStream};
use std;
use std::cmp::Ordering;

pub struct NewSession {
    stream: Stream,
}

#[derive(Debug)]
pub struct WaitingSession {
    id:u32,
    stream: Stream,
}

#[derive(Debug)]
pub struct PlayingSession{
    left:u32,
    max_len:u32,
    guesses:Vec<u32>,
    results:Vec<Ordering>,
    stream:Stream,
}

pub fn new_session<A: ToSocketAddrs>(address: &A) -> std::io::Result<NewSession> {
    let stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;
    Ok(NewSession {
        stream: BufStream::new(stream),
    })
}

pub enum JoinResult{
    Playing(PlayingSession),
    JoinFail(NewSession),
    IoError(Error),
}

impl NewSession {
    pub fn create(mut self) -> Result<WaitingSession> {
        self.stream.send(&ClientMessage::Create)?;
        match receive(&mut self.stream){
            Ok(ServerMessage::Created(id))=>Ok(WaitingSession{stream:self.stream,id:id}),
            Ok(_)=>Err(protocol_error()),
            Err(e)=>Err(e),
        }
    }
    pub fn join(mut self,id:u32) -> JoinResult {
        if let Err(e)=self.stream.send(&ClientMessage::Join(id)){
            JoinResult::IoError(e)
        }else{
            match receive(&mut self.stream){
                Ok(ServerMessage::Start(data))=>JoinResult::Playing(PlayingSession::new(self.stream,data)),
                Ok(ServerMessage::JoinFail)=>JoinResult::JoinFail(self),
                Ok(_)=>JoinResult::IoError(protocol_error()),
                Err(e)=>JoinResult::IoError(e),
            }
        }
    }
}

impl WaitingSession{
    pub fn wait(mut self)->Result<PlayingSession>{
        match receive(&mut self.stream){
            Ok(ServerMessage::Start(data))=>Ok(PlayingSession::new(self.stream,data)),
            Ok(_)=>Err(protocol_error()),
            Err(e)=>Err(e),
        }
    }
    pub fn id(&self)->u32{
        self.id
    }
}

impl PlayingSession{
    fn new(stream:Stream,data:StartData)->Self{
        PlayingSession{
            left:data.total_points,
            max_len:data.number_turns,
            guesses:Vec::with_capacity(data.number_turns as usize),
            results:Vec::with_capacity(data.number_turns as usize),
            stream:stream,
        }
    }
}

type Stream=BufStream<TcpStream>;

fn receive(stream:&mut Stream)->Result<ServerMessage>{
    loop {
        if let Some(rec_res) = stream.receive() {
            return rec_res
        }
    }
}
