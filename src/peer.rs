use std::net::SocketAddr;
use std::collections::HashMap;
use std::net::AddrParseError;
use node::Tx;

pub enum State {
    Connected,
    Disconnected,
}

#[derive(Clone)]
pub struct Peer {
    pub info: PeerInfo,
    pub peers: HashMap<String, (Tx, SocketAddr)>,
    pub broadcasting_peers: HashMap<String, (Tx, SocketAddr)>,
    pub known_peers: HashMap<String, SocketAddr>,
}

impl Peer {
    pub fn new(addr: String) -> Result<Peer, AddrParseError> {
        Ok(Peer {
            info: PeerInfo {
                id: addr.clone(),
                addr: addr.parse()?,
            },
            peers: HashMap::new(),
            broadcasting_peers: HashMap::new(),
            known_peers: HashMap::new(),
        })
    }
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: String,
    pub addr: SocketAddr,
}


