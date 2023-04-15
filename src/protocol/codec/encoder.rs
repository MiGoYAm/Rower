use std::error::Error;

use bytes::{BytesMut, BufMut};
use libdeflater::{Compressor, CompressionLvl};
use tokio_util::codec::Encoder;

use crate::protocol::{packet::RawPacket};

use super::{error::FrameToobig, util::{write_varint, MAX_PACKET_SIZE}};

pub struct MinecraftEncoder;

impl MinecraftEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.data.len() + 1);

        dst.put_u8(item.id);
        dst.extend_from_slice(&item.data);
        Ok(())
    }
}

pub struct MinecraftEncoderComp {
    threshold: u32,
    compressor: Compressor
}

impl MinecraftEncoderComp {
    pub fn new(threshold: u32) -> Self {
        Self { 
            threshold,
            compressor: Compressor::new(CompressionLvl::best()) 
        }
    }
}

impl Encoder<BytesMut> for MinecraftEncoderComp {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let length = item.len();

        if (length as u32) < self.threshold {
            dst.reserve(length + 1);
            dst.put_u8(0x00);
            dst.extend_from_slice(&item);
        } else {
            dst.reserve(self.compressor.zlib_compress_bound(length) + 3);
            write_varint(length as u32, dst);
            self.compressor.zlib_compress(&item, dst)?;
        }

        Ok(())
    }
}

pub struct MinecraftEncoderVarint;

impl MinecraftEncoderVarint {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder<BytesMut> for MinecraftEncoderVarint {
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

/*
pub struct FrameEncoder<'a> {
    framed: FramedWrite<WriteHalf<'a>, MinecraftEncoder>,
    pub registry: &'static Registry
}

impl<'a> FrameEncoder<'a> {
    pub fn new(writer: WriteHalf<'a>, registry: &'static Registry) -> Self {
        Self {
            framed: FramedWrite::new(writer, MinecraftEncoder::new()),
            registry
        }
    }

    pub async fn write_raw_packet(&mut self, packet: RawPacket) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        buf.put_u8(packet.id);
        buf.extend_from_slice(&packet.data);

        self.framed.send(buf).await.unwrap();
        Ok(())
    }

    pub async fn write_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        let id = self.registry.get_id::<T>()?;
        buf.put_u8(*id);
        packet.put_buf(&mut buf, ProtocolVersion::Unknown);

        self.framed.send(buf).await
    }

    pub async fn put_packet<T: Packet + 'static>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        let id = self.registry.get_id::<T>()?;
        buf.put_u8(*id);
        packet.put_buf(&mut buf, ProtocolVersion::Unknown);

        self.framed.feed(buf).await
    }
}
*/