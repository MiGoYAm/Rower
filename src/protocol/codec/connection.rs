use anyhow::{anyhow, ensure};
use std::net::SocketAddr;

use bytes::Buf;
use futures::{SinkExt, StreamExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::protocol::{
    packet::{Packet, PacketType, RawPacket},
    Direction, ProtocolVersion, State
};

use super::{
    decoder::MinecraftDecoder,
    encoder::MinecraftEncoder,
    registry::{ProtocolRegistry, HANDSHAKE_REG, get_protocol_registry},
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
    fn create(stream: TcpStream, protocol: ProtocolVersion, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REG.get_registry(&direction);
        let (reader, writer) = stream.into_split();

        Self {
            protocol,
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
        let tcp = TcpStream::connect(addr).await?;
        tcp.set_nodelay(true)?;
        Ok(Self::create(tcp, version, direction))
    }

    pub fn change_state(&mut self, state: State) {
        (self.receive_registry, self.send_registry) = get_protocol_registry(state, self.protocol, self.direction);
    }

    pub async fn auto_read(&mut self) -> anyhow::Result<PacketType> {
        let mut packet = self.read_raw_packet().await?;

        if let Some(producer) = self.receive_registry.get_packet(packet.id()) {
            let mut data = packet.data();
            let result = producer(&mut data, self.protocol)?;
            ensure!(data.is_empty(), "Packet was not been fully read");
            Ok(result)
        } else {
            Ok(PacketType::Raw(packet))
        }
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> anyhow::Result<T> {
        let mut frame = self.read_raw_packet().await?.buffer;
        let registry_id = self.receive_registry.get_id::<T>()?;
        let id = frame.get_u8();

        ensure!(registry_id == &id, "Invalid provided packet. Packet id: Provided: 0x{:02X?}, Got: 0x{:02X?}", registry_id, id);

        let result = T::from_bytes(&mut frame, self.protocol)?;
        ensure!(frame.is_empty(), "Packet was not been fully read");
        Ok(result)
    }

    pub async fn read_raw_packet(&mut self) -> anyhow::Result<RawPacket> {
        match self.framed_read.next().await {
            Some(result) => Ok(RawPacket { buffer: result? }),
            None => Err(anyhow!("Connection aborted")),
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> anyhow::Result<()> {
        self.framed_write.feed(packet).await?;

        if self.framed_read.read_buffer().is_empty() {
            self.framed_write.flush().await
        } else {
            Ok(())
        }
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
        let mut raw_packet = RawPacket::new();
        raw_packet.set_id(*self.send_registry.get_id::<T>()?);

        let mut data = raw_packet.data();
        packet.put_buf(&mut data, self.protocol);
        raw_packet.buffer.unsplit(data);

        Ok(raw_packet)
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.framed_write.close().await
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        self.framed_read.decoder_mut().enable_compression();
        self.framed_write.encoder_mut().enable_compression(threshold);
    }

    pub fn enable_encryption(&mut self) {
        todo!()
    }

    pub fn split(self) -> (ReadHalf, WriteHalf) {
        (ReadHalf {
            protocol: self.protocol,
            receive_registry: self.receive_registry,
            framed_read: self.framed_read,
        },
        WriteHalf {
            protocol: self.protocol,
            send_registry: self.send_registry,
            framed_write: self.framed_write,
        })
    }
}

pub struct WriteHalf {
    pub protocol: ProtocolVersion,
    send_registry: &'static ProtocolRegistry,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

impl WriteHalf {
    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> anyhow::Result<()> {
        self.framed_write.feed(packet).await
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
        let mut raw_packet = RawPacket::new();
        raw_packet.set_id(*self.send_registry.get_id::<T>()?);

        let mut data = raw_packet.data();
        packet.put_buf(&mut data, self.protocol);
        raw_packet.buffer.unsplit(data);

        Ok(raw_packet)
    }

    pub async fn flush(&mut self) -> anyhow::Result<()> {
        self.framed_write.flush().await
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.framed_write.close().await
    }
}

pub struct ReadHalf {
    pub protocol: ProtocolVersion,
    receive_registry: &'static ProtocolRegistry,
    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
}

impl ReadHalf {
    pub async fn auto_read(&mut self) -> anyhow::Result<PacketType> {
        let mut packet = self.read_raw_packet().await?;

        if let Some(producer) = self.receive_registry.get_packet(packet.id()) {
            let mut data = packet.data();
            let result = producer(&mut data, self.protocol)?;
            ensure!(data.is_empty(), "Packet was not been fully read");
            Ok(result)
        } else {
            Ok(PacketType::Raw(packet))
        }
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> anyhow::Result<T> {
        let mut frame = self.read_raw_packet().await?.buffer;
        let registry_id = self.receive_registry.get_id::<T>()?;
        let id = frame.get_u8();

        ensure!(registry_id == &id, "Invalid provided packet. Packet id: Provided: 0x{:02X?}, Got: 0x{:02X?}", registry_id, id);

        let result = T::from_bytes(&mut frame, self.protocol)?;
        ensure!(frame.is_empty(), "Packet was not been fully read");
        Ok(result)
    }

    pub async fn read_raw_packet(&mut self) -> anyhow::Result<RawPacket> {
        match self.framed_read.next().await {
            Some(result) => Ok(RawPacket { buffer: result? }),
            None => Err(anyhow!("Connection aborted")),
        }
    }

    pub fn is_buffer_empty(&self) -> bool {
        self.framed_read.read_buffer().is_empty()
    }
}
