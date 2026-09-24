//! Starting external programs (ffmpeg, streamlink, yt-dlp, hook commands).

use std::ffi::OsStr;

/// Without this, a GUI-subsystem host such as the desktop app gets a new
/// console window for every console program it starts.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// [`std::process::Command::new`] that opens no console window on Windows.
pub fn std_command(program: impl AsRef<OsStr>) -> std::process::Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// [`tokio::process::Command::new`] that opens no console window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    tokio::process::Command::from(std_command(program))
}
