use super::model::{Danmaku, DanmakuKind};
use super::{number, special};
use crate::{ConvertError, ErrorKind, limits};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::io::{self, BufReader, Read};

#[derive(Clone, Copy)]
pub(crate) struct Budget {
    pub xml_bytes: u64,
    pub decoded_bytes: usize,
    pub comments: usize,
}

impl Budget {
    pub const RAW: Self = Self {
        xml_bytes: limits::MAX_XML_BYTES,
        decoded_bytes: limits::MAX_XML_BYTES as usize,
        comments: limits::MAX_COMMENTS,
    };
    pub const NORMALIZED: Self = Self {
        xml_bytes: limits::MAX_NORMALIZED_XML_BYTES,
        ..Self::RAW
    };
}

struct BoundedRead<R> {
    inner: R,
    remaining: u64,
}
impl<R: Read> Read for BoundedRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let length = buf
            .len()
            .min(self.remaining.saturating_add(1).min(usize::MAX as u64) as usize);
        let count = self.inner.read(&mut buf[..length])?;
        if count as u64 > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "XML byte budget exceeded",
            ));
        }
        self.remaining -= count as u64;
        Ok(count)
    }
}

/// The frozen Windows reader opened XML in CRT text mode. Translate raw CRLF
/// before XML entity decoding, retaining bare CR and references such as &#13;.
/// Raw bytes are budgeted by BoundedRead before this adapter can shrink them.
#[cfg(any(windows, test))]
struct CrlfRead<R> {
    inner: R,
    buffer: [u8; 8192],
    position: usize,
    length: usize,
    pending_cr: bool,
    eof: bool,
}

#[cfg(any(windows, test))]
impl<R> CrlfRead<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: [0; 8192],
            position: 0,
            length: 0,
            pending_cr: false,
            eof: false,
        }
    }
}

#[cfg(any(windows, test))]
impl<R: Read> Read for CrlfRead<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < output.len() {
            if self.position == self.length && !self.eof {
                // Return existing progress before a potentially failing read.
                // Input is fetched in blocks, never a file syscall per byte.
                if written != 0 {
                    break;
                }
                self.length = self.inner.read(&mut self.buffer)?;
                self.position = 0;
                self.eof = self.length == 0;
            }
            if self.pending_cr {
                self.pending_cr = false;
                if self.position < self.length && self.buffer[self.position] == b'\n' {
                    output[written] = b'\n';
                    self.position += 1;
                } else {
                    output[written] = b'\r';
                }
                written += 1;
            } else if self.position == self.length {
                break;
            } else if self.buffer[self.position] == b'\r' {
                self.pending_cr = true;
                self.position += 1;
            } else {
                let remaining = &self.buffer[self.position..self.length];
                let remaining = &remaining[..remaining.len().min(output.len() - written)];
                let count = remaining
                    .iter()
                    .position(|&byte| byte == b'\r')
                    .unwrap_or(remaining.len());
                output[written..written + count].copy_from_slice(&remaining[..count]);
                self.position += count;
                written += count;
            }
        }
        Ok(written)
    }
}

fn malformed(detail: impl std::fmt::Display) -> ConvertError {
    ConvertError::new(ErrorKind::InputRead, format!("XML: {detail}"))
}

fn valid_xml_text(text: &str) -> Result<(), ConvertError> {
    if text
        .chars()
        .any(|ch| !matches!(ch as u32, 9|10|13|0x20..=0xd7ff|0xe000..=0xfffd|0x10000..=0x10ffff))
    {
        return Err(malformed("illegal XML character"));
    }
    Ok(())
}

fn attributes(start: &BytesStart<'_>) -> Result<Option<String>, ConvertError> {
    std::str::from_utf8(start.as_ref()).map_err(malformed)?;
    let mut p = None;
    for attribute in start.attributes() {
        let attribute = attribute.map_err(malformed)?;
        let value = attribute.unescape_value().map_err(malformed)?;
        valid_xml_text(&value)?;
        if attribute.key.as_ref() == b"p" {
            p = Some(value.into_owned());
        }
    }
    Ok(p)
}

