use std::error::Error;

use bytes::{Buf, BufMut, BytesMut};

#[inline(always)]
pub fn get_varint(buf: &mut impl Buf) -> Result<i32, Box<dyn Error>> {
    let mut i = 0;
    let max_read = 5.min(buf.remaining());

    for j in 0..max_read {
        let b = buf.get_u8();
        i |= ((b & 0x7F) as i32) << (j * 7);

        if (b & 0x80) != 128 {
            return Ok(i);
        }
    }

    Err("Varint too long".into())
}
#[inline(always)]
pub fn put_varint(buf: &mut impl BufMut, value: u32) {
    if (value & (0xFFFFFFFF << 7)) == 0 {
        buf.put_u8(value as u8);
    } else if (value & (0xFFFFFFFF << 14)) == 0 {
        let w = (value & 0x7F | 0x80) << 8 | (value >> 7);
        buf.put_u16(w as u16);
    } else if (value & (0xFFFFFFFF << 21)) == 0 {
        let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
        buf.put_u16(w as u16);
        buf.put_u8((w >> 14) as u8);
    } else if (value & (0xFFFFFFFF << 28)) == 0 {
        let w= (value & 0x7F | 0x80) << 24 | (((value >> 7) & 0x7F | 0x80) << 16)
                | ((value >> 14) & 0x7F | 0x80) << 8 | (value >> 21);
        buf.put_u32(w);
    } else {
        let w = (value & 0x7F | 0x80) << 24 | ((value >> 7) & 0x7F | 0x80) << 16
                | ((value >> 14) & 0x7F | 0x80) << 8 | ((value >> 21) & 0x7F | 0x80);
        buf.put_u32(w);
        buf.put_u8((value >> 28) as u8);
    }
}

#[inline(always)]
pub fn get_bool(buf: &mut dyn Buf) -> Result<bool, Box<dyn Error>> {
    match buf.get_u8() {
        0x00 => Ok(false),
        0x01 => Ok(true),
        byte => Err(format!("couldn't get bool value from byte: {}", byte))?
    }
}
#[inline(always)]
pub fn get_string(buf: &mut BytesMut, cap: i32) -> Result<String, Box<dyn Error>> {
    let len = get_varint(buf)?;
    if len < 0 {
        return Err("String lenght is negative".into());
    }
    if len > 3 * cap {
        return Err("String is too long".into());
    }
    let bytes = buf.split_to(len as usize);
    Ok(String::from_utf8(bytes.to_vec())?)
}
#[inline(always)]
pub fn put_string(buf: &mut BytesMut, s: &String) {
    let s = s.as_bytes();
    put_varint(buf, s.len() as u32);
    buf.extend_from_slice(s);
}
#[inline(always)]
pub fn put_str(buf: &mut BytesMut, s: &str) {
    let s = s.as_bytes();
    put_varint(buf, s.len() as u32);
    buf.extend_from_slice(s);
}
#[inline(always)]
pub fn put_byte_array(buf: &mut BytesMut, bytes: &Vec<u8>) {
    put_varint(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}
#[inline(always)]
pub fn put_bool(buf: &mut impl BufMut, b: bool) {
    buf.put_u8(
        if b { 0x01 } else { 0x00 }
    );
}
