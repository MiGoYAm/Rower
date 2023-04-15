use bytes::{BytesMut};
use tokio_util::codec::Decoder;

use super::util::read_varint;

pub enum DecodeState {
    ReadVarint(i32, i32),
    Data(i32),
}
pub struct MinecraftDecoder {
    state: DecodeState,
}

impl MinecraftDecoder {
    pub fn new() -> Self {
        Self { state: DecodeState::ReadVarint(0, 0) }
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

        Ok(Some(
            src.split_to(length)
        ))
    }
}

/*
pub struct FrameDecoder<'a> {
    framed: FramedRead<ReadHalf<'a>, MinecraftDecoder>,
    pub registry: &'static Registry
}

impl<'a> FrameDecoder<'a> {
    pub fn new(reader: ReadHalf<'a>, registry: &'static Registry) -> Self {
        Self {
            framed: FramedRead::new(reader, MinecraftDecoder::new()),
            registry
        }
    }

    async fn next_frame(&mut self) -> tokio::io::Result<BytesMut> {
        match self.framed.next().await {
            Some(r) => r,
            None => todo!(),
        }
    }
    
    pub async fn next_packet(&mut self) -> tokio::io::Result<NextPacket> {
        let frame = self.next_frame().await?;

        Ok(self.registry.decode(frame, ProtocolVersion::Unknown))
    }

    pub async fn read_packet<T: Packet + 'static>(&mut self) -> Result<T, Box<dyn Error>> {
        let mut frame = self.next_frame().await?;
        let id = frame.get_u8();
        let registry_id = self.registry.get_id::<T>()?;

        if registry_id != &id {
            Err(format!("Invalid provided packet. Packet id: Provided: {}, Got: {}", registry_id, id))?;
        }

        T::from_bytes(&mut frame, ProtocolVersion::Unknown)
    }
}

pub struct FrameDecompressor<'a> {
    framed: FrameDecoder<'a>,
    decompressor: Decompressor
}
impl FrameDecompressor<'_> {
    pub async fn next_frame(&mut self) -> Result<BytesMut, Box<dyn Error>> {
        let mut frame = self.framed.next_frame().await?;
        
        let data_lenght = get_varint(&mut frame)? as usize;
        if data_lenght == 0 {
            return Ok(frame);
        }

        let mut data = BytesMut::with_capacity(data_lenght);
        self.decompressor.zlib_decompress(&mut frame, &mut data)?;

        Ok(data)
    }
}

struct CompressionCodec {
    compressor: Compressor,
}

impl Encoder<BytesMut> for CompressionCodec {
    type Error = Box<dyn Error>;

    fn encode(&mut self, src: BytesMut, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let mut data = BytesMut::with_capacity(self.compressor.zlib_compress_bound(src.len()));
        let data_length = self.compressor.zlib_compress(&src, &mut data)?;
        put_varint(dst, data_length as u32);
        dst.extend_from_slice(&data);
        Ok(())
    }
}
*/
