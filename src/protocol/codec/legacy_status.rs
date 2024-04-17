use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::protocol::{ProtocolVersion, Direction};

use super::{decoder::MinecraftDecoder, encoder::MinecraftEncoder};

pub struct Conn {
    pub protocol: ProtocolVersion,
    direction: Direction,

    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

pub struct HandshakeConnection {   
}