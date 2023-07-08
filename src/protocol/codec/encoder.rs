use std::cell::RefCell;

use bytes::{BufMut, BytesMut};
use libdeflater::{CompressionLvl, Compressor};
use tokio_util::codec::Encoder;

use crate::protocol::packet::RawPacket;

use super::util::write_varint;

thread_local!(static COMPRESSOR: RefCell<Compressor> = RefCell::new(Compressor::new(CompressionLvl::best())));

pub struct MinecraftEncoder {
    threshold: Option<usize>,
}

impl MinecraftEncoder {
    pub fn new() -> Self {
        Self { threshold: None }
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.threshold = Some(threshold as usize)
    }

    /*
    fn convert_packet(&self, packet: RawPacket) -> BytesMut {
        let mut data = BytesMut::with_capacity(packet.data.len() + 1);
        data.put_u8(packet.id);
        data.extend_from_slice(&packet.data);
        data
    }
    */
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> anyhow::Result<()> {
        //let packet = self.convert_packet(item);
        let packet = item.buffer;

        let result = if let Some(threshold) = &self.threshold {
            let mut result = BytesMut::with_capacity(packet.len() + 3);

            if packet.len() < *threshold {
                result.put_u8(0x00);
                result.extend_from_slice(&packet);
                result
            } else {
                write_varint(&mut result, packet.len() as u32);

                let mut payload = result.split_off(result.len());
                payload.resize(packet.len(), 0x00);

                let r = COMPRESSOR.with(|c| {
                    c.borrow_mut().zlib_compress(&packet, &mut payload)
                })?;
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
