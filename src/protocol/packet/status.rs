use crate::protocol::{buffer::BufMutExt, ProtocolVersion};
use crate::Component;
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use serde::{Serialize, Serializer};
use uuid::Uuid;

use super::Packet;

pub struct StatusRequest;

impl Packet for StatusRequest {
    fn from_bytes(_: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self)
    }

    fn put_buf(self, _: &mut BytesMut, _: ProtocolVersion) {}
}

#[derive(Serialize)]
pub struct Version {
    pub name: &'static str,
    pub protocol: i32,
}

#[derive(Serialize)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<SamplePlayer>,
}

#[derive(Serialize)]
pub struct SamplePlayer {
    pub name: &'static str,
    pub id: Uuid,
}
pub enum Motd {
    Component(Component),
    Plain(String),
}

impl Serialize for Motd {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Motd::Component(c) => c.serialize(serializer),
            Motd::Plain(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Serialize)]
pub struct Status {
    pub version: Version,
    pub players: Players,

    pub description: Motd,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    //pub previews_chat: bool,
    //pub enforces_secure_chat: bool,
}

pub struct StatusResponse<'a> {
    pub status: &'a Vec<u8>,
}

impl Packet for StatusResponse<'_> {
    fn from_bytes(_buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        unimplemented!("read status response")
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_byte_array(self.status)
    }
}

pub struct Ping(i64);

impl Packet for Ping {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self(buf.get_i64()))
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_i64(self.0);
    }
}
