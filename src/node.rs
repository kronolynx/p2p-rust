use tokio;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use std::net::SocketAddr;
use std::sync::Arc;
use peer::{Peer, PeerInfo, State};
use std::collections::HashMap;
use std::net::AddrParseError;
use futures::sync::mpsc;
use futures::{Stream, Sink};
use tokio_codec::Decoder;
use codec::TestBytes;
use futures::sync::mpsc::{channel, Sender, Receiver};
use futures::future::join_all;
use serde_json;
use std::sync::RwLock;

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

            // send ping
            mpsc::UnboundedSender::unbounded_send(&tx, Node::serde_encode(&Msg::Ping((peer.read().unwrap().info.id.clone(), peer.read().unwrap().info.addr)))).expect("Failed to send message");

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

    pub fn run(&self, addrs: Vec<&str>) {
        println!("starting node");
        let peer = self.peer.clone();
        let server = Node::serve(peer.clone());

        let mut futures = vec![server];

        addrs.iter().flat_map(|s| s.parse()).for_each(|addr: SocketAddr| {
            println!("connecting to client {}", addr.to_string());
            let peer = self.peer.clone();
            let client = Node::client(peer, addr);

            futures.push(client);
        });
        let joined_futures = join_all(futures);

        tokio::run(joined_futures.then(|_| {
            println!("ending");
            Ok(())
        }));
    }

    fn serde_decode(frame: &[u8]) -> Msg {
        let s = String::from_utf8(frame.to_vec()).unwrap();
        serde_json::from_str(&s).unwrap()
    }

    fn serde_encode(msg: &Msg) -> Vec<u8> {
        let mut s = serde_json::to_string(msg).unwrap().as_bytes().to_vec();
        s.push(b'\n');
        s
    }

    pub fn process(frame: Vec<u8>, tx: Tx, peer: Arc<RwLock<Peer>>) {
        // this should bo done with flat buffers but for testing serde is good
        match Node::serde_decode(&frame) {
            Msg::Payload(m) => {
                println!("payload => {}", m);
                mpsc::UnboundedSender::unbounded_send(&tx, Node::serde_encode(&Msg::Payload(format!("got {}", m)))).expect("Failed to send message");
            }
            Msg::Ping(m) => {
                println!("ping {:?}", m);
                mpsc::UnboundedSender::unbounded_send(&tx, Node::serde_encode(&Msg::Pong((peer.read().unwrap().info.id.clone(), peer.read().unwrap().info.addr)))).expect("Failed to send message");
            }
            Msg::Pong(m) => {
                println!("pong");
            }
            _ => println!("unknown message")
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
}