use super::Packet;
use crate::protocol::{util::{get_varint, get_string, put_varint, put_string}, ProtocolVersion};
use bytes::{Buf, BytesMut, BufMut};
use std::error::Error;

pub struct Handshake {
    pub protocol: i32,
    pub server_address: String,
    pub port: u16,
    pub state: u8,
}

impl Packet for Handshake {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            protocol: get_varint(buf)?,
            server_address: get_string(buf, 255)?,
            port: buf.get_u16(),
            state: buf.get_u8(),
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.protocol as u32);
        put_string(buf, &self.server_address);
        buf.put_u16(self.port);
        buf.put_u8(self.state);
    }
}
