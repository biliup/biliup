use super::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[test]
fn time_correction_cannot_overflow_before_the_offset_budget_filter() {
    assert_eq!(corrected_time(1001, -2000, 5).unwrap(), -994);
    assert_eq!(corrected_time(i64::MAX, 0, 0).unwrap(), i64::MAX);
    assert!(corrected_time(i64::MAX, 1, 0).is_err());
    assert!(corrected_time(i64::MIN, 0, -1).is_err());
    assert!(corrected_time(i64::MAX, 1, -1).is_err());
}

#[test]
fn normalization_expansion_bound_uses_the_production_path() {
    use danmaku_factory::limits::{
        NORMALIZED_COMMENT_OVERHEAD, NORMALIZED_DOCUMENT_OVERHEAD, normalized_xml_budget,
    };
    for body in [
        "",
        "&amp;&amp;&amp;",
        "&lt;&gt;",
        "中文",
        "😀👩‍💻",
        "e\u{301}",
        "&amp;lt;&amp;#55357;",
        "&#38;&#x1F600;",
        "<![CDATA[&&<<>>]]>",
        "ab\n&lt;&amp;<![CDATA[&amp;]]> tail",
    ] {
        let xml = format!("<i><d p=\"0,9,1000,16777215\">{body}</d></i>");
        let parsed = parse_xml(xml.as_bytes()).unwrap();
        let logical_bytes: usize = parsed.comments.iter().map(|c| c.text.len()).sum();
        assert!(logical_bytes <= xml.len(), "{body}");
        let comments: Vec<_> = parsed.comments.iter().map(|c| (c, 900_000_000)).collect();
        let normalized = normalize_xml(&comments, -900_000_000).unwrap();
        assert!(
            normalized.len() as u64
                <= normalized_xml_budget(xml.len() as u64, comments.len() as u64).unwrap()
        );
        assert_eq!(
            parse_xml(normalized.as_bytes())
                .unwrap()
                .comments
                .iter()
                .map(|c| &c.text)
                .collect::<Vec<_>>(),
            parsed.comments.iter().map(|c| &c.text).collect::<Vec<_>>()
        );
    }
    // Reach both maxima, rather than merely checking a loose upper bound.
    let comment = XmlComment {
        elapsed_ms: 0,
        unix_ms: None,
        mode: 9,
        size: 1000,
        color: 16777215,
        text: "&".repeat(200),
    };
    let normalized = normalize_xml(&[(&comment, 900_000_000)], -900_000_000).unwrap();
    assert_eq!(
        normalized.len() as u64,
        normalized_xml_budget(200, 1).unwrap()
    );
    assert_eq!(NORMALIZED_COMMENT_OVERHEAD, 48);
    assert_eq!(NORMALIZED_DOCUMENT_OVERHEAD, 47);

    // Deterministic mixtures of raw text, entity text and CDATA; the oracle
    // for the bound is byte arithmetic and the original logical string.
    let parts = [
        "&amp;",
        "&lt;",
        "&gt;",
        "中文",
        "👩‍💻",
        "e\u{301}",
        "&#38;",
        "<![CDATA[&<>]]>",
        "\n",
    ];
    let mut seed = 0x3813e93bu64;
    for _ in 0..256 {
        let mut body = String::new();
        for _ in 0..64 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            body.push_str(parts[(seed >> 32) as usize % parts.len()]);
        }
        let xml = format!("<i><d p=\"1,1,25,0\">{body}</d></i>");
        let parsed = parse_xml(xml.as_bytes()).unwrap();
        let c = &parsed.comments[0];
        assert!(c.text.len() <= xml.len());
        let normalized = normalize_xml(&[(c, 1000)], 0).unwrap();
        assert!(normalized.len() as u64 <= normalized_xml_budget(xml.len() as u64, 1).unwrap());
        assert_eq!(
            parse_xml(normalized.as_bytes()).unwrap().comments[0].text,
            c.text
        );
    }
}

