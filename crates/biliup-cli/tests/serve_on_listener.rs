//! `entry::serve` with a host-bound listener and a host-owned shutdown, as the
//! desktop app uses it. Kept in its own test binary because `work_dir`
//! switches the working directory of the whole process.

use std::time::Duration;

use biliup_cli::entry::{ServeOptions, init_tracing_with, serve};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn get_status_line(port: u16) -> Option<String> {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .ok()?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await
        .ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.ok()?;
    let response = String::from_utf8_lossy(&response);
    response.lines().next().map(str::to_owned)
}

#[tokio::test(flavor = "multi_thread")]
async fn serves_on_prebound_listener_until_host_shutdown() {
    let work_dir = tempfile::tempdir().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
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
    }));

    let mut status = None;
    for _ in 0..100 {
        status = get_status_line(port).await;
        if status.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let status = status.expect("server never answered on the pre-bound port");
    assert!(status.starts_with("HTTP/1.1 200"), "{status}");
    assert!(work_dir.path().join("data/data.sqlite3").exists());

    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("server did not stop after the host shutdown fired")
        .unwrap()
        .unwrap();
    assert!(
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
        "listener still open after shutdown"
    );
}
