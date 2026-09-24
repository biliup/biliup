//! Desktop shell: runs the biliup Web server inside this process and shows
//! its Web UI in a WebView.

mod data_dir;

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
/// Where the installer puts ffmpeg under the resource directory; the release
/// workflow adds it through `tauri.ffmpeg.conf.json`.
#[cfg(windows)]
const BUNDLED_FFMPEG: [&str; 2] = ["ffmpeg", "ffmpeg.exe"];
#[cfg(not(windows))]
const BUNDLED_FFMPEG: [&str; 2] = ["ffmpeg", "ffmpeg"];

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
                .title("biliup")
                .inner_size(1280.0, 800.0)
                .build()?;
            let app = app.handle().clone();
            // The data directory dialog blocks, which must not happen on the
            // main thread.
            std::thread::spawn(move || {
                let Some(install_dir) = install_dir() else {
                    show_error(&window, "无法确定程序所在目录");
                    return;
                };
                let legacy_roots = data_dir::legacy_roots(&install_dir);
                match data_dir::resolve(&app, &window, &install_dir, &legacy_roots) {
                    Ok(Some(data_dir)) => {
                        let ffmpeg = bundled_ffmpeg(app.path().resource_dir().ok());
                        start(&app, &window, &data_dir, &legacy_roots, ffmpeg)
                    }
                    Ok(None) => app.exit(0),
                    Err(message) => show_error(&window, &message),
                }
            });
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
fn start(
    app: &AppHandle,
    window: &WebviewWindow,
    data_dir: &Path,
    legacy_roots: &[PathBuf],
    ffmpeg: Option<PathBuf>,
) {
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
    match &ffmpeg {
        Some(path) => tracing::info!(path = %path.display(), "bundled ffmpeg found"),
        None => tracing::info!("no bundled ffmpeg; using the ffmpeg_path setting or PATH"),
    }

    if let Some(legacy_dir) = data_dir::find_legacy_root(legacy_roots) {
        match data_dir::migrate_legacy_data(legacy_dir, data_dir) {
            Ok(true) => tracing::info!(
                from = %legacy_dir.join("data").display(),
                to = %data_dir.join("data").display(),
                "copied data/ left by a previous version; the original is kept"
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
        ffmpeg,
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

fn bundled_ffmpeg(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    // On Windows resource_dir() is a `\\?\` path; this one ends up in logs and GET /v1/tools.
    let mut path = dunce::simplified(&resource_dir?).to_path_buf();
    path.extend(BUNDLED_FFMPEG);
    Some(path).filter(|path| path.is_file())
}

/// Where this executable lives. The old PyInstaller sidecar used it as its
/// working directory, so an older install may have left data/ there.
fn install_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
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
    fn percent_encoding_round_trips_through_decode_uri_component() {
        assert_eq!(percent_encode("a b/ç%"), "a%20b%2F%C3%A7%25");
    }

    #[test]
    fn bundled_ffmpeg_is_used_only_when_installed() {
        let resources = std::env::temp_dir().join(format!(
            "biliup-desktop-test-resources-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&resources);
        fs::create_dir_all(&resources).unwrap();
        assert_eq!(bundled_ffmpeg(Some(resources.clone())), None);
        assert_eq!(bundled_ffmpeg(None), None);
        let ffmpeg = resources.join("ffmpeg").join(BUNDLED_FFMPEG[1]);
        fs::create_dir_all(ffmpeg.parent().unwrap()).unwrap();
        fs::write(&ffmpeg, b"").unwrap();
        assert_eq!(bundled_ffmpeg(Some(resources.clone())), Some(ffmpeg));
        fs::remove_dir_all(&resources).unwrap();
    }
}
