use super::*;
use crate::danmaku_worker::tests::Hook;
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

fn work_with(input: &str) -> Arc<ManagedRoot> {
    let work = Arc::new(ManagedRoot::new(tempfile::tempdir().unwrap()).unwrap());
    std::fs::copy(golden(input), work.path().join(XML_FILE)).unwrap();
    work
}

fn input(work: &ManagedRoot) -> Vec<u8> {
    std::fs::read(work.path().join(XML_FILE)).unwrap()
}

async fn test_worker(
    work: &Arc<ManagedRoot>,
    request: &Request,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<WorkerOutput, String> {
    run_worker(work, input(work), request, cancel, timeout).await
}

async fn ready(path: &Path) -> (u32, PathBuf) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(bytes) = tokio::fs::read(path).await
                && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
            {
                return (
                    value["pid"].as_u64().unwrap() as u32,
                    PathBuf::from(value["work"].as_str().unwrap()),
                );
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("worker readiness marker")
}

fn process_exists(pid: u32) -> bool {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).is_some()
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
async fn worker_writes_what_the_cli_wrote() {
    for name in ["basic_cn", "dense"] {
        let work = work_with(&format!("inputs/{name}.xml"));
        let options = factory_options(&gui_defaults(), 1920, 1080);
        let output = convert_in_worker(&work, input(&work), &options, &CancellationToken::new())
            .await
            .unwrap();
        assert!(output.stats.danmaku_count > 0);
        let expected = std::fs::read(golden(&format!("expected/{name}-biliup.ass"))).unwrap();
        assert_matches_golden_ass(&output.ass, &expected);
        assert!(no_attempts(work.path()));
    }
}

#[tokio::test]
async fn paths_with_spaces_and_chinese_never_reach_the_worker() {
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
    let output = convert_in_worker(&work, xml, &options, &CancellationToken::new())
        .await
        .unwrap();
    let expected = std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap();
    assert_matches_golden_ass(&output.ass, &expected);
}

#[tokio::test]
async fn concurrent_previews_use_separate_workers() {
    let options = factory_options(&gui_defaults(), 1920, 1080);
    let work = work_with("inputs/dense.xml");
    let xml = input(&work);
    let started = Instant::now();
    let results = futures::future::join_all((0..4).map(|_| {
        let options = options.clone();
        let work = &work;
        let xml = xml.clone();
        async move { convert_in_worker(work, xml, &options, &CancellationToken::new()).await }
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
async fn cancelling_kills_the_worker_promptly() {
    let work = work_with("inputs/basic_cn.xml");
    let cancel = CancellationToken::new();
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("ready");
    let started = Instant::now();
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: false,
        }),
    );
    let result = test_worker(&work, &request, &cancel, CONVERT_TIMEOUT);
    let (error, (pid, attempt)) = tokio::join!(async { result.await.unwrap_err() }, async {
        let ready = ready(&marker).await;
        cancel.cancel();
        ready
    });
    assert!(error.contains("取消"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!work.path().join(ASS_FILE).exists());
    assert!(!work.path().join(RESULT_FILE).exists());
    assert!(!attempt.exists());
    assert!(
        !process_exists(pid),
        "worker must be reaped before completion"
    );
}

#[tokio::test]
async fn a_hung_worker_is_stopped_by_the_timeout() {
    let work = work_with("inputs/basic_cn.xml");
    let started = Instant::now();
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("ready");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: false,
        }),
    );
    let cancel = CancellationToken::new();
    let (result, (pid, attempt)) = tokio::join!(
        test_worker(&work, &request, &cancel, Duration::from_secs(2),),
        ready(&marker)
    );
    let error = result.unwrap_err();
    assert!(
        error.contains("弹幕转换失败") && error.contains("没有推进"),
        "{error}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!work.path().join(ASS_FILE).exists());
    assert!(!attempt.exists());
    assert!(!process_exists(pid));
}

#[tokio::test]
async fn dropping_the_request_kills_the_worker() {
    // An HTTP preview whose client went away drops the future mid-conversion.
    let work = work_with("inputs/basic_cn.xml");
    let path = work.path().to_path_buf();
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("ready");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: true,
        }),
    );
    let task = tokio::spawn(async move {
        test_worker(&work, &request, &CancellationToken::new(), CONVERT_TIMEOUT).await
    });
    let (pid, attempt) = ready(&marker).await;
    assert!(attempt.join(ASS_FILE).is_file());
    assert!(!attempt.join(RESULT_FILE).exists());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() || process_exists(pid) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("detached supervisor must reap then remove the final root owner");
}

