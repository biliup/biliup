//! Codec implementations for platform-specific protocols.

pub mod protobuf;
pub mod stt;
pub mod tars;

use std::io::Read;

use crate::error::{DanmakuError, Result};

pub(crate) const MAX_DECOMPRESSED_SIZE: usize = 16 * 1024 * 1024;

pub(crate) fn decompress_limited(reader: impl Read, format: &str, limit: usize) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|error| DanmakuError::Compression(format!("{format}: {error}")))?;
    if data.len() > limit {
        return Err(DanmakuError::Compression(format!(
            "{format}: decompressed message exceeds {limit} bytes"
        )));
    }
    Ok(data)
}
