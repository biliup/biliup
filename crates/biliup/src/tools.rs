//! Starting external programs (ffmpeg, streamlink, yt-dlp, hook commands).

use std::ffi::OsStr;

/// Without this, a host that has no console (the desktop app is a
/// GUI-subsystem program) gets a new console window for every console program
/// it starts. A host running in a terminal must not use it: its children would
/// get their own hidden console instead of sharing the terminal, so Ctrl+C and
/// their output would no longer reach it.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
}

/// Whether child processes are started with `CREATE_NO_WINDOW`: on Windows when
/// this process has no console window, never elsewhere.
pub fn hides_console_windows() -> bool {
    #[cfg(windows)]
    {
        // SAFETY: takes no arguments and only reads this process's console state.
        unsafe { GetConsoleWindow() }.is_null()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// [`std::process::Command::new`] that opens no console window on Windows when
/// this process has no console of its own.
pub fn std_command(program: impl AsRef<OsStr>) -> std::process::Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    if hides_console_windows() {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// [`tokio::process::Command::new`] that opens no console window on Windows
/// when this process has no console of its own.
pub fn command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    tokio::process::Command::from(std_command(program))
}