#[tokio::test]
async fn a_crashing_worker_is_reported_and_leaves_no_ass() {
    let work = work_with("inputs/basic_cn.xml");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Abort),
    );
    let error = test_worker(&work, &request, &CancellationToken::new(), CONVERT_TIMEOUT)
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
    let error = convert_in_worker(&work, input(&work), &options, &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(error.contains("弹幕转换参数无效"), "{error}");
    std::fs::write(work.path().join(XML_FILE), "<i></i>").unwrap();
    let error = convert_in_worker(
        &work,
        input(&work),
        &factory_options(&gui_defaults(), 1920, 1080),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error, "弹幕转换没有读到弹幕");
    assert!(!work.path().join(ASS_FILE).exists());
}

#[tokio::test]
async fn caller_files_are_preserved_and_attempt_results_are_private() {
    let work = work_with("inputs/basic_cn.xml");
    for name in [ASS_FILE, RESULT_FILE, REQUEST_FILE] {
        std::fs::write(work.path().join(name), b"earlier successful attempt").unwrap();
    }
    let output = convert_in_worker(
        &work,
        input(&work),
        &factory_options(&gui_defaults(), 1920, 1080),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(decode_ass(&output.ass).contains("[Events]"));
    for name in [ASS_FILE, RESULT_FILE, REQUEST_FILE] {
        assert_eq!(
            std::fs::read(work.path().join(name)).unwrap(),
            b"earlier successful attempt"
        );
    }
    let mut invalid = factory_options(&gui_defaults(), 1920, 1080);
    invalid.width = 0;
    convert_in_worker(&work, input(&work), &invalid, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(
        std::fs::read(work.path().join(ASS_FILE)).unwrap(),
        b"earlier successful attempt"
    );
    assert!(no_attempts(work.path()));
}

#[tokio::test]
async fn published_ass_without_response_is_cancelled_and_reclaimed() {
    let work = work_with("inputs/basic_cn.xml");
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("published");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: true,
        }),
    );
    let cancel = CancellationToken::new();
    let (result, (pid, attempt)) = tokio::join!(
        test_worker(&work, &request, &cancel, CONVERT_TIMEOUT),
        async {
            let observed = ready(&marker).await;
            assert!(observed.1.join(ASS_FILE).is_file());
            assert!(!observed.1.join(RESULT_FILE).exists());
            cancel.cancel();
            observed
        }
    );
    assert!(result.unwrap_err().contains("取消"));
    assert!(!process_exists(pid));
    assert!(!attempt.exists());
}

#[tokio::test]
async fn externally_killed_worker_cannot_commit_its_published_ass() {
    let work = work_with("inputs/basic_cn.xml");
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("published");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: true,
        }),
    );
    let cancel = CancellationToken::new();
    let (result, (pid, attempt)) = tokio::join!(
        test_worker(&work, &request, &cancel, CONVERT_TIMEOUT),
        async {
            let observed = ready(&marker).await;
            assert!(observed.1.join(ASS_FILE).is_file());
            let mut system = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(observed.0);
            system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            assert!(system.process(pid).unwrap().kill());
            observed
        }
    );
    assert!(result.unwrap_err().contains("处理程序失败"));
    assert!(!cancel.is_cancelled());
    assert!(
        !process_exists(pid),
        "the supervisor must reap the externally killed child"
    );
    assert!(!attempt.exists());
}

