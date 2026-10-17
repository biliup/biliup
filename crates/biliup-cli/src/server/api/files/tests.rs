use super::*;
use axum::body::to_bytes;
use sqlx::sqlite::SqlitePoolOptions;

async fn pool() -> ConnectionPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE render_jobs(id INTEGER PRIMARY KEY,state TEXT,output_path TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

fn recording(name: &str) -> FileRequest {
    FileRequest {
        kind: "recording".into(),
        file_name: Some(name.into()),
        id: None,
    }
}
fn render(id: i64) -> FileRequest {
    FileRequest {
        kind: "render".into(),
        file_name: None,
        id: Some(id),
    }
}

#[test]
fn gui_requires_loopback_peer_loopback_host_and_no_forwarding() {
    let mut headers = HeaderMap::new();
    let local: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    for host in [
        "localhost:19159",
        "LOCALHOST:19159",
        "127.0.0.1:19159",
        "[::1]:19159",
    ] {
        headers.insert(header::HOST, host.parse().unwrap());
        assert!(direct_local(Some(local), &headers), "{host}");
    }
    assert!(!direct_local(None, &headers));
    assert!(direct_local(
        Some("[::ffff:127.0.0.1]:40000".parse().unwrap()),
        &headers
    ));
    assert!(!direct_local(
        Some("192.168.1.9:40000".parse().unwrap()),
        &headers
    ));
    for host in [
        "example.com:19159",
        "localhost.example.com",
        "127.0.0.1.example.com",
        "127.0.0.2.evil",
        "evil@localhost",
    ] {
        headers.insert(header::HOST, host.parse().unwrap());
        assert!(!direct_local(Some(local), &headers), "{host}");
    }
    headers.insert(header::HOST, "localhost:19159".parse().unwrap());
    for name in [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-real-ip",
    ] {
        headers.insert(name, "127.0.0.1".parse().unwrap());
        assert!(!direct_local(Some(local), &headers), "{name}");
        headers.remove(name);
    }
}

#[tokio::test]
async fn file_resolution_rejects_raw_paths_missing_files_and_incomplete_jobs() {
    let pool = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("data/renders");
    std::fs::create_dir_all(&root).unwrap();
    let source = dir.path().join("录像 #百分之%.mp4");
    std::fs::write(&source, b"source").unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &recording("录像 #百分之%.mp4"))
            .await
            .unwrap(),
        source.canonicalize().unwrap()
    );
    for bad in [
        "",
        "../录像 #百分之%.mp4",
        "data/录像.mp4",
        "data\\录像.mp4",
        "/etc/passwd",
        "ds_update.log",
    ] {
        assert_eq!(
            resolve_file(&pool, &root, dir.path(), &recording(bad))
                .await
                .unwrap_err()
                .status(),
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &recording("missing.mp4"))
            .await
            .unwrap_err()
            .status(),
        StatusCode::NOT_FOUND
    );
    let output = root.join("切片-3.mp4");
    std::fs::write(&output, b"render").unwrap();
    sqlx::query("INSERT INTO render_jobs VALUES(3,'ready',?)")
        .bind(output.to_str())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(3))
            .await
            .unwrap(),
        output.canonicalize().unwrap()
    );
    sqlx::query("UPDATE render_jobs SET state='running' WHERE id=3")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(3))
            .await
            .unwrap_err()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(99))
            .await
            .unwrap_err()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(-1))
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let ambiguous = FileRequest {
        kind: "render".into(),
        file_name: Some("x.mp4".into()),
        id: Some(3),
    };
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &ambiguous)
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_escape_is_rejected_for_recordings_and_render_outputs() {
    let pool = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = dir.path().join("renders");
    std::fs::create_dir_all(&root).unwrap();
    let secret = outside.path().join("private.mp4");
    std::fs::write(&secret, b"private").unwrap();
    std::os::unix::fs::symlink(&secret, dir.path().join("escape.mp4")).unwrap();
    std::os::unix::fs::symlink(&secret, root.join("escape.mp4")).unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &recording("escape.mp4"))
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("INSERT INTO render_jobs VALUES(1,'ready',?)")
        .bind(root.join("escape.mp4").to_str())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(1))
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE render_jobs SET output_path=? WHERE id=1")
        .bind(secret.to_str())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        resolve_file(&pool, &root, dir.path(), &render(1))
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
    // Even an owned render root cannot point outside the active work directory.
    assert_eq!(
        resolve_file(&pool, outside.path(), dir.path(), &render(1))
            .await
            .unwrap_err()
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn remote_reveal_never_launches_native_commands() {
    let pool = pool().await;
    let jobs = Arc::new(RenderJobs::new(pool.clone(), "data/renders"));
    let response = reveal(
        State(pool),
        State(jobs),
        Some(Extension(ConnectInfo("192.168.1.8:2000".parse().unwrap()))),
        HeaderMap::new(),
        Json(recording("anything.mp4")),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains("复制服务器文件路径")
    );
}

#[tokio::test]
async fn remote_location_returns_a_copyable_path_without_gui_capability() {
    let pool = pool().await;
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix(".file-location-test-")
        .suffix(".mp4")
        .tempfile_in(&cwd)
        .unwrap();
    let jobs = Arc::new(RenderJobs::new(pool.clone(), "data/renders"));
    let mut headers = HeaderMap::new();
    headers.insert(header::HOST, "127.0.0.1:19159".parse().unwrap());
    let response = location(
        State(pool),
        State(jobs),
        Some(Extension(ConnectInfo("192.168.1.8:2000".parse().unwrap()))),
        headers,
        Query(recording(
            file.path().file_name().unwrap().to_str().unwrap(),
        )),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(
        json["path"],
        file.path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(json["can_open"], false);
    assert!(
        serde_json::from_value::<FileRequest>(serde_json::json!({
            "kind": "recording", "file_name": "x.mp4", "path": "/etc/passwd"
        }))
        .is_err()
    );
}
