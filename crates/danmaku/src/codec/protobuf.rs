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
        // 长度来自对端，可达 u64::MAX：`pos + len` 必须做溢出检查
        let len = usize::try_from(self.read_varint()?).ok()?;
        let end = self.pos.checked_add(len)?;
        if end > self.data.len() {
            return None;
        }
        let bytes = self.data[self.pos..end].to_vec();
        self.pos = end;
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
                if let Some(end) = self
                    .read_varint()
                    .and_then(|len| usize::try_from(len).ok())
                    .and_then(|len| self.pos.checked_add(len))
                    && end <= self.data.len()
                {
                    self.pos = end;
                    return true;
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
                                if s.chars()
                                    .all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
                                {
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

    /// Parse all fields and fail if the message contains a truncated or
    /// otherwise invalid field.  `parse_all` is intentionally lenient for a
    /// few legacy callers, but protocol decoders should use this variant so a
    /// damaged frame is not mistaken for an empty/partial message.
    pub fn parse_all_strict(&mut self) -> Option<HashMap<u32, Vec<ProtoValue>>> {
        let mut fields: HashMap<u32, Vec<ProtoValue>> = HashMap::new();

        while !self.is_empty() {
            let (field_num, wire_type) = self.read_tag()?;
            if field_num == 0 {
                return None;
            }

            let value = match wire_type {
                WireType::Varint => ProtoValue::Varint(self.read_varint()?),
                WireType::Fixed64 => ProtoValue::Fixed64(self.read_fixed64()?),
                WireType::Fixed32 => ProtoValue::Fixed32(self.read_fixed32()?),
                WireType::LengthDelimited => {
                    let bytes = self.read_bytes()?;
                    if let Ok(s) = String::from_utf8(bytes.clone())
                        && s.chars()
                            .all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
                    {
                        ProtoValue::String(s)
                    } else {
                        ProtoValue::Bytes(bytes)
                    }
                }
            };

            fields.entry(field_num).or_default().push(value);
        }

        Some(fields)
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

    #[test]
    fn strict_parser_rejects_truncated_length_delimited_field() {
        let mut reader = ProtoReader::new(&[0x0a, 0x02, b'x']);
        assert!(reader.parse_all_strict().is_none());
    }

    /// 长度前缀为 u64::MAX（10 字节 varint）：修复前 `pos + len` 溢出（debug）
    /// 或回绕后以 end < start 切片（release），两种构建都会 panic。
    #[test]
    fn huge_length_prefix_is_rejected_without_panicking() {
        let data = [
            0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
        ];
        assert!(ProtoReader::new(&data).parse_all_strict().is_none());
        assert!(ProtoReader::new(&data[1..]).read_bytes().is_none());
        assert!(!ProtoReader::new(&data[1..]).skip_field(WireType::LengthDelimited));
    }
}
