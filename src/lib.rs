#![crate_type = "lib"]
#![crate_name = "p2p"]

pub mod codec;

extern crate futures;
extern crate tokio;
extern crate uuid;
extern crate serde;
extern crate serde_json;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate bytes;

