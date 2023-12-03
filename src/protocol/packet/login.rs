use crate::protocol::buffer::{BufExt, BufMutExt};
use crate::protocol::util::{get_array, get_property, put_array, put_property};
use crate::protocol::ProtocolVersion;
use crate::Component;
use anyhow::Result;
use bytes::{BytesMut, Bytes};
use uuid::Uuid;

use super::Packet;

pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl Packet for LoginStart {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            username: buf.get_string(16)?,
            uuid: buf.get_option(|b| Ok(b.get_uuid()))?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.username);
        match self.uuid {
            Some(uuid) => {
                buf.put_bool(true);
                buf.put_uuid(uuid);
            }
            None => buf.put_bool(false),
        }
    }
}

pub struct Property {
    pub name: String,
    pub value: String,
    pub signature: Option<String>
}

pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
    pub properties: Vec<Property>
}

impl Packet for LoginSuccess {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            uuid: buf.get_uuid(),
            username: buf.get_string(16)?,
            properties: get_array(buf, get_property)?
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_uuid(self.uuid);
        buf.put_string(&self.username);
        put_array(buf, self.properties, put_property);
    }
}

pub struct Disconnect {
    pub reason: Component,
}

impl Packet for Disconnect {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            reason: buf.get_component()?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_component(&self.reason).unwrap();
    }
}

pub struct SetCompression {
    pub threshold: i32,
}

impl Packet for SetCompression {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self { threshold: buf.get_varint()? })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.threshold)
    }
}

pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Bytes,
    pub verify_token: Bytes,
}

impl Packet for EncryptionRequest {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            server_id: buf.get_string(20)?,
            public_key: buf.get_byte_array()?,
            verify_token: buf.get_byte_array()?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.server_id);
        buf.put_byte_array(&self.public_key);
        buf.put_byte_array(&self.verify_token);
    }
}

pub struct EncryptionResponse {
    pub shared_secret: Bytes,
    pub verify_token: Bytes,
}

impl Packet for EncryptionResponse {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            shared_secret: buf.get_byte_array()?,
            verify_token: buf.get_byte_array()?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_byte_array(&self.shared_secret);
        buf.put_byte_array(&self.verify_token);
    }
}

pub struct LoginPluginRequest {
    pub message_id: i32,
    pub channel: String,
    pub data: BytesMut,
}

impl Packet for LoginPluginRequest {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            message_id: buf.get_varint()?,
            channel: buf.get_identifier()?,
            data: buf.split(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.message_id);
        buf.put_string(&self.channel);
        buf.extend_from_slice(&self.data);
    }
}

pub struct LoginPluginResponse {
    pub message_id: i32,
    pub successful: bool,
    pub data: Option<BytesMut>,
}

impl Packet for LoginPluginResponse {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> Result<Self> {
        let message_id = buf.get_varint()?;
        let successful = buf.get_bool()?;
        Ok(Self {
            message_id,
            successful,
            data: if successful && !buf.is_empty() { Some(buf.split()) } else { None },
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.message_id);
        buf.put_bool(self.successful);

        if self.successful {
            if let Some(data) = &self.data {
                buf.extend_from_slice(data);
            }
        }
    }
}
