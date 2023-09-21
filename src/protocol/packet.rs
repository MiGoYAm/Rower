use bytes::BytesMut;

use self::{
    login::{Disconnect, EncryptionRequest, EncryptionResponse, LoginStart, LoginSuccess, SetCompression, LoginPluginRequest, LoginPluginResponse},
    play::PluginMessage,
};

use super::ProtocolVersion;

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

pub trait Packet: Sized {
    fn from_bytes(buf: &mut BytesMut, version: ProtocolVersion) -> anyhow::Result<Self>;

    fn put_buf(self, buf: &mut BytesMut, version: ProtocolVersion);
}

pub struct RawPacket {
    pub buffer: BytesMut,
}

impl RawPacket {
    pub fn new() -> Self {
        Self { buffer: BytesMut::zeroed(1) }
    }

    pub fn id(&mut self) -> u8 {
        self.buffer[0]
    }

    pub fn set_id(&mut self, id: u8) {
        self.buffer[0] = id;
    }

    pub fn data(&mut self) -> BytesMut {
        self.buffer.split_off(1)
    }
}

pub enum PacketType {
    Raw(RawPacket),

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
