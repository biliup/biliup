// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The server converts danmaku by starting this executable as a worker
    // (biliup_cli::danmaku_worker). Handle that before Tauri: no window, and
    // no single-instance check that would hand the request to the running app.
    if let Some(code) = biliup_cli::danmaku_worker::run_if_requested(std::env::args_os()) {
        std::process::exit(code);
    }
    biliup_desktop_lib::run()
}
