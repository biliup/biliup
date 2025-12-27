//! XML output writer for Bilibili-compatible danmaku format.
//!
//! Output format is compatible with Bilibili's danmaku XML format:
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <i>
//!   <d p="time,type,size,color,timestamp,0,uid,0">content</d>
//!   <s timestamp="..." uid="..." ...>content</s>
//!   <o timestamp="...">raw_data</o>
//! </i>
//! ```

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::error::Result;
use crate::message::{ChatMessage, DanmakuEvent, GiftMessage, GuardBuyMessage, SuperChatMessage};

/// Configuration for the XML writer.
#[derive(Debug, Clone)]
pub struct XmlWriterConfig {
    /// Whether to save raw message data.
    pub save_raw: bool,
    /// Whether to save detailed info (uid, username attributes).
    pub save_detail: bool,
    /// Auto-save interval in seconds.
    pub save_interval: u64,
}

impl Default for XmlWriterConfig {
    fn default() -> Self {
        Self {
            save_raw: false,
            save_detail: false,
            save_interval: 10,
        }
    }
}

/// XML writer for danmaku output.
pub struct XmlWriter {
    /// Output file path.
    file_path: PathBuf,
    /// XML writer instance.
    writer: Writer<BufWriter<File>>,
    /// Start time for calculating relative timestamps.
    start_time: Instant,
    /// Number of messages written.
    message_count: u64,
    /// Last save time.
    last_save: Instant,
    /// Configuration.
    config: XmlWriterConfig,
}

impl XmlWriter {
    /// Create a new XML writer.
    pub fn new(file_path: impl AsRef<Path>, config: XmlWriterConfig) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)?;

        let buf_writer = BufWriter::new(file);
        let mut writer = Writer::new_with_indent(buf_writer, b'\t', 1);

        // Write XML declaration
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // Write root element start
        let root = BytesStart::new("i");
        writer.write_event(Event::Start(root))?;

        let now = Instant::now();

        Ok(Self {
            file_path,
            writer,
            start_time: now,
            message_count: 0,
            last_save: now,
            config,
        })
    }

    /// Write a danmaku event.
    pub fn write_event(&mut self, event: &DanmakuEvent) -> Result<()> {
        match event {
            DanmakuEvent::Chat(msg) => self.write_chat(msg)?,
            DanmakuEvent::Gift(msg) => self.write_gift(msg)?,
            DanmakuEvent::SuperChat(msg) => self.write_superchat(msg)?,
            DanmakuEvent::GuardBuy(msg) => self.write_guard_buy(msg)?,
            DanmakuEvent::Enter(_) => {
                // Enter messages are typically not written to XML
            }
            DanmakuEvent::Other { raw_data } => {
                if self.config.save_raw {
                    self.write_raw(raw_data)?;
                }
            }
        }

        self.message_count += 1;

        // Auto-save periodically
        if self.last_save.elapsed().as_secs() >= self.config.save_interval {
            self.flush()?;
            self.last_save = Instant::now();
        }

        Ok(())
    }

    /// Write a chat message.
    fn write_chat(&mut self, msg: &ChatMessage) -> Result<()> {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let timestamp = msg.timestamp.timestamp();
        let uid = msg.uid.unwrap_or(0);

        // Format: time,type,size,color,timestamp,0,uid,0
        // type: 1=scroll, 4=bottom, 5=top
        // size: 25 is standard
        let p = format!(
            "{:.3},1,25,{},{},0,{},0",
            elapsed, msg.color, timestamp, uid
        );

        let mut elem = BytesStart::new("d");
        elem.push_attribute(("p", p.as_str()));

        if self.config.save_detail {
            elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
            elem.push_attribute(("uid", uid.to_string().as_str()));
            if let Some(ref name) = msg.name {
                elem.push_attribute(("user", name.as_str()));
            }
        }

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&msg.content)))?;
        self.writer.write_event(Event::End(BytesEnd::new("d")))?;

        Ok(())
    }

    /// Write a gift message.
    fn write_gift(&mut self, msg: &GiftMessage) -> Result<()> {
        if !self.config.save_detail {
            return Ok(());
        }

        let timestamp = msg.timestamp.timestamp();

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        elem.push_attribute(("username", msg.name.as_str()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "gift"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        elem.push_attribute(("giftname", msg.gift_name.as_str()));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&msg.content)))?;
        self.writer.write_event(Event::End(BytesEnd::new("s")))?;

        Ok(())
    }

    /// Write a Super Chat message.
    fn write_superchat(&mut self, msg: &SuperChatMessage) -> Result<()> {
        if !self.config.save_detail {
            return Ok(());
        }

        let timestamp = msg.timestamp.timestamp();

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        elem.push_attribute(("username", msg.name.as_str()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "super_chat"));
        elem.push_attribute(("num", "1"));
        elem.push_attribute(("giftname", "醒目留言"));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&msg.content)))?;
        self.writer.write_event(Event::End(BytesEnd::new("s")))?;

        Ok(())
    }

    /// Write a guard buy message.
    fn write_guard_buy(&mut self, msg: &GuardBuyMessage) -> Result<()> {
        if !self.config.save_detail {
            return Ok(());
        }

        let timestamp = msg.timestamp.timestamp();
        let content = format!(
            "{}上了{}个月{}",
            msg.name, msg.num, msg.gift_name
        );

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        elem.push_attribute(("username", msg.name.as_str()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "guard_buy"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        elem.push_attribute(("giftname", msg.gift_name.as_str()));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&content)))?;
        self.writer.write_event(Event::End(BytesEnd::new("s")))?;

        Ok(())
    }

    /// Write raw message data.
    fn write_raw(&mut self, raw_data: &str) -> Result<()> {
        let timestamp = Utc::now().timestamp();

        let mut elem = BytesStart::new("o");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(raw_data)))?;
        self.writer.write_event(Event::End(BytesEnd::new("o")))?;

        Ok(())
    }

    /// Flush the writer to disk.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.get_mut().flush()?;
        Ok(())
    }

    /// Finish writing and close the file.
    pub fn finish(mut self) -> Result<PathBuf> {
        // Write closing tag
        self.writer.write_event(Event::End(BytesEnd::new("i")))?;
        self.flush()?;
        Ok(self.file_path)
    }

    /// Get the current file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Get the number of messages written.
    pub fn message_count(&self) -> u64 {
        self.message_count
    }

    /// Rename the output file.
    pub fn rename(&mut self, new_path: impl AsRef<Path>) -> Result<()> {
        let new_path = new_path.as_ref().to_path_buf();

        // Finish current file
        self.writer.write_event(Event::End(BytesEnd::new("i")))?;
        self.flush()?;

        // Rename the file
        std::fs::rename(&self.file_path, &new_path)?;
        self.file_path = new_path;

        Ok(())
    }
}
