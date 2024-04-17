use std::cell::RefCell;

use anyhow::Result;
use bytes::{BufMut, BytesMut};
use libdeflater::Compressor;
use openssl::symm::Crypter;
use tokio_util::codec::Encoder;

use crate::{config::config, protocol::packet::RawPacket};

use super::util::{varint_length_usize, write_varint};

thread_local!(
    static COMPRESSOR: RefCell<Compressor> = RefCell::new(Compressor::new(config().compression_level))
);

pub struct MinecraftEncoder {
    threshold: Option<usize>,
    cipher: Option<Crypter>,
}

impl MinecraftEncoder {
    pub fn new() -> Self {
        Self {
            threshold: None,
            cipher: None,
        }
    }

    pub fn enable_compression(&mut self, threshold: u32) {
        self.threshold = Some(threshold as usize)
    }

    pub fn enable_encryption(&mut self, key: [u8; 16]) -> Result<()> {
        self.cipher = Some(Crypter::new(
            openssl::symm::Cipher::aes_128_cfb8(),
            openssl::symm::Mode::Encrypt,
            &key,
            Some(&key),
        )?);
        Ok(())
    }
}

impl Encoder<RawPacket> for MinecraftEncoder {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> Result<()> {
        let packet = item.buffer;
        let uncompressed_length = packet.len() as u32;

        if let Some(threshold) = self.threshold {
            if packet.len() >= threshold {
                let buffer = dst.split();
                dst.reserve(packet.len() + 6);

                let mut data = dst.split_off(3);

                write_varint(&mut data, uncompressed_length);
                let header = data.len();
                unsafe {
                    data.set_len(data.capacity());
                }

                let compressed_length = COMPRESSOR
                    .with_borrow_mut(|c| c.zlib_compress(&packet, &mut data[header..]))?;
                unsafe {
                    data.set_len(header + compressed_length);
                }

                write_21bit_varint(data.len() as u32, dst);

                dst.unsplit(data);
                dst.unsplit(buffer);
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

        if let Some(cipher) = &mut self.cipher {
            let len = dst.len();
            let buffer = dst.split();
            dst.reserve(len);
            cipher.update(&buffer, dst)?;
        }

        Ok(())
    }
}

fn write_21bit_varint(value: u32, buf: &mut BytesMut) {
    let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
    buf.put_u16((w >> 8) as u16);
    buf.put_u8(w as u8);
}
