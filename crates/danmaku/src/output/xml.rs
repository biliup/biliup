//! XML output writer for Bilibili-compatible danmaku format.
//!
//! Output format is compatible with Bilibili's danmaku XML format:
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <i>
//!   <d p="time,type,size,color,timestamp,0,uid,0">content</d>
//!   <s timestamp="..." uid="..." type="..." detail="...">content</s>
//!   <o timestamp="..." raw="..." />
//! </i>
//! ```

use std::borrow::Cow;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use quick_xml::Writer;
use quick_xml::escape::escape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use serde_json::Value;

use crate::error::Result;
use crate::message::{ChatMessage, DanmakuEvent, GiftMessage, GuardBuyMessage, SuperChatMessage};
use crate::protocols::bilibili::Notification;

/// Configuration for the XML writer.
#[derive(Debug, Clone)]
pub struct XmlWriterConfig {
    /// Whether to save raw message data.
    pub save_raw: bool,
    /// Whether to save detailed info (uid, username attributes).
    pub save_detail: bool,
    /// Trusted XML fragment to embed before the root closing tag.
    pub xml_preview: Option<&'static [u8]>,
    /// Auto-save interval in seconds.
    pub save_interval: u64,
}

impl Default for XmlWriterConfig {
    fn default() -> Self {
        Self {
            save_raw: false,
            save_detail: false,
            xml_preview: None,
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

fn xml_line(text: &str) -> Cow<'_, str> {
    let escaped = escape(text);
    if escaped.contains(['\r', '\n', '\t']) {
        Cow::Owned(
            escaped
                .replace('\r', "&#13;")
                .replace('\n', "&#10;")
                .replace('\t', "&#9;"),
        )
    } else {
        escaped
    }
}

fn push_line_attribute(elem: &mut BytesStart<'_>, key: &str, value: &str) {
    let safe = xml_safe(value);
    let escaped = xml_line(&safe);
    // Byte attributes are already escaped. In particular, keep attribute
    // whitespace as character references so XML readers return it unchanged.
    elem.push_attribute((key.as_bytes(), escaped.as_bytes()));
}

fn push_json_attribute(elem: &mut BytesStart<'_>, key: &str, json: &str) {
    // These characters can occur in JSON strings but are forbidden in XML 1.0.
    let json = if json.contains(['\u{fffe}', '\u{ffff}']) {
        Cow::Owned(
            json.replace('\u{fffe}', r"\ufffe")
                .replace('\u{ffff}', r"\uffff"),
        )
    } else {
        Cow::Borrowed(json)
    };
    push_line_attribute(elem, key, &json);
}

fn push_scalar_attributes(elem: &mut BytesStart<'_>, data: &Value) {
    let Some(fields) = data.as_object() else {
        return;
    };
    for (key, value) in fields {
        match value {
            Value::String(text) => push_line_attribute(elem, key, text),
            Value::Number(number) => {
                elem.push_attribute((key.as_str(), number.to_string().as_str()));
            }
            Value::Bool(flag) => {
                elem.push_attribute((key.as_str(), if *flag { "true" } else { "false" }));
            }
            _ => {}
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

        let mut buf_writer = BufWriter::new(file);
        // Danmaku2ASS detects Bilibili XML by the adjacent declaration and root.
        let declaration = BytesDecl::new("1.0", Some("UTF-8"), None);
        Writer::new(&mut buf_writer).write_event(Event::Decl(declaration))?;
        let mut writer = Writer::new_with_indent(buf_writer, b'\t', 1);

        // Write root element start
        // Retain the wall-clock origin of p[0] for recording/post-edit alignment.
        // The optional root attribute remains compatible with ordinary XML readers.
        let origin = Utc::now().timestamp_millis().to_string();
        let now = Instant::now();
        let mut root = BytesStart::new("i");
        root.push_attribute(("recording_start_time_ms", origin.as_str()));
        writer.write_event(Event::Start(root))?;

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
        let written = self.write_structured_event(event, None)?;
        self.finish_event(written)
    }

    pub(crate) fn write_bilibili_notification(
        &mut self,
        notification: &Notification,
    ) -> Result<()> {
        if let Some(event) = &notification.event
            && self.write_structured_event(event, Some(notification))?
        {
            return self.finish_event(true);
        }
        let detail = notification
            .detail
            .as_ref()
            .filter(|_| self.config.save_detail);
        let tag = match detail {
            Some(_) => "s",
            None if self.config.save_raw && notification.raw.is_some() => "o",
            None => return Ok(()),
        };
        let mut elem = BytesStart::new(tag);
        elem.push_attribute(("timestamp", Utc::now().timestamp().to_string().as_str()));
        elem.push_attribute(("ts", self.elapsed().as_str()));
        if let Some((kind, _)) = detail {
            elem.push_attribute(("type", *kind));
        }
        self.push_notification_attributes(&mut elem, Some(notification));
        if let Some(items) = detail.and_then(|(_, data)| data["items"].as_array()) {
            self.write_gift_batch(elem, items)?;
        } else {
            self.writer.write_event(Event::Empty(elem))?;
        }
        self.finish_event(true)
    }

    fn write_structured_event(
        &mut self,
        event: &DanmakuEvent,
        notification: Option<&Notification>,
    ) -> Result<bool> {
        let written = match event {
            DanmakuEvent::Chat(msg) => {
                self.write_chat(msg, notification)?;
                true
            }
            DanmakuEvent::Gift(msg) if self.config.save_detail => {
                self.write_gift(msg, notification)?;
                true
            }
            DanmakuEvent::SuperChat(msg) if self.config.save_detail => {
                self.write_superchat(msg, notification)?;
                true
            }
            DanmakuEvent::GuardBuy(msg) if self.config.save_detail => {
                self.write_guard_buy(msg, notification)?;
                true
            }
            DanmakuEvent::Other { raw_data } if self.config.save_raw => {
                let mut elem = BytesStart::new("o");
                elem.push_attribute(("timestamp", Utc::now().timestamp().to_string().as_str()));
                self.write_text_element(elem, "o", raw_data)?;
                true
            }
            _ => false,
        };
        Ok(written)
    }

    fn finish_event(&mut self, written: bool) -> Result<()> {
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

    fn elapsed(&self) -> String {
        format!("{:.3}", self.start_time.elapsed().as_secs_f64())
    }

    fn push_notification_attributes(
        &self,
        elem: &mut BytesStart<'_>,
        notification: Option<&Notification>,
    ) {
        let Some(notification) = notification else {
            return;
        };
        if self.config.save_detail
            && let Some((_, detail)) = &notification.detail
        {
            push_scalar_attributes(elem, detail);
        }
        if self.config.save_raw
            && let Some(raw) = &notification.raw
        {
            push_json_attribute(elem, "raw", raw);
        }
    }

    fn write_gift_batch(&mut self, elem: BytesStart<'_>, items: &[Value]) -> Result<()> {
        self.writer.write_event(Event::Start(elem))?;
        let mut inline = Writer::new(self.writer.get_mut());
        for item in items {
            let mut element = BytesStart::new("item");
            push_scalar_attributes(&mut element, item);
            inline.write_event(Event::Empty(element))?;
        }
        // Keep the closing tag on the same line as the batch.
        self.writer.write_event(Event::Text(BytesText::new("")))?;
        self.writer.write_event(Event::End(BytesEnd::new("s")))?;
        Ok(())
    }

    fn write_text_element(&mut self, elem: BytesStart<'_>, tag: &str, content: &str) -> Result<()> {
        self.writer.write_event(Event::Start(elem))?;
        let content = xml_safe(content);
        self.writer
            .write_event(Event::Text(BytesText::from_escaped(xml_line(&content))))?;
        self.writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }

    /// Write a chat message.
    fn write_chat(&mut self, msg: &ChatMessage, notification: Option<&Notification>) -> Result<()> {
        let (mode, size, timestamp) = notification
            .and_then(|notification| notification.chat_metadata.as_ref())
            .map(|meta| (meta.mode, meta.size, meta.timestamp_ms))
            .unwrap_or_else(|| (1, 25, msg.timestamp.timestamp()));
        let uid = msg.uid.unwrap_or(0);

        // Format: time,type,size,color,timestamp,0,uid,0
        let p = format!(
            "{},{mode},{size},{},{timestamp},0,{uid},0",
            self.elapsed(),
            msg.color,
        );

        let mut elem = BytesStart::new("d");
        elem.push_attribute(("p", p.as_str()));

        if self.config.save_detail {
            elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
            elem.push_attribute(("uid", uid.to_string().as_str()));
            if let Some(ref name) = msg.name {
                push_line_attribute(&mut elem, "user", name);
            }
        }

        self.push_notification_attributes(&mut elem, notification);
        self.write_text_element(elem, "d", &msg.content)
    }

    /// Write a gift message.
    fn write_gift(&mut self, msg: &GiftMessage, notification: Option<&Notification>) -> Result<()> {
        let timestamp = msg.timestamp.timestamp();

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        push_line_attribute(&mut elem, "username", &msg.name);
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "gift"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        push_line_attribute(&mut elem, "giftname", &msg.gift_name);
        self.write_detail_element(elem, &msg.content, notification)
    }

    /// Write a Super Chat message.
    fn write_superchat(
        &mut self,
        msg: &SuperChatMessage,
        notification: Option<&Notification>,
    ) -> Result<()> {
        let timestamp = msg.timestamp.timestamp();

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        push_line_attribute(&mut elem, "username", &msg.name);
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "super_chat"));
        elem.push_attribute(("num", "1"));
        elem.push_attribute(("giftname", "醒目留言"));

        self.write_detail_element(elem, &msg.content, notification)
    }

    /// Write a guard buy message.
    fn write_guard_buy(
        &mut self,
        msg: &GuardBuyMessage,
        notification: Option<&Notification>,
    ) -> Result<()> {
        let timestamp = msg.timestamp.timestamp();
        let content = format!("{}上了{}个月{}", msg.name, msg.num, msg.gift_name);

        let mut elem = BytesStart::new("s");
        elem.push_attribute(("timestamp", timestamp.to_string().as_str()));
        elem.push_attribute(("uid", msg.uid.to_string().as_str()));
        push_line_attribute(&mut elem, "username", &msg.name);
        elem.push_attribute(("price", msg.price.to_string().as_str()));
        elem.push_attribute(("type", "guard_buy"));
        elem.push_attribute(("num", msg.num.to_string().as_str()));
        push_line_attribute(&mut elem, "giftname", &msg.gift_name);
        self.write_detail_element(elem, &content, notification)
    }

    fn write_detail_element(
        &mut self,
        mut elem: BytesStart<'_>,
        content: &str,
        notification: Option<&Notification>,
    ) -> Result<()> {
        if notification.is_some() {
            elem.push_attribute(("ts", self.elapsed().as_str()));
        }
        self.push_notification_attributes(&mut elem, notification);
        self.write_text_element(elem, "s", content)
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
            if let Some(preview) = self.config.xml_preview {
                self.writer.get_mut().write_all(preview)?;
            }
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

        let origin = content
            .split("recording_start_time_ms=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .and_then(|value| value.parse::<i64>().ok())
            .expect("XML records its clock origin");
        assert!((Utc::now().timestamp_millis() - origin).abs() < 10_000);

        let bad: Vec<char> = content.chars().filter(|&c| !allowed_in_xml(c)).collect();
        assert!(bad.is_empty(), "invalid XML chars {bad:?} in:\n{content}");
        assert!(content.contains("ACTION waves &lt;&amp;&gt; ok</d>"));
        assert!(content.contains(r#"user="badname""#));
        assert!(content.contains(r#"username="gifter""#));
        assert!(content.contains(r#"giftname="rose""#));
        assert!(content.contains(">rawdata</o>"));
    }
}
