//! Shared input budgets. These bound input and decoded text, not OS RSS.
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

pub const MAX_XML_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_COMMENTS: usize = 1_000_000;
pub const MAX_COMMENT_OFFSET_MS: i64 = 250 * 3_600_000;
pub const NORMALIZED_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><i>\n";
pub const NORMALIZED_FOOTER: &str = "</i>\n";
/// Text escapes once: `&` -> `&amp;` is the largest byte expansion.
pub const XML_ESCAPE_EXPANSION: u64 = 5;
/// Offset filtering and the minimum origin bound time to 0..=1800000.000s.
/// Mode 1..=9, size 1..=1000 and color 0..=16777215 are validated upstream.
pub const NORMALIZED_COMMENT_OVERHEAD: u64 =
    "<d p=\"1800000.000,9,1000,16777215,0,0,0,0\"></d>\n".len() as u64;
pub const NORMALIZED_DOCUMENT_OVERHEAD: u64 =
    (NORMALIZED_HEADER.len() + NORMALIZED_FOOTER.len()) as u64;

pub const fn normalized_xml_budget(source_bytes: u64, comments: u64) -> Option<u64> {
    let Some(text) = source_bytes.checked_mul(XML_ESCAPE_EXPANSION) else {
        return None;
    };
    let Some(tags) = comments.checked_mul(NORMALIZED_COMMENT_OVERHEAD) else {
        return None;
    };
    let Some(total) = text.checked_add(tags) else {
        return None;
    };
    total.checked_add(NORMALIZED_DOCUMENT_OVERHEAD)
}

pub const MAX_NORMALIZED_XML_BYTES: u64 =
    match normalized_xml_budget(MAX_XML_BYTES, MAX_COMMENTS as u64) {
        Some(value) => value,
        None => panic!("invalid built-in XML budget"),
    };

/// Enforce the byte budget while reading, including files that grow after a
/// metadata check. At most `limit + 1` bytes are retained to detect overflow.
pub fn read_bounded(reader: impl Read, limit: u64) -> io::Result<Vec<u8>> {
    let probe_limit = limit
        .checked_add(1)
        .ok_or_else(|| io::Error::other("XML budget overflow"))?;
    let mut bytes = Vec::new();
    reader.take(probe_limit).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "XML byte budget exceeded",
        ));
    }
    Ok(bytes)
}

/// Counts the bytes actually serialized. It never forwards an over-budget
/// write to the underlying writer, even when the input's metadata was stale.
pub struct BoundedWriter<W> {
    inner: W,
    remaining: u64,
}

impl<W> BoundedWriter<W> {
    pub fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "normalized XML byte budget exceeded",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_checked_and_inclusive() {
        assert_eq!(
            normalized_xml_budget(0, 0),
            Some(NORMALIZED_DOCUMENT_OVERHEAD)
        );
        assert_eq!(
            normalized_xml_budget(1, 1),
            Some(5 + NORMALIZED_COMMENT_OVERHEAD + NORMALIZED_DOCUMENT_OVERHEAD)
        );
        assert_eq!(normalized_xml_budget(u64::MAX, 0), None);
        assert_eq!(normalized_xml_budget(0, u64::MAX), None);
        assert_eq!(normalized_xml_budget(u64::MAX / 5, 1), None);
    }

    #[test]
    fn reading_counts_bytes_instead_of_trusting_metadata() {
        assert_eq!(read_bounded(&b"1234"[..], 4).unwrap(), b"1234");
        assert!(read_bounded(&b"12345"[..], 4).is_err());
        assert_eq!(read_bounded(io::empty(), 0).unwrap(), b"");
        assert!(read_bounded(&b"x"[..], 0).is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("growing.xml");
        std::fs::write(&path, b"1234").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 4);
        let mut append = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        append.write_all(b"5").unwrap();
        assert!(read_bounded(file, 4).is_err());
    }

    #[test]
    fn serialization_counts_actual_bytes_at_the_inclusive_limit() {
        let mut writer = BoundedWriter::new(Vec::new(), 4);
        writer.write_all(b"123").unwrap();
        writer.write_all(b"4").unwrap();
        assert!(writer.write_all(b"5").is_err());
        assert_eq!(writer.into_inner(), b"1234");
    }
}
