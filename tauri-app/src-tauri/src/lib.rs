use std::env;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, RunEvent};
use tauri::path::BaseDirectory;
use tauri_plugin_shell::process::{CommandChild, CommandEvent, Encoding};
use tauri_plugin_shell::ShellExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Helper function to spawn the sidecar and monitor its stdout/stderr
fn spawn_and_monitor_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Check if a sidecar process already exists
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
        let child_process = state.lock().unwrap();
        if child_process.is_some() {
            // A sidecar is already running, do not spawn a new one
            println!("[tauri] Sidecar is already running. Skipping spawn.");
            return Ok(()); // Exit early since sidecar is already running
        }
    }
    // // `sidecar()` 只需要文件名, 不像 JavaScript 中的整个路径 -x86_64-pc-windows-msvc.exe
    // let sidecar_command = app_handle.shell().sidecar("my-sidecar").unwrap();
    // let (mut rx, mut _child) = sidecar_command
    //     .spawn()
    //     .expect("Failed to spawn sidecar");
    //
    // tauri::async_runtime::spawn(async move {
    //     // 读取诸如 stdout 之类的事件
    //     while let Some(event) = rx.recv().await {
    //         if let CommandEvent::Stdout(line) = event {
    //             window
    //                 .emit("message", Some(format!("'{}'", line)))
    //                 .expect("failed to emit event");
    //             // 写入 stdin
    //             child.write("message from Rust\n".as_bytes()).unwrap();
    //         }
    //     }
    // });
    let resource_path = app_handle.path().resolve("binaries/biliup.exe", BaseDirectory::Resource).map_err(|e1| e1.to_string())?;
    // Spawn sidecar
    let sidecar_command = app_handle
        .shell()
        .command(resource_path);
    // 获取当前可执行文件的路径
    let exe_path = env::current_exe().unwrap();
    // 获取可执行文件所在的目录
    let exe_dir = exe_path.parent().unwrap();
    println!("[tauri] Sidecar directory: {}", exe_dir.display());
    let (mut rx, child) = sidecar_command
        .args(["-P", "19159"])
        .current_dir(exe_dir)
        // .env("PYTHONUTF8", "1")
        .spawn()
        .map_err(|e| e.to_string())?;
    // Store the child process in the app state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
        println!("pid {}", child.pid());
        *state.lock().unwrap() = Some(child);
    } else {
        return Err("Failed to access app state".to_string());
    }
    println!("[tauri] Sidecar started and running.");
    // Spawn an async task to handle sidecar communication
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line_bytes) => {
                    let encoding = Encoding::for_label("GBK".as_ref()).unwrap();
                    // let line = String::from_utf8_lossy(&line_bytes);
                    let line = encoding.decode_with_bom_removal(&line_bytes).0;
                    println!("Sidecar stdout: {}", line);
                    // Emit the line to the frontend
                    app_handle
                        .emit("sidecar-stdout", line.to_string())
                        .expect("Failed to emit sidecar stdout event");
                }
                CommandEvent::Stderr(line_bytes) => {
                    let line = String::from_utf8_lossy(&line_bytes);
                    eprintln!("Sidecar stderr: {}", line);
                    // Emit the error line to the frontend
                    app_handle
                        .emit("sidecar-stderr", line.to_string())
                        .expect("Failed to emit sidecar stderr event");
                }
                _ => {}
            }
        }
    });

    Ok(())
}

// Define a command to shutdown sidecar process
#[tauri::command]
fn shutdown_sidecar(app_handle: &tauri::AppHandle) -> Result<String, String> {
    println!("[tauri] Received command to shutdown sidecar.");
    // Access the sidecar process state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
        let mut child_process = state
            .lock()
            .map_err(|_| "[tauri] Failed to acquire lock on sidecar process.")?;
        let shell = app_handle.shell();
        if let Some(mut process) = child_process.take() {
            let pid = process.pid();
            // process.kill().map_err(|e1| e1.to_string())?;
            println!("[tauri] Killing sidecar. Pid: {}", pid);
            let kill_command;
            #[cfg(target_os = "windows")]
            {
                // taskkill /F (强制) /T (杀死进程树) /PID <pid>
                // 这是在 Windows 上杀死进程及其所有子进程的最佳方式
                println!("Using 'taskkill' on Windows to force kill process tree.");
                kill_command = shell
                    .command("taskkill")
                    .args(&["/F", "/T", "/PID", &pid.to_string()])
                    .spawn();
            }
            match kill_command {
                Ok((mut rx, _)) => {
                    tauri::async_runtime::block_on(async move {
                        let encoding = Encoding::for_label("GBK".as_ref()).unwrap();
                        while let Some(event) = rx.recv().await {
                            match event {
                                CommandEvent::Stdout(line) => println!(
                                    "Force kill stdout: {:?}",
                                    encoding.decode_with_bom_removal(&line).0
                                ),
                                CommandEvent::Stderr(line) => eprintln!(
                                    "Force kill stderr: {:?}",
                                    encoding.decode_with_bom_removal(&line).0
                                ),
                                CommandEvent::Terminated(payload) => {
                                    if payload.code == Some(0) {
                                        println!("Sidecar (PID: {}) and its children force killed successfully.", pid);
                                    } else {
                                        // 在 Windows 上，如果进程已不存在，taskkill 会返回非0代码（如128），这是正常的。
                                        // 在 Linux/macOS 上，如果进程已不存在，kill/pkill 会报错，也是正常的。
                                        println!("Force kill command finished. Process was likely already dead.");
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to execute force kill command: {}", e);
                }
            }

            println!("[tauri] Sent 'sidecar shutdown' command to sidecar.");
            Ok("'sidecar shutdown' command sent.".to_string())
        } else {
            println!("[tauri] No active sidecar process to shutdown.");
            Err("No active sidecar process to shutdown.".to_string())
        }
    } else {
        Err("Sidecar process state not found.".to_string())
    }
}

// Define a command to start sidecar process.
#[tauri::command]
fn start_sidecar(app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("[tauri] Received command to start sidecar.");
    spawn_and_monitor_sidecar(app_handle)?;
    Ok("Sidecar spawned and monitoring started.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Store the initial sidecar process in the app state
            app.manage(Arc::new(Mutex::new(None::<CommandChild>)));
            // Clone the app handle for use elsewhere
            let app_handle = app.handle().clone();
            // Spawn the Python sidecar on startup
            println!("[tauri] Creating sidecar...");
            spawn_and_monitor_sidecar(app_handle).ok();
            println!("[tauri] Sidecar spawned and monitoring started.");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| match event {
            // Ensure the Python sidecar is killed when the app is closed
            RunEvent::ExitRequested { .. } => {
                shutdown_sidecar(app_handle).expect("Failed to shutdown sidecar");
                println!("[tauri] Sidecar closed.");
            }
            _ => {}
        });
}
