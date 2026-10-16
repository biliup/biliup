//! Regressions for the owned core's original option domain, independently
//! checked against the original C writer rather than the narrower legacy ABI.

use danmaku_factory_sys::{convert_full, model::Config};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Output(PathBuf);

impl Output {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "danmaku-full-options-{}-{nonce}-{}.ass",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../danmaku-factory/tests/golden/inputs/tiny.xml")
}

#[test]
fn owned_core_preserves_original_opacity_bytes_outside_legacy_abi_limits() {
    let output = Output::new();
    let config = Config {
        opacity: 256,
        outline_opacity: 256,
        ..Default::default()
    };
    let stats = convert_full(&tiny(), &output.0, &config).unwrap();
    assert_eq!(stats.danmaku_count, 9);
    let text = std::fs::read_to_string(&output.0).unwrap();
    // Original toHexOpacity always emits two characters. Its signed
    // remainder arithmetic gives "0/" for 255 - 256, not eight hex Fs.
    assert!(text.contains("Style: R2L,Microsoft YaHei,38,&H0/FFFFFF,&H00FFFFFF,&H0/000000,"));
    assert!(text.contains("Style: SP,Microsoft YaHei,38,&H00FFFFFF,&H00FFFFFF,&H0/000000,"));
}

#[test]
fn owned_core_accepts_original_options_beyond_legacy_abi_caps() {
    let output = Output::new();
    let config = Config {
        resolution_x: 20_000,
        resolution_y: 20_000,
        font_size: 5_000,
        scroll_time: 3_601.0,
        fix_time: 3_601.0,
        outline: 7.5,
        outline_blur: 125.0,
        shadow: 7.5,
        density: -2,
        top_margin: 20_000,
        bottom_margin: 20_000,
        line_spacing: 20_000,
        display_area: 1.25,
        scroll_area: 1.25,
        block_mode: 256,
        ..Default::default()
    };
    let stats = convert_full(&tiny(), &output.0, &config).unwrap();
    assert_eq!(stats.danmaku_count, 9);
    let text = std::fs::read_to_string(&output.0).unwrap();
    assert!(text.lines().any(|line| line == "PlayResX: 20000"));
    assert!(text.lines().any(|line| line == "PlayResY: 20000"));
    assert!(text.contains("Style: R2L,Microsoft YaHei,5000,"));
}

#[test]
fn owned_core_rejects_overflowing_opacity_before_creating_output() {
    for outline in [false, true] {
        let output = Output::new();
        let mut config = Config::default();
        if outline {
            config.outline_opacity = i32::MIN + 255;
        } else {
            config.opacity = i32::MIN + 255;
        }
        let error = convert_full(&tiny(), &output.0, &config).unwrap_err();
        assert_eq!(error.status, danmaku_factory_sys::DF_ERR_INVALID_ARGUMENT);
        assert!(!output.0.exists());

        if outline {
            config.outline_opacity += 1;
        } else {
            config.opacity += 1;
        }
        assert_eq!(
            convert_full(&tiny(), &output.0, &config)
                .unwrap()
                .danmaku_count,
            9
        );
    }
}
