use crate::downloader::flv_parser::{
    AACPacketType, AVCPacketType, CodecId, FrameType, ScriptData, SoundFormat, SoundRate,
    SoundSize, SoundType, TagHeader,
};

use crate::downloader::index_tap::FileTap;
use crate::downloader::util::LifecycleFile;
use byteorder::{BigEndian, WriteBytesExt};
use bytes::Bytes;
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};

use tracing::{error, info};

const FLV_HEADER: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x05, //00000101  audio tag  and video tag
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9

pub struct FlvFile<'a> {
    pub buf_writer: BufWriter<File>,
    pub file: LifecycleFile<'a>,
    /// 当前分段已交给 [`LifecycleFile::finish`]，`Drop` 不再重复改名、触发钩子。
    finished: bool,
    /// 当前分段已写的字节数（含文件头），即下一个 tag 的偏移。
    pos: u64,
    index: Option<FileTap>,
}

impl<'a> FlvFile<'a> {
    pub fn new(mut file: LifecycleFile<'a>) -> std::io::Result<Self> {
        // let file_name = util::format_filename(file_name);
        let path = file.create()?;
        let buf_writer = Self::create(path)?;
        let index = file.index.as_ref().map(|tap| tap.open(&file.path));
        Ok(Self {
            buf_writer,
            file,
            finished: false,
            pos: (FLV_HEADER.len() + 4) as u64,
            index,
        })
    }

    /// 结束当前分段并开始下一个。当前分段 flush 失败时返回错误，不再开新文件。
    pub fn create_new(&mut self) -> std::io::Result<()> {
        self.finish()?;
        let path = self.file.create()?;
        self.buf_writer = Self::create(path)?;
        self.finished = false;
        self.pos = (FLV_HEADER.len() + 4) as u64;
        self.index = self
            .file
            .index
            .as_ref()
            .map(|tap| tap.open(&self.file.path));
        Ok(())
    }

    /// flush 并检查错误 → 去掉 `.part` → 触发钩子，见 [`LifecycleFile::finish`]。
    fn finish(&mut self) -> std::io::Result<()> {
        self.finished = true;
        // 先于改名钩子发出：录制器收到分段关闭时，索引任务队列里已有这个文件的全部事件
        if let Some(index) = self.index.take() {
            index.closed(self.pos);
        }
        self.file.finish(&mut self.buf_writer)
    }

    fn create<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<BufWriter<File>> {
        let path = path.as_ref();
        let out = match File::create(path) {
            Ok(o) => o,
            Err(e) => {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("Unable to create flv file {}", path.display()),
                ));
            }
        };
        info!("create flv file {}", path.display());
        let mut buf_writer = BufWriter::new(out);
        buf_writer.write_all(&FLV_HEADER)?;
        Self::write_previous_tag_size(&mut buf_writer, 0)?;
        Ok(buf_writer)
    }

    pub fn write_tag(
        &mut self,
        tag_header: &TagHeader,
        body: &[u8],
        previous_tag_size: &[u8],
    ) -> std::io::Result<usize> {
        let offset = self.write_tag_bytes(tag_header, body, previous_tag_size)?;
        if let Some(index) = &self.index {
            index.flv_tag(
                offset,
                tag_header.tag_type as u8,
                tag_header.timestamp,
                &Bytes::copy_from_slice(body),
            );
        }
        Ok(previous_tag_size.len())
    }

    /// 同 [`Self::write_tag`]，body 已是 [`Bytes`] 时索引旁路不用复制。
    pub fn write_shared_tag(
        &mut self,
        tag_header: &TagHeader,
        body: &Bytes,
        previous_tag_size: &[u8],
    ) -> std::io::Result<usize> {
        let offset = self.write_tag_bytes(tag_header, body, previous_tag_size)?;
        if let Some(index) = &self.index {
            index.flv_tag(
                offset,
                tag_header.tag_type as u8,
                tag_header.timestamp,
                body,
            );
        }
        Ok(previous_tag_size.len())
    }

    /// 写一个 tag，返回它在文件里的起始偏移。
    fn write_tag_bytes(
        &mut self,
        tag_header: &TagHeader,
        body: &[u8],
        previous_tag_size: &[u8],
    ) -> std::io::Result<u64> {
        self.write_tag_header(tag_header)?;
        self.buf_writer.write_all(body)?;
        // write 允许部分写入，短写会静默丢字节并破坏 FLV 结构，必须用 write_all
        self.buf_writer.write_all(previous_tag_size)?;
        let len = (11 + body.len() + previous_tag_size.len()) as u64;
        self.file.bytes_written.add(len);
        let offset = self.pos;
        self.pos += len;
        Ok(offset)
    }

    pub fn write_tag_header(&mut self, tag_header: &TagHeader) -> std::io::Result<()> {
        self.buf_writer.write_u8(tag_header.tag_type as u8)?;
        self.buf_writer
            .write_u24::<BigEndian>(tag_header.data_size)?;
        self.buf_writer
            .write_u24::<BigEndian>(tag_header.timestamp & 0xffffff)?;
        let timestamp_ext = ((tag_header.timestamp >> 24) & 0xff) as u8;
        self.buf_writer.write_u8(timestamp_ext)?;
        self.buf_writer.write_u24::<BigEndian>(tag_header.stream_id)
    }

    pub fn write_previous_tag_size(
        writer: &mut impl Write,
        previous_tag_size: u32,
    ) -> std::io::Result<usize> {
        let bytes = previous_tag_size.to_be_bytes();
        writer.write_all(&bytes)?;
        Ok(bytes.len())
    }
}

impl Drop for FlvFile<'_> {
    fn drop(&mut self) {
        if !self.finished
            && let Err(e) = self.finish()
        {
            error!("{e}");
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct FlvTag<'a> {
    pub header: TagHeader,
    pub data: TagDataHeader<'a>,
}

pub fn to_json<T: ?Sized + Serialize>(mut writer: impl Write, t: &T) -> std::io::Result<usize> {
    serde_json::to_writer(&mut writer, t)?;
    writer.write_all(b"\n")?;
    Ok(1)
}

#[derive(Debug, PartialEq, Serialize)]
pub enum TagDataHeader<'a> {
    Audio {
        sound_format: SoundFormat,
        sound_rate: SoundRate,
        sound_size: SoundSize,
        sound_type: SoundType,
        packet_type: Option<AACPacketType>,
    },
    Video {
        frame_type: FrameType,
        codec_id: CodecId,
        packet_type: Option<AVCPacketType>,
        composition_time: Option<i32>,
    },
    Script(ScriptData<'a>),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 模拟每次调用最多接受 1 字节的 Writer。
    /// 对这类 Writer，`write` 只写入部分数据也返回 Ok，必须用 `write_all` 才能保证完整写入。
    struct ShortWriter(Vec<u8>);

    impl Write for ShortWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let take = buf.len().min(1);
            self.0.extend_from_slice(&buf[..take]);
            Ok(take)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn previous_tag_size_is_fully_written_even_on_short_writes() {
        let mut writer = ShortWriter(Vec::new());
        let written = FlvFile::write_previous_tag_size(&mut writer, 0x0102_0304).unwrap();
        assert_eq!(written, 4);
        assert_eq!(writer.0, [0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn to_json_writes_trailing_newline_even_on_short_writes() {
        let mut writer = ShortWriter(Vec::new());
        to_json(&mut writer, &serde_json::json!({"k": "v"})).unwrap();
        assert!(writer.0.ends_with(b"\n"));
    }
}
