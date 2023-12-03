use super::{packet::login::Property, buffer::{BufMutExt, BufExt}};
use anyhow::Result;
use bytes::{Buf, BufMut};

pub fn get_property(buf: &mut impl Buf) -> Result<Property> {
    Ok(Property {
        name: buf.get_string(32767)?,
        value: buf.get_string(32767)?,
        signature: buf.get_option(|b| b.get_string(32767))?,
    })
}

pub fn put_property(buf: &mut impl BufMut, property: &Property) {
    buf.put_string(&property.name);
    buf.put_string(&property.value);
    buf.put_option(&property.signature, |b, s| b.put_string(s));
    if let Some(signature) = &property.signature  {
        buf.put_bool(true);
        buf.put_string(signature);
    } else {
        buf.put_bool(false);
    }
}

pub fn get_array<T, B, F>(buf: &mut B, fun: F) -> Result<Vec<T>> 
where 
    B: Buf,
    F: Fn(&mut B) -> Result<T>
{
    let length = buf.get_varint()? as usize;
    let mut array = Vec::with_capacity(length);

    for _ in 0..length {
        array.push(fun(buf)?)
    }

    Ok(array)
}

pub fn put_array<T, B, F>(buf: &mut B, vec: Vec<T>, fun: F) 
where 
    B: BufMut,
    F: Fn(&mut B, &T)
{
    buf.put_varint(vec.len() as i32);
    for item in &vec {
        fun(buf, item)
    }
}
