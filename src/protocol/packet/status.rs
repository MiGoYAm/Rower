use crate::protocol::util::put_byte_array;
use crate::protocol::ProtocolVersion;
use crate::Component;
use bytes::{Buf, BufMut, BytesMut};
use serde::{Serialize, Serializer};
use uuid::Uuid;

use super::Packet;

pub struct StatusRequest;

impl Packet for StatusRequest {
    fn from_bytes(_: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
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

impl<'a> Packet for StatusResponse<'a> {
    fn from_bytes(_buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        todo!()
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_byte_array(buf, self.status)
    }
}

pub struct Ping {
    payload: i64,
}

impl Packet for Ping {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        Ok(Self { payload: buf.get_i64() })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_i64(self.payload);
    }
}
