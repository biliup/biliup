//! TARS (Tencent Application Remote Service) codec.
//!
//! Minimal implementation for Huya danmaku protocol.
//! Supports only the subset needed for WebSocket communication.

/// TARS data types.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TarsType {
    Int8 = 0,
    Int16 = 1,
    Int32 = 2,
    Int64 = 3,
    Float = 4,
    Double = 5,
    String1 = 6,
    String4 = 7,
    Map = 8,
    List = 9,
    StructBegin = 10,
    StructEnd = 11,
    Zero = 12,
    Bytes = 13,
}

impl TarsType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(TarsType::Int8),
            1 => Some(TarsType::Int16),
            2 => Some(TarsType::Int32),
            3 => Some(TarsType::Int64),
            4 => Some(TarsType::Float),
            5 => Some(TarsType::Double),
            6 => Some(TarsType::String1),
            7 => Some(TarsType::String4),
            8 => Some(TarsType::Map),
            9 => Some(TarsType::List),
            10 => Some(TarsType::StructBegin),
            11 => Some(TarsType::StructEnd),
            12 => Some(TarsType::Zero),
            13 => Some(TarsType::Bytes),
            _ => None,
        }
    }
}

/// TARS output stream for encoding.
pub struct TarsOutputStream {
    buffer: Vec<u8>,
}

impl TarsOutputStream {
    /// Create a new output stream.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Get the encoded buffer.
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Write data head.
    fn write_head(&mut self, tag: u8, tars_type: TarsType) {
        if tag < 15 {
            let head = (tag << 4) | (tars_type as u8);
            self.buffer.push(head);
        } else {
            let head = (0xF0 | (tars_type as u8)) as u16;
            self.buffer.push((head >> 8) as u8);
            self.buffer.push(tag);
        }
    }

    /// Write a boolean value.
    pub fn write_bool(&mut self, tag: u8, value: bool) {
        self.write_int8(tag, if value { 1 } else { 0 });
    }

    /// Write an int8 value.
    pub fn write_int8(&mut self, tag: u8, value: i8) {
        if value == 0 {
            self.write_head(tag, TarsType::Zero);
        } else {
            self.write_head(tag, TarsType::Int8);
            self.buffer.push(value as u8);
        }
    }

