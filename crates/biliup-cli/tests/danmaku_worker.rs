//! The real `biliup` binary as the danmaku worker: `main` must recognise the
//! worker flag before anything else, convert with the linked core, and the
//! server side must be able to drive it through the launcher.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use biliup_cli::danmaku_worker::{
    self, EXIT_BAD_REQUEST, EXIT_CONVERTED, EXIT_FAILED, Launcher, PROTOCOL, WORKER_FLAG,
};
use biliup_cli::server::workbench::renders::engine;
use serde_json::{Value, json};

#[path = "../../danmaku-factory/tests/support/mod.rs"]
mod compatibility;

const BILIUP: &str = env!("CARGO_BIN_EXE_biliup");

fn golden(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../danmaku-factory/tests/golden")
        .join(path)
}

fn read_ass(path: impl AsRef<Path>) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    if !cfg!(windows) {
        return bytes;
    }
    let body = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap();
    body.iter()
        .enumerate()
        .filter_map(|(at, &b)| {
            if b == b'\r' && body.get(at + 1) == Some(&b'\n') {
                None
            } else {
                Some(b)
            }
        })
        .collect()
}

fn gui_options() -> Value {
    json!({
        "width": 1920, "height": 1080, "font_name": "Noto Sans CJK SC", "font_size": 38,
        "font_size_strict": true, "opacity": 204, "outline": 2.0, "shadow": 0.0,
        "scroll_time": 12.0, "fix_time": 12.0, "display_area": 0.4, "density": -1
    })
}