#[test]
fn normalization_and_source_budgets_reject_overflow_at_real_boundaries() {
    let xml = b"<i><d p=\"1,1,25,0\">a</d><d p=\"2,1,25,0\">b</d></i>";
    assert!(parse_xml_with_limit(xml, xml.len() as u64, 2).is_ok());
    assert!(parse_xml_with_limit(xml, xml.len() as u64 - 1, 2).is_err());
    assert!(parse_xml_with_limit(xml, xml.len() as u64, 1).is_err());
    let parsed = parse_xml(xml).unwrap();
    let comments: Vec<_> = parsed.comments.iter().map(|c| (c, c.elapsed_ms)).collect();
    let normalized = normalize_xml(&comments, 0).unwrap();
    assert_eq!(
        normalize_xml_with_limit(&comments, 0, normalized.len() as u64).unwrap(),
        normalized
    );
    assert!(normalize_xml_with_limit(&comments, 0, normalized.len() as u64 - 1).is_err());
}

#[test]
fn validates_complete_xml_and_accepts_empty_recordings() {
    assert!(
        parse_xml(b"<?xml version=\"1.0\"?><i></i>")
            .unwrap()
            .comments
            .is_empty()
    );
    assert!(parse_xml(b"<i/>").unwrap().comments.is_empty());
    for broken in [
        "",
        "<i>",
        "<i><d p=\"1,1,25,1\">x",
        "<i><d p=\"NaN,1,25,1\">x</d></i>",
        "<i><d p=\"1,1,25,1\">&bad;</d></i>",
        "<i></i><i></i>",
        "<i></i>x",
        "<!DOCTYPE i><i/>",
    ] {
        assert!(parse_xml(broken.as_bytes()).is_err(), "accepted {broken}");
    }
}

#[test]
fn reads_precise_origin_and_estimates_old_origin_by_median() {
    let parsed = parse_xml(b"<i><recording_start_time_ms>100000</recording_start_time_ms><d p=\"1.125,1,25,16777215,101,0,0,0\">A &amp; B</d></i>").unwrap();
    assert_eq!(timing_origin(&parsed), (Some(100000), false));
    assert_eq!(parsed.comments[0].text, "A & B");
    assert_eq!(parsed.comments[0].elapsed_ms, 1125);
    let old = parse_xml(b"<i><d p=\"1,1,25,1,101\">a</d><d p=\"2,1,25,1,102\">b</d><d p=\"100,1,25,1,999\">c</d></i>").unwrap();
    assert_eq!(timing_origin(&old), (Some(100000), true));
    assert!(parse_xml(b"<i recording_start_time_ms=\"1\"><recording_start_time_ms>2</recording_start_time_ms></i>").is_err());
}

#[test]
fn emoji_fallback_marks_only_dialogue_runs_and_keeps_zwj_sequences() {
    let input = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:01.00,Default,,0000,0000,0000,,中文 😡👩‍💻 end,with comma\nComment: 0,0:00:00.00,0:00:01.00,Default,,0000,0000,0000,,😡";
    let output = add_emoji_font_fallback(input);
    assert!(
        output.contains("中文 {\\fnNoto Emoji}😡👩‍💻{\\fnNoto Sans CJK SC} end,with comma"),
        "{output}"
    );
    assert!(output.ends_with("Comment: 0,0:00:00.00,0:00:01.00,Default,,0000,0000,0000,,😡"));
}

#[test]
fn embedded_emoji_font_has_a_valid_sfnt_header() {
    let font = crate::tools::render_emoji_font_bytes();
    assert!(font.len() > 1_000_000);
    assert!(font.starts_with(b"\0\x01\0\0"));
}

fn golden(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../danmaku-factory/tests/golden")
        .join(path)
}

