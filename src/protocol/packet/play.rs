use bytes::BytesMut;

use crate::protocol::{util::{get_string, put_string}, ProtocolVersion};

use super::Packet;

pub struct PluginMessage {
    pub channel: String,
    pub data: Vec<u8>
}

impl Packet for PluginMessage {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn std::error::Error>>
    where Self: Sized {
        Ok(Self {
            channel: get_string(buf, 32700)?,
            data: buf.to_vec()
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.channel);
        buf.extend_from_slice(&self.data);
    }
}
