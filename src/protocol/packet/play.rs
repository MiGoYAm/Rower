use bytes::BytesMut;

use crate::protocol::{
    util::{get_string, put_string},
    ProtocolVersion,
};

use super::Packet;

pub struct PluginMessage {
    pub channel: String,
    pub data: BytesMut,
}

impl Packet for PluginMessage {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            channel: get_string(buf, 32700)?,
            data: buf.split(),
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.channel);
        buf.extend_from_slice(&self.data);
    }
}
