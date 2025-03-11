use crate::error::{Error, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_varint()? as usize;
        if len > 0xFFFF {
            return Err(Error::InvalidData("String too long".into()));
        }
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| Error::InvalidData("Invalid UTF-8".into()))
    }

    pub fn read_varint(&mut self) -> Result<i32> {
        let mut result = 0;
        let mut shift = 0;

        loop {
            let byte = *self.read_bytes(1)?.first().ok_or(Error::UnexpectedEof)?;

            result |= ((byte & 0x7F) as i32) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 32 {
                return Err(Error::InvalidData("VarInt too long".into()));
            }
        }

        Ok(result)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_bytes(1)?[0] != 0)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        if self.pos + 4 > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let mut slice = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        Ok(slice.read_f32::<BigEndian>().unwrap())
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        if self.pos + 8 > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let mut slice = &self.data[self.pos..self.pos + 8];
        self.pos += 8;
        Ok(slice.read_f64::<BigEndian>().unwrap())
    }

    pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.pos + len > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        self.pos += len;
        Ok(())
    }
}

pub struct BinaryWriter {
    data: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn write_string(&mut self, s: &str) {
        self.write_varint(s.len() as i32);
        self.write_bytes(s.as_bytes());
    }

    pub fn write_varint(&mut self, mut value: i32) {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.data.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    pub fn write_bool(&mut self, value: bool) {
        self.data.push(if value { 1 } else { 0 });
    }

    pub fn write_f32(&mut self, value: f32) {
        self.data.write_f32::<BigEndian>(value).unwrap();
    }

    pub fn write_f64(&mut self, value: f64) {
        self.data.write_f64::<BigEndian>(value).unwrap();
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}
