//! TARS (Tencent Application Remote Service) codec.
//!
//! Minimal implementation for Huya danmaku protocol.
//! Supports only the subset needed for WebSocket communication.

/// Maximum nesting depth when skipping Struct/List/Map fields.
/// Real Huya messages nest only a few levels; deeper nesting is malformed input.
const MAX_SKIP_DEPTH: usize = 64;

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
            self.buffer.push(0xF0 | (tars_type as u8));
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

    /// Advance the read position, never past the end of the data.
    ///
    /// Length fields come from the peer: plain `pos += len` overflows on hostile
    /// values (panic in debug builds, wrap-around in release that moves `pos`
    /// backwards and makes `skip_to_tag` loop forever).
    fn advance(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n).min(self.data.len());
    }

    /// Give up on the rest of the data (malformed input).
    fn abandon(&mut self) {
        self.pos = self.data.len();
    }

    /// Skip a field of the given type.
    fn skip_field(&mut self, tars_type: TarsType) {
        self.skip_field_at(tars_type, 0);
    }

    fn skip_field_at(&mut self, tars_type: TarsType, depth: usize) {
        // 跳过嵌套结构是递归的：一长串 StructBegin/List 字节会耗尽线程栈，
        // 栈溢出不是 panic，会直接终止整个进程
        if depth > MAX_SKIP_DEPTH {
            self.abandon();
            return;
        }
        match tars_type {
            TarsType::Int8 => self.advance(1),
            TarsType::Int16 => self.advance(2),
            TarsType::Int32 => self.advance(4),
            TarsType::Int64 => self.advance(8),
            TarsType::Float => self.advance(4),
            TarsType::Double => self.advance(8),
            TarsType::String1 => {
                if self.pos < self.data.len() {
                    let len = self.data[self.pos] as usize;
                    self.advance(1);
                    self.advance(len);
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
                    self.advance(4);
                    self.advance(len);
                }
            }
            TarsType::Map | TarsType::List => {
                let size = self.read_int32_internal().unwrap_or(0);
                let Ok(size) = usize::try_from(size) else {
                    self.abandon();
                    return;
                };
                let count = if tars_type == TarsType::Map {
                    size.saturating_mul(2)
                } else {
                    size
                };
                for _ in 0..count {
                    match self.read_head() {
                        Some((_, t)) => self.skip_field_at(t, depth + 1),
                        // 数据已读完：不要再为声明的（可能是 2^31 个）元素空转
                        None if self.is_empty() => break,
                        None => {}
                    }
                }
            }
            TarsType::Bytes => {
                self.read_head(); // Skip inner type head
                let size = self.read_int32_internal().unwrap_or(0);
                match usize::try_from(size) {
                    Ok(size) => self.advance(size),
                    Err(_) => self.abandon(),
                }
            }
            TarsType::StructBegin => {
                self.skip_to_struct_end_at(depth + 1);
            }
            TarsType::StructEnd | TarsType::Zero => {}
        }
    }

    /// Skip to struct end.
    fn skip_to_struct_end(&mut self) {
        self.skip_to_struct_end_at(0);
    }

    fn skip_to_struct_end_at(&mut self, depth: usize) {
        loop {
            if let Some((_, tars_type)) = self.read_head() {
                if tars_type == TarsType::StructEnd {
                    break;
                }
                self.skip_field_at(tars_type, depth);
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
                other => {
                    self.skip_field(other);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Read the struct at `tag` and invoke `f` on its fields.
    pub fn read_struct<T>(&mut self, tag: u8, f: impl FnOnce(&mut Self) -> T) -> Option<T> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        let (_, tars_type) = self.read_head()?;
        if tars_type != TarsType::StructBegin {
            self.skip_field(tars_type);
            return None;
        }
        let result = f(self);
        self.skip_to_struct_end();
        Some(result)
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
                        if let Some(end) = self.pos.checked_add(len)
                            && end <= self.data.len()
                        {
                            let s = String::from_utf8_lossy(&self.data[self.pos..end]).to_string();
                            self.pos = end;
                            return Some(s);
                        }
                    }
                    None
                }
                other => {
                    self.skip_field(other);
                    None
                }
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
                self.skip_field(tars_type);
                return None;
            }
            // Read inner type head (should be int8)
            self.read_head()?;
            // 负长度按 `as usize` 会变成接近 usize::MAX 的值，随后 `pos + size` 溢出
            let size = usize::try_from(self.read_int32_internal()?).ok()?;
            let end = self.pos.checked_add(size)?;
            if end <= self.data.len() {
                let bytes = self.data[self.pos..end].to_vec();
                self.pos = end;
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

    #[test]
    fn extended_tags_round_trip() {
        let mut oos = TarsOutputStream::new();
        oos.write_int32(15, 12345);
        oos.write_string(255, "extended");
        let mut ios = TarsInputStream::new(oos.get_buffer());
        assert_eq!(ios.read_int32(15), Some(12345));
        assert_eq!(ios.read_string(255), Some("extended".to_string()));
    }

    /// 跳过 tag 0 的 SimpleList 时长度为 -7：修复前 `pos += size as usize`
    /// 在 debug 下溢出 panic，在 release 下回绕回 0，`skip_to_tag` 永远死循环。
    #[test]
    fn negative_bytes_length_while_skipping_does_not_hang_or_panic() {
        let data = [0x0D, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xF9];
        let mut ios = TarsInputStream::new(&data);
        assert_eq!(ios.read_int64(1), None);
    }

    /// 读取长度为 -1 的 SimpleList：修复前 `pos + size` 溢出（debug）或回绕后
    /// 以 end < start 切片（release），都会 panic。
    #[test]
    fn negative_bytes_length_when_reading_returns_none() {
        let data = [0x1D, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut ios = TarsInputStream::new(&data);
        assert_eq!(ios.read_bytes(1), None);
    }

    /// 元素个数为 i32::MAX 的 Map：修复前 `size * 2` 溢出 panic（debug），
    /// 数据读完后还会继续空转约 2^32 次。
    #[test]
    fn huge_map_count_terminates_at_end_of_data() {
        let data = [0x08, 0x02, 0x7F, 0xFF, 0xFF, 0xFF];
        let mut ios = TarsInputStream::new(&data);
        assert_eq!(ios.read_int64(1), None);
    }

    /// 一长串 StructBegin：修复前每个字节递归一层，耗尽线程栈后整个进程被终止
    /// （栈溢出不是 panic，`catch_unwind` 拦不住）。
    #[test]
    fn deeply_nested_structs_do_not_overflow_the_stack() {
        let data = vec![0x0A; 1_000_000];
        let mut ios = TarsInputStream::new(&data);
        assert_eq!(ios.read_int64(1), None);
    }

    #[test]
    fn nested_struct_before_target_tag_is_still_skipped() {
        let mut oos = TarsOutputStream::new();
        oos.write_struct_begin(0);
        oos.write_struct_begin(0);
        oos.write_string(1, "inner");
        oos.write_struct_end();
        oos.write_struct_end();
        oos.write_int64(1, 1400);
        let mut ios = TarsInputStream::new(oos.get_buffer());
        assert_eq!(ios.read_int64(1), Some(1400));
    }
}
