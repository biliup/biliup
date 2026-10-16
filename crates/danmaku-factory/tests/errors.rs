//! Failures are reported with their cause and never leave an output file;
//! inputs are never modified.

use danmaku_factory::{ConvertOptions, ErrorKind, convert_xml_to_ass};
use std::path::{Path, PathBuf};

/// Relative to the package directory, where tests run (see golden.rs).
fn golden(path: &str) -> PathBuf {
    Path::new("tests/golden").join(path)
}

fn biliup_options() -> ConvertOptions {
    ConvertOptions {
        font_name: "Noto Sans CJK SC".into(),
        font_size_strict: true,
        opacity: 204,
        outline: 2.,
        shadow: 0.,
        fix_time: 12.,
        display_area: 0.4,
        density: -1,
        ..Default::default()
    }
}

fn convert_text(
    xml: &str,
    options: &ConvertOptions,
) -> Result<String, danmaku_factory::ConvertError> {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.xml");
    std::fs::write(&input, xml).unwrap();
    let output = dir.path().join("out.ass");
    let result = convert_xml_to_ass(&input, &output, options);
    if result.is_err() {
        assert!(!output.exists(), "a failed conversion left {output:?}");
    }
    assert_eq!(std::fs::read_to_string(&input).unwrap(), xml);
    result.map(|_| std::fs::read_to_string(&output).unwrap())
}

const ONE: &str = "<i>\n<d p=\"1.000,1,25,16777215,0,0,0,0\">hello</d>\n</i>\n";

#[test]
fn missing_and_unreadable_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.ass");
    let error = convert_xml_to_ass(&dir.path().join("missing.xml"), &output, &biliup_options())
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InputOpen, "{error}");
    #[cfg(unix)]
    assert_eq!(error.os_error, Some(2), "{error}");
    assert!(!output.exists());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let invalid = Path::new(std::ffi::OsStr::from_bytes(b"bad\0path"));
        assert_eq!(
            convert_xml_to_ass(invalid, &output, &biliup_options())
                .unwrap_err()
                .kind,
            ErrorKind::InvalidArgument
        );
    }

    // fopen() accepts a directory on Linux; the first read fails.
    #[cfg(target_os = "linux")]
    {
        let error = convert_xml_to_ass(dir.path(), &output, &biliup_options()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InputRead, "{error}");
        assert!(!output.exists());
    }
}

#[test]
fn inputs_without_danmaku() {
    for xml in [
        "<?xml version=\"1.0\"?><i></i>",
        "<i/>",
        "",
        "not xml at all",
        "<i><unknown/></i>",
    ] {
        let error = convert_text(xml, &biliup_options()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::NoDanmaku, "{xml:?}: {error}");
        assert_eq!(error.upstream_code, 8);
    }
}

#[test]
fn special_danmaku_without_data_is_rejected_instead_of_crashing() {
    // Type -5 counts as special, but readXml only parses type 7's payload.
    let xml = "<i><d p=\"1,-5,25,0\">x</d><d p=\"2,1,25,0\">ok</d></i>";
    let error = convert_text(xml, &biliup_options()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::UnsupportedInput, "{error}");
}

#[test]
fn outputs_are_never_overwritten_and_failures_leave_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.xml");
    std::fs::write(&input, ONE).unwrap();
    let existing = dir.path().join("existing.ass");
    std::fs::write(&existing, "keep").unwrap();
    let error = convert_xml_to_ass(&input, &existing, &biliup_options()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutputExists);
    assert_eq!(std::fs::read_to_string(&existing).unwrap(), "keep");
    // Converting a file onto itself would destroy the source.
    let error = convert_xml_to_ass(&input, &input, &biliup_options()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutputExists);
    assert_eq!(std::fs::read_to_string(&input).unwrap(), ONE);

    let missing_dir = dir.path().join("no-such-dir").join("out.ass");
    let error = convert_xml_to_ass(&input, &missing_dir, &biliup_options()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutputCreate, "{error}");
    #[cfg(unix)]
    assert_eq!(error.os_error, Some(2), "{error}");
}