fn begin(p: Option<String>, sequence: usize) -> Result<Danmaku, ConvertError> {
    let p = p.ok_or_else(|| malformed("d element missing p attribute"))?;
    let mut fields = p.split(',');
    let time = fields.next().ok_or_else(|| malformed("missing time"))?;
    let mode = fields.next().ok_or_else(|| malformed("missing type"))?;
    let size = fields
        .next()
        .ok_or_else(|| malformed("missing font size"))?;
    let color = fields.next().ok_or_else(|| malformed("missing color"))?;
    // Legacy strGetLeftPart only returns a field with a following comma.
    // A valid four-field p attribute therefore has color 0, whereas the
    // normalized business format has four extra fields and reads its color.
    let color = if fields.next().is_some() {
        number::int_prefix(color)?
    } else {
        0
    };
    let item = Danmaku::new(
        sequence,
        number::source_time_ms(time)?,
        number::int_prefix(mode)?,
        number::int_prefix(size)?,
        color,
    );
    if item.kind() == DanmakuKind::Special && item.blocked().is_some() {
        return Err(ConvertError::new(
            ErrorKind::UnsupportedInput,
            "special danmaku without data",
        ));
    }
    Ok(item)
}

fn finish(
    (mut item, mut text): (Danmaku, String),
    pool: &mut Vec<Danmaku>,
) -> Result<(), ConvertError> {
    if item.kind() == DanmakuKind::Special {
        let (special, parsed_text) = special::parse(&text)?;
        item.normalized_mut().special = Some(Box::new(special));
        text = parsed_text;
    }
    item.text = text.into_boxed_str();
    pool.try_reserve(1)
        .map_err(|e| ConvertError::new(ErrorKind::OutOfMemory, e.to_string()))?;
    pool.push(item);
    Ok(())
}

#[cfg(test)]
pub(crate) fn read(reader: impl Read, budget: Budget) -> Result<Vec<Danmaku>, ConvertError> {
    read_with_capacity(reader, budget, 0)
}

// The shortest accepted d node has a one-byte name/attribute key, four empty
// numeric fields separated by three commas, and empty-element syntax. Quotes
// can be single or double; both forms use 12 raw bytes. Body, whitespace and
// longer fields only increase the byte count. CRLF translation cannot reduce
// that minimum, since its syntax contains no line endings.
const MIN_COMMENT_XML_BYTES: u64 = b"<d p=\",,,\"/>".len() as u64;

fn initial_capacity(file_bytes: Option<u64>, budget: Budget) -> usize {
    file_bytes
        .map(|bytes| {
            // Capacity is only a heuristic. Bound its metadata allocation by
            // the source byte count as well as the shortest-node upper bound;
            // reserving bytes/12 over-allocated small files and faulted extra
            // transparent huge pages on the measured Linux host.
            let bytes_per_reserved_record =
                MIN_COMMENT_XML_BYTES.max(std::mem::size_of::<Danmaku>() as u64);
            let possible = bytes.min(budget.xml_bytes) / bytes_per_reserved_record;
            possible.min(budget.comments as u64) as usize
        })
        .unwrap_or(0)
}

/// The same opened handle supplies an optional allocation hint and the bytes.
/// Metadata never rejects or authorizes input: all actual budgets are still
/// enforced by read_with_capacity, including when the file grows afterwards.
pub(crate) fn read_file(file: std::fs::File, budget: Budget) -> Result<Vec<Danmaku>, ConvertError> {
    let capacity = initial_capacity(file.metadata().ok().map(|metadata| metadata.len()), budget);
    read_with_capacity(file, budget, capacity)
}

