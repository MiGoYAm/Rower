use std::error::Error;

use bytes::{Buf, BufMut, BytesMut};
use serde::Serialize;
use crate::protocol::util::put_byte_array;
use crate::TextComponent;

use super::Packet;

pub struct StatusRequest;

impl Packet for StatusRequest {
    fn from_bytes(_: &mut BytesMut, _: i32) -> Result<Self, Box<dyn Error>> {
        Ok(Self)
    }

    fn put_buf(&self, _: &mut BytesMut, _: i32) {}
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub version: Version,
    pub players: Players,

    pub description: TextComponent,

    pub previews_chat: bool,
    pub enforces_secure_chat: bool,
}

impl Packet for StatusResponse {
    
    fn from_bytes(buf: &mut BytesMut, version: i32) -> Result<Self, Box<dyn Error>>
    where Self: Sized {
        todo!()
    }

    fn put_buf(&self, buf: &mut BytesMut, _: i32) {
        put_byte_array(buf, serde_json::to_vec(&self).unwrap())
    }
}

// request/response
pub struct Ping {
    v: i64,
}

impl Packet for Ping {
    fn from_bytes(buf: &mut BytesMut, _: i32) -> Result<Self, Box<dyn Error>> where Self: Sized {
        Ok(Ping { v: buf.get_i64() })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: i32) {
        buf.put_i64(self.v);
    }
}
