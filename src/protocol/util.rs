use anyhow::{anyhow, ensure};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

use crate::component::Component;

use super::packet::login::Property;

pub fn get_varint(buf: &mut impl Buf) -> anyhow::Result<i32> {
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
        let w = (value & 0x7F | 0x80) << 24 | (((value >> 7) & 0x7F | 0x80) << 16) | ((value >> 14) & 0x7F | 0x80) << 8 | (value >> 21);
        buf.put_u32(w);
    } else {
        let w = (value & 0x7F | 0x80) << 24 | ((value >> 7) & 0x7F | 0x80) << 16 | ((value >> 14) & 0x7F | 0x80) << 8 | ((value >> 21) & 0x7F | 0x80);
        buf.put_u32(w);
        buf.put_u8((value >> 28) as u8);
    }
}

pub fn get_bool(buf: &mut dyn Buf) -> anyhow::Result<bool> {
    match buf.get_u8() {
        0x00 => Ok(false),
        0x01 => Ok(true),
        byte => Err(anyhow!("Could not get bool value from byte: {}", byte)),
    }
}

pub fn put_bool(buf: &mut impl BufMut, b: bool) {
    buf.put_u8(if b { 0x01 } else { 0x00 })
}

pub fn get_string(buf: &mut BytesMut, cap: i32) -> anyhow::Result<String> {
    let len = get_varint(buf)?;

    ensure!(len >= 0, "String lenght is negative");
    ensure!(len <= 3 * cap, "String is too long");

    let bytes = buf.split_to(len as usize);
    Ok(String::from_utf8(bytes.to_vec())?)
}

pub fn put_string(buf: &mut BytesMut, str: &str) {
    put_varint(buf, str.len() as u32);
    buf.extend_from_slice(str.as_bytes());
}

pub fn get_identifier(buf: &mut BytesMut) -> anyhow::Result<String> {
    get_string(buf, 32767)
}

pub fn get_byte_array(buf: &mut BytesMut) -> anyhow::Result<Vec<u8>> {
    let lenght = get_varint(buf)? as usize;

    ensure!(lenght <= buf.remaining(), "Invalid byte array lenght");

    let mut array = vec![0; lenght];
    buf.copy_to_slice(&mut array);
    Ok(array)
}

pub fn put_byte_array(buf: &mut BytesMut, bytes: &Vec<u8>) {
    put_varint(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

pub fn get_option<T>(buf: &mut BytesMut, fun: fn(&mut BytesMut) -> anyhow::Result<T>) -> anyhow::Result<Option<T>> {
    if get_bool(buf)? {
        Ok(Some(fun(buf)?))
    } else {
        Ok(None)
    }
}

pub fn get_property(buf: &mut BytesMut) -> anyhow::Result<Property> {
    Ok(Property {
        name: get_string(buf, 32767)?,
        value: get_string(buf, 32767)?,
        signature: if get_bool(buf)? { Some(get_string(buf, 32767)?) } else { None },
    })
}

pub fn put_property(buf: &mut BytesMut, property: &Property) {
    put_string(buf, &property.name);
    put_string(buf, &property.value);
    if let Some(signature) = &property.signature  {
        put_bool(buf, true);
        put_string(buf, signature);
    } else {
        put_bool(buf, false);
    }
}

pub fn get_array<T>(buf: &mut BytesMut, fun: fn(&mut BytesMut) -> anyhow::Result<T>) -> anyhow::Result<Vec<T>> {
    let length = get_varint(buf)? as usize;
    let mut array = Vec::with_capacity(length);

    for _ in 0..length {
        array.push(fun(buf)?)
    }

    Ok(array)
}

pub fn put_array<T>(buf: &mut BytesMut, vec: Vec<T>, fun: fn(&mut BytesMut, &T)) {
    put_varint(buf, vec.len() as u32);
    for item in &vec {
        fun(buf, item)
    }
}

pub fn get_component(buf: &mut BytesMut) -> anyhow::Result<Component> {
    Ok(serde_json::from_str::<Component>(get_string(buf, 262144)?.as_str())?)
}

pub fn put_component(buf: &mut BytesMut, component: &Component) {
    put_string(buf, &serde_json::to_string(component).unwrap());
}

pub fn get_uuid(buf: &mut BytesMut) -> Uuid {
    let mut bytes = [0; 16];
    buf.copy_to_slice(&mut bytes);
    Uuid::from_bytes(bytes)
}

pub fn put_uuid(buf: &mut BytesMut, uuid: Uuid) {
    buf.extend_from_slice(uuid.as_bytes());
}