/// A write that fails half way (a full disk) leaves no truncated ASS behind.
/// The file size limit applies to the whole process, so the conversion runs
/// in a child: this test binary, filtered to this test.
#[cfg(unix)]
#[test]
fn a_write_error_removes_the_partial_output() {
    const CHILD: &str = "DANMAKU_FACTORY_TEST_WRITE_LIMIT_DIR";
    if let Some(dir) = std::env::var_os(CHILD) {
        // SAFETY: plain libc calls on this single-threaded child process.
        unsafe {
            libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
            let limit = libc::rlimit {
                rlim_cur: 4096,
                rlim_max: 4096,
            };
            assert_eq!(libc::setrlimit(libc::RLIMIT_FSIZE, &limit), 0);
        }
        let output = Path::new(&dir).join("out.ass");
        let error = convert_xml_to_ass(&golden("inputs/dense.xml"), &output, &biliup_options())
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::OutputWrite, "{error}");
        assert_eq!(error.os_error, Some(libc::EFBIG), "{error}");
        assert!(!output.exists(), "the truncated output was left behind");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_write_error_removes_the_partial_output",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, dir.path())
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && report.contains("1 passed"),
        "{report}{}",
        String::from_utf8_lossy(&child.stderr)
    );
    // The child really converted: it got as far as writing (and removing) out.ass.
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn every_option_is_range_checked() {
    let base = biliup_options;
    let invalid: Vec<(&str, ConvertOptions)> = vec![
        ("width 0", ConvertOptions { width: 0, ..base() }),
        (
            "width",
            ConvertOptions {
                width: 16385,
                ..base()
            },
        ),
        (
            "width overflow",
            ConvertOptions {
                width: u32::MAX,
                ..base()
            },
        ),
        (
            "height 0",
            ConvertOptions {
                height: 0,
                ..base()
            },
        ),
        (
            "display 0",
            ConvertOptions {
                display_area: 0.,
                ..base()
            },
        ),
        (
            "display NaN",
            ConvertOptions {
                display_area: f32::NAN,
                ..base()
            },
        ),
        (
            "display >1",
            ConvertOptions {
                display_area: 1.1,
                ..base()
            },
        ),
        (
            "scroll area",
            ConvertOptions {
                scroll_area: f32::INFINITY,
                ..base()
            },
        ),
        (
            "scroll 0",
            ConvertOptions {
                scroll_time: 0.,
                ..base()
            },
        ),
        (
            "scroll NaN",
            ConvertOptions {
                scroll_time: f32::NAN,
                ..base()
            },
        ),
        (
            "scroll long",
            ConvertOptions {
                scroll_time: 3601.,
                ..base()
            },
        ),
        (
            "fix 0",
            ConvertOptions {
                fix_time: 0.,
                ..base()
            },
        ),
        (
            "density",
            ConvertOptions {
                density: -2,
                ..base()
            },
        ),
        (
            "spacing",
            ConvertOptions {
                line_spacing: 20000,
                ..base()
            },
        ),
        (
            "margin",
            ConvertOptions {
                top_margin: -20000,
                ..base()
            },
        ),
        (
            "font 0",
            ConvertOptions {
                font_size: 0,
                ..base()
            },
        ),
        (
            "font big",
            ConvertOptions {
                font_size: 4097,
                ..base()
            },
        ),
        (
            "font name empty",
            ConvertOptions {
                font_name: String::new(),
                ..base()
            },
        ),
        (
            "font name comma",
            ConvertOptions {
                font_name: "A,B".into(),
                ..base()
            },
        ),
        (
            "font name control",
            ConvertOptions {
                font_name: "A\nB".into(),
                ..base()
            },
        ),
        (
            "font name NUL",
            ConvertOptions {
                font_name: "A\0B".into(),
                ..base()
            },
        ),
        (
            "font name long",
            ConvertOptions {
                font_name: "字".repeat(43),
                ..base()
            },
        ),
        (
            "opacity 0",
            ConvertOptions {
                opacity: 0,
                ..base()
            },
        ),
        (
            "outline",
            ConvertOptions {
                outline: 4.5,
                ..base()
            },
        ),
        (
            "outline negative",
            ConvertOptions {
                outline: -0.1,
                ..base()
            },
        ),
        (
            "blur",
            ConvertOptions {
                outline_blur: 101.,
                ..base()
            },
        ),
        (
            "shadow",
            ConvertOptions {
                shadow: f32::NAN,
                ..base()
            },
        ),
        (
            "block",
            ConvertOptions {
                block_mode: 128,
                ..base()
            },
        ),
    ];
    for (name, options) in invalid {
        let error = convert_text(ONE, &options).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidArgument, "{name}: {error}");
    }
    // The limits themselves are accepted.
    for options in [
        ConvertOptions {
            width: 16384,
            height: 1,
            display_area: 1.,
            ..base()
        },
        ConvertOptions {
            font_name: "字".repeat(42),
            font_size: 4096,
            ..base()
        },
        ConvertOptions {
            opacity: 255,
            outline: 4.,
            shadow: 4.,
            density: -1,
            ..base()
        },
        ConvertOptions {
            scroll_time: 3600.,
            fix_time: 0.0001,
            block_mode: 127,
            ..base()
        },
    ] {
        convert_text(ONE, &options).unwrap();
    }
}

