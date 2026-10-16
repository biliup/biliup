//! Exercise Python's real extension loader and the production worker launcher.
use biliup_cli::danmaku_worker::{Launcher, PROTOCOL, WORKER_FLAG};
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;

struct Work(PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn python_loads_the_parent_extension_and_converts_before_starting_the_server() {
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let built = deps.join(format!(
        "{}stream_gears{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(
        built.is_file(),
        "cargo must build the extension before this integration test: {}",
        built.display()
    );
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let work_path =
        std::env::temp_dir().join(format!("biliup-python-弹幕 {}-{stamp}", std::process::id()));
    std::fs::create_dir(&work_path).unwrap();
    let work = Work(work_path);
    let module = if cfg!(target_os = "linux") {
        built
    } else {
        let target = work.0.join(if cfg!(windows) {
            "stream_gears.pyd"
        } else {
            "stream_gears.so"
        });
        std::fs::copy(built, &target).unwrap();
        target
    };
    let python = std::env::var_os("PYO3_PYTHON").unwrap_or_else(|| {
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    });
    let resolved = Command::new(python)
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .expect("Python is required for the extension worker integration test");
    assert!(resolved.status.success());
    let python = PathBuf::from(String::from_utf8(resolved.stdout).unwrap().trim());
    let Launcher::Program { program, args } = Launcher::python(Some(python), Some(module)) else {
        panic!("real extension launcher is unavailable")
    };
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../danmaku-factory/tests/golden");
    std::fs::copy(golden.join("inputs/basic_cn.xml"), work.0.join("in.xml")).unwrap();
    let cases: serde_json::Value =
        serde_json::from_slice(&std::fs::read(golden.join("cases.json")).unwrap()).unwrap();
    let options = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "basic_cn-biliup")
        .unwrap()["options"]
        .clone();
    std::fs::write(work.0.join("request.json"),json!({"protocol":PROTOCOL,"input":"in.xml","output":"out.ass","result":"result.json","options":options}).to_string()).unwrap();
    let mut child = Command::new(program)
        .args(args)
        .args([WORKER_FLAG, "request.json"])
        .current_dir(&work.0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > std::time::Duration::from_secs(30) {
            let _ = child.kill();
            child.wait().unwrap();
            panic!("Python worker did not exit before the integration-test deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value =
        serde_json::from_slice(&std::fs::read(work.0.join("result.json")).unwrap()).unwrap();
    assert_eq!(response["outcome"], "converted");
    assert_eq!(response["protocol"], PROTOCOL);
    assert_eq!(response["stats"]["danmaku_count"], 40);
    assert!(response["version"].as_str().unwrap().contains("3813e93b"));
    let bytes = std::fs::read(work.0.join("out.ass")).unwrap();
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let bytes: Vec<_> = bytes
        .iter()
        .enumerate()
        .filter_map(|(at, &b)| {
            if cfg!(windows) && b == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
                None
            } else {
                Some(b)
            }
        })
        .collect();
    assert_eq!(
        bytes,
        std::fs::read(golden.join("expected/basic_cn-biliup.ass")).unwrap()
    );
    assert!(
        !work.0.join("data").exists(),
        "worker mode must precede server initialization"
    );
    std::fs::rename(work.0.join("out.ass"), work.0.join("已释放.ass")).unwrap();
}
