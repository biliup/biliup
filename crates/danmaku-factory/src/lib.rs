//! Safe Rust XML → ASS conversion, preserving the DanmakuFactory layout.
//!
//! Conversion blocks the calling thread. Callers needing cancellation and a
//! timeout use biliup's isolated worker process. Output is written completely
//! into a same-directory temporary file, checked and synced, then published
//! without replacing an existing path. Forced termination can leave a temporary
//! name; its owning work directory is responsible for recovery.
#![forbid(unsafe_code)]

mod api;
#[doc(hidden)]
pub mod limits;
mod native;
pub use api::{ConvertError, ConvertOptions, ConvertStats, ErrorKind, block};
use std::path::Path;

pub fn version() -> &'static str {
    "biliup Rust converter; DanmakuFactory baseline 3813e93b13b087e95901d1822baf9c3540e3f3f6"
}

/// Convert raw UTF-8 XML (at most 128 MiB) to a new ASS file. An existing
/// output, including the input itself, is never overwritten or deleted.
pub fn convert_xml_to_ass(
    input: &Path,
    output: &Path,
    options: &ConvertOptions,
) -> Result<ConvertStats, ConvertError> {
    convert(input, output, options, native::parse::Budget::RAW)
}

/// Internal worker entry for the business-normalized XML envelope. The JSON
/// worker protocol stays unchanged; input bytes use the proven expansion bound,
/// while complete decoded payload and comment counts keep their raw budgets.
#[doc(hidden)]
pub fn convert_normalized_xml_to_ass(
    input: &Path,
    output: &Path,
    options: &ConvertOptions,
) -> Result<ConvertStats, ConvertError> {
    convert(input, output, options, native::parse::Budget::NORMALIZED)
}

fn convert(
    input: &Path,
    output: &Path,
    options: &ConvertOptions,
    budget: native::parse::Budget,
) -> Result<ConvertStats, ConvertError> {
    native::validate(options)?;
    if input.as_os_str().is_empty()
        || output.as_os_str().is_empty()
        || input.as_os_str().as_encoded_bytes().contains(&0)
        || output.as_os_str().as_encoded_bytes().contains(&0)
    {
        return Err(ConvertError::new(
            ErrorKind::InvalidArgument,
            "empty input or output path",
        ));
    }
    if std::fs::symlink_metadata(output).is_ok() {
        return Err(ConvertError::new(
            ErrorKind::OutputExists,
            output.display().to_string(),
        ));
    }
    let file = std::fs::File::open(input).map_err(|error| ConvertError {
        kind: ErrorKind::InputOpen,
        upstream_code: 1,
        os_error: if cfg!(unix) {
            error.raw_os_error()
        } else {
            None
        },
        detail: format!("open {}: {error}", input.display()),
    })?;
    let mut pool = native::parse::read_file(file, budget)?;
    native::model::normalize_and_sort(&mut pool, options);
    let events = native::layout::layout(pool, options)?;
    let stats = ConvertStats {
        danmaku_count: events.len() as u32,
        blocked_count: events
            .iter()
            .filter(|event| event.blocked().is_some())
            .count() as u32,
    };
    native::transaction::publish(output, |writer| {
        native::writer::write(writer, &events, options)
    })?;
    Ok(stats)
}
