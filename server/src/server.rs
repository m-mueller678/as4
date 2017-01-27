use protocol::*;
use mio::tcp::{TcpStream, TcpListener};
use mio::*;
use std::collections::hash_map::{HashMap, Entry};
use rand::{thread_rng, Rng};
use std::rc::Rc;
use std::mem::replace;
use std::cmp::{max, min};
use std::cell::RefCell;
use std::net::ToSocketAddrs;
use std::fmt::Debug;

type JoinId = u32;
type PlayerId = usize;

type Stream = BufStream<TcpStream>;

#[derive(Clone, Debug)]
enum PlayerState {
    None,
    Waiting(JoinId),
    Playing((usize, Rc<RefCell<Game>>)),
}

#[derive(Debug)]
struct Game {
    max_len: usize,
    left: [u32; 2],
    guesses: [Vec<u32>; 2],
}

impl Game {
    fn new() -> Self {
        Game {
            max_len: 7,
            left: [700; 2],
            guesses: [Vec::with_capacity(7), Vec::with_capacity(7)],
        }
    }
    fn handle_message(&mut self, player: usize, msg: ClientMessage, mut streams: [&mut Stream; 2]) -> bool {
        if let ClientMessage::Move(n) = msg {
            if self.guesses[player].len() < self.max_len && self.left[player] >= n {
                let index = self.guesses[player].len();
                self.guesses[player].push(n);
                self.left[player] -= n;
                if let Some(other_guess) = self.guesses[player ^ 1].get(index) {
                    let result = other_guess.cmp(&n);
                    streams[player].send(&ServerMessage::TurnResult(ord_to_i8(result))).is_ok()
                        && streams[player ^ 1].send(&ServerMessage::TurnResult(ord_to_i8(result.reverse()))).is_ok()
                } else {
                    true
                }
            } else {
                streams[player].send(&ServerMessage::ProtocolError).is_ok();
                false
            }
        } else {
            streams[player].send(&ServerMessage::ProtocolError).is_ok();
            false
        }
    }
    fn is_over(&self) -> bool {
        self.guesses[0].len() >= self.max_len && self.guesses[1].len() >= self.max_len
    }
}

struct Server {
    players: Vec<PlayerState>,
    streams: Vec<Option<Stream>>,
    poll: Poll,
    open_games: HashMap<JoinId, PlayerId>,
    listener: TcpListener,
}

pub fn run<A: ToSocketAddrs + Debug>(a: &A) -> ! {
    let s = Server::new(a);
    s.run()
}

const SERVER_MAX_CONNECTIONS: usize = 256;

