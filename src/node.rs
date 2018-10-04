use tokio;
use tokio::prelude::*;
use tokio::net::TcpListener;
use tokio::io;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use peer::{Peer, PeerInfo, State};
use std::collections::HashMap;
use std::net::AddrParseError;
use futures::sync::mpsc;
use futures::{Stream, Sink};
use tokio_codec::Decoder;
use codec::TestBytes;
use futures::sync::mpsc::{channel, Sender, Receiver};

#[derive(Clone)]
pub struct Node {
    //        pub config: P2PConfig,
    pub node: Peer,
    stop: Arc<AtomicBool>,
}

impl Node {
    // todo create own error to wrap possible problems to handle
    pub fn new(addr: String) -> Result<Node, AddrParseError> {
        Ok(Node {
            node: Peer::new(addr)?,
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn run<I: Iterator<Item=SocketAddr>>(&self, addrs: I) {
        let addr = &self.node.info.addr;
        TcpListener::bind(&addr).map(|listener| {
            let server = listener.incoming()
                .map_err(|e| eprintln!("Error accepting connection: {:?}", e))
                .for_each(|socket| {
                    let socket_addr = socket.peer_addr().unwrap().to_string();
                    println!("accepted socket; addr={:?}", &socket_addr);


                    // split the socket into a framed sink and stream
                    let (writer, reader) = TestBytes.framed(socket).split();

                    // mpsc (multi producer single consumer)
                    // split the mpsc into (sender, receiver)
                    let (tx, rx) = mpsc::unbounded();

                    // clone the mpsc sender
                    let conntx = tx.clone();

                    // send message, it could be stored and used anywhere
                    mpsc::UnboundedSender::send(&conntx, "hey there\n".as_bytes().to_vec());

                    // read from the split socket
                    let read = reader.for_each(move |frame| {
                        // convert to string
                        let s = String::from_utf8(frame).unwrap();
                        println!("{}", s);
                        // add messages to the mpsc sender
                        mpsc::UnboundedSender::send(&conntx, format!("got {}\n", s).as_bytes().to_vec());
                        Ok(())
                    }).map_err(|e| eprintln!("Error: {:?}", e));
                    // spawn a tokio thread for the read
                    tokio::spawn(read);

                    // write to the stream part of the split socket by reading from the receiver part of the mpsc
                    let write = writer.send_all(rx.map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "rx shouldn't have an error")
                    })).then(|_| Ok(()));
                    // spawn tokio thread for the write
                    tokio::spawn(write);

                    Ok(())
                });
            tokio::run(server);
        }).unwrap_or(println!("Failed to bind to {}", &self.node.info.id));
    }
}


#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Msg {
    Ping((String, SocketAddr)),
    Pong((String, SocketAddr)),
    Payload(String),
    AddrVec(Vec<(String, SocketAddr)>),
}