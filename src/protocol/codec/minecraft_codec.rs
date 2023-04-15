use std::{error::Error, io::ErrorKind};

use bytes::{BytesMut, Buf, BufMut};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpStream, tcp::{ReadHalf, WriteHalf}};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::protocol::{packet::{NextPacket, Packet, RawPacket}, Direction, ProtocolVersion};

use super::{decoder::MinecraftDecoder, encoder::{MinecraftEncoder, MinecraftEncoderComp}, reg::{ProtocolRegistry, HANDSHAKE_REG, StateRegistry}};


pub struct Connection<'a> {
    pub protocol: ProtocolVersion,
    direction: Direction,

    send_registry: &'static ProtocolRegistry,
    receive_registry: &'static ProtocolRegistry,
    
    decoder: FramedRead<ReadHalf<'a>, MinecraftDecoder>,
    encoder: FramedWrite<WriteHalf<'a>, MinecraftEncoder>,
}

impl<'a> Connection<'a> {
    pub fn new(stream: &'a mut TcpStream, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REG.get_registry(&direction, &ProtocolVersion::Unknown);
        let (reader, writer) = stream.split();

        Self { 
            protocol: ProtocolVersion::Unknown,
            direction,

            send_registry,
            receive_registry,

            decoder: FramedRead::new(reader, MinecraftDecoder::new()),
            encoder: FramedWrite::new(writer, MinecraftEncoder::new()),
        }
    }

    pub fn set_registry(&mut self, registry: &'static StateRegistry) {
        let (receive_registry, send_registry) = registry.get_registry(&self.direction, &self.protocol);
        self.receive_registry = receive_registry;
        self.send_registry = send_registry;
    }

    pub async fn next_packet(&mut self) -> Result<NextPacket, tokio::io::Error> {
        let frame = self.read_frame().await?;

        Ok(self.receive_registry.decode(frame, self.protocol))
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
        match self.decoder.next().await {
            Some(r) => r,
            None => Err(ErrorKind::ConnectionAborted.into()),
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> Result<(), Box<dyn Error>> {
        self.encoder.send(packet).await
    }

    pub async fn write_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut raw_packet = RawPacket {
            id: self.send_registry.get_id::<T>()?.clone(),
            data: BytesMut::new()
        };

        packet.put_buf(&mut raw_packet.data, self.protocol);

        self.encoder.send(raw_packet).await
    }

    pub async fn put_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut raw_packet = RawPacket {
            id: self.send_registry.get_id::<T>()?.clone(),
            data: BytesMut::new()
        };

        packet.put_buf(&mut raw_packet.data, self.protocol);

        self.encoder.feed(raw_packet).await
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        //self.codec.close().await?;
        //self.codec.get_mut().shutdown().await?;
        Ok(())
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        //self.encoder.map_encoder(|e| {});
        //self.codec.codec_mut().encryption = true;
    }
}


/*
pub struct Connection<'a> {
    pub protocol: i32,
    direction: Direction,
    send_registry: &'static Registry,
    receive_registry: &'static Registry,
    
    pub decoder: FrameDecoder<'a>,
    pub encoder: FrameEncoder<'a>,
}

impl<'a> Connection<'a> {
    pub fn new(stream: &'a mut TcpStream, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REGISTRY.get_registry(&direction);
        let (reader, writer) = stream.split();

        Self { 
            protocol: 0,
            direction,

            send_registry,
            receive_registry,

            decoder: FrameDecoder::new(reader, receive_registry),
            encoder: FrameEncoder::new(writer, send_registry),
        }
    }

    pub fn set_registry(&mut self, registry: &'static StateRegistry) {
        let (receive_registry, send_registry) = registry.get_registry(&self.direction);
        self.decoder.registry = receive_registry;
        self.encoder.registry = send_registry;
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        //self.codec.close().await?;
        //self.codec.get_mut().shutdown().await?;
        Ok(())
    }

    pub fn enable_compression(&mut self) {
        //self.codec.codec_mut().encryption = true;
    }
}
*/
