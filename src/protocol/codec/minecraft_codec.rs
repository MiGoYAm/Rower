use anyhow::anyhow;
use std::net::SocketAddr;

use bytes::{Buf, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::protocol::{
    packet::{Packet, PacketType, RawPacket},
    Direction, ProtocolVersion, HANDSHAKE, LOGIN, PLAY, STATUS,
};

use super::{
    decoder::MinecraftDecoder,
    encoder::MinecraftEncoder,
    registry::{ProtocolRegistry, StateRegistry, HANDSHAKE_REG, LOGIN_REG, PLAY_REG, STATUS_REG},
};

pub struct Connection {
    pub protocol: ProtocolVersion,
    direction: Direction,

    send_registry: &'static ProtocolRegistry,
    receive_registry: &'static ProtocolRegistry,

    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

impl Connection {
    fn create(stream: TcpStream, version: ProtocolVersion, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REG.get_registry(&direction, &ProtocolVersion::Unknown);
        let (reader, writer) = stream.into_split();

        Self {
            protocol: version,
            direction,

            send_registry,
            receive_registry,

            framed_read: FramedRead::new(reader, MinecraftDecoder::new()),
            framed_write: FramedWrite::new(writer, MinecraftEncoder::new()),
        }
    }

    pub fn new(stream: TcpStream, direction: Direction) -> Self {
        Self::create(stream, ProtocolVersion::Unknown, direction)
    }

    pub async fn connect(addr: SocketAddr, version: ProtocolVersion, direction: Direction) -> anyhow::Result<Self> {
        Ok(Self::create(TcpStream::connect(addr).await?, version, direction))
    }

    pub fn change_state(&mut self, state: u8) {
        let registry = match state {
            HANDSHAKE => &HANDSHAKE_REG,
            STATUS => &STATUS_REG,
            LOGIN => &LOGIN_REG,
            PLAY => &PLAY_REG,
            _ => panic!("invalid state"),
        };
        self.set_registry(registry);
    }

    fn set_registry(&mut self, registry: &'static StateRegistry) {
        (self.receive_registry, self.send_registry) = registry.get_registry(&self.direction, &self.protocol);
    }

    pub async fn next_packet(&mut self) -> anyhow::Result<PacketType> {
        let frame = self.read_frame().await?;

        self.receive_registry.decode(frame, self.protocol)
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> anyhow::Result<T> {
        let mut frame = self.read_frame().await?;
        let id = frame.get_u8();
        let registry_id = self.receive_registry.get_id::<T>()?;

        if registry_id != &id {
            return Err(anyhow!("Invalid provided packet. Packet id: Provided: {}, Got: {}", registry_id, id));
        }

        T::from_bytes(&mut frame, self.protocol)
    }

    async fn read_frame(&mut self) -> anyhow::Result<BytesMut> {
        match self.framed_read.next().await {
            Some(r) => r,
            None => Err(anyhow!("Connection aborted")),
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> anyhow::Result<()> {
        self.framed_write.send(packet).await
    }

    pub async fn write_packet<T: Packet + 'static>(&mut self, packet: T) -> anyhow::Result<()> {
        let raw_packet = self.serialize_packet(packet)?;
        self.framed_write.send(raw_packet).await
    }

    pub async fn queue_packet<T: Packet + 'static>(&mut self, packet: T) -> anyhow::Result<()> {
        let raw_packet = self.serialize_packet(packet)?;
        self.framed_write.feed(raw_packet).await
    }

    fn serialize_packet<T: Packet + 'static>(&self, packet: T) -> anyhow::Result<RawPacket> {
        let mut raw_packet = RawPacket {
            id: *self.send_registry.get_id::<T>()?,
            data: BytesMut::new(),
        };

        packet.put_buf(&mut raw_packet.data, self.protocol);
        Ok(raw_packet)
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.framed_write.close().await
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.framed_read.decoder_mut().enable_compression();
        self.framed_write.encoder_mut().enable_compression(threshold);
    }

    pub fn enable_encryption(&mut self) {
        todo!()
    }
}
