use protocol::*;
use std::net::{ToSocketAddrs,TcpStream};
use std;

#[derive(Clone)]
pub enum PlayerState{
    None,
    Waiting,
}

pub struct Session {
    stream: BufStream<TcpStream>,
    state: PlayerState,
}

impl Session{
    pub fn new<A:ToSocketAddrs>(address:&A)->std::io::Result<Self>{
        let stream=TcpStream::connect(address)?;
        stream.set_nodelay(true)?;
        Ok(Session{
            stream:BufStream::new(stream),
            state:PlayerState::None,
        })
    }
    pub fn create(&mut self)->Result<u32>{
        self.stream.send(&ClientMessage::Create)?;
        loop{
            if let Some(rec_res)=self.stream.receive(){
                return if let ServerMessage::Created(id)=rec_res?{
                    self.state=PlayerState::Waiting;
                    Ok(id)
                }else{
                    Err(protocol_error())
                }
            }else{}//repeat
        }
    }
    pub fn state(&self)->PlayerState{
        self.state.clone()
    }
}