    /// Write an int16 value.
    pub fn write_int16(&mut self, tag: u8, value: i16) {
        if value >= -128 && value <= 127 {
            self.write_int8(tag, value as i8);
        } else {
            self.write_head(tag, TarsType::Int16);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    /// Write an int32 value.
    pub fn write_int32(&mut self, tag: u8, value: i32) {
        if value >= -32768 && value <= 32767 {
            self.write_int16(tag, value as i16);
        } else {
            self.write_head(tag, TarsType::Int32);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    /// Write an int64 value.
    pub fn write_int64(&mut self, tag: u8, value: i64) {
        if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
            self.write_int32(tag, value as i32);
        } else {
            self.write_head(tag, TarsType::Int64);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    /// Write a string value.
    pub fn write_string(&mut self, tag: u8, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len();

        if len <= 255 {
            self.write_head(tag, TarsType::String1);
            self.buffer.push(len as u8);
        } else {
            self.write_head(tag, TarsType::String4);
            self.buffer.extend_from_slice(&(len as u32).to_be_bytes());
        }
        self.buffer.extend_from_slice(bytes);
    }

    /// Write bytes value.
    pub fn write_bytes(&mut self, tag: u8, value: &[u8]) {
        self.write_head(tag, TarsType::Bytes);
        self.write_head(0, TarsType::Int8);
        self.write_int32(0, value.len() as i32);
        self.buffer.extend_from_slice(value);
    }

    /// Write struct begin marker.
    pub fn write_struct_begin(&mut self, tag: u8) {
        self.write_head(tag, TarsType::StructBegin);
    }

    /// Write struct end marker.
    pub fn write_struct_end(&mut self) {
        self.write_head(0, TarsType::StructEnd);
    }
}

impl Default for TarsOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

/// TARS input stream for decoding.
pub struct TarsInputStream<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> TarsInputStream<'a> {
    /// Create a new input stream.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Check if at end.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Peek at the next tag and type without advancing position.
    fn peek_head(&self) -> Option<(u8, TarsType)> {
        if self.pos >= self.data.len() {
            return None;
        }

        let byte = self.data[self.pos];
        let tag = (byte >> 4) & 0x0F;
        let type_id = byte & 0x0F;

        let (tag, _len) = if tag >= 15 {
            if self.pos + 1 >= self.data.len() {
                return None;
            }
            (self.data[self.pos + 1], 2)
        } else {
            (tag, 1)
        };

        TarsType::from_u8(type_id).map(|t| (tag, t))
    }

    /// Read the data head.
    fn read_head(&mut self) -> Option<(u8, TarsType)> {
        if self.pos >= self.data.len() {
            return None;
        }

        let byte = self.data[self.pos];
        let tag = (byte >> 4) & 0x0F;
        let type_id = byte & 0x0F;
        self.pos += 1;

        let tag = if tag >= 15 {
            if self.pos >= self.data.len() {
                return None;
            }
            let t = self.data[self.pos];
            self.pos += 1;
            t
        } else {
            tag
        };

        TarsType::from_u8(type_id).map(|t| (tag, t))
    }

    /// Skip to a specific tag.
    fn skip_to_tag(&mut self, target_tag: u8) -> bool {
        while self.pos < self.data.len() {
            if let Some((tag, tars_type)) = self.peek_head() {
                if tars_type == TarsType::StructEnd {
                    return false;
                }
                if tag == target_tag {
                    return true;
                }
                if tag > target_tag {
                    return false;
                }
                // Skip this field
                self.read_head();
                self.skip_field(tars_type);
            } else {
                break;
            }
        }
        false
    }

    /// Skip a field of the given type.
    fn skip_field(&mut self, tars_type: TarsType) {
        match tars_type {
            TarsType::Int8 => self.pos += 1,
            TarsType::Int16 => self.pos += 2,
            TarsType::Int32 => self.pos += 4,
            TarsType::Int64 => self.pos += 8,
            TarsType::Float => self.pos += 4,
            TarsType::Double => self.pos += 8,
            TarsType::String1 => {
                if self.pos < self.data.len() {
                    let len = self.data[self.pos] as usize;
                    self.pos += 1 + len;
                }
            }
            TarsType::String4 => {
                if self.pos + 4 <= self.data.len() {
                    let len = u32::from_be_bytes([
                        self.data[self.pos],
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                    ]) as usize;
                    self.pos += 4 + len;
                }
            }
            TarsType::Map => {
                let size = self.read_int32_internal().unwrap_or(0);
                for _ in 0..size * 2 {
                    if let Some((_, t)) = self.read_head() {
                        self.skip_field(t);
                    }
                }
            }
            TarsType::List => {
                let size = self.read_int32_internal().unwrap_or(0);
                for _ in 0..size {
                    if let Some((_, t)) = self.read_head() {
                        self.skip_field(t);
                    }
                }
            }
            TarsType::Bytes => {
                self.read_head(); // Skip inner type head
                let size = self.read_int32_internal().unwrap_or(0);
                self.pos += size as usize;
            }
            TarsType::StructBegin => {
                self.skip_to_struct_end();
            }
            TarsType::StructEnd | TarsType::Zero => {}
        }
    }

    /// Skip to struct end.
    fn skip_to_struct_end(&mut self) {
        loop {
            if let Some((_, tars_type)) = self.read_head() {
                if tars_type == TarsType::StructEnd {
                    break;
                }
                self.skip_field(tars_type);
            } else {
                break;
            }
        }
    }

    /// Read int32 value internally (for size fields).
    fn read_int32_internal(&mut self) -> Option<i32> {
        if let Some((_, tars_type)) = self.read_head() {
            match tars_type {
                TarsType::Zero => Some(0),
                TarsType::Int8 => {
                    if self.pos < self.data.len() {
                        let v = self.data[self.pos] as i8 as i32;
                        self.pos += 1;
                        Some(v)
                    } else {
                        None
                    }
                }
                TarsType::Int16 => {
                    if self.pos + 2 <= self.data.len() {
                        let v = i16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
                        self.pos += 2;
                        Some(v as i32)
                    } else {
                        None
                    }
                }
                TarsType::Int32 => {
                    if self.pos + 4 <= self.data.len() {
                        let v = i32::from_be_bytes([
                            self.data[self.pos],
                            self.data[self.pos + 1],
                            self.data[self.pos + 2],
                            self.data[self.pos + 3],
                        ]);
                        self.pos += 4;
                        Some(v)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Read an int32 value at a tag.
    pub fn read_int32(&mut self, tag: u8) -> Option<i32> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        self.read_int32_internal()
    }

    /// Read an int64 value at a tag.
    pub fn read_int64(&mut self, tag: u8) -> Option<i64> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        if let Some((_, tars_type)) = self.read_head() {
            match tars_type {
                TarsType::Zero => Some(0),
                TarsType::Int8 => {
                    if self.pos < self.data.len() {
                        let v = self.data[self.pos] as i8 as i64;
                        self.pos += 1;
                        Some(v)
                    } else {
                        None
                    }
                }
                TarsType::Int16 => {
                    if self.pos + 2 <= self.data.len() {
                        let v = i16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
                        self.pos += 2;
                        Some(v as i64)
                    } else {
                        None
                    }
                }
                TarsType::Int32 => {
                    if self.pos + 4 <= self.data.len() {
                        let v = i32::from_be_bytes([
                            self.data[self.pos],
                            self.data[self.pos + 1],
                            self.data[self.pos + 2],
                            self.data[self.pos + 3],
                        ]);
                        self.pos += 4;
                        Some(v as i64)
                    } else {
                        None
                    }
                }
                TarsType::Int64 => {
                    if self.pos + 8 <= self.data.len() {
                        let v = i64::from_be_bytes([
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
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Read a string value at a tag.
    pub fn read_string(&mut self, tag: u8) -> Option<String> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        if let Some((_, tars_type)) = self.read_head() {
            match tars_type {
                TarsType::String1 => {
                    if self.pos < self.data.len() {
                        let len = self.data[self.pos] as usize;
                        self.pos += 1;
                        if self.pos + len <= self.data.len() {
                            let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len])
                                .to_string();
                            self.pos += len;
                            return Some(s);
                        }
                    }
                    None
                }
                TarsType::String4 => {
                    if self.pos + 4 <= self.data.len() {
                        let len = u32::from_be_bytes([
                            self.data[self.pos],
                            self.data[self.pos + 1],
                            self.data[self.pos + 2],
                            self.data[self.pos + 3],
                        ]) as usize;
                        self.pos += 4;
                        if self.pos + len <= self.data.len() {
                            let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len])
                                .to_string();
                            self.pos += len;
                            return Some(s);
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Read bytes value at a tag.
    pub fn read_bytes(&mut self, tag: u8) -> Option<Vec<u8>> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        if let Some((_, tars_type)) = self.read_head() {
            if tars_type != TarsType::Bytes {
                return None;
            }
            // Read inner type head (should be int8)
            self.read_head()?;
            let size = self.read_int32_internal()? as usize;
            if self.pos + size <= self.data.len() {
                let bytes = self.data[self.pos..self.pos + size].to_vec();
                self.pos += size;
                return Some(bytes);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_int() {
        let mut oos = TarsOutputStream::new();
        oos.write_int32(0, 1);
        oos.write_int64(1, 12345);
        oos.write_int64(6, 0);

        let buffer = oos.get_buffer();
        assert!(!buffer.is_empty());

        // Verify with input stream
        let mut ios = TarsInputStream::new(buffer);
        assert_eq!(ios.read_int32(0), Some(1));
        assert_eq!(ios.read_int64(1), Some(12345));
        assert_eq!(ios.read_int64(6), Some(0));
    }

    #[test]
    fn test_write_string() {
        let mut oos = TarsOutputStream::new();
        oos.write_string(0, "hello");
        oos.write_string(1, "world");

        let buffer = oos.get_buffer();

        let mut ios = TarsInputStream::new(buffer);
        assert_eq!(ios.read_string(0), Some("hello".to_string()));
        assert_eq!(ios.read_string(1), Some("world".to_string()));
    }

    #[test]
    fn test_write_bytes() {
        let mut oos = TarsOutputStream::new();
        oos.write_bytes(0, b"test data");

        let buffer = oos.get_buffer();

        let mut ios = TarsInputStream::new(buffer);
        assert_eq!(ios.read_bytes(0), Some(b"test data".to_vec()));
    }
}
