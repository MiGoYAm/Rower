use std::error::Error;

use bytes::{Buf, BufMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
    Err("Varint too long")?
}
#[inline(always)]
pub fn put_varint(buf: &mut impl BufMut, value: i32) {
    let mut value = value as u32;
    loop {
        if (value & 0xFFFFFF80) == 0 {
            buf.put_u8(value as u8);
            return;
        }

        buf.put_u8((value & 0x7F | 0x80) as u8);
        value >>= 7;
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
pub async fn read_varint(reader: &mut (impl AsyncReadExt + Unpin)) -> Result<i32, Box<dyn Error>> {
    let mut i: i32 = 0;
    //let max_read = 5;

    for j in 0..5 {
        let b = reader.read_u8().await?;
        i |= ((b & 0x7F) as i32) << (j * 7);

        if (b & 0x80) != 128 {
            return Ok(i);
        }
    }
    Err("Varint too long")?
}
#[inline(always)]
pub async fn write_varint(
    writer: &mut (impl AsyncWriteExt + Unpin),
    value: i32,
) -> tokio::io::Result<()> {
    let mut value = value as u32;
    loop {
        if (value & 0xFFFFFF80) == 0 {
            writer.write_u8(value as u8).await?;
            return Ok(());
        }

        writer.write_u8((value & 0x7F | 0x80) as u8).await?;
        value >>= 7;
    }
}

pub fn get_string(buf: &mut impl Buf, cap: i32) -> Result<String, Box<dyn Error>> {
    let len = get_varint(buf)?;
    if len < 0 {
        Err("String lenght is negative")?
    }
    if len > 3 * cap {
        Err("String is too long")?
    }

    let bytes = buf.copy_to_bytes(len as usize).to_vec();
    Ok(String::from_utf8(bytes)?)
}
pub fn put_string(buf: &mut impl BufMut, s: &String) {
    let s = s.as_bytes();
    put_varint(buf, s.len() as i32);
    buf.put_slice(s);
}

pub fn put_str(buf: &mut impl BufMut, s: &str) {
    let s = s.as_bytes();
    put_varint(buf, s.len() as i32);
    buf.put_slice(s);
}

pub fn put_byte_array(buf: &mut impl BufMut, bytes: Vec<u8>) {
    put_varint(buf, bytes.len() as i32);
    //buf.put(bytes.);
    buf.put_slice(&bytes)
}

pub fn put_bool(buf: &mut impl BufMut, b: bool) {
    let b = if b { 0x01 } else { 0x00 };
    buf.put_u8(b)
}
