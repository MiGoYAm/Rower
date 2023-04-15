use super::Packet;
use crate::protocol::{util, ProtocolVersion};
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
            protocol: util::get_varint(buf)?,
            server_address: util::get_string(buf, 255)?,
            port: buf.get_u16(),
            state: buf.get_u8(),
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        util::put_varint(buf, self.protocol as u32);
        util::put_string(buf, &self.server_address);
        buf.put_u16(self.port);
        buf.put_u8(self.state);
    }
}
