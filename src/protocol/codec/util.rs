use bytes::{BytesMut, Buf, BufMut};

use super::{error::VarintTooBig, decoder::DecodeState};

pub const MAX_PACKET_SIZE: usize = 2097151;
pub const MAX_HEADER_LENGTH: i32 = 3;

#[inline(always)]
pub fn read_varint(mut value: i32, readed_bytes: i32, src: &mut BytesMut) -> Result<DecodeState, VarintTooBig> {
    let max_read = i32::min(MAX_HEADER_LENGTH, src.len() as i32);

    for i in readed_bytes..max_read {
        let byte = src.get_u8();
        value |= ((byte & 0x7F) as i32) << (i * 7);

        if (byte & 0x80) != 128 {
            return Ok(DecodeState::Data(value));
        }
    }

    if max_read < MAX_HEADER_LENGTH {
        return Ok(DecodeState::ReadVarint(value, max_read));
    }
    Err(VarintTooBig) 
}

#[inline(always)]
pub fn write_varint(value: u32, dst: &mut BytesMut) {
    if (value & (0xFFFFFFFF << 7)) == 0 {
        dst.put_u8(value as u8);
    } else if (value & (0xFFFFFFFF << 14)) == 0 {
        let w = (value & 0x7F | 0x80) << 8 | (value >> 7);
        dst.put_u16(w as u16);
    } else {
        let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
        dst.put_u16((w >> 8) as u16);
        dst.put_u8(w as u8);
    }
}