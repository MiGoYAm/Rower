use crate::protocol::ProtocolVersion;
use crate::protocol::util::{get_string, put_byte_array, put_str, self, get_varint, put_varint};
use crate::TextComponent;
use bytes::{Buf, BufMut, BytesMut};
use std::error::Error;
use uuid::Uuid;

use super::Packet;

pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>
}

impl Packet for LoginStart {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        Ok(LoginStart { 
            username: get_string(buf, 16)?,
            uuid: if util::get_bool(buf)? { Some(Uuid::from_u128(buf.get_u128())) } else { None }
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        util::put_string(buf, &self.username);
        match self.uuid {
            Some(uuid) => buf.put_u128(uuid.as_u128()),
            None => util::put_bool(buf, false)
        }
    }
}

pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

impl Packet for LoginSuccess {

    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized 
    {
        Ok(LoginSuccess { 
            uuid: Uuid::from_u128(buf.get_u128()), 
            username: util::get_string(buf, 16)?
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_u128(self.uuid.as_u128());
        //put_string(buf, &self.username);
        put_str(buf, &self.username);
        // length of properties
        buf.put_u8(0);
    }
}

pub struct Disconnect {
    pub reason: TextComponent,
}

impl Packet for Disconnect {
    
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where Self: Sized {
        let binding = get_string(buf, 262144)?;
        let s = binding.as_str();
        Ok(Self { 
            reason: serde_json::from_str::<TextComponent>(s)? 
        })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_byte_array(buf, &serde_json::to_vec(&self.reason).unwrap())
    }
}

pub struct SetCompression {
    threshold: i32
}

impl Packet for SetCompression {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where Self: Sized {
        Ok(Self { threshold: get_varint(buf)? })
    }

    fn put_buf(&self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.threshold as u32)
    }
}
