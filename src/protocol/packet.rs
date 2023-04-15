use std::error::Error;

use bytes::BytesMut;

use self::{handshake::Handshake, login::{LoginStart, LoginSuccess, Disconnect, SetCompression}, status::{Ping, StatusResponse}};

use super::ProtocolVersion;

pub mod handshake;
pub mod login;
pub mod status;

pub trait Packet {

    fn from_bytes(buf: &mut BytesMut, version: ProtocolVersion) -> Result<Self, Box<dyn Error>>
    where Self: Sized;

    fn put_buf(&self, buf: &mut BytesMut, version: ProtocolVersion);
}

pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut
}

pub enum NextPacket<'a> {
    RawPacket(RawPacket),
    Handshake(Lazy<Handshake>),

    StatusRequest,
    StatusResponse(Lazy<StatusResponse<'a>>),
    Ping(Lazy<Ping>),

    Disconnect(Lazy<Disconnect>),
    LoginStart(Lazy<LoginStart>),
    SetCompression(Lazy<SetCompression>),
    LoginSuccess(Lazy<LoginSuccess>),

}

pub struct Lazy<T: Packet> {
    buf: BytesMut,
    version: ProtocolVersion,
    f: fn(&mut BytesMut, ProtocolVersion) -> Result<T, Box<dyn Error>>
}

impl<T: Packet> Lazy<T> {
    pub fn new(buf: BytesMut, version: ProtocolVersion) -> Self {
        Self{buf, version, f: T::from_bytes}
    }

    pub fn get(&mut self) -> Result<T, Box<dyn Error>> {
        (self.f)(&mut self.buf, self.version)
    }
}

