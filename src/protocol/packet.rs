use bytes::BytesMut;

use self::{
    handshake::Handshake,
    login::{Disconnect, EncryptionRequest, EncryptionResponse, LoginStart, LoginSuccess, SetCompression, LoginPluginRequest, LoginPluginResponse},
    play::PluginMessage,
    status::{Ping, StatusResponse},
};

use super::ProtocolVersion;

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

pub trait Packet {
    fn from_bytes(buf: &mut BytesMut, version: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn put_buf(&self, buf: &mut BytesMut, version: ProtocolVersion);
}

/*
pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut,
}
*/
pub struct RawPacket {
    pub buffer: BytesMut,
}

impl RawPacket {
    pub fn new() -> Self {
        Self { buffer: BytesMut::zeroed(1) }
    }

    pub fn set_id(&mut self, id: u8) {
        self.buffer[0] = id;
    }

    pub fn data(&mut self) -> BytesMut {
        self.buffer.split_off(1)
    }
}

pub enum PacketType<'a> {
    Raw(RawPacket),
    Handshake(Handshake),

    StatusRequest,
    StatusResponse(StatusResponse<'a>),
    Ping(Ping),

    LoginStart(LoginStart),
    EncryptionRequest(EncryptionRequest),
    EncryptionResponse(EncryptionResponse),
    SetCompression(SetCompression),
    LoginSuccess(LoginSuccess),
    LoginPluginRequest(LoginPluginRequest),
    LoginPluginResponse(LoginPluginResponse),
    Disconnect(Disconnect),

    PluginMessage(PluginMessage),
}