fn assert_matches_golden_ass(actual: &[u8], expected: &[u8]) {
    let text = decode_ass(actual);
    #[cfg(windows)]
    let text = {
        assert!(
            actual.starts_with(b"\xef\xbb\xbf"),
            "Windows ASS needs its BOM"
        );
        assert!(
            !text.replace("\r\n", "").contains('\n'),
            "Windows ASS must use CRLF"
        );
        text.replace("\r\n", "\n")
    };
    assert_eq!(text, decode_ass(expected));
}

/// What the golden corpus calls "biliup": the options the GUI defaults
/// produced on the command line of the former external DanmakuFactory.
fn gui_defaults() -> DanmakuSettings {
    DanmakuSettings {
        enabled: true,
        ..Default::default()
    }
}

fn work_with(input: &str) -> Arc<ManagedRoot> {
    let work = Arc::new(ManagedRoot::new(tempfile::tempdir().unwrap()).unwrap());
    std::fs::copy(golden(input), work.path().join(XML_FILE)).unwrap();
    work
}

fn input(work: &ManagedRoot) -> Vec<u8> {
    std::fs::read(work.path().join(XML_FILE)).unwrap()
}

fn no_attempts(work: &Path) -> bool {
    std::fs::read_dir(work).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".danmaku-attempt-")
    })
}

#[test]
fn gui_settings_map_to_the_former_command_line() {
    let options = factory_options(&gui_defaults(), 1920, 1080);
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(golden("cases.json")).unwrap()).unwrap();
    let golden_options = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "basic_cn-biliup")
        .unwrap()["options"]
        .clone();
    let golden_options: ConvertOptions = serde_json::from_value(golden_options).unwrap();
    assert_eq!(options, golden_options);
    // --opacity used round(opacity * 255), at least 1.
    let mut settings = gui_defaults();
    settings.opacity = 0.001;
    assert_eq!(factory_options(&settings, 2, 2).opacity, 1);
    settings.opacity = 1.;
    settings.font_size = 37.5;
    let options = factory_options(&settings, 2, 2);
    assert_eq!((options.opacity, options.font_size), (255, 38));
}

#[tokio::test]
async fn a_conversion_writes_what_the_cli_wrote() {
    for name in ["basic_cn", "dense"] {
        let work = work_with(&format!("inputs/{name}.xml"));
        let options = factory_options(&gui_defaults(), 1920, 1080);
        let output = convert_in_attempt(&work, input(&work), &options, &CancellationToken::new())
            .await
            .unwrap();
        assert!(output.stats.danmaku_count > 0);
        let expected = std::fs::read(golden(&format!("expected/{name}-biliup.ass"))).unwrap();
        assert_matches_golden_ass(&output.ass, &expected);
        assert!(no_attempts(work.path()));
    }
}

#[tokio::test]
async fn paths_with_spaces_and_chinese_convert_without_leaving_an_attempt() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("录播 目录");
    std::fs::create_dir_all(&parent).unwrap();
    let work = Arc::new(
        ManagedRoot::new(
            tempfile::Builder::new()
                .prefix(".render-段 1")
                .tempdir_in(parent)
                .unwrap(),
        )
        .unwrap(),
    );
    let xml = std::fs::read(golden("inputs/basic_cn.xml")).unwrap();
    let options = factory_options(&gui_defaults(), 1920, 1080);
    let output = convert_in_attempt(&work, xml, &options, &CancellationToken::new())
        .await
        .unwrap();
    let expected = std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap();
    assert_matches_golden_ass(&output.ass, &expected);
    assert!(no_attempts(work.path()));
}

#[tokio::test]
async fn concurrent_conversions_use_separate_attempts() {
    let options = factory_options(&gui_defaults(), 1920, 1080);
    let work = work_with("inputs/dense.xml");
    let xml = input(&work);
    let started = Instant::now();
    let results = futures::future::join_all((0..4).map(|_| {
        let options = options.clone();
        let work = &work;
        let xml = xml.clone();
        async move { convert_in_attempt(work, xml, &options, &CancellationToken::new()).await }
    }))
    .await;
    let expected = std::fs::read(golden("expected/dense-biliup.ass")).unwrap();
    for result in results {
        assert_matches_golden_ass(&result.unwrap().ass, &expected);
    }
    assert!(no_attempts(work.path()));
    assert!(started.elapsed() < Duration::from_secs(60));
}

