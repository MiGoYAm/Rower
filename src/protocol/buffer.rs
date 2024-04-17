use anyhow::{anyhow, ensure, Ok, Result};
use bytes::{Buf, BufMut, Bytes};
use uuid::Uuid;

use crate::component::Component;

pub trait BufExt: Buf {
    fn rest(&mut self) -> Bytes {
        self.copy_to_bytes(self.remaining())
    }

    fn get_varint(&mut self) -> Result<i32> {
        let mut i = 0;
        let max_read = 5.min(self.remaining());

        for j in 0..max_read {
            let b = self.get_u8();
            i |= ((b & 0x7F) as i32) << (j * 7);

            if (b & 0x80) != 128 {
                return Ok(i);
            }
        }

        Err(anyhow!("Varint too long"))
    }

    fn get_bool(&mut self) -> Result<bool> {
        match self.get_u8() {
            0x00 => Ok(false),
            0x01 => Ok(true),
            byte => Err(anyhow!("Could not get bool value from byte: {}", byte)),
        }
    }

    fn get_string(&mut self, cap: i32) -> Result<String> {
        let len = self.get_varint()?;

        ensure!(len >= 0, "String lenght is negative");
        ensure!(len <= 3 * cap, "String is too long");

        let bytes = self.copy_to_bytes(len as usize);
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    fn get_identifier(&mut self) -> Result<String> {
        self.get_string(32767)
    }

    fn get_component(&mut self) -> Result<Component> {
        let len = self.get_varint()? as usize;
        let reader = self.take(len).reader();
        Ok(serde_json::from_reader(reader)?)
    }

    fn get_uuid(&mut self) -> Uuid {
        let mut bytes = [0; 16];
        self.copy_to_slice(&mut bytes);
        Uuid::from_bytes(bytes)
    }

    fn get_bitset(&mut self) -> Result<Vec<i64>> {
        let len = self.get_varint()?;
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(self.get_i64())
        }
        Ok(vec)
    }

    fn get_bytes(&mut self) -> Result<Bytes> {
        let len = self.get_varint()? as usize;
        ensure!(len <= self.remaining(), "Invalid byte array lenght");
        Ok(self.copy_to_bytes(len))
    }

    fn get_byte_array<const L: usize>(&mut self) -> Result<[u8; L]> {
        let len = self.get_varint()? as usize;
        ensure!(
            len == L,
            "Invalid byte array lenght, expected {} got {}",
            L,
            len,
        );
        ensure!(L <= self.remaining(), "Invalid byte array lenght");
        let mut bytes = [0; L];
        self.copy_to_slice(&mut bytes);
        Ok(bytes)
    }

    fn get_option<T>(&mut self, fun: impl Fn(&mut Self) -> Result<T>) -> Result<Option<T>> {
        if self.get_bool()? {
            Ok(Some(fun(self)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: Buf> BufExt for T {}

pub trait BufMutExt: BufMut {
    fn put_uvarint(&mut self, value: u32) {
        if (value & (0xFFFFFFFF << 7)) == 0 {
            self.put_u8(value as u8);
        } else if (value & (0xFFFFFFFF << 14)) == 0 {
            let w = (value & 0x7F | 0x80) << 8 | (value >> 7);
            self.put_u16(w as u16);
        } else if (value & (0xFFFFFFFF << 21)) == 0 {
            self.put_slice(&[
                (value & 0x7F | 0x80) as u8,
                ((value >> 7) & 0x7F | 0x80) as u8,
                (value >> 14) as u8,
            ]);
        } else if (value & (0xFFFFFFFF << 28)) == 0 {
            self.put_u32(
                (value & 0x7F | 0x80) << 24
                    | (((value >> 7) & 0x7F | 0x80) << 16)
                    | ((value >> 14) & 0x7F | 0x80) << 8
                    | (value >> 21),
            );
        } else {
            self.put_slice(&[
                (value & 0x7F | 0x80) as u8,
                ((value >> 7) & 0x7F | 0x80) as u8,
                ((value >> 14) & 0x7F | 0x80) as u8,
                ((value >> 21) & 0x7F | 0x80) as u8,
                (value >> 28) as u8,
            ]);
        }
    }

    fn put_varint(&mut self, value: i32) {
        self.put_uvarint(value as u32);
    }

    fn put_bool(&mut self, bool: bool) {
        self.put_u8(bool.into())
    }

    fn put_string(&mut self, str: &str) {
        let str = str.as_bytes();
        self.put_varint(str.len() as i32);
        self.put_slice(str);
    }

    fn put_component(&mut self, component: &Component) -> Result<()> {
        self.put_byte_array(&serde_json::to_vec(component)?);
        Ok(())
    }

    fn put_uuid(&mut self, uuid: Uuid) {
        self.put_slice(uuid.as_bytes());
    }

    fn put_bitset(&mut self, vec: Vec<i64>) {
        self.put_varint(vec.len() as i32);
        for i in vec {
            self.put_i64(i)
        }
    }

    fn put_byte_array(&mut self, bytes: &[u8]) {
        self.put_varint(bytes.len() as i32);
        self.put_slice(bytes);
    }

    fn put_option<T>(&mut self, option: &Option<T>, fun: impl Fn(&mut Self, &T)) {
        if let Some(value) = option {
            self.put_bool(true);
            fun(self, value);
        } else {
            self.put_bool(false)
        }
    }
}

impl<T: BufMut> BufMutExt for T {}
