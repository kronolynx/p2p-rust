use std::io;
use bytes::BytesMut;
use tokio::codec::{Encoder, Decoder};



#[derive(Debug)]
pub struct TestBytes;// {

impl Decoder for TestBytes {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // Remove the serialized frame from the buffer.
            let line = buf.split_to(i + 1);
            Ok(Some(line.to_vec()))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for TestBytes {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, chunk: Self::Item, buf: &mut BytesMut) -> Result<(), io::Error> {
        use bytes::BufMut;

        buf.reserve(chunk.len() + 1);
        buf.put(chunk);
        Ok(())
    }
}