#[tokio::test]
async fn converted_response_requires_successful_exit_protocol_and_readable_ass() {
    for (hook, message) in [
        (Hook::MissingAss, "可读取"),
        (Hook::WrongProtocol, "版本不一致"),
        (Hook::MalformedResponse, "无法识别"),
        (Hook::FailAfterResponse, "处理程序失败"),
        (Hook::ResultUnwritable, "处理程序失败"),
    ] {
        let work = work_with("inputs/basic_cn.xml");
        let request = request(factory_options(&gui_defaults(), 1920, 1080), Some(hook));
        let error = test_worker(&work, &request, &CancellationToken::new(), CONVERT_TIMEOUT)
            .await
            .unwrap_err();
        assert!(error.contains(message), "{error}");
        assert!(no_attempts(work.path()));
    }
}

#[tokio::test]
async fn cleanup_failure_preserves_residue_and_reports_the_path() {
    let work = work_with("inputs/basic_cn.xml");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::CleanupFailure),
    );
    let error = test_worker(&work, &request, &CancellationToken::new(), CONVERT_TIMEOUT)
        .await
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
async fn worker_holds_the_root_lease_after_the_parent_releases_its_lease() {
    use crate::danmaku_worker::{ROOT_LEASE_FILE, acquire_root_lease, mark_attempt};
    let root = tempfile::tempdir().unwrap();
    let lease = acquire_root_lease(root.path()).unwrap();
    let attempt = tempfile::Builder::new()
        .prefix(".danmaku-attempt-")
        .tempdir_in(root.path())
        .unwrap();
    mark_attempt(root.path(), attempt.path()).unwrap();
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("worker");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: false,
        }),
    );
    std::fs::write(
        attempt.path().join(REQUEST_FILE),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let mut command =
        crate::danmaku_worker::command(attempt.path(), Path::new(REQUEST_FILE)).unwrap();
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let (pid, _) = ready(&marker).await;
    drop(lease);
    let recovery = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join(ROOT_LEASE_FILE))
        .unwrap();
    assert!(matches!(
        recovery.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    assert!(!process_exists(pid));
    recovery.try_lock().unwrap();
    drop(recovery);
}

#[tokio::test]
async fn killing_the_parent_leaves_a_leased_worker_that_recovery_must_skip() {
    use crate::danmaku_worker::{ROOT_LEASE_FILE, acquire_root_lease, mark_attempt};
    let root = tempfile::tempdir().unwrap();
    drop(acquire_root_lease(root.path()).unwrap());
    let attempt = tempfile::Builder::new()
        .prefix(".danmaku-attempt-")
        .tempdir_in(root.path())
        .unwrap();
    mark_attempt(root.path(), attempt.path()).unwrap();
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("worker");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::Pause {
            ready: marker.clone(),
            after_publish: false,
        }),
    );
    std::fs::write(
        attempt.path().join(REQUEST_FILE),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let mut command =
        crate::danmaku_worker::tests::parent_proxy_command(root.path(), attempt.path());
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut parent = command.spawn().unwrap();
    let (pid, _) = ready(&marker).await;
    parent.kill().await.unwrap();
    parent.wait().await.unwrap();
    assert!(
        process_exists(pid),
        "worker must survive the killed parent for this recovery case"
    );
    let recovery = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join(ROOT_LEASE_FILE))
        .unwrap();
    assert!(matches!(
        recovery.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    let mut system = sysinfo::System::new();
    let child_pid = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[child_pid]), true);
    assert!(system.process(child_pid).unwrap().kill());
    // The OS reparents the orphan; it is no longer this test's child to wait.
    // Successful exclusive locking proves exit released all worker leases.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match recovery.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
                Err(error) => panic!("recovery lease: {error}"),
            }
        }
    })
    .await
    .expect("orphan worker must release its lease on exit");
    drop(recovery);
}

