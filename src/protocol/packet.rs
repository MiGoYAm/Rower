use std::error::Error;

use bytes::BytesMut;

use self::{handshake::Handshake, login::{LoginStart, LoginSuccess, Disconnect, SetCompression, EncryptionRequest, EncryptionResponse}, status::{Ping, StatusResponse}, play::PluginMessage};

use super::ProtocolVersion;

pub mod handshake;
pub mod status;
pub mod login;
pub mod play;

pub trait Packet {
    fn from_bytes(buf: &mut BytesMut, version: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where Self: Sized;

    fn put_buf(&self, buf: &mut BytesMut, version: ProtocolVersion);
}

pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut
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
    Disconnect(Disconnect),

    PluginMessage(PluginMessage)
}
