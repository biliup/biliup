//! Minimal Protobuf decoder for Douyin danmaku.
//!
//! This implements just enough protobuf parsing to decode Douyin's
//! PushFrame, Response, and ChatMessage structures.

use std::collections::HashMap;

/// Protobuf wire types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5,
}

impl WireType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(WireType::Varint),
            1 => Some(WireType::Fixed64),
            2 => Some(WireType::LengthDelimited),
            5 => Some(WireType::Fixed32),
            _ => None,
        }
    }
}

/// A simple protobuf value.
#[derive(Debug, Clone)]
pub enum ProtoValue {
    Varint(u64),
    Fixed64(u64),
    Fixed32(u32),
    Bytes(Vec<u8>),
    String(String),
}

impl ProtoValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ProtoValue::Varint(v) => Some(*v),
            ProtoValue::Fixed64(v) => Some(*v),
            ProtoValue::Fixed32(v) => Some(*v as u64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ProtoValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ProtoValue::Bytes(b) => Some(b),
            _ => None,
        }
    }
}

/// A simple protobuf message reader.
pub struct ProtoReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ProtoReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Read a varint.
    pub fn read_varint(&mut self) -> Option<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;

        loop {
            if self.pos >= self.data.len() {
                return None;
            }

            let byte = self.data[self.pos];
            self.pos += 1;

            result |= ((byte & 0x7F) as u64) << shift;

            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return None;
            }
        }

        Some(result)
    }

    /// Read a field tag (field number + wire type).
    pub fn read_tag(&mut self) -> Option<(u32, WireType)> {
        let v = self.read_varint()?;
        let field_num = (v >> 3) as u32;
        let wire_type = WireType::from_u8((v & 0x07) as u8)?;
        Some((field_num, wire_type))
    }

    /// Read bytes (length-delimited).
    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        let len = self.read_varint()? as usize;
        if self.pos + len > self.data.len() {
            return None;
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(bytes)
    }

    /// Read a string.
    pub fn read_string(&mut self) -> Option<String> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).ok()
    }

    /// Read fixed32.
    pub fn read_fixed32(&mut self) -> Option<u32> {
        if self.pos + 4 > self.data.len() {
            return None;
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Some(v)
    }

    /// Read fixed64.
    pub fn read_fixed64(&mut self) -> Option<u64> {
        if self.pos + 8 > self.data.len() {
            return None;
        }
        let v = u64::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Some(v)
    }

    /// Skip a field value based on wire type.
    pub fn skip_field(&mut self, wire_type: WireType) -> bool {
        match wire_type {
            WireType::Varint => self.read_varint().is_some(),
            WireType::Fixed64 => {
                if self.pos + 8 <= self.data.len() {
                    self.pos += 8;
                    true
                } else {
                    false
                }
            }
            WireType::Fixed32 => {
                if self.pos + 4 <= self.data.len() {
                    self.pos += 4;
                    true
                } else {
                    false
                }
            }
            WireType::LengthDelimited => {
                if let Some(len) = self.read_varint() {
                    let len = len as usize;
                    if self.pos + len <= self.data.len() {
                        self.pos += len;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Parse all fields into a map.
    pub fn parse_all(&mut self) -> HashMap<u32, Vec<ProtoValue>> {
        let mut fields: HashMap<u32, Vec<ProtoValue>> = HashMap::new();

        while !self.is_empty() {
            if let Some((field_num, wire_type)) = self.read_tag() {
                let value = match wire_type {
                    WireType::Varint => self.read_varint().map(ProtoValue::Varint),
                    WireType::Fixed64 => self.read_fixed64().map(ProtoValue::Fixed64),
                    WireType::Fixed32 => self.read_fixed32().map(ProtoValue::Fixed32),
                    WireType::LengthDelimited => {
                        self.read_bytes().map(|b| {
                            // Try to interpret as string if valid UTF-8
                            if let Ok(s) = String::from_utf8(b.clone()) {
                                if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
                                    return ProtoValue::String(s);
                                }
                            }
                            ProtoValue::Bytes(b)
                        })
                    }
                };

                if let Some(v) = value {
                    fields.entry(field_num).or_default().push(v);
                }
            } else {
                break;
            }
        }

        fields
    }
}

/// Protobuf writer for building messages.
pub struct ProtoWriter {
    buffer: Vec<u8>,
}

impl ProtoWriter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn into_buffer(self) -> Vec<u8> {
        self.buffer
    }

    /// Write a varint.
    pub fn write_varint(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.buffer.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// Write a field tag.
    pub fn write_tag(&mut self, field_num: u32, wire_type: WireType) {
        let tag = ((field_num as u64) << 3) | (wire_type as u64);
        self.write_varint(tag);
    }

    /// Write a string field.
    pub fn write_string(&mut self, field_num: u32, value: &str) {
        self.write_tag(field_num, WireType::LengthDelimited);
        self.write_varint(value.len() as u64);
        self.buffer.extend_from_slice(value.as_bytes());
    }

    /// Write a bytes field.
    pub fn write_bytes(&mut self, field_num: u32, value: &[u8]) {
        self.write_tag(field_num, WireType::LengthDelimited);
        self.write_varint(value.len() as u64);
        self.buffer.extend_from_slice(value);
    }

    /// Write a varint field.
    pub fn write_varint_field(&mut self, field_num: u32, value: u64) {
        self.write_tag(field_num, WireType::Varint);
        self.write_varint(value);
    }
}

impl Default for ProtoWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint() {
        let mut writer = ProtoWriter::new();
        writer.write_varint(300);

        let mut reader = ProtoReader::new(writer.get_buffer());
        assert_eq!(reader.read_varint(), Some(300));
    }

    #[test]
    fn test_string_field() {
        let mut writer = ProtoWriter::new();
        writer.write_string(1, "hello");

        let mut reader = ProtoReader::new(writer.get_buffer());
        let fields = reader.parse_all();

        assert!(fields.contains_key(&1));
        assert_eq!(fields[&1][0].as_str(), Some("hello"));
    }
}
