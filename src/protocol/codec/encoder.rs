use std::cell::RefCell;

use bytes::{BytesMut, BufMut};
use libdeflater::{CompressionLvl, Compressor};
use tokio_util::codec::Encoder;

use crate::protocol::packet::RawPacket;

use super::util::{write_varint, varint_length_usize, varint_length};

thread_local!(static COMPRESSOR: RefCell<Compressor> = RefCell::new(Compressor::new(CompressionLvl::best())));

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
        /* 
        let (data, data_length, be) = if self.threshold.is_some_and(|ref t| packet.len() >= *t) {
            //let mut payload = BytesMut::zeroed(packet.len());
            self.buf.resize(packet.len(), 0x00);
            let mut payload = self.buf.split();

            COMPRESSOR.with(|c| {
                c.borrow_mut().zlib_compress(&packet, &mut payload)
            })?;

            (payload, packet.len() as u32, true)
        } else {
            (packet, 0, false)
        };

        let mut frame_length = data.len();
        if be || self.threshold.is_some() {
            frame_length += varint_length_usize(data_length);
        }

        dst.reserve(frame_length);

        // write frame lenght
        write_varint(dst, frame_length as u32);
        // write data length
        if be || self.threshold.is_some() {
            write_varint(dst, data_length);
        }
        // write data
        dst.extend_from_slice(&data);
        */

        Ok(())
    }

    /*
    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> anyhow::Result<()> {
        let packet = item.buffer;

        let (data, data_length, be) = if self.threshold.is_some_and(|ref t| packet.len() >= *t) {
            //let mut payload = BytesMut::zeroed(packet.len());
            self.buf.resize(packet.len(), 0x00);
            let mut payload = self.buf.split();

            COMPRESSOR.with(|c| {
                c.borrow_mut().zlib_compress(&packet, &mut payload)
            })?;

            (payload, packet.len() as u32, true)
        } else {
            (packet, 0, false)
        };

        let mut frame_length = data.len();
        if be || self.threshold.is_some() {
            frame_length += varint_length_usize(data_length);
        }

        dst.reserve(frame_length);

        // write frame lenght
        write_varint(dst, frame_length as u32);
        // write data length
        if be || self.threshold.is_some() {
            write_varint(dst, data_length);
        }
        // write data
        dst.extend_from_slice(&data);

        Ok(())
    }
    */

    /*
    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> anyhow::Result<()> {
        //let packet = self.convert_packet(item);
        let packet = item.buffer;

        let data = if let Some(threshold) = &self.threshold {
            let mut result = BytesMut::with_capacity(packet.len() + 3);

            if packet.len() < *threshold {
                result.put_u8(0x00);
                result.extend_from_slice(&packet);
                result
            } else {
                write_varint(&mut result, packet.len() as u32);

                let mut payload = result.split_off(result.len());
                payload.resize(packet.len(), 0x00);

                COMPRESSOR.with(|c| {
                    c.borrow_mut().zlib_compress(&packet, &mut payload)
                })?;

                result.unsplit(payload);
                result
            }
        } else {
            packet
        };

        let frame_length = data.len() as u32;
        dst.reserve(data.len() + varint_length_usize(frame_length));

        write_varint(dst, frame_length);
        dst.extend_from_slice(&data);

        Ok(())
    }
    */
}
