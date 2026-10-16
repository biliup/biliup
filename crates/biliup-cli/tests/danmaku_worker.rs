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

const BILIUP: &str = env!("CARGO_BIN_EXE_biliup");

fn golden(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../danmaku-factory/tests/golden")
        .join(path)
}

fn gui_options() -> Value {
    json!({
        "width": 1920, "height": 1080, "font_name": "Noto Sans CJK SC", "font_size": 38,
        "font_size_strict": true, "opacity": 204, "outline": 2.0, "shadow": 0.0,
        "scroll_time": 12.0, "fix_time": 12.0, "display_area": 0.4, "density": -1
    })
}

fn run_worker(work: &Path, request: Value) -> (i32, Option<Value>, String) {
    std::fs::write(work.join("request.json"), request.to_string()).unwrap();
    let output = Command::new(BILIUP)
        .args([WORKER_FLAG, "request.json"])
        .current_dir(work)
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    let result = std::fs::read(work.join("result.json"))
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
        std::fs::read(work.path().join("out.ass")).unwrap(),
        std::fs::read(golden("expected/emoji-biliup.ass")).unwrap()
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
    assert!(result.is_some_and(|r| r["outcome"] == "failed")); // the earlier result
    assert!(!work.path().join("other.json").exists());
    let output = Command::new(BILIUP)
        .arg(WORKER_FLAG)
        .current_dir(work.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(danmaku_worker::EXIT_USAGE));
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
