//! Runs this test binary again as a child process with different consoles and
//! lets the child report what `hides_console_windows()` sees.
#![cfg(windows)]

use std::os::windows::process::CommandExt;

const PROBE: &str = "BILIUP_CONSOLE_PROBE";
const TEST: &str = "children_share_the_console_only_when_there_is_one";
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
// Distinct from the test harness's own exit codes.
const HIDES: i32 = 21;
const SHARES: i32 = 20;

#[test]
fn children_share_the_console_only_when_there_is_one() {
    if std::env::var_os(PROBE).is_some() {
        let hides = biliup::tools::hides_console_windows();
        std::process::exit(if hides { HIDES } else { SHARES });
    }
    let run = |flags: u32| {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--test-threads=1"])
            .env(PROBE, "1")
            .creation_flags(flags)
            .status()
            .unwrap()
            .code()
    };
    // Like the desktop app: no console at all, or a console without a window.
    assert_eq!(run(DETACHED_PROCESS), Some(HIDES));
    assert_eq!(run(CREATE_NO_WINDOW), Some(HIDES));
    // Like `biliup server` in a terminal.
    assert_eq!(run(CREATE_NEW_CONSOLE), Some(SHARES));
}
