use bytes::{BytesMut, Buf};
use criterion::{criterion_group, criterion_main, Criterion};

use libdeflater::Decompressor;
use tokio_util::codec::Decoder;
use anyhow::{anyhow, Result};

pub const MAX_HEADER_LENGTH: i32 = 3;

#[inline(always)]
pub fn read_varint(mut value: i32, readed_bytes: i32, src: &mut BytesMut) -> Result<DecodeState> {
    let max_read = i32::min(MAX_HEADER_LENGTH, src.len() as i32);

    for i in readed_bytes..max_read {
        let byte = src.get_u8();
        value |= ((byte & 0x7F) as i32) << (i * 7);

        if (byte & 0x80) != 128 {
            return Ok(DecodeState::Data(value));
        }
    }

    if max_read < MAX_HEADER_LENGTH {
        return Ok(DecodeState::Length(value, max_read));
    }
    Err(anyhow!("Varint too big"))
}

#[inline(always)]
pub fn get_varint(buf: &mut impl Buf) -> Result<i32> {
    let mut i = 0;
    let max_read = 5.min(buf.remaining());

    for j in 0..max_read {
        let b = buf.get_u8();
        i |= ((b & 0x7F) as i32) << (j * 7);

        if (b & 0x80) != 128 {
            return Ok(i);
        }
    }

    Err(anyhow!("Varint too long"))
}

struct Decompression {
    decompressor: Decompressor,
    //buf: BytesMut,
}

pub enum DecodeState {
    Length(i32, i32),
    Data(i32),
}

pub struct MinecraftDecoder {
    state: DecodeState,
    decompression: Option<Decompression>,
}

impl MinecraftDecoder {
    pub fn new() -> Self {
        Self {
            state: DecodeState::Length(0, 0),
            decompression: None,
        }
    }

    pub fn enable_compression(&mut self) {
        self.decompression = Some(Decompression { 
            decompressor: Decompressor::new(), 
            //buf: BytesMut::with_capacity(1024) 
        })
    }
}

impl Decoder for MinecraftDecoder {
    type Item = BytesMut;
    type Error = anyhow::Error;

    #[inline(always)]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
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

        if let Some(Decompression { decompressor, }) = &mut self.decompression {
            let data_length = get_varint(&mut data)?;

            if data_length == 0 {
                return Ok(Some(data));
            }
            /* 
            let data_length = data_length as usize;
            buf.reserve(data_length);
            unsafe {
                buf.set_len(data_length);
            }
            let mut buf = buf.split_to(data_length);
            */
            let data_length = data_length as usize;
            let mut buf = BytesMut::with_capacity(data_length);

            let result = decompressor.zlib_decompress(&data, &mut buf)?;

            if data_length != buf.len() {
                println!("data_lenght: {}, readed_bytes: {}, result: {}", data_length, buf.len(), result);
            }
            
            return Ok(Some(buf));
        }

        Ok(Some(data))
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("decoder");

    group.bench_function("decode", |b| {
        let buf: &[u8] = &[ 0x0D, 0x12, 0x6D, 0x69, 0x6E, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74, 0x3A, 0x72, 0x65, 0x67, 0x69, 0x73, 0x74, 0x65, 0x72, 0x66, 0x61, 0x62, 0x72, 0x69, 0x63, 0x3A, 0x63, 0x6F, 0x6E, 0x74, 0x61, 0x69, 0x6E, 0x65, 0x72, 0x2F, 0x6F, 0x70, 0x65, 0x6E, 0x00, 0x66, 0x61, 0x62, 0x72, 0x69, 0x63, 0x3A, 0x72, 0x65, 0x67, 0x69, 0x73, 0x74, 0x72, 0x79, 0x2F, 0x73, 0x79, 0x6E, 0x63, 0x00, 0x66, 0x61, 0x62, 0x72, 0x69, 0x63, 0x3A, 0x72, 0x65, 0x67, 0x69, 0x73, 0x74, 0x72, 0x79, 0x2F, 0x73, 0x79, 0x6E, 0x63, 0x2F, 0x64, 0x69, 0x72, 0x65, 0x63, 0x74, 0x00, 0x66, 0x61, 0x62, 0x72, 0x69, 0x63, 0x2D, 0x73, 0x63, 0x72, 0x65, 0x65, 0x6E, 0x2D, 0x68, 0x61, 0x6E, 0x64, 0x6C, 0x65, 0x72, 0x2D, 0x61, 0x70, 0x69, 0x2D, 0x76, 0x31, 0x3A, 0x6F, 0x70, 0x65, 0x6E, 0x5F, 0x73, 0x63, 0x72, 0x65, 0x65, 0x6E, ];

        let mut decoder = MinecraftDecoder::new();
        decoder.enable_compression();

        b.iter(|| decoder.decode(&mut BytesMut::from(buf)))
    });
    
    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
