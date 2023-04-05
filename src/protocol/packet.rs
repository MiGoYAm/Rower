use std::error::{Error};

use bytes::BytesMut;

use self::{handshake::Handshake, login::{LoginStart, LoginSuccess, Disconnect, SetCompression}, status::{StatusRequest, Ping}};

pub mod handshake;
pub mod login;
pub mod status;

pub trait Packet {

    fn from_bytes(buf: &mut BytesMut, version: i32) -> Result<Self, Box<dyn Error>>
    where Self: Sized;

    fn put_buf(&self, buf: &mut BytesMut, version: i32);
}

pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut
}

pub enum NextPacket {
    RawPacket(RawPacket),
    Handshake(Lazy<Handshake>),

    StatusRequest(Lazy<StatusRequest>),
    Ping(Lazy<Ping>),

    Disconnect(Lazy<Disconnect>),
    LoginStart(Lazy<LoginStart>),
    SetCompression(Lazy<SetCompression>),
    LoginSuccess(Lazy<LoginSuccess>),

}

pub struct Lazy<T: Packet> {
    buf: BytesMut,
    version: i32,
    f: fn(&mut BytesMut, i32) -> Result<T, Box<dyn Error>>
}

impl<T: Packet> Lazy<T> {
    pub fn new(buf: BytesMut, version: i32) -> Self {
        Self{buf, version, f: T::from_bytes}
    }

    pub fn get(&mut self) -> Result<T, Box<dyn Error>> {
        (self.f)(&mut self.buf, self.version)
    }
}

