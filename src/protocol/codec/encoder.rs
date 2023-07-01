use std::error::Error;

use bytes::{BytesMut, BufMut};
use libdeflater::{Compressor, CompressionLvl};
use tokio_util::codec::Encoder;

use crate::protocol::packet::RawPacket;

use super::util::write_varint;

struct Compression {
    threshold: usize,
    compressor: Compressor,
}

pub struct MinecraftEncoder {
    compression: Option<Compression>
}

impl MinecraftEncoder {
    pub fn new() -> Self {
        Self { compression: None }
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.compression = Some(Compression { threshold: threshold as usize, compressor: Compressor::new(CompressionLvl::best()) })
    }

    fn convert_packet(&self, packet: RawPacket) -> BytesMut {
        let mut data = BytesMut::with_capacity(packet.data.len() + 1);
        data.put_u8(packet.id);
        data.extend_from_slice(&packet.data);
        data
    }
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut packet = self.convert_packet(item);

        let result = if let Some(Compression { threshold, compressor }) = &mut self.compression {
            let mut result = BytesMut::with_capacity(packet.len() + 3);

            if packet.len() < *threshold {
                result.put_u8(0x00);
                result.extend_from_slice(&mut packet);
                result
            } else {
                write_varint(&mut result, packet.len() as u32);

                let mut payload = result.split_off(result.len());
                payload.resize(packet.len(), 0x00);

                let r = compressor.zlib_compress(&packet, &mut payload)?;
                payload.resize(r, 0x00);

                result.unsplit(payload);
                result

            }
        } else {
            packet
        };


        dst.reserve(result.len() + 3);
        write_varint(dst, result.len() as u32);
        dst.extend_from_slice(&result);

        Ok(())
    }
}
