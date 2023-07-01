use std::io::Read;

use bytes::{BytesMut, Buf};
use flate2::read::ZlibDecoder;
use libdeflater::Decompressor;
use tokio_util::codec::Decoder;

use crate::protocol::util::get_varint;

use super::util::read_varint;

pub enum DecodeState {
    ReadVarint(i32, i32),
    Data(i32),
}

pub struct MinecraftDecoder {
    state: DecodeState,
    decompression: Option<Decompressor>,
}

impl MinecraftDecoder {
    pub fn new() -> Self {
        Self { state: DecodeState::ReadVarint(0, 0), decompression: None }
    }

    pub fn enable_compression(&mut self) {
        self.decompression = Some(Decompressor::new())
    }
}

impl Decoder for MinecraftDecoder {
    type Item = BytesMut;
    type Error = tokio::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let length = match self.state {
            DecodeState::Data(length) => length,
            DecodeState::ReadVarint(value, readed_bytes) => {
                self.state = read_varint(value, readed_bytes, src).unwrap();

                match self.state {
                    DecodeState::Data(length) => length,
                    DecodeState::ReadVarint(_, _) => return Ok(None)
                }
            },
        } as usize;

        src.reserve(length.saturating_sub(src.len()));

        if src.len() < length { 
            return Ok(None);
        }
        self.state = DecodeState::ReadVarint(0, 0);

        let mut src = src.split_to(length);

        if let Some(_decompressor) = &mut self.decompression {
            let data_lenght = get_varint(&mut src).unwrap();

            if data_lenght == 0 {
                return Ok(Some(src));
            }

            //let mut buf = BytesMut::with_capacity(data_lenght as usize);
            let mut buf = Vec::with_capacity(data_lenght as usize);

            //decompressor.zlib_decompress(&src, &mut buf).unwrap();
            let mut z = ZlibDecoder::new(src.reader());
            let _result = z.read_to_end(&mut buf).unwrap();
            //println!("in: {}, out: {}, result: {}", z.total_in(), z.total_out(), result);

            return Ok(Some(BytesMut::from_iter(buf)))
        }

        Ok(Some(src))
    }
}
