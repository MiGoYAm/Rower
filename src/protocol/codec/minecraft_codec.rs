use std::{error::Error, io::ErrorKind};

use bytes::{BytesMut, Buf};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpStream, tcp::{OwnedWriteHalf, OwnedReadHalf}};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::protocol::{packet::{PacketType, Packet, RawPacket}, Direction, ProtocolVersion, HANDSHAKE, STATUS, LOGIN, PLAY};

use super::{decoder::MinecraftDecoder, encoder::MinecraftEncoder, registry::{ProtocolRegistry, HANDSHAKE_REG, StateRegistry, STATUS_REG, LOGIN_REG, PLAY_REG}};

pub struct Connection {
    pub protocol: ProtocolVersion,
    direction: Direction,

    send_registry: &'static ProtocolRegistry,
    receive_registry: &'static ProtocolRegistry,
    
    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

impl Connection {
    

    pub fn new(stream: TcpStream, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REG.get_registry(&direction, &ProtocolVersion::Unknown);
        let (reader, writer) = stream.into_split();

        Self { 
            protocol: ProtocolVersion::Unknown,
            direction,

            send_registry,
            receive_registry,

            framed_read: FramedRead::new(reader, MinecraftDecoder::new()),
            framed_write: FramedWrite::new(writer, MinecraftEncoder::new()),
        }
    }

    pub async fn connect(addr: &str, version: ProtocolRegistry, direction: Direction) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(TcpStream::connect(addr).await?, direction))
    }

    pub fn change_state(&mut self, state: u8) {
        let registry = match state {
            HANDSHAKE => &HANDSHAKE_REG,
            STATUS => &STATUS_REG,
            LOGIN => &LOGIN_REG,
            PLAY => &PLAY_REG,
            _ => panic!("invalid state")
        };
        self.set_registry(registry);
    }

    fn set_registry(&mut self, registry: &'static StateRegistry) {
        (self.receive_registry, self.send_registry) = registry.get_registry(&self.direction, &self.protocol);
    }

    pub async fn next_packet(&mut self) -> Result<PacketType, Box<dyn Error>> {
        let frame = self.read_frame().await?;

        self.receive_registry.decode(frame, self.protocol)
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> Result<T, Box<dyn Error>> {
        let mut frame = self.read_frame().await?;
        let id = frame.get_u8();
        let registry_id = self.receive_registry.get_id::<T>()?;

        if registry_id != &id {
            return Err(format!("Invalid provided packet. Packet id: Provided: {}, Got: {}", registry_id, id).into());
        }

        T::from_bytes(&mut frame, self.protocol)
    }

    async fn read_frame(&mut self) -> Result<BytesMut, tokio::io::Error> {
        match self.framed_read.next().await {
            Some(r) => r,
            None => Err(ErrorKind::ConnectionAborted.into()),
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> Result<(), Box<dyn Error>> {
        self.framed_write.send(packet).await
    }

    pub async fn write_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let raw_packet = self.serialize_packet(packet)?;
        self.framed_write.send(raw_packet).await
    }

    pub async fn queue_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let raw_packet = self.serialize_packet(packet)?;
        self.framed_write.feed(raw_packet).await
    }

    fn serialize_packet<T: Packet + 'static>(&self, packet: T) -> Result<RawPacket, Box<dyn Error>> {
        let mut raw_packet = RawPacket {
            id: self.send_registry.get_id::<T>()?.clone(),
            data: BytesMut::new()
        };

        packet.put_buf(&mut raw_packet.data, self.protocol);
        Ok(raw_packet)
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
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
