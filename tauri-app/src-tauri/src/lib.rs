//! Desktop shell: runs the biliup Web server inside this process and shows
//! its Web UI in a WebView.

use std::fmt::Write as _;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use std::{fs, io};

use biliup_cli::entry::{self, Logging, ServeOptions};
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

const DEFAULT_PORT: u16 = 19159;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const WINDOW_LABEL: &str = "main";
/// Where Tauri serves the bundled `static/` directory.
#[cfg(windows)]
const STARTUP_PAGE: &str = "http://tauri.localhost/";
#[cfg(not(windows))]
const STARTUP_PAGE: &str = "tauri://localhost/";

struct Server {
    stop: Mutex<Option<oneshot::Sender<()>>>,
    task: Mutex<Option<JoinHandle<()>>>,
    logging: Mutex<Option<Logging>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
                .title("biliup")
                .inner_size(1280.0, 800.0)
                .build()?;
            let data_dir = app.path().app_data_dir()?;
            start(app.handle(), &window, &data_dir);
            Ok(())
        })
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|app, event| {
            if let RunEvent::Exit = event {
                stop_server(app);
            }
        }),
        Err(err) => {
            eprintln!("[biliup] failed to start the desktop app: {err}");
            std::process::exit(1);
        }
    }
}

/// Moves into the data directory, migrates legacy data and starts the server.
/// Failures are shown in the window instead of aborting the app.
fn start(app: &AppHandle, window: &WebviewWindow, data_dir: &Path) {
    // The server keeps data/, ds_update.log and recordings relative to the
    // working directory, and the log file is opened relative to it as well.
    if let Err(err) =
        fs::create_dir_all(data_dir).and_then(|()| std::env::set_current_dir(data_dir))
    {
        show_error(
            window,
            &format!("无法使用数据目录 {}：{err}", data_dir.display()),
        );
        return;
    }
    let env_filter = std::env::var("RUST_LOG").ok();
    let logging = entry::init_tracing_with(
        &entry::log_filter_directives(None, env_filter.as_deref()),
        true,
    );
    tracing::info!(data_dir = %data_dir.display(), "biliup desktop starting");

    if let Some(legacy_dir) = legacy_dir() {
        match migrate_legacy_data(&legacy_dir, data_dir) {
            Ok(true) => tracing::info!(
                from = %legacy_dir.join("data").display(),
                to = %data_dir.join("data").display(),
                "copied data/ from the previous install directory; the original is kept"
            ),
            Ok(false) => {}
            Err(err) => {
                tracing::error!("migrating legacy data failed: {err}");
                show_error(
                    window,
                    &format!(
                        "无法把旧版数据从 {} 复制到 {}：{err}\n请确认旧版 biliup 已退出后重新打开。",
                        legacy_dir.join("data").display(),
                        data_dir.join("data").display()
                    ),
                );
                return;
            }
        }
    }

    let listener = match bind_listener() {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("could not open a local port: {err}");
            show_error(window, &format!("无法监听本地端口：{err}"));
            return;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => {
            show_error(window, &format!("无法监听本地端口：{err}"));
            return;
        }
    };
    if port != DEFAULT_PORT {
        tracing::warn!(
            port,
            "port {DEFAULT_PORT} is in use, using a free port instead"
        );
    }

    let (stop, stopped) = oneshot::channel::<()>();
    let options = ServeOptions {
        bind: Ipv4Addr::LOCALHOST.to_string(),
        port,
        auth: false,
        secure_session_cookie: false,
        config: None,
        user_cookie: PathBuf::from("cookies.json"),
        work_dir: None,
        log_handle: logging.handle(),
        listener: Some(listener),
        shutdown: Some(Box::pin(async move {
            let _ = stopped.await;
        })),
    };

    let failed = Arc::new(AtomicBool::new(false));
    let task = {
        let failed = failed.clone();
        let window = window.clone();
        async_runtime::spawn(async move {
            if let Err(err) = entry::serve(options).await {
                failed.store(true, Ordering::SeqCst);
                tracing::error!("biliup server stopped: {err:?}");
                show_error(&window, &format!("{err:?}"));
            }
        })
    };
    app.manage(Server {
        stop: Mutex::new(Some(stop)),
        task: Mutex::new(Some(task)),
        logging: Mutex::new(Some(logging)),
    });

    let window = window.clone();
    async_runtime::spawn(async move {
        let started = Instant::now();
        loop {
            if failed.load(Ordering::SeqCst) {
                return;
            }
            if web_ui_ready(port).await {
                break;
            }
            if started.elapsed() > STARTUP_TIMEOUT {
                tracing::error!(port, "Web UI did not become ready in time");
                show_error(
                    &window,
                    &format!(
                        "服务在 {} 秒内没有就绪（端口 {port}）",
                        STARTUP_TIMEOUT.as_secs()
                    ),
                );
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let url = format!("http://127.0.0.1:{port}/");
        tracing::info!(%url, "Web UI ready");
        match Url::parse(&url) {
            Ok(url) => {
                if let Err(err) = window.navigate(url) {
                    tracing::error!("could not open the Web UI: {err}");
                }
            }
            Err(err) => tracing::error!("invalid Web UI url {url}: {err}"),
        }
    });
}

/// Stops the server and waits for it to finish, so recordings are closed and
/// the log is flushed before the process exits.
fn stop_server(app: &AppHandle) {
    let Some(server) = app.try_state::<Server>() else {
        return;
    };
    if let Some(stop) = take(&server.stop) {
        let _ = stop.send(());
    }
    if let Some(task) = take(&server.task) {
        let stopped =
            async_runtime::block_on(async { tokio::time::timeout(SHUTDOWN_TIMEOUT, task).await });
        match stopped {
            Ok(_) => tracing::info!("biliup server stopped"),
            Err(_) => tracing::warn!(
                "biliup server did not stop within {} s, exiting anyway",
                SHUTDOWN_TIMEOUT.as_secs()
            ),
        }
    }
    drop(take(&server.logging));
}

fn take<T>(slot: &Mutex<Option<T>>) -> Option<T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner).take()
}

