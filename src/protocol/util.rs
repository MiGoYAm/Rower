use bytes::BytesMut;
use super::{packet::login::Property, buffer::{BufMutExt, BufExt}};

pub fn get_property(buf: &mut BytesMut) -> anyhow::Result<Property> {
    Ok(Property {
        name: buf.get_string(32767)?,
        value: buf.get_string(32767)?,
        signature: if buf.get_bool()? { Some(buf.get_string(32767)?) } else { None },
    })
}

pub fn put_property(buf: &mut BytesMut, property: &Property) {
    buf.put_string(&property.name);
    buf.put_string(&property.value);
    if let Some(signature) = &property.signature  {
        buf.put_bool(true);
        buf.put_string(signature);
    } else {
        buf.put_bool(false);
    }
}

pub fn get_array<T>(buf: &mut BytesMut, fun: fn(&mut BytesMut) -> anyhow::Result<T>) -> anyhow::Result<Vec<T>> {
    let length = buf.get_varint()? as usize;
    let mut array = Vec::with_capacity(length);

    for _ in 0..length {
        array.push(fun(buf)?)
    }

    Ok(array)
}

pub fn put_array<T>(buf: &mut BytesMut, vec: Vec<T>, fun: fn(&mut BytesMut, &T)) {
    buf.put_varint(vec.len() as i32);
    for item in &vec {
        fun(buf, item)
    }
}
