use super::*;
use crate::danmaku_worker::tests::Hook;
use std::path::PathBuf;
use std::time::Instant;

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

/// What the golden corpus calls "biliup": the options the GUI defaults
/// produced on the command line of the former external DanmakuFactory.
fn gui_defaults() -> DanmakuSettings {
    DanmakuSettings {
        enabled: true,
        ..Default::default()
    }
}

fn request(options: ConvertOptions, test_hook: Option<Hook>) -> Request {
    Request {
        protocol: PROTOCOL,
        input: XML_FILE.into(),
        output: ASS_FILE.into(),
        result: RESULT_FILE.into(),
        options,
        test_hook,
    }
}

fn work_with(input: &str) -> tempfile::TempDir {
    let work = tempfile::tempdir().unwrap();
    std::fs::copy(golden(input), work.path().join(XML_FILE)).unwrap();
    work
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
async fn worker_writes_what_the_cli_wrote() {
    for name in ["basic_cn", "dense", "emoji", "special_chars"] {
        let work = work_with(&format!("inputs/{name}.xml"));
        let options = factory_options(&gui_defaults(), 1920, 1080);
        let stats = convert_in_worker(work.path(), &options, &CancellationToken::new())
            .await
            .unwrap();
        assert!(stats.danmaku_count > 0);
        let actual = std::fs::read(work.path().join(ASS_FILE)).unwrap();
        let expected = std::fs::read(golden(&format!("expected/{name}-biliup.ass"))).unwrap();
        assert_eq!(decode_ass(&actual), decode_ass(&expected), "{name}");
        // The worker leaves exactly its result next to the files it was given.
        assert!(work.path().join(RESULT_FILE).is_file());
        assert!(!work.path().join("danmaku-result.partial").exists());
    }
}

#[tokio::test]
async fn paths_with_spaces_and_chinese_never_reach_the_worker() {
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("录播 目录").join(".render-段 1");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::copy(golden("inputs/basic_cn.xml"), work.join(XML_FILE)).unwrap();
    let options = factory_options(&gui_defaults(), 1920, 1080);
    convert_in_worker(&work, &options, &CancellationToken::new())
        .await
        .unwrap();
    let expected = std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap();
    assert_eq!(
        decode_ass(&std::fs::read(work.join(ASS_FILE)).unwrap()),
        decode_ass(&expected)
    );
}

#[tokio::test]
async fn concurrent_previews_use_separate_workers() {
    let options = factory_options(&gui_defaults(), 1920, 1080);
    let works: Vec<_> = (0..4).map(|_| work_with("inputs/dense.xml")).collect();
    let started = Instant::now();
    let results = futures::future::join_all(works.iter().map(|work| {
        let options = options.clone();
        async move { convert_in_worker(work.path(), &options, &CancellationToken::new()).await }
    }))
    .await;
    for result in results {
        result.unwrap();
    }
    let expected = decode_ass(&std::fs::read(golden("expected/dense-biliup.ass")).unwrap());
    for work in &works {
        assert_eq!(
            decode_ass(&std::fs::read(work.path().join(ASS_FILE)).unwrap()),
            expected
        );
    }
    assert!(started.elapsed() < Duration::from_secs(60));
}

#[tokio::test]
async fn cancelling_kills_the_worker_promptly() {
    let work = work_with("inputs/basic_cn.xml");
    let cancel = CancellationToken::new();
    let cancelling = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancelling.cancel();
    });
    let started = Instant::now();
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::SleepMs(20_000)),
    );
    let error = run_worker(work.path(), &request, &cancel, CONVERT_TIMEOUT)
        .await
        .unwrap_err();
    assert!(error.contains("取消"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!work.path().join(ASS_FILE).exists());
    assert!(!work.path().join(RESULT_FILE).exists());
}

#[tokio::test]
async fn a_hung_worker_is_stopped_by_the_timeout() {
    let work = work_with("inputs/basic_cn.xml");
    let started = Instant::now();
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::SleepMs(20_000)),
    );
    let error = run_worker(
        work.path(),
        &request,
        &CancellationToken::new(),
        Duration::from_millis(500),
    )
    .await
    .unwrap_err();
    assert!(
        error.contains("弹幕转换失败") && error.contains("没有推进"),
        "{error}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!work.path().join(ASS_FILE).exists());
}

#[tokio::test]
async fn dropping_the_request_kills_the_worker() {
    // An HTTP preview whose client went away drops the future mid-conversion.
    let work = work_with("inputs/basic_cn.xml");
    let path = work.path().to_path_buf();
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::SleepMs(1_500)),
    );
    let task = tokio::spawn(async move {
        run_worker(&path, &request, &CancellationToken::new(), CONVERT_TIMEOUT).await
    });
    tokio::time::sleep(Duration::from_millis(400)).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // Had the worker survived it would have converted by now.
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    assert!(!work.path().join(ASS_FILE).exists());
    assert!(!work.path().join(RESULT_FILE).exists());
}

#[tokio::test]
async fn a_crashing_worker_is_reported_and_leaves_no_ass() {
    let work = work_with("inputs/basic_cn.xml");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Abort),
    );
    let error = run_worker(
        work.path(),
        &request,
        &CancellationToken::new(),
        CONVERT_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert!(
        error.contains("弹幕转换失败") && error.contains("aborting"),
        "{error}"
    );
    assert!(!work.path().join(ASS_FILE).exists());
}

#[tokio::test]
async fn conversion_errors_are_reported_without_a_fallback() {
    let work = work_with("inputs/basic_cn.xml");
    let mut options = factory_options(&gui_defaults(), 1920, 1080);
    options.width = 0;
    let error = convert_in_worker(work.path(), &options, &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(error.contains("弹幕转换参数无效"), "{error}");
    std::fs::write(work.path().join(XML_FILE), "<i></i>").unwrap();
    let error = convert_in_worker(
        work.path(),
        &factory_options(&gui_defaults(), 1920, 1080),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error, "弹幕转换没有读到弹幕");
    assert!(!work.path().join(ASS_FILE).exists());
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
    let work = tempfile::tempdir().unwrap();
    let preview = generate(
        &source,
        &gui_defaults(),
        320,
        180,
        work.path(),
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
