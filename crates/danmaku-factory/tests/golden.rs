//! Compare the native converter with immutable DanmakuFactory CLI output from
//! commit 3813e93b13b087e95901d1822baf9c3540e3f3f6 plus biliup's fixes.
//! Reviewed text and stable-sort changes have independent assertions.

use danmaku_factory::{ConvertOptions, convert_xml_to_ass};
use serde::Deserialize;
use std::path::PathBuf;
mod support;

#[derive(Deserialize)]
struct Case {
    name: String,
    input: String,
    options: ConvertOptions,
    #[serde(default)]
    #[allow(dead_code)]
    upstream_bug: Option<String>,
}

/// Tests run in the package directory; a relative path also works when the
/// Windows build is tested under wine, where the build host's absolute
/// paths do not exist.
fn golden() -> PathBuf {
    PathBuf::from("tests/golden")
}

fn cases() -> Vec<Case> {
    let text = std::fs::read_to_string(golden().join("cases.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// writeAss opens its output in text mode and adds a BOM on Windows; the
/// corpus was written on Linux.
fn platform_neutral(bytes: Vec<u8>) -> Vec<u8> {
    if !cfg!(windows) {
        return bytes;
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        if !(b == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
            out.push(b);
        }
    }
    out
}

fn first_difference(actual: &[u8], expected: &[u8]) -> String {
    let at = actual
        .iter()
        .zip(expected)
        .position(|(a, b)| a != b)
        .unwrap_or(actual.len().min(expected.len()));
    let line = expected[..at.min(expected.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1;
    let show = |bytes: &[u8]| {
        let start = at.saturating_sub(60).min(bytes.len());
        let end = (at + 60).min(bytes.len());
        String::from_utf8_lossy(&bytes[start..end]).into_owned()
    };
    format!(
        "first difference at byte {at} (line {line}, lengths {} vs {})\n actual:   {:?}\n expected: {:?}",
        actual.len(),
        expected.len(),
        show(actual),
        show(expected)
    )
}

#[test]
fn every_case_matches_the_cli_byte_for_byte() {
    let cases = cases();
    assert!(cases.len() >= 30, "corpus is incomplete");
    let dir = tempfile::tempdir().unwrap();
    for case in cases {
        let input = golden().join("inputs").join(&case.input);
        let before = std::fs::read(&input).unwrap();
        let output = dir.path().join(format!("{}.ass", case.name));
        let stats = convert_xml_to_ass(&input, &output, &case.options)
            .unwrap_or_else(|e| panic!("{}: {e}", case.name));
        let legacy =
            std::fs::read(golden().join("expected").join(format!("{}.ass", case.name))).unwrap();
        let expected = support::expected(&case.input, legacy);
        let actual = platform_neutral(std::fs::read(&output).unwrap());
        assert!(
            actual == expected,
            "{}: {}",
            case.name,
            first_difference(&actual, &expected)
        );
        assert!(stats.danmaku_count > 0 && stats.blocked_count <= stats.danmaku_count);
        assert_eq!(
            std::fs::read(&input).unwrap(),
            before,
            "{}: input changed",
            case.name
        );
    }
}

#[test]
fn counts_report_what_was_read_and_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let case = cases()
        .into_iter()
        .find(|case| case.name == "repeat-block")
        .unwrap();
    let stats = convert_xml_to_ass(
        &golden().join("inputs").join(&case.input),
        &dir.path().join("out.ass"),
        &case.options,
    )
    .unwrap();
    let ass = std::fs::read_to_string(dir.path().join("out.ass")).unwrap();
    let dialogues = ass.lines().filter(|l| l.starts_with("Dialogue:")).count() as u32;
    assert_eq!(stats.danmaku_count, 60);
    // save_blocked is off: blocked danmaku are not written at all.
    assert_eq!(dialogues + stats.blocked_count, stats.danmaku_count);
    assert!(stats.blocked_count > 0);
}