impl Server {
    fn new<A: ToSocketAddrs + Debug>(address: &A) -> Self {
        let listener_opt = address.to_socket_addrs().unwrap().map(|x| TcpListener::bind(&x)).find(|x| x.is_ok());
        if let Some(listener) = listener_opt {
            let s = Server {
                streams: Vec::new(),
                players: Vec::new(),
                poll: Poll::new().unwrap(),
                open_games: Default::default(),
                listener: listener.unwrap(),
            };
            s.poll.register(&s.listener, Token(SERVER_MAX_CONNECTIONS), Ready::readable(), PollOpt::edge()).unwrap();
            s
        } else {
            panic!("can not open listener on {:?}", address)
        }
    }
    fn run(mut self) -> ! {
        info!("server started");
        let mut events = Events::with_capacity(64);
        loop {
            self.poll.poll(&mut events, None).unwrap();
            for evt in events.iter() {
                let ready = evt.kind();
                let player = evt.token().0;
                if player == SERVER_MAX_CONNECTIONS {
                    if ready.is_error() || ready.is_hup() {
                        panic!("error polling listener");
                    } else if let Ok((stream, address)) = self.listener.accept() {
                        let index = if let Some(i) = self.streams.iter().position(Option::is_none) {
                            self.streams[i] = Some(BufStream::new(stream));
                            Some(i)
                        } else if self.streams.len() < SERVER_MAX_CONNECTIONS {
                            self.streams.push(Some(BufStream::new(stream)));
                            self.players.push(PlayerState::None);
                            Some(self.streams.len() - 1)
                        } else {
                            info!("server full, dropped connection from {:?}", address);
                            None
                        };
                        if let Some(index) = index {
                            if self.poll.register(self.streams[index].as_ref().unwrap().raw(), Token(index), Ready::readable(), PollOpt::edge()).is_err() {
                                self.remove(index);
                                error!("error registering stream {:?}", address);
                            }
                            info!("connect from {:?} as {}", address, index);
                        }
                    }
                } else {
                    if ready.is_error() {
                        self.remove(player);
                    } else if ready.is_hup() {
                        self.remove(player);
                    } else if ready.is_readable() {
                        self.read_from_player(player)
                    }
                }
            }
        }
    }
    fn read_from_player(&mut self, from: PlayerId) {
        while let Some(msg) = self.receive_single(from) {
            if let Ok(msg) = msg {
                if !self.handle_message(from, msg) {
                    self.remove(from);
                }
            } else {
                self.remove(from);
            }
        }
    }
    fn receive_single(&mut self, p: PlayerId) -> Option<Result<ClientMessage>> {
        if let Some(ref mut stream) = self.streams[p] {
            stream.receive()
        } else {
            None
        }
    }
    fn remove(&mut self, id: PlayerId) {
        let state = replace(&mut self.players[id], PlayerState::None);
        if self.streams[id].take().is_some() {
            match state {
                PlayerState::None => {},
                PlayerState::Waiting(_) => {},
                PlayerState::Playing((other, _)) => {
                    if let Some(mut stream) = self.streams[other].take() {
                        stream.send(&ServerMessage::ConnectionLost).is_ok();
                    }
                    self.players[other] = PlayerState::None;
                },
            }
        }
    }
    fn handle_message(&mut self, from: PlayerId, msg: ClientMessage) -> bool {
        debug!("received {:?} from {}, state: {:?}", msg, from, self.players[from]);
        match self.players[from].clone() {
            PlayerState::Waiting(_) => {
                self.streams[from].as_mut().unwrap().send(&ServerMessage::ProtocolError).is_ok();
                false
            },
            PlayerState::None => {
                self.handle_start_message(from, msg)
            },
            PlayerState::Playing((other_id, game)) => {
                let res = {
                    let is_first = from < other_id;
                    let mut game_ref = game.borrow_mut();
                    let (slice_1, slice_2) = self.streams.split_at_mut(max(from, other_id));
                    let streams = [slice_1[min(from, other_id)].as_mut().unwrap(), slice_2[0].as_mut().unwrap()];
                    game_ref.handle_message(if is_first { 0 } else { 1 }, msg, streams)
                };
                if game.borrow().is_over() {
                    self.players[from] = PlayerState::None;
                    self.players[other_id] = PlayerState::None;
                    self.streams[from].take().unwrap().send(&ServerMessage::EndOfGame).is_ok();
                    self.streams[other_id].take().unwrap().send(&ServerMessage::EndOfGame).is_ok();
                }
                res
            }
        }
    }
    fn handle_start_message(&mut self, from: PlayerId, msg: ClientMessage) -> bool {
        match msg {
            ClientMessage::Create => {
                self.create_game(from)
            },
            ClientMessage::Join(join_id) => {
                if let Some(other_id) = self.open_games.remove(&join_id) {
                    self.start_game(from, other_id)
                } else {
                    self.streams[from].as_mut().unwrap().send(&ServerMessage::JoinFail).is_ok()
                }
            },
            _ => {
                self.streams[from].as_mut().unwrap().send(&ServerMessage::ProtocolError).is_ok();
                false
            }
        }
    }
    fn create_game(&mut self, player_id: PlayerId) -> bool {
        loop {
            let key = thread_rng().gen();
            match self.open_games.entry(key) {
                Entry::Vacant(ent) => {
                    self.players[player_id] = PlayerState::Waiting(key);
                    return if self.streams[player_id].as_mut().unwrap().send(&ServerMessage::Created(key)).is_ok() {
                        ent.insert(player_id);
                        true
                    } else {
                        false
                    }
                },
                Entry::Occupied(_) => {},
            }
        }
    }
    fn start_game(&mut self, joined: PlayerId, created: PlayerId) -> bool {
        assert!(created != joined);
        if self.streams[created].as_mut().unwrap().send(&ServerMessage::Start).is_ok() {
            if self.streams[joined].as_mut().unwrap().send(&ServerMessage::Start).is_ok() {
                let game = Rc::new(RefCell::new(Game::new()));
                self.players[joined] = PlayerState::Playing((created, game.clone()));
                self.players[created] = PlayerState::Playing((joined, game));
                true
            } else {
                false
            }
        } else {
            self.streams[joined].as_mut().unwrap().send(&ServerMessage::JoinFail).is_ok()
        }
    }
}