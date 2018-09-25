use std::io;
use std::str;
use serde_json;
use uuid::Uuid;
use std::net::SocketAddr;
use tokio::codec::{Encoder, Decoder};
use bytes::{BufMut, BytesMut};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Msg {
    Ping((Uuid, SocketAddr)),
    Pong((Uuid, SocketAddr)),
    Payload(String),
    PeerAddresses(Vec<(Uuid, SocketAddr)>),
}

pub struct MsgCodec; // json line

impl Decoder for MsgCodec {
    type Item = Msg;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // Remove the serialized frame from the buffer.
            let line = buf.split_to(i);
            // Remove the '\n'.
            buf.split_to(1);
            // Turn the line into a UTF string.
            let s = str::from_utf8(&line)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // Parse into json.
            serde_json::from_str(&s)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MsgCodec {
    type Item = Msg;
    type Error = io::Error;

    fn encode(&mut self, msg: Msg, buf: &mut BytesMut)
              -> io::Result<()> {
        let json_msg = serde_json::to_string(&msg)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        // String implements IntoBuf, a trait used by the `bytes` API to work with
        // types that can be expressed as a sequence of bytes.
        buf.put(json_msg);
        // Put the '\n' in the buffer.
        buf.put_u8(b'\n');
        // Return ok to signal that no error occured.
        Ok(())
    }
}

