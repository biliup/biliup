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

use std::borrow::Cow;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

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

/// XML 1.0 的 `Char` 产生式：`#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] |
/// [#x10000-#x10FFFF]`。
fn is_xml_char(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..
    )
}

/// 去掉 XML 1.0 不允许出现的字符。quick-xml 只转义 `<>&'"`，其余控制字符
/// （如 Twitch `/me` 消息的 `\x01ACTION ...\x01`）会原样写出，一条这样的弹幕就让
/// 整个文件无法被严格的 XML 解析器读取（字符引用 `&#1;` 在 XML 1.0 里同样非法）。
fn xml_safe(s: &str) -> Cow<'_, str> {
    if s.chars().all(is_xml_char) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(s.chars().filter(|&c| is_xml_char(c)).collect())
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
    finalized: bool,
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
            finalized: false,
            config,
        })
    }

    /// Write a danmaku event.
    pub fn write_event(&mut self, event: &DanmakuEvent) -> Result<()> {
        let written = match event {
            DanmakuEvent::Chat(msg) => {
                self.write_chat(msg)?;
                true
            }
            DanmakuEvent::Gift(msg) => {
                if self.config.save_detail {
                    self.write_gift(msg)?;
                    true
                } else {
                    false
                }
            }
            DanmakuEvent::SuperChat(msg) => {
                if self.config.save_detail {
                    self.write_superchat(msg)?;
                    true
                } else {
                    false
                }
            }
            DanmakuEvent::GuardBuy(msg) => {
                if self.config.save_detail {
                    self.write_guard_buy(msg)?;
                    true
                } else {
                    false
                }
            }
            DanmakuEvent::Enter(_) => false,
            DanmakuEvent::Other { raw_data } => {
                if self.config.save_raw {
                    self.write_raw(raw_data)?;
                    true
                } else {
                    false
                }
            }
        };

        if written {
            self.message_count += 1;
        }

        // Auto-save periodically
        if written && self.last_save.elapsed().as_secs() >= self.config.save_interval {
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
                elem.push_attribute(("user", xml_safe(name).as_ref()));
            }
        }

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&xml_safe(&msg.content))))?;
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
        elem.push_attribute(("username", xml_safe(&msg.name).as_ref()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "gift"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        elem.push_attribute(("giftname", xml_safe(&msg.gift_name).as_ref()));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&xml_safe(&msg.content))))?;
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
        elem.push_attribute(("username", xml_safe(&msg.name).as_ref()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "super_chat"));
        elem.push_attribute(("num", "1"));
        elem.push_attribute(("giftname", "醒目留言"));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&xml_safe(&msg.content))))?;
        self.writer.write_event(Event::End(BytesEnd::new("s")))?;

        Ok(())
    }

    /// Write a guard buy message.
    fn write_guard_buy(&mut self, msg: &GuardBuyMessage) -> Result<()> {
        if !self.config.save_detail {
            return Ok(());
        }

        let timestamp = msg.timestamp.timestamp();
        let content = format!("{}上了{}个月{}", msg.name, msg.num, msg.gift_name);

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        elem.push_attribute(("username", xml_safe(&msg.name).as_ref()));
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "guard_buy"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        elem.push_attribute(("giftname", xml_safe(&msg.gift_name).as_ref()));

        self.writer.write_event(Event::Start(elem))?;
        self.writer
            .write_event(Event::Text(BytesText::new(&xml_safe(&content))))?;
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
            .write_event(Event::Text(BytesText::new(&xml_safe(raw_data))))?;
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
        self.finalize()?;
        Ok(self.file_path)
    }

    /// Finish the current XML document and flush it to disk.
    pub fn finalize(&mut self) -> Result<()> {
        if !self.finalized {
            self.writer.write_event(Event::End(BytesEnd::new("i")))?;
            self.finalized = true;
        }
        self.flush()?;
        Ok(())
    }

    /// Get the current file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn has_messages(&self) -> bool {
        self.message_count > 0
    }

    /// Get the number of messages written.
    pub fn message_count(&self) -> u64 {
        self.message_count
    }

    /// Rename the output file.
    pub fn rename(&mut self, new_path: impl AsRef<Path>) -> Result<()> {
        let new_path = new_path.as_ref().to_path_buf();

        self.finalize()?;

        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if new_path.exists() {
            fs::remove_file(&new_path)?;
        }
        fs::rename(&self.file_path, &new_path)?;
        self.file_path = new_path;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_enter_event_does_not_mark_xml_as_having_messages() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-enter-empty-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("danmaku.xml");
        let mut writer = XmlWriter::new(&path, XmlWriterConfig::default()).unwrap();

        writer
            .write_event(&DanmakuEvent::Enter(crate::message::EnterMessage {
                name: "tester".to_string(),
                uid: Some(1),
                timestamp: chrono::Utc::now(),
            }))
            .unwrap();

        assert!(!writer.has_messages());
        let _ = writer.finish().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    /// XML 1.0 的 Char 产生式；不在其中的字符会让整个文件无法被严格解析器读取。
    fn allowed_in_xml(c: char) -> bool {
        matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..)
    }

    #[test]
    fn control_characters_are_stripped_so_the_xml_stays_well_formed() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-xml-ctrl-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let config = XmlWriterConfig {
            save_raw: true,
            save_detail: true,
            ..XmlWriterConfig::default()
        };
        let mut writer = XmlWriter::new(dir.join("danmaku.xml"), config).unwrap();

        // Twitch 的 `/me` 消息正文是 `\x01ACTION ...\x01`
        let chat = ChatMessage::new("\u{1}ACTION waves\u{1} <&> \u{8}ok\u{fffe}".to_string())
            .with_name("bad\u{0}name\u{b}");
        writer.write_event(&DanmakuEvent::Chat(chat)).unwrap();
        writer
            .write_event(&DanmakuEvent::Gift(GiftMessage {
                name: "gifter\u{1f}".to_string(),
                uid: 1,
                gift_name: "rose\u{c}".to_string(),
                price: 1,
                num: 1,
                content: "gifter\u{1f}投喂了1个rose\u{c}".to_string(),
                timestamp: Utc::now(),
            }))
            .unwrap();
        writer
            .write_event(&DanmakuEvent::Other {
                raw_data: "raw\u{2}data".to_string(),
            })
            .unwrap();
        let path = writer.finish().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let bad: Vec<char> = content.chars().filter(|&c| !allowed_in_xml(c)).collect();
        assert!(bad.is_empty(), "invalid XML chars {bad:?} in:\n{content}");
        assert!(content.contains("ACTION waves &lt;&amp;&gt; ok</d>"));
        assert!(content.contains(r#"user="badname""#));
        assert!(content.contains(r#"username="gifter""#));
        assert!(content.contains(r#"giftname="rose""#));
        assert!(content.contains(">rawdata</o>"));
    }
}
