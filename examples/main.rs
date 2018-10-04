extern crate p2p;
extern crate getopts;

use std::env;
use p2p::node::Node;

// # to run it:
// cargo run --example main -- -a 127.0.0.1:21335 -p "127.0.0.1:21337,127.0.0.1:21337" // second parameter peer addresses
fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = getopts::Options::new();
    //
    opts.optopt("a", "", "set the host address", "ADDR");
    opts.optopt("p", "", "initial peers (CSV)", "PEERS");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    let peer_str = matches.opt_str("p").unwrap_or("".to_owned());
    let peer_tokens: Vec<&str> = peer_str.split(",").collect() ;
    let peers = peer_tokens.into_iter().map(|p| p.parse().unwrap());
    println!("Peers => {:?}", peers);

    let add = matches.opt_str("a").unwrap_or("127.0.0.1:21337".to_owned());
    let node = Node::new(add).unwrap();
    node.run(peers)
}
