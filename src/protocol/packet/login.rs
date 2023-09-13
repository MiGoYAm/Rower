use crate::protocol::util::{get_bool, get_byte_array, get_string, get_varint, put_bool, put_byte_array, put_str, put_string, put_varint};
use crate::protocol::ProtocolVersion;
use crate::Component;
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

use super::Packet;

pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl Packet for LoginStart {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            username: get_string(buf, 16)?,
            uuid: if get_bool(buf)? { Some(Uuid::from_u128(buf.get_u128())) } else { None },
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.username);
        match self.uuid {
            Some(uuid) => {
                put_bool(buf, true);
                buf.extend_from_slice(uuid.as_bytes());
            }
            None => put_bool(buf, false),
        }
    }
}

pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

impl Packet for LoginSuccess {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            uuid: Uuid::from_u128(buf.get_u128()),
            username: get_string(buf, 16)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_u128(self.uuid.as_u128());
        //put_string(buf, &self.username);
        put_str(buf, &self.username);
        // length of properties
        buf.put_u8(0);
    }
}

pub struct Disconnect {
    pub reason: Component,
}

impl Packet for Disconnect {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            reason: serde_json::from_str::<Component>(get_string(buf, 262144)?.as_str())?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &serde_json::to_string(&self.reason).unwrap());
    }
}

pub struct SetCompression {
    pub threshold: i32,
}

impl Packet for SetCompression {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self { threshold: get_varint(buf)? })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.threshold as u32)
    }
}

pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl Packet for EncryptionRequest {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            server_id: get_string(buf, 20)?,
            public_key: get_byte_array(buf)?,
            verify_token: get_byte_array(buf)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.server_id);
        put_byte_array(buf, &self.public_key);
        put_byte_array(buf, &self.verify_token);
    }
}

pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl Packet for EncryptionResponse {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            shared_secret: get_byte_array(buf)?,
            verify_token: get_byte_array(buf)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_byte_array(buf, &self.shared_secret);
        put_byte_array(buf, &self.verify_token);
    }
}

pub struct LoginPluginRequest {
    pub message_id: i32,
    pub channel: String,
    pub data: BytesMut,
}

impl Packet for LoginPluginRequest {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            message_id: get_varint(buf)?,
            channel: get_string(buf, 32767)?,
            data: buf.split(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.message_id as u32);
        put_string(buf, &self.channel);
        buf.extend_from_slice(&self.data);
    }
}

pub struct LoginPluginResponse {
    pub message_id: i32,
    pub successful: bool,
    pub data: Option<BytesMut>,
}

impl Packet for LoginPluginResponse {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let message_id = get_varint(buf)?;
        let successful = get_bool(buf)?;
        Ok(Self {
            message_id,
            successful,
            data: if successful { Some(buf.split()) } else { None },
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_varint(buf, self.message_id as u32);
        put_bool(buf, self.successful);

        if !self.successful {
            return;
        }
        if let Some(data) = &self.data {
            buf.extend_from_slice(data);
        }
    }
}
