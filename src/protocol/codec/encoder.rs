use std::cell::RefCell;

use bytes::{BytesMut, BufMut};
use libdeflater::{CompressionLvl, Compressor};
use tokio_util::codec::Encoder;

use crate::protocol::packet::RawPacket;

use super::util::{write_varint, varint_length_usize, varint_length};

thread_local!(
    static COMPRESSOR: RefCell<Compressor> = RefCell::new(Compressor::new(CompressionLvl::best()))
);

pub struct MinecraftEncoder {
    threshold: Option<usize>,
    buf: BytesMut
}

impl MinecraftEncoder {
    pub fn new() -> Self {
        Self { threshold: None, buf: BytesMut::zeroed(1024) }
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.threshold = Some(threshold as usize)
    }
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> anyhow::Result<()> {
        let packet = item.buffer;
        let data_length = packet.len() as u32;

        if let Some(threshold) = self.threshold {
            if packet.len() >= threshold {
                self.buf.resize(packet.len(), 0x00);
                let mut payload = self.buf.split();

                COMPRESSOR.with(|c| {
                    c.borrow_mut().zlib_compress(&packet, &mut payload)
                })?;
                let payload_length = payload.len() as u32;

                dst.reserve(payload.len() + varint_length_usize(payload_length) + varint_length_usize(data_length));

                write_varint(dst, payload_length + varint_length(data_length));
                write_varint(dst, data_length);
                dst.extend_from_slice(&payload);
            } else {
                dst.reserve(packet.len() + varint_length_usize(data_length) + 1);

                write_varint(dst, data_length + 1);
                dst.put_u8(0x00);
                dst.extend_from_slice(&packet);
            }
        } else {
            dst.reserve(packet.len() + varint_length_usize(data_length));
            write_varint(dst, data_length);
            dst.extend_from_slice(&packet);
        }

        Ok(())
    }
}