#[tokio::test]
async fn cancelling_answers_at_once_and_keeps_the_attempt_until_the_core_stops() {
    let owner = tempfile::tempdir().unwrap();
    let work = Arc::new(
        ManagedRoot::new(
            tempfile::Builder::new()
                .prefix(".render-")
                .tempdir_in(owner.path())
                .unwrap(),
        )
        .unwrap(),
    );
    let path = work.path().to_owned();
    // The test thread must not block on the runtime: `started` is awaited, and
    // only the conversion thread itself waits for its release.
    let (started, running) = tokio::sync::oneshot::channel::<()>();
    let (release, blocked) = std::sync::mpsc::channel::<()>();
    let cancel = CancellationToken::new();
    let task = {
        let work = work.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let _held_by_the_core = work;
            supervised(CONVERT_TIMEOUT, &cancel, move || {
                let _ = started.send(());
                let _ = blocked.recv();
                Ok::<(), String>(())
            })
            .await
        })
    };
    running.await.unwrap();
    let answered = Instant::now();
    cancel.cancel();
    let error = task.await.unwrap().unwrap_err();
    assert!(error.contains("取消"), "{error}");
    assert!(answered.elapsed() < Duration::from_secs(5));
    // The caller has returned, but the core still owns the root: nothing may
    // reclaim the directory while it can still write into it.
    assert!(path.exists());
    release.send(()).unwrap();
    drop(work);
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the core's own release must reclaim the root");
}

