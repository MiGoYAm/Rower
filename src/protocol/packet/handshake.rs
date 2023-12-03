use anyhow::{anyhow, Result};
use super::Packet;
use crate::protocol::{ProtocolVersion, buffer::{BufExt, BufMutExt}};
use bytes::{Buf, BufMut, BytesMut};

pub struct Handshake {
    pub protocol: i32,
    pub server_address: String,
    pub port: u16,
    pub state: NextState,
}

impl Packet for Handshake {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            protocol: buf.get_varint()?,
            server_address: buf.get_string(255)?,
            port: buf.get_u16(),
            state: NextState::try_from(buf.get_u8())?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.protocol);
        buf.put_string(&self.server_address);
        buf.put_u16(self.port);
        buf.put_u8(self.state as u8);
    }
}

pub enum NextState {
    Status = 1,
    Login
}

impl TryFrom<u8> for NextState {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Status),
            2 => Ok(Self::Login),
            _ => Err(anyhow!("Handshake packet with unknown next state")),
        }
    }
}
