#![crate_type = "lib"]
#![crate_name = "p2p"]

pub mod codec;
pub mod node;
pub mod peer;

extern crate serde;
extern crate serde_json;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
extern crate tokio;
extern crate tokio_codec;
extern crate futures;

extern crate getopts;
extern crate core;