fn run_worker(work: &Path, request: Value) -> (i32, Option<Value>, String) {
    let result_path = work.join(request["result"].as_str().unwrap());
    std::fs::write(work.join("request.json"), request.to_string()).unwrap();
    let output = Command::new(BILIUP)
        .args([WORKER_FLAG, "request.json"])
        .current_dir(work)
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    let result = std::fs::read(result_path)
        .ok()
        .map(|bytes| serde_json::from_slice(&bytes).unwrap());
    (
        output.status.code().unwrap_or(-1),
        result,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn the_binary_converts_like_the_cli_and_starts_nothing_else() {
    let work = tempfile::tempdir().unwrap();
    std::fs::copy(golden("inputs/emoji.xml"), work.path().join("in.xml")).unwrap();
    let started = Instant::now();
    let (code, result, stderr) = run_worker(
        work.path(),
        json!({"protocol": PROTOCOL, "input": "in.xml", "output": "out.ass", "result": "result.json",
               "options": gui_options()}),
    );
    assert_eq!(code, EXIT_CONVERTED, "{stderr}");
    let result = result.unwrap();
    assert_eq!(result["outcome"], "converted");
    assert_eq!(result["stats"]["danmaku_count"], 9);
    assert!(result["version"].as_str().unwrap().contains("3813e93b"));
    assert_eq!(
        read_ass(work.path().join("out.ass")),
        compatibility::expected(
            "emoji.xml",
            std::fs::read(golden("expected/emoji-biliup.ass")).unwrap()
        )
    );
    // Worker mode returns before logging, configuration or the server.
    let mut left: Vec<_> = std::fs::read_dir(work.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, ["in.xml", "out.ass", "request.json", "result.json"]);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn the_real_worker_publishes_ass_and_response_in_a_long_unicode_directory() {
    let work = tempfile::tempdir().unwrap();
    let mut nested = work.path().to_path_buf();
    while nested.as_os_str().len() < 320 {
        nested.push("弹幕 0123456789abcdef");
    }
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::copy(golden("inputs/basic_cn.xml"), nested.join("in.xml")).unwrap();
    // Windows cannot create a process with a cwd beyond MAX_PATH. Keep the
    // worker directory short while all conversion/result paths stay long.
    let (code, result, stderr) = run_worker(
        work.path(),
        json!({"protocol":PROTOCOL,"input":nested.join("in.xml"),"output":nested.join("out.ass"),
               "result":nested.join("result.json"),"options":gui_options()}),
    );
    assert_eq!(code, EXIT_CONVERTED, "{stderr}");
    assert_eq!(result.unwrap()["outcome"], "converted");
    assert_eq!(
        read_ass(nested.join("out.ass")),
        std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap()
    );
    assert_eq!(std::fs::read_dir(&nested).unwrap().count(), 3);
    std::fs::rename(nested.join("out.ass"), nested.join("已释放.ass")).unwrap();
    std::fs::remove_dir_all(&nested).unwrap();
}

#[test]
fn failures_are_reported_in_the_result_and_exit_code() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("in.xml"), "<i></i>").unwrap();
    let (code, result, _) = run_worker(
        work.path(),
        json!({"protocol": PROTOCOL, "input": "in.xml", "output": "out.ass", "result": "result.json",
               "options": gui_options()}),
    );
    assert_eq!(code, EXIT_FAILED);
    let result = result.unwrap();
    assert_eq!(result["outcome"], "failed");
    assert_eq!(result["error"]["kind"], "no_danmaku");
    assert!(!work.path().join("out.ass").exists());

    // A request from another build is refused before converting anything.
    let (code, result, stderr) = run_worker(
        work.path(),
        json!({"protocol": PROTOCOL + 1, "input": "in.xml", "output": "out.ass", "result": "other.json",
               "options": {}}),
    );
    assert_eq!(code, EXIT_BAD_REQUEST, "{stderr}");
    assert!(result.is_none());
    let earlier: Value =
        serde_json::from_slice(&std::fs::read(work.path().join("result.json")).unwrap()).unwrap();
    assert_eq!(earlier["outcome"], "failed");
    assert_eq!(earlier["error"]["kind"], "no_danmaku");
    assert!(!work.path().join("other.json").exists());
    let output = Command::new(BILIUP)
        .arg(WORKER_FLAG)
        .current_dir(work.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(danmaku_worker::EXIT_USAGE));
}

#[test]
fn the_real_worker_preserves_advanced_motion_fade_rotation_and_blocking() {
    let work = tempfile::tempdir().unwrap();
    std::fs::copy(golden("inputs/types.xml"), work.path().join("in.xml")).unwrap();
    let (code, result, stderr) = run_worker(
        work.path(),
        json!({"protocol":PROTOCOL,"input":"in.xml","output":"out.ass","result":"result.json","options":gui_options()}),
    );
    assert_eq!(code, EXIT_CONVERTED, "{stderr}");
    let result = result.unwrap();
    assert_eq!(result["stats"]["danmaku_count"], 12);
    assert_eq!(result["stats"]["blocked_count"], 1);
    let ass = read_ass(work.path().join("out.ass"));
    assert_eq!(
        ass,
        std::fs::read(golden("expected/types-biliup.ass")).unwrap()
    );
    let text = std::str::from_utf8(&ass).unwrap();
    assert!(text.contains("\\move(192,216,1536,972,200,1700)\\c&H0000FF\\fry30\\fade(0,0,127,0,0,200,4500)\\fn微软雅黑"));
    assert!(text.contains("\\pos(100,200)\\fs36\\fade(50,50,50,0,0,0,3000)"));
}

#[test]
fn real_workers_race_for_one_complete_output_without_deleting_the_winner() {
    let work = tempfile::tempdir().unwrap();
    std::fs::copy(golden("inputs/dense.xml"), work.path().join("in.xml")).unwrap();
    let mut children = Vec::new();
    for index in 0..8 {
        let request = format!("request-{index}.json");
        std::fs::write(work.path().join(&request),json!({"protocol":PROTOCOL,"input":"in.xml","output":"out.ass","result":format!("result-{index}.json"),"options":gui_options()}).to_string()).unwrap();
        children.push(
            Command::new(BILIUP)
                .args([WORKER_FLAG, &request])
                .current_dir(work.path())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    let statuses: Vec<_> = children
        .iter_mut()
        .map(|child| child.wait().unwrap())
        .collect();
    assert_eq!(statuses.iter().filter(|status| status.success()).count(), 1);
    for (index, status) in statuses.iter().enumerate() {
        let response: Value = serde_json::from_slice(
            &std::fs::read(work.path().join(format!("result-{index}.json"))).unwrap(),
        )
        .unwrap();
        if status.success() {
            assert_eq!(response["outcome"], "converted");
        } else {
            assert_eq!(status.code(), Some(EXIT_FAILED));
            assert_eq!(response["error"]["kind"], "output_exists");
        }
    }
    assert_eq!(
        read_ass(work.path().join("out.ass")),
        std::fs::read(golden("expected/dense-biliup.ass")).unwrap()
    );
    assert!(!std::fs::read_dir(work.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".biliup-danmaku-")
    }));
    std::fs::rename(work.path().join("out.ass"), work.path().join("renamed.ass")).unwrap();
}

#[test]
fn response_publication_failure_cannot_replace_an_existing_result() {
    let work = tempfile::tempdir().unwrap();
    std::fs::copy(golden("inputs/basic_cn.xml"), work.path().join("in.xml")).unwrap();
    std::fs::write(work.path().join("result.json"), b"keep earlier task result").unwrap();
    std::fs::write(work.path().join("request.json"),json!({"protocol":PROTOCOL,"input":"in.xml","output":"out.ass","result":"result.json","options":gui_options()}).to_string()).unwrap();
    let status = Command::new(BILIUP)
        .args([WORKER_FLAG, "request.json"])
        .current_dir(work.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(danmaku_worker::EXIT_RESULT_UNWRITABLE));
    assert_eq!(
        std::fs::read(work.path().join("result.json")).unwrap(),
        b"keep earlier task result"
    );
    assert_eq!(
        read_ass(work.path().join("out.ass")),
        std::fs::read(golden("expected/basic_cn-biliup.ass")).unwrap()
    );
}

#[tokio::test]
async fn the_server_drives_the_binary_through_the_launcher() {
    danmaku_worker::set_launcher(Launcher::Program {
        program: BILIUP.into(),
        args: Vec::new(),
    });
    let status = engine::capabilities().await;
    assert!(status.danmaku_factory, "{:?}", status.error);
    // The other checks depend on the machine's ffmpeg and fonts.
    if let Some(error) = &status.error {
        assert!(!error.contains("弹幕转换"), "{error}");
    }

    danmaku_worker::set_launcher(Launcher::Unavailable("no worker here".into()));
    let status = engine::capabilities().await;
    assert!(!status.danmaku_factory);
    assert!(status.error.unwrap().contains("no worker here"));
}
