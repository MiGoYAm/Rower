use std::cell::RefCell;

use bytes::{BytesMut, BufMut};
use libdeflater::{CompressionLvl, Compressor};
use tokio_util::codec::Encoder;

use crate::protocol::packet::RawPacket;

use super::util::{write_varint, varint_length_usize};

thread_local!(
    static COMPRESSOR: RefCell<Compressor> = RefCell::new(Compressor::new(CompressionLvl::best()))
);

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
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> anyhow::Result<()> {
        let packet = item.buffer;
        let uncompressed_length = packet.len() as u32;

        if let Some(threshold) = self.threshold {
            if packet.len() >= threshold {
                let ds = dst.split();
                dst.reserve(packet.len() + 6);

                let mut data = dst.split_off(3);

                write_varint(&mut data, uncompressed_length);
                let d = data.len();
                unsafe { data.set_len(data.capacity()); }


                let compressed_length = COMPRESSOR.with(|c| {
                    c.borrow_mut().zlib_compress(&packet, &mut data[d..])
                })?;
                
                unsafe { data.set_len(d + compressed_length); }

                write_21bit_varint(data.len() as u32, dst);

                dst.unsplit(data);
                dst.unsplit(ds);
            } else {
                dst.reserve(packet.len() + varint_length_usize(uncompressed_length) + 1);

                write_varint(dst, uncompressed_length + 1);
                dst.put_u8(0x00);
                dst.extend_from_slice(&packet);
            }
        } else {
            dst.reserve(packet.len() + varint_length_usize(uncompressed_length));
            write_varint(dst, uncompressed_length);
            dst.extend_from_slice(&packet);
        }

        Ok(())
    }
}

fn write_21bit_varint(value: u32, buf: &mut BytesMut) {
    let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
    buf.put_u16((w >> 8) as u16);
    buf.put_u8(w as u8);
}