#[tokio::test]
async fn a_conversion_that_makes_no_progress_is_stopped_by_the_timeout() {
    let cancel = CancellationToken::new();
    let started = Instant::now();
    let error = supervised(Duration::from_millis(50), &cancel, || {
        std::thread::sleep(Duration::from_millis(500));
        Ok::<(), String>(())
    })
    .await
    .unwrap_err();
    assert!(
        error.contains("弹幕转换失败") && error.contains("没有推进"),
        "{error}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn a_conversion_task_that_panics_is_reported() {
    let error = supervised(
        CONVERT_TIMEOUT,
        &CancellationToken::new(),
        || -> Result<(), String> { panic!("conversion panicked") },
    )
    .await
    .unwrap_err();
    assert!(error.contains("弹幕转换任务失败"), "{error}");
}

#[tokio::test]
async fn conversion_errors_are_reported_without_a_fallback() {
    let work = work_with("inputs/basic_cn.xml");
    let mut options = factory_options(&gui_defaults(), 1920, 1080);
    options.width = 0;
    let error = convert_in_attempt(&work, input(&work), &options, &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(error.contains("弹幕转换参数无效"), "{error}");
    std::fs::write(work.path().join(XML_FILE), "<i></i>").unwrap();
    let error = convert_in_attempt(
        &work,
        input(&work),
        &factory_options(&gui_defaults(), 1920, 1080),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error, "弹幕转换没有读到弹幕");
    assert!(!work.path().join(ASS_FILE).exists());
    assert!(no_attempts(work.path()));
}

#[tokio::test]
async fn caller_files_are_preserved_and_attempt_results_are_private() {
    let work = work_with("inputs/basic_cn.xml");
    std::fs::write(work.path().join(ASS_FILE), b"earlier successful attempt").unwrap();
    std::fs::write(work.path().join("earlier.json"), b"earlier task result").unwrap();
    let output = convert_in_attempt(
        &work,
        input(&work),
        &factory_options(&gui_defaults(), 1920, 1080),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(decode_ass(&output.ass).contains("[Events]"));
    assert_eq!(
        std::fs::read(work.path().join(ASS_FILE)).unwrap(),
        b"earlier successful attempt"
    );
    assert_eq!(
        std::fs::read(work.path().join("earlier.json")).unwrap(),
        b"earlier task result"
    );
    let mut invalid = factory_options(&gui_defaults(), 1920, 1080);
    invalid.width = 0;
    convert_in_attempt(&work, input(&work), &invalid, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(
        std::fs::read(work.path().join(ASS_FILE)).unwrap(),
        b"earlier successful attempt"
    );
    assert!(no_attempts(work.path()));
}

#[tokio::test]
async fn cleanup_failure_preserves_residue_and_reports_the_path() {
    let work = work_with("inputs/basic_cn.xml");
    let error = run_attempt(
        &work,
        input(&work),
        &factory_options(&gui_defaults(), 1920, 1080),
        Cleanup::Retain,
    )
    .unwrap_err();
    assert!(
        error.contains("cleanup ") && error.contains("injected attempt cleanup failure"),
        "{error}"
    );
    let attempt = std::fs::read_dir(work.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".danmaku-attempt-")
        })
        .unwrap();
    assert!(attempt.join(ASS_FILE).is_file());
    assert!(error.contains(&attempt.display().to_string()));
    std::fs::remove_dir_all(&attempt).unwrap();
    assert!(no_attempts(work.path()));
    let root_path = work.path().to_owned();
    drop(work);
    assert!(
        root_path.exists(),
        "failed cleanup must retain its managed identity"
    );
    assert!(super::super::jobs::cleanup_inactive_danmaku_root(&root_path).unwrap());
    assert!(
        !root_path.exists(),
        "managed recovery must retry the retained root"
    );
}

#[tokio::test]
async fn the_converter_self_test_passes() {
    check_converter(&CancellationToken::new()).await.unwrap();
}

fn source_with(xml: &str, dir: &Path) -> RenderSource {
    let path = dir.join("source.xml");
    std::fs::write(&path, xml).unwrap();
    RenderSource {
        source_session_id: 0,
        segment_id: 1,
        path: dir.join("source.flv"),
        danmaku_path: Some(path),
        start_ms: 0,
        end_ms: 60_000,
        source_origin_ms: Some(100_000),
        trim_start_ms: 0,
        trim_end_ms: 60_000,
    }
}

#[tokio::test]
async fn generate_keeps_negative_origins_and_drops_impossible_times() {
    let dir = tempfile::tempdir().unwrap();
    // The XML clock starts 2 s before the video: the first comment begins
    // before the first frame and the ASS clock is shifted, not truncated.
    let source = source_with(
        "<i recording_start_time_ms=\"98000\"><d p=\"1,1,25,16777215\">early</d>\
         <d p=\"5,1,25,16777215\">later</d><d p=\"999999999,1,25,16777215\">bogus</d></i>",
        dir.path(),
    );
    let work = Arc::new(ManagedRoot::new(tempfile::tempdir().unwrap()).unwrap());
    let preview = generate(
        &source,
        &gui_defaults(),
        320,
        180,
        &work,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(preview.time_origin_ms, -1000);
    assert_eq!(preview.comment_count, 3);
    assert!(
        preview.ass.contains("0:00:00.00,0:00:12.00,R2L"),
        "{}",
        preview.ass
    );
    assert!(
        preview.ass.contains("0:00:04.00,0:00:16.00,R2L"),
        "{}",
        preview.ass
    );
    assert!(!preview.ass.contains("bogus"));
    // Style values are exact after layout.
    assert!(preview.ass.contains("Style: R2L,Noto Sans CJK SC,38,"));
}

#[test]
fn ass_is_decoded_without_bom_or_broken_characters() {
    assert_eq!(decode_ass(b"\xEF\xBB\xBF[Script Info]"), "[Script Info]");
    // a comment cut by the core in the middle of "长"
    assert_eq!(decode_ass(b"a\xE9\x95\nb"), "a\nb");
    assert_eq!(decode_ass("中文".as_bytes()), "中文");
}
