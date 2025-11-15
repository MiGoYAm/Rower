use bytes::{Buf, BufMut, BytesMut};

use super::decoder::DecodeState;

use anyhow::{anyhow, Result};

pub const MAX_PACKET_SIZE: usize = 2097151;
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

    if max_read <= MAX_HEADER_LENGTH {
        return Ok(DecodeState::Length(value, max_read));
    }
    Err(anyhow!("Varint too big"))
}

#[inline(always)]
pub fn write_varint(dst: &mut BytesMut, value: u32) {
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

#[inline(always)]
pub const fn varint_length(v: u32) -> u32 {
    match v {
        0..=127 => 1,
        128..=16383 => 2,
        //16384..=2097151 => 3,
        //2097152..=268435456 => 4,
        //_ => 5
        _ => 3,
    }
}

#[inline(always)]
pub const fn varint_length_usize(v: u32) -> usize {
    match v {
        0..=127 => 1,
        128..=16383 => 2,
        16384..=2097151 => 3,
        2097152..=268435456 => 4,
        _ => 5,
    }
}

macro_rules! produce {
    ( $packet:ident ) => {
        Some(|b, v| Ok(PacketType::$packet($packet::from_bytes(b, v)?)))
    };
}
pub(crate) use produce;
