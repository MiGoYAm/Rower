use anyhow::{anyhow, ensure, Context, Result};
use std::{any::type_name, net::SocketAddr};

use bytes::Buf;
use futures::{SinkExt, StreamExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{
    component::Component,
    protocol::{
        packet::{login::Disconnect, Packet, PacketType, RawPacket},
        Direction, ProtocolVersion, State,
    },
};

use super::{
    decoder::MinecraftDecoder,
    encoder::MinecraftEncoder,
    registry::{get_protocol_registry, ProtocolRegistry, HANDSHAKE_REG},
};

pub struct Connection {
    pub protocol: ProtocolVersion,
    direction: Direction,

    receive_registry: &'static ProtocolRegistry,
    send_registry: &'static ProtocolRegistry,

    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

impl Connection {
    fn create(stream: TcpStream, protocol: ProtocolVersion, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REG.get_registry(direction);
        let (reader, writer) = stream.into_split();

        Self {
            protocol,
            direction,

            receive_registry,
            send_registry,

            framed_read: FramedRead::new(reader, MinecraftDecoder::new()),
            framed_write: FramedWrite::new(writer, MinecraftEncoder::new()),
        }
    }

    pub fn new(stream: TcpStream, direction: Direction) -> Self {
        Self::create(stream, ProtocolVersion::Unknown, direction)
    }

    pub async fn connect_to(
        addr: SocketAddr,
        version: ProtocolVersion,
        direction: Direction,
    ) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self::create(stream, version, direction))
    }

    pub fn change_state(&mut self, state: State) {
        (self.receive_registry, self.send_registry) =
            get_protocol_registry(state, self.protocol, self.direction);
    }

    pub async fn auto_read(&mut self) -> Result<PacketType> {
        let mut packet = self.recv_raw_packet().await?;

        if let Some(producer) = self.receive_registry.get_packet(packet.id()) {
            let mut data = packet.data();
            let result = producer(&mut data, self.protocol)?;
            ensure!(data.is_empty(), "Packet was not been fully read");
            Ok(result)
        } else {
            Ok(PacketType::Raw(packet))
        }
    }

    pub async fn recv_packet<T: Packet + 'static>(&mut self) -> Result<T> {
        let expected_id = self.receive_registry.get_id::<T>()?;

        let mut frame = self.recv_raw_packet().await?.buffer;
        let id = frame.get_u8();

        ensure!(
            *expected_id == id,
            "Invalid provided packet. Packet id: Provided: {:#04X?}, Got: {:#04X?}",
            expected_id,
            id
        );

        let packet = T::from_bytes(&mut frame, self.protocol).context(type_name::<T>())?;
        ensure!(
            frame.is_empty(),
            "Packet was not been fully read. Packet: {:}",
            type_name::<T>()
        );
        Ok(packet)
    }

    pub async fn recv_raw_packet(&mut self) -> Result<RawPacket> {
        match self.framed_read.next().await {
            Some(result) => Ok(RawPacket { buffer: result? }),
            None => Err(anyhow!("Connection aborted")),
        }
    }

    pub async fn send_raw_packet(&mut self, packet: RawPacket) -> Result<()> {
        self.framed_write.send(packet).await
    }

    pub async fn queue_raw_packet(&mut self, packet: RawPacket) -> Result<()> {
        self.framed_write.feed(packet).await
    }

    pub async fn auto_send_raw_packet(&mut self, packet: RawPacket) -> Result<()> {
        if self.framed_read.read_buffer().is_empty() {
            self.framed_write.send(packet).await
        } else {
            self.framed_write.feed(packet).await
        }
    }

    pub async fn send_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<()> {
        let id = self.send_registry.get_id::<T>()?;
        let raw_packet = self.serialize_packet(packet, *id)?;
        self.send_raw_packet(raw_packet).await
    }

    pub async fn queue_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<()> {
        let id = self.send_registry.get_id::<T>()?;
        let raw_packet = self.serialize_packet(packet, *id)?;
        self.queue_raw_packet(raw_packet).await
    }

    pub async fn auto_send_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<()> {
        let id = self.send_registry.get_id::<T>()?;
        let raw_packet = self.serialize_packet(packet, *id)?;
        self.auto_send_raw_packet(raw_packet).await
    }

    fn serialize_packet<T: Packet + 'static>(&self, packet: T, id: u8) -> Result<RawPacket> {
        let mut raw_packet = RawPacket::new();
        raw_packet.set_id(id);

        let mut data = raw_packet.data();
        packet.put_buf(&mut data, self.protocol);
        raw_packet.buffer.unsplit(data);

        Ok(raw_packet)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.framed_write.close().await
    }

    pub async fn disconnect(mut self, reason: Component) -> Result<()> {
        self.send_packet(Disconnect { reason }).await?;
        self.shutdown().await
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.framed_read.decoder_mut().enable_compression();
        self.framed_write
            .encoder_mut()
            .enable_compression(threshold);
    }

    pub fn enable_encryption(&mut self, key: [u8; 16]) -> Result<()> {
        self.framed_write.encoder_mut().enable_encryption(key)
    }

    pub fn mix(self, connection: Connection) -> (Connection, Connection) {
        (
            Connection {
                protocol: self.protocol,
                direction: self.direction,

                receive_registry: self.receive_registry,
                send_registry: connection.send_registry,

                framed_read: self.framed_read,
                framed_write: connection.framed_write,
            },
            Connection {
                protocol: connection.protocol,
                direction: connection.direction,

                receive_registry: connection.receive_registry,
                send_registry: self.send_registry,

                framed_read: connection.framed_read,
                framed_write: self.framed_write,
            },
        )
    }
}
