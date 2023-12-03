use std::collections::HashMap;

use anyhow::{ensure, Result};
use bytes::{Buf, BufMut};

#[derive(Debug)]
pub struct Compound(String, HashMap<String, Tag>);

impl Compound {
    pub fn read(buf: &mut impl Buf) -> Result<Self> {
        let id = buf.get_u8();
        ensure!(id == 0x0a, "nbt this isnt a compund. id: {}", id);
        let name = read_string(buf)?;

        Ok(Compound(name, read_compund(buf)?))
    }

    pub fn write(self, buf: &mut impl BufMut) {
        buf.put_u8(0xa);
        write_string(&self.0, buf);
        write_compund(&self.1, buf);
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum Tag {
    Byte(i8) = 1,
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>), // todo vec<i8>
    String(String),
    List(Vec<Tag>),
    Compound(HashMap<String, Tag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    pub fn read(id: u8, buf: &mut impl Buf) -> Result<Self> {
        Ok(match id {
            1 => Tag::Byte(buf.get_i8()),
            2 => Tag::Short(buf.get_i16()),
            3 => Tag::Int(buf.get_i32()),
            4 => Tag::Long(buf.get_i64()),
            5 => Tag::Float(buf.get_f32()),
            6 => Tag::Double(buf.get_f64()),
            7 => {
                let length = buf.get_i32() as usize;
                let mut vec = vec![0; length];
                buf.copy_to_slice(&mut vec);
                Tag::ByteArray(vec)
                /*
                let mut vec = mem::ManuallyDrop::new(vec![0; length]);
                buf.copy_to_slice(&mut vec);

                unsafe {
                    let byte_array = Vec::from_raw_parts(vec.as_mut_ptr() as *mut i8, vec.len(), vec.capacity()); 
                    Tag::ByteArray(byte_array)
                }
                */
            }
            8 => Tag::String(read_string(buf)?),
            9 => {
                let id = buf.get_u8();
                let length = buf.get_i32() as usize;
                let mut vec = Vec::with_capacity(length);

                for _ in 0..length {
                    let tag = Self::read(id, buf)?;
                    ensure!(id == tag.id(), "nbt list has different tags");
                    vec.push(tag);
                }

                Tag::List(vec)
            }
            10 => Tag::Compound(read_compund(buf)?),
            11 => unimplemented!("nbt int list"),
            12 => unimplemented!("nbt long list"),
            _ => panic!("nbt invalid id")
        })
    }

    pub fn write(&self, buf: &mut impl BufMut) {
        match self {
            Tag::Byte(v) => buf.put_i8(*v),
            Tag::Short(v) => buf.put_i16(*v),
            Tag::Int(v) => buf.put_i32(*v),
            Tag::Long(v) => buf.put_i64(*v),
            Tag::Float(v) => buf.put_f32(*v),
            Tag::Double(v) => buf.put_f64(*v),
            Tag::ByteArray(v) => {
                buf.put_i32(v.len() as i32);
                buf.put_slice(v);
            }
            Tag::String(v) => write_string(v, buf),
            Tag::List(v) => {
                if v.is_empty() {
                    buf.put_u8(0x00);
                    buf.put_i32(0);
                    return;
                }
                let list_id = v[0].id();
                buf.put_u8(list_id);
                buf.put_i32(v.len() as i32);

                for value in v {
                    value.write(buf);
                }
            }
            Tag::Compound(v) => write_compund(v, buf),
            Tag::IntArray(_) => unimplemented!("nbt int list"),
            Tag::LongArray(_) => unimplemented!("nbt long list"),
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(_) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        }
    }
}

fn read_compund(buf: &mut impl Buf) -> Result<HashMap<String, Tag>> {
    let mut map = HashMap::new();

    while let id @ 1.. = buf.get_u8()  {
        let tag_name = read_string(buf)?;
        let tag = Tag::read(id, buf)?;
        map.insert(tag_name, tag);
    }
    Ok(map)
}

fn write_compund(map: &HashMap<String, Tag>, buf: &mut impl BufMut) {
    for (name, tag) in map.iter() {
        buf.put_u8(tag.id());
        write_string(name, buf);
        tag.write(buf);
    }
    buf.put_u8(0x00);
}

fn read_string(buf: &mut impl Buf) -> Result<String> {
    let name_length = buf.get_u16() as usize;
    let mut vec = vec![0; name_length];
    buf.copy_to_slice(&mut vec);

    Ok(String::from_utf8(vec)?)
}

fn write_string(str: &String, buf: &mut impl BufMut) {
    buf.put_u16(str.len() as u16);
    buf.put_slice(str.as_bytes());
}