#[tokio::test]
async fn cancellation_at_the_success_acceptance_checkpoint_is_not_success() {
    let work = work_with("inputs/basic_cn.xml");
    let signals = tempfile::tempdir().unwrap();
    let marker = signals.path().join("accept");
    let request = request(
        factory_options(&gui_defaults(), 1920, 1080),
        Some(Hook::PauseBeforeAccept {
            ready: marker.clone(),
        }),
    );
    let cancel = CancellationToken::new();
    let (result, ()) = tokio::join!(
        test_worker(&work, &request, &cancel, CONVERT_TIMEOUT),
        async {
            let (_, attempt) = ready(&marker).await;
            assert!(attempt.join(ASS_FILE).is_file());
            assert!(attempt.join(RESULT_FILE).is_file());
            cancel.cancel();
        }
    );
    assert!(result.unwrap_err().contains("取消"));
    assert!(no_attempts(work.path()));
}

#[tokio::test]
async fn a_delayed_worker_rejects_a_root_already_renamed_for_recovery() {
    use crate::danmaku_worker::{ROOT_RETIRED_FILE, acquire_root_lease, mark_attempt};
    let owner = tempfile::tempdir().unwrap();
    let root = tempfile::Builder::new()
        .prefix(".render-")
        .tempdir_in(owner.path())
        .unwrap();
    drop(acquire_root_lease(root.path()).unwrap());
    let attempt = root.path().join(".danmaku-attempt-delayed");
    std::fs::create_dir(&attempt).unwrap();
    mark_attempt(root.path(), &attempt).unwrap();
    let request = request(factory_options(&gui_defaults(), 1920, 1080), None);
    std::fs::write(
        attempt.join(REQUEST_FILE),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    std::fs::write(attempt.join(XML_FILE), SELF_TEST_XML).unwrap();
    // Recovery must also exclude late workers after closing the internal
    // lease and before renaming the directory (required by Windows).
    let lease = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join(crate::danmaku_worker::ROOT_LEASE_FILE))
        .unwrap();
    lease.try_lock().unwrap();
    std::fs::File::create_new(root.path().join(ROOT_RETIRED_FILE))
        .unwrap()
        .sync_all()
        .unwrap();
    drop(lease);
    assert!(acquire_root_lease(root.path()).is_err());
    let mut command = crate::danmaku_worker::command(&attempt, Path::new(REQUEST_FILE)).unwrap();
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let output = command.output().await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(crate::danmaku_worker::EXIT_BAD_REQUEST)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("retired for recovery"));
    assert!(!attempt.join(ASS_FILE).exists());
    assert!(!attempt.join(RESULT_FILE).exists());
    // Keep the existing identity check independent of retirement.
    std::fs::remove_file(root.path().join(ROOT_RETIRED_FILE)).unwrap();
    let moved = owner.path().join("recovery-tombstone");
    std::fs::rename(root.path(), &moved).unwrap();
    let moved_attempt = moved.join(".danmaku-attempt-delayed");
    let mut command =
        crate::danmaku_worker::command(&moved_attempt, Path::new(REQUEST_FILE)).unwrap();
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let output = command.output().await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(crate::danmaku_worker::EXIT_BAD_REQUEST)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid managed directory"));
    assert!(!moved_attempt.join(ASS_FILE).exists());
    assert!(!moved_attempt.join(RESULT_FILE).exists());
    // Recursive recovery may already have removed the marker while another
    // unremovable file keeps this directory alive. An explicit managed launch
    // must reject that state instead of falling back to a standalone worker.
    std::fs::remove_file(moved_attempt.join(crate::danmaku_worker::ATTEMPT_LEASE_FILE)).unwrap();
    let mut command =
        crate::danmaku_worker::command(&moved_attempt, Path::new(REQUEST_FILE)).unwrap();
    command
        .env(crate::danmaku_worker::MANAGED_WORKER_ENV, "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let output = command.output().await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(crate::danmaku_worker::EXIT_BAD_REQUEST)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("marker is missing"));
    assert!(!moved_attempt.join(ASS_FILE).exists());
    assert!(!moved_attempt.join(RESULT_FILE).exists());
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
