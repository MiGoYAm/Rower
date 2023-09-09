use std::cell::RefCell;

use bytes::BytesMut;
use libdeflater::Decompressor;
use tokio_util::codec::Decoder;

use crate::protocol::util::get_varint;

use super::util::read_varint;

thread_local!(static DECOMPRESSOR: RefCell<Decompressor> = RefCell::new(Decompressor::new()));

pub enum DecodeState {
    Length(i32, i32),
    Data(i32),
}

pub struct MinecraftDecoder {
    state: DecodeState,
    decompression: Option<BytesMut>,
    pub completed: bool,
}

impl MinecraftDecoder {
    pub fn new() -> Self {
        Self {
            state: DecodeState::Length(0, 0),
            decompression: None,
            completed: false,
        }
    }

    pub fn enable_compression(&mut self) {
        self.decompression = Some(BytesMut::with_capacity(1024))
    }
}

impl Decoder for MinecraftDecoder {
    type Item = BytesMut;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> anyhow::Result<Option<Self::Item>> {
        let length = match self.state {
            DecodeState::Length(value, readed_bytes) => {
                self.state = read_varint(value, readed_bytes, src)?;

                match self.state {
                    DecodeState::Data(length) => length,
                    DecodeState::Length(_, _) => return Ok(None),
                }
            }
            DecodeState::Data(length) => length,
        } as usize;

        src.reserve(length.saturating_sub(src.len()));

        if src.len() < length {
            return Ok(None);
        }

        self.state = DecodeState::Length(0, 0);
        let mut data = src.split_to(length);

        self.completed = src.is_empty();

        if let Some(buf) = &mut self.decompression {
            let data_length = get_varint(&mut data)?;

            if data_length == 0 {
                return Ok(Some(data));
            }

            let data_length = data_length as usize;
            buf.reserve(data_length);
            unsafe {
                buf.set_len(data_length);
            }
            let mut buf = buf.split_to(data_length);

            let result = DECOMPRESSOR.with(|d| {
                d.borrow_mut().zlib_decompress(&data, &mut buf)
            })?;

            if data_length != buf.len() {
                println!("data_lenght: {}, readed_bytes: {}, result: {}", data_length, buf.len(), result);
            }
            
            return Ok(Some(buf));
        }

        Ok(Some(data))
    }
}
