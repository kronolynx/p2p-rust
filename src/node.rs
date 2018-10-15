use tokio;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use std::net::SocketAddr;
use std::sync::Arc;
use peer::Peer;
use std::net::AddrParseError;
use futures::sync::mpsc;
use futures::{Stream, Sink};
use tokio_codec::Decoder;
use codec::TestBytes;
use serde_json;
use std::sync::RwLock;
use std::thread;

#[derive(Clone)]
pub struct Node {
    pub peer: Arc<RwLock<Peer>>,
}

impl Node {
    // todo create own error to wrap possible problems to handle
    pub fn new(addr: String) -> Result<Node, AddrParseError> {
        Ok(Node {
            peer: Arc::new(RwLock::new(Peer::new(addr)?)),
        })
    }

    pub fn serve(peer: Arc<RwLock<Peer>>) -> Box<Future<Item=(), Error=io::Error> + Send> {
        let info = peer.read().unwrap().info.clone();
        println!("starting node {}", &info.id);

        let socket = TcpListener::bind(&info.addr).unwrap();
        let server = socket.incoming()
            .for_each(move |socket| {
                let socket_addr = socket.peer_addr().unwrap().to_string();
                println!("accepted socket; addr={:?}", &socket_addr);


                // split the socket into a framed sink and stream
                let (writer, reader) = TestBytes.framed(socket).split();

                // mpsc (multi producer single consumer)
                // split the mpsc into (sender, receiver)
                let (tx, rx) = mpsc::unbounded();

                // clone the mpsc sender
                // must clone here to avoid error can't move out of closure
                let peer = peer.clone();


                // read from the split socket
                let read = reader.for_each(move |frame| {
                    Node::process(frame, tx.clone(), peer.clone());
                    Ok(())
                }).map_err(|e| eprintln!("Error: {:?}", e));
                // spawn a tokio thread for read
                tokio::spawn(read);

                // write to the stream part of the split socket by reading from the receiver part of the mpsc
                let write = writer.send_all(rx.map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "rx shouldn't have an error")
                })).then(|_| Ok(()));
                // spawn tokio thread for write
                tokio::spawn(write);

                Ok(())
            });
        Box::new(server)
    }

    pub fn client(peer: Arc<RwLock<Peer>>, addr: SocketAddr) -> Box<Future<Item=(), Error=io::Error> + Send> {
        let socket = TcpStream::connect(&addr);
        let client = socket.and_then(move |socket| {
            println!("{} connected to peer {}", socket.local_addr().unwrap(), socket.peer_addr().unwrap());
            let (writer, reader) = TestBytes.framed(socket).split();
            let (tx, rx) = mpsc::unbounded();

            // send ping when connection opens
            mpsc::UnboundedSender::unbounded_send(&tx, Node::encode(&Msg::Ping((peer.read().unwrap().info.id.clone(), peer.read().unwrap().info.addr)))).expect("Failed to send message");

            peer.write().map(|mut peer| {
                peer.peers.insert(addr.to_string(), (tx.clone(), addr));
            }).unwrap();

            // read from the split socket
            let read = reader.for_each(move |frame| {
                Node::process(frame, tx.clone(), peer.clone());
                Ok(())
            }).map_err(|e| eprintln!("Error: {:?}", e));
            // spawn a tokio thread for read
            tokio::spawn(read);

            // write to the stream part of the split socket by reading from the receiver part of the mpsc
            let write = writer.send_all(rx.map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "rx shouldn't have an error")
            })).then(|_| Ok(()));
            // spawn tokio thread for write
            tokio::spawn(write);

            Ok(())
        });
        Box::new(client)
    }

    pub fn spawn_peer(peer:Arc<RwLock<Peer>>, addr: SocketAddr) {
        thread::spawn(move || {
            let client = Node::client(peer, addr);
            tokio::run(client.map_err(|e| eprintln!("client error = {:?}", e)));
        });
    }

    pub fn run(&self, addrs: Vec<&str>) {

        let peer = self.peer.clone();
        let server = Node::serve(peer.clone());

        addrs.iter().flat_map(|s| s.parse()).for_each(|addr|{
            Node::spawn_peer(peer.clone(), addr);
        });

        tokio::run(server.map_err(|e| eprintln!("Error spawning server {:?}", e)));
    }

    fn decode(frame: &[u8]) -> Msg {
        let s = String::from_utf8(frame.to_vec()).unwrap();
        serde_json::from_str(&s).unwrap()
    }

    fn encode(msg: &Msg) -> Vec<u8> {
        let mut s = serde_json::to_string(msg).unwrap().as_bytes().to_vec();
        s.push(b'\n');
        s
    }

    fn add_peer(id: String, tx: Tx, socket_addr: SocketAddr, peer: Arc<RwLock<Peer>>) {
        println!("Adding peer {}", &id);
        peer.write().map(|mut peer| {
            peer.peers.insert(id, (tx, socket_addr));
            println!("updated peers {:?}", &peer.peers.keys());
        }).unwrap();
    }

    pub fn process(frame: Vec<u8>, tx: Tx, peer: Arc<RwLock<Peer>>) {
        // this should bo done with flat buffers but for testing serde is good
        match Node::decode(&frame) {
            Msg::Payload(m) => {
                println!("payload => {}", m);
                mpsc::UnboundedSender::unbounded_send(&tx, Node::encode(&Msg::Payload(format!("got {}", m)))).expect("Failed to send message");
            }
            // client sends ping on first connection (this message is received by the server)
            Msg::Ping((id, _socket_addr)) => {
                println!("got ping from {:?}", id);
                mpsc::UnboundedSender::unbounded_send(&tx, Node::encode(&Msg::Pong((peer.read().unwrap().info.id.clone(), peer.read().unwrap().info.addr)))).expect("Failed to send ping");
                // TODO add it to known connections used for broadcasting
            }
            // server responds to ping (message received by the client)
            Msg::Pong((id, socket_addr)) => {
                println!("got pong from {}", id);
                Node::add_peer(id, tx.clone(), socket_addr, peer);
                mpsc::UnboundedSender::unbounded_send(&tx, Node::encode(&Msg::GetPeers)).expect("Failed to ask for peers");
            }
            Msg::AddrVec(addr) => {
                println!("got peers {:?}", addr);
                let id = peer.read().unwrap().info.id.clone();
                // TODO when node already has 8 peers connected store the socket in another list
                addr.iter().for_each(|(s, socket_addr)| {
                    // checks that we don't open a new client to the current node or a node already in peers
                    println!("about to check peers {}", &s);
                    if s != &id && peer.read().unwrap().peers.get(s).is_none() {
                        Node::spawn_peer(peer.clone(), socket_addr.clone());
                    }
                });
            }
            Msg::GetPeers => {
                println!("got peer request");
                let peers = peer.read().unwrap().peers.iter().map(|(id, (_, socket))| {
                    (id.to_owned(), socket.clone())
                }).collect::<Vec<_>>();
                mpsc::UnboundedSender::unbounded_send(&tx, Node::encode(&Msg::AddrVec(peers))).expect("Failed to send peers");
            }
        }
    }
}

pub type Tx = mpsc::UnboundedSender<Vec<u8>>;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Msg {
    Ping((String, SocketAddr)),
    Pong((String, SocketAddr)),
    Payload(String),
    AddrVec(Vec<(String, SocketAddr)>),
    GetPeers,
}