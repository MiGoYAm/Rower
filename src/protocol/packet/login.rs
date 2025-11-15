use crate::online::Property;
use crate::protocol::buffer::{BufExt, BufMutExt};
use crate::protocol::util::{get_array, get_property, put_array, put_property};
use crate::protocol::{Direction, ProtocolVersion, State};
use crate::Component;
use anyhow::{anyhow, ensure, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;

use super::{Packet, Packets, RawPacket};

pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl Packet for LoginStart {
    fn from_bytes(buf: &mut impl Buf, version: ProtocolVersion) -> Result<Self> {
        let username = buf.get_string(16)?;
        let uuid = if version >= ProtocolVersion::V1_20_2 {
            Some(buf.get_uuid())
        } else if version >= ProtocolVersion::V1_19_2 {
            buf.get_option(|b| Ok(b.get_uuid()))?
        } else {
            None
        };

        Ok(Self { username, uuid })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.username);
        buf.put_option(&self.uuid, |b, u| b.put_uuid(*u));
    }
}

pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
    pub properties: Vec<Property>,
}

impl Packet for LoginSuccess {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            uuid: buf.get_uuid(),
            username: buf.get_string(16)?,
            properties: get_array(buf, get_property)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_uuid(self.uuid);
        buf.put_string(&self.username);
        put_array(buf, self.properties, put_property);
    }
}

pub struct LoginAcknowledged;

impl Packet for LoginAcknowledged {
    fn from_bytes(_: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self)
    }

    fn put_buf(self, _: &mut BytesMut, _: ProtocolVersion) {}
}

pub struct Disconnect {
    pub reason: Component,
}

impl Packet for Disconnect {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
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
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            threshold: buf.get_varint()?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.threshold)
    }
}

pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: [u8; 4],
}

impl Packet for EncryptionRequest {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            server_id: buf.get_string(20)?,
            public_key: buf.get_bytes()?.to_vec(),
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
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            shared_secret: buf.get_bytes()?,
            verify_token: buf.get_bytes()?,
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
    pub data: Bytes,
}

impl Packet for LoginPluginRequest {
    fn from_bytes(buf: &mut impl Buf, version: ProtocolVersion) -> Result<Self> {
        ensure!(
            version >= ProtocolVersion::V1_13,
            "LoginPluginRequest is not available in this version"
        );
        Ok(Self {
            message_id: buf.get_varint()?,
            channel: buf.get_identifier()?,
            data: buf.rest(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.message_id);
        buf.put_string(&self.channel);
        buf.put_slice(&self.data);
    }
}

pub struct LoginPluginResponse {
    pub message_id: i32,
    pub successful: bool,
    pub data: Option<Bytes>,
}

impl Packet for LoginPluginResponse {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        let message_id = buf.get_varint()?;
        let successful = buf.get_bool()?;
        Ok(Self {
            message_id,
            successful,
            data: if successful && buf.has_remaining() {
                Some(buf.rest())
            } else {
                None
            },
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_varint(self.message_id);
        buf.put_bool(self.successful);

        if self.successful {
            if let Some(data) = &self.data {
                buf.put_slice(data);
            }
        }
    }
}

pub enum LoginPackets {
    EncryptionRequest(EncryptionRequest),
    SetCompression(SetCompression),
    LoginSuccess(LoginSuccess),
    LoginPluginRequest(LoginPluginRequest),
    Disconnect(Disconnect),
}

impl Packets for LoginPackets {
    fn decode(
        direction: Direction,
        state: State,
        version: ProtocolVersion,
        mut packet: RawPacket,
    ) -> Result<Self> {
        match (direction, state) {
            (Direction::Clientbound, State::Login) => {}
            _ => panic!("LoginPackets only available for clientbound login state"),
        }

        Ok(match packet.id() {
            0x01 => {
                Self::EncryptionRequest(EncryptionRequest::from_bytes(&mut packet.data(), version)?)
            }
            0x03 => Self::SetCompression(SetCompression::from_bytes(&mut packet.data(), version)?),
            0x02 => Self::LoginSuccess(LoginSuccess::from_bytes(&mut packet.data(), version)?),
            0x00 => Self::Disconnect(Disconnect::from_bytes(&mut packet.data(), version)?),
            0x04 => Self::LoginPluginRequest(LoginPluginRequest::from_bytes(
                &mut packet.data(),
                version,
            )?),
            _ => return Err(anyhow!("Unknown packet id in login packets")),
        })
    }
}