#[test]
fn chinese_and_space_paths() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("弹幕 测试 dir").join("场次 1");
    std::fs::create_dir_all(&nested).unwrap();
    let input = nested.join("中文 文件 😂.xml");
    std::fs::copy(golden("inputs/basic_cn.xml"), &input).unwrap();
    let output = nested.join("输出 文件.ass");
    let mut options = biliup_options();
    options.font_name = "Noto Sans CJK SC".into();
    convert_xml_to_ass(&input, &output, &options).unwrap();
    let expected = std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap();
    let mut actual = std::fs::read(&output).unwrap();
    if cfg!(windows) {
        actual = actual.strip_prefix(b"\xEF\xBB\xBF").unwrap().to_vec();
        actual.retain(|&b| b != b'\r');
    }
    assert!(actual == expected);
}

/// Beyond MAX_PATH _wfopen needs `\\?\` paths.
#[cfg(windows)]
#[test]
fn long_paths_on_windows() {
    let dir = tempfile::tempdir().unwrap();
    let mut nested = dir.path().to_path_buf();
    while nested.as_os_str().len() < 300 {
        nested.push("目录 0123456789abcdef");
    }
    std::fs::create_dir_all(&nested).unwrap();
    let input = nested.join("输入.xml");
    std::fs::copy(golden("inputs/basic_cn.xml"), &input).unwrap();
    let output = nested.join("输出.ass");
    convert_xml_to_ass(&input, &output, &biliup_options()).unwrap();
    assert!(std::fs::metadata(&output).unwrap().len() > 1000);
    let complete = std::fs::read(&output).unwrap();
    assert_eq!(
        convert_xml_to_ass(&input, &output, &biliup_options())
            .unwrap_err()
            .kind,
        ErrorKind::OutputExists
    );
    assert_eq!(std::fs::read(&output).unwrap(), complete);
    assert_eq!(std::fs::read_dir(&nested).unwrap().count(), 2);
    std::fs::rename(&output, nested.join("released.ass")).unwrap();
    // Relative paths work as well.
    let relative = Path::new("relative-out.ass");
    let _ = std::fs::remove_file(relative);
    convert_xml_to_ass(&input, relative, &biliup_options()).unwrap();
    std::fs::remove_file(relative).unwrap();
}

/// Callers racing for one output path: exactly one converts, the others are
/// refused, and the file left is complete.
#[test]
fn racing_callers_never_replace_or_delete_each_others_output() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("shared.ass");
    let results: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    convert_xml_to_ass(&golden("inputs/dense.xml"), &output, &biliup_options())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    for result in results.iter().filter_map(|r| r.as_ref().err()) {
        assert_eq!(result.kind, ErrorKind::OutputExists, "{result}");
    }
    let actual = std::fs::read(&output).unwrap();
    if !cfg!(windows) {
        assert!(actual == std::fs::read(golden("expected/dense-biliup.ass")).unwrap());
    }
}

#[test]
fn concurrent_callers_get_correct_results() {
    let expected = std::fs::read(golden("expected/dense-biliup.ass")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::thread::scope(|scope| {
        for thread in 0..8 {
            let dir = dir.path();
            let expected = &expected;
            scope.spawn(move || {
                for round in 0..3 {
                    let output = dir.join(format!("{thread}-{round}.ass"));
                    convert_xml_to_ass(&golden("inputs/dense.xml"), &output, &biliup_options())
                        .unwrap();
                    let actual = std::fs::read(&output).unwrap();
                    if !cfg!(windows) {
                        assert!(actual == *expected, "thread {thread} round {round}");
                    }
                }
            });
        }
    });
}