fn read_with_capacity(
    reader: impl Read,
    budget: Budget,
    capacity: usize,
) -> Result<Vec<Danmaku>, ConvertError> {
    let reader = BoundedRead {
        inner: reader,
        remaining: budget.xml_bytes,
    };
    #[cfg(windows)]
    let reader = CrlfRead::new(reader);
    let mut reader = Reader::from_reader(BufReader::new(reader));
    let mut buffer = Vec::new();
    let mut pool = Vec::new();
    // A metadata estimate must not make an otherwise small valid input fail.
    // Failed speculative reservation leaves Vec unchanged; actual insertion
    // still returns OutOfMemory from finish() if its required growth fails.
    let _ = pool.try_reserve_exact(capacity.min(budget.comments));
    let mut pending: Option<(Danmaku, String)> = None;
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut preceding_text = false;
    let mut decoded = 0usize;
    loop {
        match reader.read_event_into(&mut buffer).map_err(|error| {
            let mut mapped = malformed(&error);
            if let quick_xml::Error::Io(source) = &error {
                mapped.upstream_code = 2;
                if cfg!(unix) {
                    mapped.os_error = source.raw_os_error();
                }
            }
            mapped
        })? {
            Event::Start(start) => {
                let p = attributes(&start)?;
                if depth == 0 {
                    if root_seen || preceding_text || start.name().as_ref() != b"i" {
                        return Err(malformed("expected single i root"));
                    }
                    root_seen = true;
                } else if depth == 1 && start.name().as_ref() == b"d" {
                    if pool.len() >= budget.comments {
                        return Err(malformed("comment budget exceeded"));
                    }
                    pending = Some((begin(p, pool.len())?, String::new()));
                } else if pending.is_some() {
                    return Err(malformed("nested d content"));
                }
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| malformed("depth overflow"))?;
            }
            Event::Empty(start) => {
                let p = attributes(&start)?;
                if depth == 0 {
                    if root_seen || preceding_text || start.name().as_ref() != b"i" {
                        return Err(malformed("expected single i root"));
                    }
                    root_seen = true;
                    root_closed = true;
                } else if depth == 1 && start.name().as_ref() == b"d" {
                    if pool.len() >= budget.comments {
                        return Err(malformed("comment budget exceeded"));
                    }
                    finish((begin(p, pool.len())?, String::new()), &mut pool)?;
                } else if pending.is_some() {
                    return Err(malformed("nested d content"));
                }
            }
            Event::Text(text) => {
                let text = text.unescape().map_err(malformed)?;
                valid_xml_text(&text)?;
                if let Some((_, body)) = &mut pending {
                    decoded = decoded
                        .checked_add(text.len())
                        .filter(|&n| n <= budget.decoded_bytes)
                        .ok_or_else(|| malformed("decoded text budget exceeded"))?;
                    body.push_str(&text);
                } else if depth == 0 && !text.trim().is_empty() {
                    // Preserve NoDanmaku for non-XML input, but reject trailing
                    // text after a real root, as the business validator does.
                    if root_seen {
                        return Err(malformed("content outside root"));
                    }
                    preceding_text = true;
                }
            }
            Event::CData(text) => {
                let text = std::str::from_utf8(&text).map_err(malformed)?;
                valid_xml_text(text)?;
                if let Some((_, body)) = &mut pending {
                    decoded = decoded
                        .checked_add(text.len())
                        .filter(|&n| n <= budget.decoded_bytes)
                        .ok_or_else(|| malformed("decoded text budget exceeded"))?;
                    body.push_str(text);
                } else if depth == 0 {
                    return Err(malformed("CDATA outside root"));
                }
            }
            Event::End(end) => {
                std::str::from_utf8(end.as_ref()).map_err(malformed)?;
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| malformed("unexpected end"))?;
                if depth == 1 && end.name().as_ref() == b"d" {
                    finish(
                        pending
                            .take()
                            .ok_or_else(|| malformed("unexpected d end"))?,
                        &mut pool,
                    )?;
                }
                if depth == 0 {
                    root_closed = true;
                }
            }
            Event::DocType(_) => return Err(malformed("DTD is not allowed")),
            Event::Decl(decl) => {
                std::str::from_utf8(decl.as_ref()).map_err(malformed)?;
                // Like the frozen C byte reader and the business validator,
                // ignore the label rather than rejecting actual UTF-8 bytes.
                // quick-xml's encoding feature is disabled, so all text and
                // attributes still decode strictly as UTF-8, without recoding.
            }
            Event::Comment(text) => {
                std::str::from_utf8(text.as_ref()).map_err(malformed)?;
            }
            Event::PI(text) => {
                std::str::from_utf8(text.as_ref()).map_err(malformed)?;
            }
            Event::Eof => break,
        }
        buffer.clear();
    }
    if depth != 0 || pending.is_some() || root_seen && !root_closed {
        return Err(malformed("unclosed document"));
    }
    if pool.is_empty() {
        let mut error = ConvertError::new(ErrorKind::NoDanmaku, "the input contains no danmaku");
        error.upstream_code = 8;
        return Err(error);
    }
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(xml: &str) -> Vec<Danmaku> {
        read(xml.as_bytes(), Budget::RAW).unwrap()
    }

    #[test]
    fn text_entities_cdata_and_unicode_decode_once_without_truncation() {
        let tail = "中😀e\u{301}".repeat(2048);
        let xml = format!(
            "<i><d p=\"1.001,1,25,16777215\">&amp;lt;&amp;#55357;<![CDATA[&amp;]]>&#x1F600;{tail}</d></i>"
        );
        let pool = parse(&xml);
        assert_eq!(pool[0].text.as_ref(), format!("&lt;&#55357;&amp;😀{tail}"));
        assert_eq!(pool[0].source_time_ms, 1001);
    }

    #[test]
    fn actual_utf8_bytes_are_preserved_regardless_of_encoding_declaration_label() {
        for encoding in ["UTF-8", "ISO-8859-1", "GBK", "UTF-16", "unknown-encoding"] {
            let xml = format!(
                "<?xml version=\"1.0\" encoding=\"{encoding}\"?><i><d p=\"1,1,25,16777215,0\">ASCII 中文😀</d></i>"
            );
            assert_eq!(parse(&xml)[0].text.as_ref(), "ASCII 中文😀", "{encoding}");
        }
        let actual_gbk = b"<?xml version=\"1.0\" encoding=\"GBK\"?><i><d p=\"1,1,25,0,0\">\xd6\xd0\xce\xc4</d></i>";
        assert_eq!(
            read(&actual_gbk[..], Budget::RAW).unwrap_err().kind,
            ErrorKind::InputRead
        );
        let actual_latin1 =
            b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><i><d p=\"1,1,25,0,0\">\xe9</d></i>";
        assert_eq!(
            read(&actual_latin1[..], Budget::RAW).unwrap_err().kind,
            ErrorKind::InputRead
        );
        let actual_utf16: Vec<u8> =
            "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-16\"?><i><d p=\"1,1,25,0,0\">x</d></i>"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
        assert_eq!(
            read(&actual_utf16[..], Budget::RAW).unwrap_err().kind,
            ErrorKind::InputRead
        );
    }

    #[test]
    fn line_endings_follow_the_legacy_platform_reader_before_entity_decoding() {
        for (raw, expected) in [
            ("A\nB", "A\nB"),
            ("A\r\nB", if cfg!(windows) { "A\nB" } else { "A\r\nB" }),
            ("A\rB", "A\rB"),
            ("A&#13;&#10;B", "A\r\nB"),
            ("A\r&#10;B", "A\r\nB"),
            ("A&#13;\nB", "A\r\nB"),
            (
                "<![CDATA[A\r\nB]]>",
                if cfg!(windows) { "A\nB" } else { "A\r\nB" },
            ),
        ] {
            let xml = format!("<i><d p=\"0,1,25,16777215,0\">{raw}</d></i>");
            assert_eq!(parse(&xml)[0].text.as_ref(), expected, "{raw:?}");
        }
    }

    #[test]
    fn crlf_adapter_handles_input_output_boundaries_and_budgets_raw_bytes() {
        struct Tiny<'a> {
            input: &'a [u8],
            chunk: usize,
        }
        impl Read for Tiny<'_> {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                let count = output.len().min(self.input.len()).min(self.chunk);
                output[..count].copy_from_slice(&self.input[..count]);
                self.input = &self.input[count..];
                Ok(count)
            }
        }
        let raw = b"\r\nA\rB\nC\r\r\nD\r\n\r";
        for input_chunk in [1, 2, 3, 8192] {
            for output_chunk in [1, 2, 3, 8192] {
                let mut reader = CrlfRead::new(Tiny {
                    input: raw,
                    chunk: input_chunk,
                });
                let mut actual = Vec::new();
                let mut buffer = vec![0; output_chunk];
                loop {
                    let count = reader.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    actual.extend_from_slice(&buffer[..count]);
                }
                assert_eq!(
                    actual, b"\nA\rB\nC\r\nD\n\r",
                    "{input_chunk}/{output_chunk}"
                );
            }
        }
        let mut exact = CrlfRead::new(BoundedRead {
            inner: &raw[..],
            remaining: raw.len() as u64,
        });
        let mut output = Vec::new();
        exact.read_to_end(&mut output).unwrap();
        assert_eq!(output, b"\nA\rB\nC\r\nD\n\r");
        let mut excessive = CrlfRead::new(BoundedRead {
            inner: &raw[..],
            remaining: raw.len() as u64 - 1,
        });
        assert_eq!(
            excessive.read_to_end(&mut Vec::new()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        // Run the same pre-decode adapter on Linux to verify the Windows-only
        // production path without asserting that Windows itself was executed.
        let raw = b"<i><d p=\"0,1,25,16777215,0\">A\r\nB&#13;&#10;C</d></i>";
        let filtered = CrlfRead::new(Tiny {
            input: raw,
            chunk: 1,
        });
        assert_eq!(
            read(filtered, Budget::RAW).unwrap()[0].text.as_ref(),
            "A\nB\r\nC"
        );
    }

    #[test]
    fn four_field_color_matches_the_frozen_delimiter_parser() {
        assert_eq!(parse("<i><d p=\"1,1,25,16777215\">x</d></i>")[0].color(), 0);
        assert_eq!(
            parse("<i><d p=\"1,1,25,16777215,0\">x</d></i>")[0].color(),
            0xffffff
        );
    }

    #[test]
    fn malformed_xml_and_invalid_utf8_are_input_errors() {
        for xml in [
            "<i>",
            "<i><d p=\"1,1,25,0\">x</i>",
            "<i/><i/>",
            "<!DOCTYPE i><i/>",
            "<i><d>x</d></i>",
            "<i><d p=\"1,1,25,0\">&#55357;</d></i>",
            "<i><d p=\"1,1,25,0\">&#0;</d></i>",
            "<i><d p=\"1,1,25,0\">&#1;</d></i>",
            "<i><d p=\"NaN,1,25,0\">x</d></i>",
            "<i><d p=\"1,1,25,0\"><x/></d></i>",
            "<i><d p=\"1,1,25,0\">&bad;</d></i>",
        ] {
            assert!(read(xml.as_bytes(), Budget::RAW).is_err(), "{xml}");
        }
        assert_eq!(
            read(&b"<i><d p=\"1,1,25,0\">\xff</d></i>"[..], Budget::RAW)
                .unwrap_err()
                .kind,
            ErrorKind::InputRead
        );
        for xml in ["", "not xml at all", "<i/>", "<i></i>"] {
            let e = read(xml.as_bytes(), Budget::RAW).unwrap_err();
            assert_eq!((e.kind, e.upstream_code), (ErrorKind::NoDanmaku, 8));
        }
    }

    #[test]
    fn reproducible_malformed_bytes_never_panic() {
        let mut seed = 0x3813e93bu64;
        let budget = Budget {
            xml_bytes: 1024,
            decoded_bytes: 1024,
            comments: 16,
        };
        for _ in 0..1024 {
            let mut bytes = Vec::new();
            for _ in 0..128 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                bytes.push((seed >> 32) as u8);
            }
            let _ = read(bytes.as_slice(), budget);
        }
    }

    #[test]
    fn byte_count_and_full_payload_limits_apply_during_building() {
        let xml = b"<i><d p=\"1,1,25,0\">abcd</d></i>";
        let budget = Budget {
            xml_bytes: xml.len() as u64,
            decoded_bytes: 4,
            comments: 1,
        };
        assert!(read(&xml[..], budget).is_ok());
        assert!(
            read(
                &xml[..],
                Budget {
                    xml_bytes: budget.xml_bytes - 1,
                    ..budget
                }
            )
            .is_err()
        );
        assert!(
            read(
                &xml[..],
                Budget {
                    decoded_bytes: 3,
                    ..budget
                }
            )
            .is_err()
        );
        assert!(
            read(
                &xml[..],
                Budget {
                    comments: 0,
                    ..budget
                }
            )
            .is_err()
        );
        let special = b"<i><d p=\"1,7,25,0\">[0,0,\"1-1\",3,\"\",0,0,0,0,1,0,true,\"\",1]</d></i>";
        assert!(
            read(
                &special[..],
                Budget {
                    xml_bytes: 1000,
                    ..budget
                }
            )
            .is_err()
        );
    }

    #[test]
    fn file_capacity_hint_has_a_proven_minimum_and_bounded_fallback() {
        const MINIMAL: &str = "<d p=\",,,\"/>";
        assert_eq!(MIN_COMMENT_XML_BYTES, 12);
        assert_eq!(MINIMAL.len(), 12);
        let parsed = parse(&format!("<i>{MINIMAL}</i>"));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].kind(), DanmakuKind::Unknown(0));
        assert_eq!(parsed[0].source_time_ms, 0);
        assert!(parsed[0].text.is_empty());
        for shorter in ["<d p=\",,\"/>", "<d p=\",\"/>", "<d p=\"\"/>"] {
            assert!(read(format!("<i>{shorter}</i>").as_bytes(), Budget::RAW).is_err());
        }
        assert_eq!(initial_capacity(None, Budget::RAW), 0);
        assert_eq!(initial_capacity(Some(0), Budget::RAW), 0);
        assert_eq!(initial_capacity(Some(11), Budget::RAW), 0);
        assert_eq!(initial_capacity(Some(12), Budget::RAW), 0);
        let record_bytes = std::mem::size_of::<Danmaku>() as u64;
        assert_eq!(initial_capacity(Some(record_bytes), Budget::RAW), 1);
        assert_eq!(
            initial_capacity(Some(u64::MAX), Budget::RAW),
            limits::MAX_COMMENTS
        );
        let small = Budget {
            xml_bytes: 36,
            decoded_bytes: 12,
            comments: 2,
        };
        assert_eq!(initial_capacity(Some(u64::MAX), small), 0);
        for bytes in [
            0,
            12,
            63,
            64,
            192,
            824_543,
            2_029_047,
            limits::MAX_XML_BYTES,
            u64::MAX,
        ] {
            let capacity = initial_capacity(Some(bytes), Budget::RAW);
            assert!(
                (capacity as u64).checked_mul(record_bytes).unwrap()
                    <= bytes.min(Budget::RAW.xml_bytes)
            );
            assert!(capacity <= Budget::RAW.comments);
        }
        let empty = tempfile::tempfile().unwrap();
        assert_eq!(
            read_file(empty, Budget::RAW).unwrap_err().kind,
            ErrorKind::NoDanmaku
        );
    }

    #[test]
    fn file_hint_reserves_once_but_actual_read_and_comment_budgets_still_decide() {
        use std::io::{Seek, SeekFrom, Write};
        let mut xml = b"<i><d p=\"0,1,25,16777215,0\">body</d></i>".to_vec();
        xml.resize(std::mem::size_of::<Danmaku>() * 3, b' ');
        let budget = Budget {
            xml_bytes: xml.len() as u64,
            decoded_bytes: 4,
            comments: 3,
        };
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&xml).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let pool = read_file(file, budget).unwrap();
        assert_eq!(pool.capacity(), 3);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].text.as_ref(), "body");

        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&xml).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(
            read_file(
                file,
                Budget {
                    xml_bytes: budget.xml_bytes - 1,
                    ..budget
                }
            )
            .unwrap_err()
            .kind,
            ErrorKind::InputRead
        );

        // Simulate a stale zero-length metadata hint, then grow the same
        // already opened file. This exercises the shared read path with the
        // exact earlier hint rather than relying on a timing race.
        let mut file = tempfile::tempfile().unwrap();
        let stale_capacity = initial_capacity(Some(file.metadata().unwrap().len()), budget);
        assert_eq!(stale_capacity, 0);
        file.write_all(&xml).unwrap();
        file.write_all(b" ").unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(
            read_with_capacity(file, budget, stale_capacity)
                .unwrap_err()
                .kind,
            ErrorKind::InputRead
        );

        let two = b"<i><d p=\",,,\"/><d p=\",,,\"/></i>";
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(two).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(
            read_file(
                file,
                Budget {
                    xml_bytes: 100,
                    decoded_bytes: 0,
                    comments: 1
                }
            )
            .unwrap_err()
            .kind,
            ErrorKind::InputRead
        );
    }

    #[test]
    fn failed_speculative_reservation_falls_back_to_actual_required_growth() {
        let xml = b"<i><d p=\"0,1,25,16777215,0\">body</d></i>";
        let budget = Budget {
            xml_bytes: xml.len() as u64,
            decoded_bytes: 4,
            comments: usize::MAX,
        };
        // This is a deterministic capacity overflow, not an attempted huge
        // allocation or an allocator mock. The shared production path ignores
        // only the hint failure, then parses and allocates the one actual node.
        let pool = read_with_capacity(&xml[..], budget, usize::MAX).unwrap();
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].text.as_ref(), "body");
        assert_eq!(
            read_with_capacity(
                &xml[..],
                Budget {
                    decoded_bytes: 3,
                    ..budget
                },
                usize::MAX
            )
            .unwrap_err()
            .kind,
            ErrorKind::InputRead
        );
    }
}