/// Binds 127.0.0.1:19159, or a free port when that one is taken.
fn bind_listener() -> io::Result<std::net::TcpListener> {
    async_runtime::block_on(async {
        match listen(DEFAULT_PORT) {
            Ok(listener) => Ok(listener),
            Err(err) => {
                tracing::warn!("could not listen on port {DEFAULT_PORT}: {err}");
                listen(0)
            }
        }
    })
}

/// Must run inside the tokio runtime.
fn listen(port: u16) -> io::Result<std::net::TcpListener> {
    let socket = tokio::net::TcpSocket::new_v4()?;
    // Same as `biliup server`: lets a restart reuse the port while old
    // connections are in TIME_WAIT. Not set on Windows, where it would allow
    // two processes to share the port.
    #[cfg(unix)]
    socket.set_reuseaddr(true)?;
    socket.bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
    socket.listen(1024)?.into_std()
}

async fn web_ui_ready(port: u16) -> bool {
    let request = async {
        let mut stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await?;
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await?;
        let mut head = [0u8; 12];
        stream.read_exact(&mut head).await?;
        io::Result::Ok(head.starts_with(b"HTTP/1.1 200"))
    };
    matches!(
        tokio::time::timeout(Duration::from_secs(5), request).await,
        Ok(Ok(true))
    )
}

/// The old PyInstaller sidecar ran with the install directory (where this
/// executable lives) as its working directory, so its data/ is there.
fn legacy_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// Copies `legacy_root/data` to `data_root/data` once, when the former exists
/// and the latter does not. The original is left in place. Returns whether a
/// copy was made.
fn migrate_legacy_data(legacy_root: &Path, data_root: &Path) -> io::Result<bool> {
    let from = legacy_root.join("data");
    let to = data_root.join("data");
    if !from.is_dir() || to.exists() || same_dir(&from, &to) {
        return Ok(false);
    }
    // Copy to a temporary name first so an interrupted copy is retried on the
    // next start instead of being mistaken for migrated data.
    let partial = data_root.join("data.migrating");
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    copy_dir(&from, &partial)
        .and_then(|()| fs::rename(&partial, &to))
        .inspect_err(|_| {
            let _ = fs::remove_dir_all(&partial);
        })?;
    Ok(true)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Shows `message` on the bundled startup page via `#error=…`.
fn show_error(window: &WebviewWindow, message: &str) {
    let current = window.url().ok().filter(|url| url.scheme() != "about");
    if current.as_ref().is_some_and(|url| url.port().is_some()) {
        // Already on the Web UI; the server stopped after it had started.
        return;
    }
    let Some(mut url) = current.or_else(|| Url::parse(STARTUP_PAGE).ok()) else {
        return;
    };
    url.set_fragment(Some(&format!("error={}", percent_encode(message))));
    if let Err(err) = window.navigate(url) {
        tracing::error!("could not show the startup error: {err}");
    }
}

fn percent_encode(text: &str) -> String {
    text.bytes().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_data_once_and_keeps_the_original() {
        let legacy = tempfile_dir("legacy");
        let data = tempfile_dir("appdata");
        fs::create_dir_all(legacy.join("data/nested")).unwrap();
        fs::write(legacy.join("data/data.sqlite3"), b"db").unwrap();
        fs::write(legacy.join("data/nested/42.json"), b"{}").unwrap();

        assert!(migrate_legacy_data(&legacy, &data).unwrap());
        assert_eq!(fs::read(data.join("data/data.sqlite3")).unwrap(), b"db");
        assert_eq!(fs::read(data.join("data/nested/42.json")).unwrap(), b"{}");
        assert!(legacy.join("data/data.sqlite3").exists());
        assert!(!data.join("data.migrating").exists());

        fs::write(legacy.join("data/data.sqlite3"), b"newer").unwrap();
        assert!(!migrate_legacy_data(&legacy, &data).unwrap());
        assert_eq!(fs::read(data.join("data/data.sqlite3")).unwrap(), b"db");
    }

    #[test]
    fn skips_migration_without_legacy_data() {
        let legacy = tempfile_dir("empty-legacy");
        let data = tempfile_dir("empty-appdata");
        assert!(!migrate_legacy_data(&legacy, &data).unwrap());
        assert!(!data.join("data").exists());
    }

    #[test]
    fn percent_encoding_round_trips_through_decode_uri_component() {
        assert_eq!(percent_encode("a b/ç%"), "a%20b%2F%C3%A7%25");
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("biliup-desktop-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
