//! `GET /v1/tools` follows where ffmpeg comes from: the `ffmpeg_path` setting,
//! then the host-provided one, then `PATH`. Kept in its own test binary because
//! `work_dir` switches the working directory of the whole process.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use biliup_cli::entry::{ServeOptions, init_tracing_with, serve};
use serde_json::Value;

fn fake_ffmpeg(dir: &Path, version: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("ffmpeg");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf 'ffmpeg version {version}\\nconfiguration: --enable-gpl --enable-version3\\n'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn get_json(client: &reqwest::Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    assert!(
        response.status().is_success(),
        "GET {url}: {}",
        response.status()
    );
    serde_json::from_slice(&response.bytes().await.unwrap()).unwrap()
}

async fn set_ffmpeg_path(client: &reqwest::Client, base: &str, path: Value) {
    let mut config = get_json(client, &format!("{base}/v1/configuration")).await;
    config["ffmpeg_path"] = path;
    let response = client
        .put(format!("{base}/v1/configuration"))
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&config).unwrap())
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "PUT /v1/configuration: {}",
        response.status()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn tools_report_follows_setting_then_host_ffmpeg() {
    let work_dir = tempfile::tempdir().unwrap();
    let bundled = fake_ffmpeg(&work_dir.path().join("bundled"), "bundled-test");
    let configured = fake_ffmpeg(&work_dir.path().join("configured"), "configured-test");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let logging = init_tracing_with("info", false);

    let server = tokio::spawn(serve(ServeOptions {
        bind: "127.0.0.1".into(),
        port: 1,
        auth: false,
        secure_session_cookie: false,
        config: None,
        user_cookie: "cookies.json".into(),
        work_dir: Some(work_dir.path().to_path_buf()),
        log_handle: logging.handle(),
        listener: Some(listener),
        shutdown: Some(Box::pin(async move {
            let _ = stopped.await;
        })),
        ffmpeg: Some(bundled.clone()),
        fleet: Default::default(),
    }));

    let client = reqwest::Client::new();
    let mut ready = false;
    for _ in 0..100 {
        if client.get(format!("{base}/")).send().await.is_ok() {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ready, "server never answered");
    let tools = format!("{base}/v1/tools");

    let ffmpeg = get_json(&client, &tools).await["ffmpeg"].clone();
    assert_eq!(ffmpeg["available"], true, "{ffmpeg}");
    assert_eq!(ffmpeg["source"], "bundled");
    assert_eq!(ffmpeg["path"], bundled.to_str().unwrap());
    assert_eq!(ffmpeg["version"], "ffmpeg version bundled-test");
    assert_eq!(ffmpeg["license"], "GPL-3.0-or-later");

    set_ffmpeg_path(&client, &base, configured.to_str().unwrap().into()).await;
    let ffmpeg = get_json(&client, &tools).await["ffmpeg"].clone();
    assert_eq!(ffmpeg["source"], "config", "{ffmpeg}");
    assert_eq!(ffmpeg["path"], configured.to_str().unwrap());
    assert_eq!(ffmpeg["version"], "ffmpeg version configured-test");

    let missing = work_dir.path().join("missing/ffmpeg");
    set_ffmpeg_path(&client, &base, missing.to_str().unwrap().into()).await;
    let ffmpeg = get_json(&client, &tools).await["ffmpeg"].clone();
    assert_eq!(ffmpeg["available"], false, "{ffmpeg}");
    assert_eq!(ffmpeg["source"], "config");
    assert!(
        ffmpeg["error"].as_str().unwrap().contains("找不到"),
        "{ffmpeg}"
    );

    set_ffmpeg_path(&client, &base, "  ".into()).await;
    let ffmpeg = get_json(&client, &tools).await["ffmpeg"].clone();
    assert_eq!(ffmpeg["source"], "bundled", "{ffmpeg}");
    assert_eq!(ffmpeg["version"], "ffmpeg version bundled-test");

    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("server did not stop")
        .unwrap()
        .unwrap();
}
