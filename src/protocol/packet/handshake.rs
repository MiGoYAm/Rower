use anyhow::anyhow;
use super::Packet;
use crate::protocol::{
    util::{get_string, get_varint, put_string, put_varint},
    ProtocolVersion,
};
use bytes::{Buf, BufMut, BytesMut};

pub struct Handshake {
    pub protocol: i32,
    pub server_address: String,
    pub port: u16,
    pub state: NextState,
}

impl Packet for Handshake {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        Ok(Self {
            protocol: get_varint(buf)?,
            server_address: get_string(buf, 255)?,
            port: buf.get_u16(),
            state: NextState::try_from(buf.get_u8())?,
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.protocol as u32);
        put_string(buf, &self.server_address);
        buf.put_u16(self.port);
        buf.put_u8(self.state.u8());
    }
}

pub enum NextState {
    Status,
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

impl NextState {
    fn u8(&self) -> u8{
        match self {
            NextState::Status => 1,
            NextState::Login => 2,
        }
    }
}