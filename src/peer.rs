use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::net::AddrParseError;

pub enum State {
    Connected,
    Disconnected,
}

#[derive(Clone)]
pub struct Peer {
    pub info: PeerInfo,
    state: Arc<RwLock<State>>,
}

impl Peer {
    pub fn new(addr: String) -> Result<Peer, AddrParseError> {
        Ok(Peer {
            info: PeerInfo {
                id: addr.clone(),
                addr: addr.parse()?,
                peers: HashMap::new(),
            },
            state: Arc::new(RwLock::new(State::Disconnected)),
        })
    }
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: String,
    pub addr: SocketAddr,
    // todo peers should have a handle to the handler of the peer?
    pub peers: HashMap<String, SocketAddr>,
}


pub struct Peers {
    peers: RwLock<HashMap<SocketAddr, Arc<RwLock<Peer>>>>,
}
