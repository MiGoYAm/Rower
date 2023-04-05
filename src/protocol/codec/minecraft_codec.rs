use std::{error::Error};

use bytes::{BytesMut, Buf, BufMut};
use futures::{SinkExt, StreamExt};
use tokio::{net::TcpStream, io::AsyncWriteExt};
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::protocol::{packet::{NextPacket, Packet, RawPacket}, Direction};

use super::{error::{FrameToobig, VarintTooBig, ConnectionClosed}, registry::{Registry, HANDSHAKE_REGISTRY, StateRegistry}};

pub const MAX_PACKET_SIZE: usize = 2097151;

pub struct Connection {
    pub protocol: i32,
    direction: Direction,
    send_registry: &'static Registry,
    receive_registry: &'static Registry,
    codec: Framed<TcpStream, MinecraftCodec>
}

impl Connection {
    pub fn new(stream: TcpStream, direction: Direction) -> Self {
        let (receive_registry, send_registry) = HANDSHAKE_REGISTRY.get_registry(&direction);
        Self { 
            protocol: 0,
            direction,
            receive_registry,
            send_registry,
            codec: Framed::new(stream, MinecraftCodec::new())
        }
    }

    pub fn set_registry(&mut self, registry: &'static StateRegistry) {
        let (receive_registry, send_registry) = registry.get_registry(&self.direction);
        self.receive_registry = receive_registry;
        self.send_registry = send_registry;
    }

    pub async fn next_packet(&mut self) -> Result<NextPacket, Box<dyn Error>> {
        let frame = self.read_frame().await?;

        Ok(self.receive_registry.decode(frame, self.protocol))
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> Result<T, Box<dyn Error>> {
        let mut frame = self.read_frame().await?;
        let id = frame.get_u8();
        let registry_id = self.receive_registry.get_id::<T>()?;

        if registry_id != &id {
            Err(format!("Invalid provided packet. Packet id: Provided: {}, Got: {}", registry_id, id))?;
        }

        T::from_bytes(&mut frame, self.protocol)
    }

    async fn read_frame(&mut self) -> Result<BytesMut, Box<dyn Error>> {
        match self.codec.next().await {
            Some(r) => r,
            None => Err(ConnectionClosed.into()),
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        buf.put_u8(packet.id);
        buf.extend_from_slice(&packet.data);

        self.codec.send(buf).await
    }

    pub async fn write_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        let id = self.send_registry.get_id::<T>()?;
        buf.put_u8(*id);
        packet.put_buf(&mut buf, self.protocol);

        self.codec.send(buf).await
    }

    pub async fn put_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        let id = self.send_registry.get_id::<T>()?;
        buf.put_u8(*id);
        packet.put_buf(&mut buf, self.protocol);

        self.codec.feed(buf).await
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.codec.close().await?;
        self.codec.get_mut().shutdown().await?;
        Ok(())
    }
}

pub struct MinecraftCodec {
    state: DecodeState
}

impl MinecraftCodec {
    pub fn new() -> Self {
        Self { state: DecodeState::ReadVarint(0, 0) }
    }
}

fn write_varint(mut value: u32, dst: &mut BytesMut) {
    loop {
        if (value & 0xFFFFFF80) == 0 {
            dst.put_u8(value as u8);
            return;
        }

        dst.put_u8((value & 0x7F | 0x80) as u8);
        value >>= 7;
    }
}

impl Encoder<BytesMut> for MinecraftCodec {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let length = item.len();
        if length > MAX_PACKET_SIZE {
            return Err(FrameToobig.into());
        }

        dst.reserve(length + 3);

        write_varint(length as u32, dst);
        dst.extend_from_slice(&item);
        Ok(())
    }
}

enum DecodeState {
    ReadVarint(i32, i32),
    Data(i32),
}

const MAX_HEADER_LENGTH: i32 = 3;
fn read_varint(mut value: i32, readed_bytes: i32, src: &mut BytesMut) -> Result<DecodeState, Box<dyn Error>> {
    let max_read = i32::min(MAX_HEADER_LENGTH, src.len() as i32);

    for i in readed_bytes..max_read {
        let byte = src.get_u8();
        value |= ((byte & 0x7F) as i32) << (i * 7);

        if (byte & 0x80) != 128 {
            return Ok(DecodeState::Data(value));
        }
    }

    if max_read < MAX_HEADER_LENGTH {
        return Ok(DecodeState::ReadVarint(value, max_read));
    }
    Err(VarintTooBig.into()) 
}

impl Decoder for MinecraftCodec {
    type Item = BytesMut;
    type Error = Box<dyn std::error::Error>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        //println!("start decoding");
        //for byte in src.to_vec() {
        //   print!("{} ", byte)
        //}
        //println!();
        //println!("buf length: {}", src.len());
        let length = match self.state {
            DecodeState::Data(length) => length,
            DecodeState::ReadVarint(value, readed_bytes) => {
                self.state = read_varint(value, readed_bytes, src)?;
                //println!("reading varint");

                match self.state {
                    DecodeState::Data(length) => length,
                    DecodeState::ReadVarint(_, _) => {
                        //println!("got read varint");
                        //println!("{} {}", x, y);
                        return Ok(None)
                    },
                }
            },
        } as usize;
        //println!("got length: {}", length);
        src.reserve(length.saturating_sub(src.len()));

        if src.len() < length { 
            return Ok(None);
        }
        self.state = DecodeState::ReadVarint(0, 0);

        //println!("packed decoded");
        Ok(Some(
            src.split_to(length)
        ))
    }
}
