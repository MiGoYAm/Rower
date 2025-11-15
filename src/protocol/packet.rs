use anyhow::Result;
use bytes::{Buf, BytesMut};

use self::{
    login::{Disconnect, EncryptionRequest, EncryptionResponse, LoginPluginRequest, LoginStart, LoginSuccess, SetCompression},
    play::{BossBar, ChatCommand, PluginMessage},
};

use super::{Direction, ProtocolVersion, State};

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

pub trait Packet: Sized {
    fn from_bytes(buf: &mut impl Buf, version: ProtocolVersion) -> Result<Self>;

    fn put_buf(self, buf: &mut BytesMut, version: ProtocolVersion);
}

pub trait Packets {
    fn decode(
        direction: Direction,
        state: State,
        version: ProtocolVersion,
        packet: RawPacket,
    ) -> Result<Self>
    where
        Self: Sized;
}

pub struct RawPacket {
    pub buffer: BytesMut,
}

impl RawPacket {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::zeroed(1),
        }
    }

    pub fn id(&self) -> u8 {
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

    EncryptionRequest(EncryptionRequest),
    SetCompression(SetCompression),
    LoginSuccess(LoginSuccess),
    LoginPluginRequest(LoginPluginRequest),
    Disconnect(Disconnect),

    LoginStart(LoginStart),
    EncryptionResponse(EncryptionResponse),

    PluginMessage(PluginMessage),
    BossBar(BossBar),
    ChatCommand(ChatCommand),
}
