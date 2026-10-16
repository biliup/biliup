//! The internal danmaku worker: XML -> ASS with the native Rust converter
//! compiled into this program (`danmaku-factory`), run in a child process
//! started from the same program.
//!
//! Each conversion retains process isolation and a bounded lifetime. A
//! supervisor kills and reaps the child on timeout, cancellation, or a dropped
//! request before reclaiming its exclusive attempt directory.
//!
//! Every host must start the worker before doing anything else:
//! - the `biliup` binary and the desktop app call [`run_if_requested`] at the
//!   top of `main`, and the worker is started as `current_exe() FLAG REQUEST`;
//! - under Python, `current_exe()` is the interpreter, so `stream_gears`
//!   registers [`Launcher::python`], which loads the very same extension
//!   module file and calls `main_loop()`, which checks for the flag first.
//!
//! The request and the result are JSON files in the conversion's private
//! work directory, which is also the child's working directory, so no path
//! of the user's (long, non-ASCII or with spaces) appears on the command
//! line. A managed worker independently holds its root's shared lease, so a
//! parent crash cannot make a still-running attempt eligible for recovery.

use danmaku_factory::{ConvertError, ConvertOptions, ConvertStats};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

/// `argv[1]` that selects worker mode. Not a subcommand: it is checked before
/// command-line parsing, logging, the runtime and the configuration.
pub const WORKER_FLAG: &str = "--biliup-internal-danmaku-worker";
/// Bumped when [`Request`] or [`Response`] change, so a parent never talks to
/// a different build (e.g. after the binary was replaced while running).
pub const PROTOCOL: u32 = 1;

/// Managed render roots use a shared lock held independently by parent and
/// worker. Recovery must take the exclusive lock before reclaiming a root.
pub const ROOT_LEASE_FILE: &str = ".biliup-danmaku-lease";
pub const ATTEMPT_LEASE_FILE: &str = ".biliup-danmaku-attempt";
pub(crate) const ROOT_RETIRED_FILE: &str = ".biliup-danmaku-retired";
/// A delayed managed child must not fall back to the standalone path after
/// recovery has already removed its in-directory identity marker.
pub(crate) const MANAGED_WORKER_ENV: &str = "BILIUP_DANMAKU_MANAGED_WORKER";
/// A Windows path has at most 32,767 UTF-16 code units: valid filename
/// characters need at most three UTF-8 bytes each (escaped backslashes need
/// two), leaving room for JSON fields under 128 KiB. Unix's 4096-byte paths
/// need less than 25 KiB even with six-byte JSON control escapes.
pub const ROOT_LEASE_MAX_BYTES: u64 = 128 * 1024;

/// The only global location scanned for abandoned preview/self-check roots.
/// Export roots remain under their managed session directories.
#[doc(hidden)]
pub fn managed_preview_root_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("biliup")
        .join("danmaku-workers")
}

#[doc(hidden)]
pub fn managed_preview_root() -> io::Result<tempfile::TempDir> {
    let base = managed_preview_root_dir();
    std::fs::create_dir_all(&base)?;
    tempfile::Builder::new().prefix(".render-").tempdir_in(base)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootLease {
    root: PathBuf,
    token: String,
}

fn invalid_lease(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn require_active_root(root: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(root.join(ROOT_RETIRED_FILE)) {
        Ok(_) => Err(invalid_lease("managed root is retired for recovery")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn read_lease(file: &File) -> io::Result<RootLease> {
    let mut file = file.try_clone()?;
    file.rewind()?;
    let mut bytes = Vec::new();
    file.take(ROOT_LEASE_MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > ROOT_LEASE_MAX_BYTES {
        return Err(invalid_lease("managed root lease is too large"));
    }
    serde_json::from_slice(&bytes).map_err(|e| invalid_lease(e.to_string()))
}

/// Verifies the locked marker still names this unique, unrenamed root. A
/// delayed worker must fail this check after recovery renames the directory.
pub fn validate_root_lease(root: &Path, file: &File) -> io::Result<()> {
    // Recheck after acquiring the shared lock: recovery may have retired the
    // root while a late worker was waiting for its exclusive lease to release.
    require_active_root(root)?;
    let lease = read_lease(file)?;
    if lease.root != std::fs::canonicalize(root)? || lease.token.is_empty() {
        return Err(invalid_lease("managed root lease identity changed"));
    }
    let current = read_lease(&File::open(lease.root.join(ROOT_LEASE_FILE))?)?;
    if current != lease {
        return Err(invalid_lease("managed root lease was replaced"));
    }
    Ok(())
}

/// Acquires one parent's shared lease. Concurrent attempts in the same root
/// share the marker but hold independent OS locks.
pub fn acquire_root_lease(root: &Path) -> io::Result<File> {
    let root = std::fs::canonicalize(root)?;
    require_active_root(&root)?;
    let marker = root.join(ROOT_LEASE_FILE);
    if !marker.try_exists()? {
        let mut temporary = tempfile::Builder::new()
            .prefix(".lease-")
            .tempfile_in(&root)?;
        let lease = RootLease {
            root: root.clone(),
            token: temporary
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        };
        serde_json::to_writer(temporary.as_file_mut(), &lease).map_err(io::Error::other)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        if let Err(error) = temporary.persist_noclobber(&marker)
            && error.error.kind() != io::ErrorKind::AlreadyExists
        {
            return Err(error.error);
        }
    }
    if !std::fs::symlink_metadata(&marker)?.is_file() {
        return Err(invalid_lease("managed root lease is not a regular file"));
    }
    let file = OpenOptions::new().read(true).write(true).open(&marker)?;
    file.lock_shared()?;
    validate_root_lease(&root, &file)?;
    Ok(file)
}

pub(crate) fn mark_attempt(root: &Path, attempt: &Path) -> io::Result<()> {
    let lease = read_lease(&File::open(root.join(ROOT_LEASE_FILE))?)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(attempt.join(ATTEMPT_LEASE_FILE))?;
    serde_json::to_writer(&mut file, &lease).map_err(io::Error::other)?;
    file.flush()?;
    file.sync_all()
}

fn worker_lease() -> io::Result<Option<File>> {
    let attempt = std::env::current_dir()?;
    let marker = attempt.join(ATTEMPT_LEASE_FILE);
    if !marker.try_exists()? {
        if std::env::var_os(MANAGED_WORKER_ENV).is_some() {
            return Err(invalid_lease("managed worker attempt marker is missing"));
        }
        return Ok(None);
    }
    let expected = read_lease(&File::open(marker)?)?;
    let root = attempt
        .parent()
        .ok_or_else(|| invalid_lease("attempt has no managed root"))?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(ROOT_LEASE_FILE))?;
    file.lock_shared()?;
    validate_root_lease(root, &file)?;
    let current = read_lease(&file)?;
    if expected != current {
        return Err(invalid_lease("attempt belongs to another managed root"));
    }
    Ok(Some(file))
}

pub const EXIT_CONVERTED: i32 = 0;
/// The conversion failed; the result file says why.
pub const EXIT_FAILED: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_BAD_REQUEST: i32 = 3;
pub const EXIT_RESULT_UNWRITABLE: i32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    /// Relative to the work directory, which is the child's working directory.
    pub input: PathBuf,
    pub output: PathBuf,
    pub result: PathBuf,
    pub options: ConvertOptions,
    /// Lets unit tests make the worker hang or crash.
    #[cfg(test)]
    #[serde(default)]
    pub test_hook: Option<tests::Hook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Converted {
        protocol: u32,
        stats: ConvertStats,
        version: String,
    },
    Failed {
        protocol: u32,
        error: ConvertError,
    },
}

/// How to start this program in worker mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launcher {
    /// Runs `program args.. WORKER_FLAG REQUEST`.
    Program {
        program: PathBuf,
        args: Vec<OsString>,
    },
    /// No worker can be started; the reason is shown to the user.
    Unavailable(String),
}

/// Loads the extension module named by `argv[1]` from its file (so the child
/// runs the parent's exact build, whatever its `sys.path`), drops that
/// argument and hands the rest to `main_loop()`.
/// One line: a newline inside a Windows command-line argument is fragile.
const PYTHON_BOOTSTRAP: &str = "import sys, importlib.util as u; \
s = u.spec_from_file_location('stream_gears', sys.argv.pop(1)); \
m = u.module_from_spec(s); s.loader.exec_module(m); m.main_loop()";

impl Launcher {
    /// The worker of a Python host: `executable` is `sys.executable` and
    /// `module` the file of the `stream_gears` extension module.
    pub fn python(executable: Option<PathBuf>, module: Option<PathBuf>) -> Self {
        let executable = executable.filter(|path| !path.as_os_str().is_empty());
        let Some(executable) = executable else {
            return Self::Unavailable(
                "无法启动弹幕转换进程：Python 没有提供解释器路径（sys.executable 为空）".into(),
            );
        };
        let Some(module) = module.filter(|path| path.is_file()) else {
            return Self::Unavailable(
                "无法启动弹幕转换进程：找不到 stream_gears 扩展模块文件".into(),
            );
        };
        // The child runs in the conversion's work directory.
        let absolute = |path: PathBuf| std::path::absolute(&path).unwrap_or(path);
        Self::Program {
            program: absolute(executable),
            args: vec![
                "-c".into(),
                PYTHON_BOOTSTRAP.into(),
                absolute(module).into_os_string(),
            ],
        }
    }
}

static LAUNCHER: RwLock<Option<Launcher>> = RwLock::new(None);

/// Replaces how the worker is started; hosts whose executable is not this
/// program (Python) must call it before the server starts.
pub fn set_launcher(launcher: Launcher) {
    *LAUNCHER.write().unwrap_or_else(PoisonError::into_inner) = Some(launcher);
}

/// The configured launcher, or this executable.
pub fn launcher() -> Launcher {
    if let Some(launcher) = LAUNCHER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    {
        return launcher;
    }
    match std::env::current_exe() {
        Ok(program) => Launcher::Program {
            program,
            args: Vec::new(),
        },
        Err(error) => {
            Launcher::Unavailable(format!("无法定位当前程序，弹幕转换进程无法启动：{error}"))
        }
    }
}

/// The command that converts `request` (relative to `work_dir`). Its stdio
/// is set by the caller.
pub fn command(work_dir: &Path, request: &Path) -> Result<tokio::process::Command, String> {
    #[cfg(test)]
    if LAUNCHER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .is_none()
    {
        return Ok(tests::harness_command(work_dir, request));
    }
    match launcher() {
        Launcher::Program { program, args } => {
            let mut command = crate::tools::command(program);
            command
                .args(args)
                .arg(WORKER_FLAG)
                .arg(request)
                .current_dir(work_dir);
            Ok(command)
        }
        Launcher::Unavailable(reason) => Err(reason),
    }
}

/// Runs the worker and returns its exit code when `args` (including
/// `argv[0]`) select worker mode; `None` otherwise. Call it first in `main`:
/// it starts nothing else (no logging, runtime, configuration or server).
pub fn run_if_requested<I, T>(args: I) -> Option<i32>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    let _program = args.next()?;
    if args.next()? != WORKER_FLAG {
        return None;
    }
    let (Some(request), None) = (args.next(), args.next()) else {
        eprintln!("usage: biliup {WORKER_FLAG} REQUEST.json (internal)");
        return Some(EXIT_USAGE);
    };
    Some(run(Path::new(&request)))
}

fn run(request_path: &Path) -> i32 {
    // Held throughout conversion and response publication. Once this function
    // returns, the worker performs no further I/O in its private directory.
    // Parent death therefore cannot make an active attempt look disposable.
    let _lease = match worker_lease() {
        Ok(lease) => lease,
        Err(error) => {
            eprintln!("danmaku worker: invalid managed directory: {error}");
            return EXIT_BAD_REQUEST;
        }
    };
    let request: Request = match std::fs::read(request_path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
    {
        Ok(request) => request,
        Err(error) => {
            eprintln!(
                "danmaku worker: cannot read {}: {error}",
                request_path.display()
            );
            return EXIT_BAD_REQUEST;
        }
    };
    if request.protocol != PROTOCOL {
        eprintln!(
            "danmaku worker: protocol {} requested, this build speaks {PROTOCOL}",
            request.protocol
        );
        return EXIT_BAD_REQUEST;
    }
    #[cfg(test)]
    if let Some(hook) = &request.test_hook {
        tests::run_hook(hook);
    }
    let (response, code) = match danmaku_factory::convert_normalized_xml_to_ass(
        &request.input,
        &request.output,
        &request.options,
    ) {
        Ok(stats) => (
            Response::Converted {
                protocol: PROTOCOL,
                stats,
                version: danmaku_factory::version().to_owned(),
            },
            EXIT_CONVERTED,
        ),
        Err(error) => (
            Response::Failed {
                protocol: PROTOCOL,
                error,
            },
            EXIT_FAILED,
        ),
    };
    #[cfg(test)]
    let response = {
        let mut response = response;
        tests::after_convert(&request, &mut response);
        response
    };
    // A response has its own checked, no-overwrite publication. It cannot
    // commit the ASS and result as one transaction; the parent confirms both.
    let written = publish_response(&request.result, &response);
    if let Err(error) = written {
        eprintln!(
            "danmaku worker: cannot write {}: {error}",
            request.result.display()
        );
        return EXIT_RESULT_UNWRITABLE;
    }
    #[cfg(test)]
    if matches!(request.test_hook, Some(tests::Hook::FailAfterResponse)) {
        return EXIT_FAILED;
    }
    #[cfg(test)]
    if matches!(request.test_hook, Some(tests::Hook::MalformedResponse)) {
        std::fs::write(&request.result, b"not json").unwrap();
    }
    code
}

fn publish_response(path: &Path, response: &Response) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // tempfile publishes through Win32 APIs; give both names the verbatim
    // directory spelling needed beyond MAX_PATH, including a long worker cwd.
    #[cfg(windows)]
    let parent = parent.canonicalize()?;
    #[cfg(windows)]
    let destination =
        parent.join(path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "response has no filename")
        })?);
    #[cfg(windows)]
    let path = destination.as_path();
    let mut temp = tempfile::Builder::new()
        .prefix(".danmaku-response-")
        .tempfile_in(parent)?;
    let written = (|| {
        let mut writer = std::io::BufWriter::new(temp.as_file_mut());
        serde_json::to_writer(&mut writer, response).map_err(io::Error::other)?;
        writer.flush()?;
        let file = writer.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()
    })();
    if let Err(primary) = written {
        let residual = temp.path().to_owned();
        return match temp.close() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(io::Error::other(format!(
                "{primary}; cleanup {}: {cleanup}",
                residual.display()
            ))),
        };
    }
    match temp.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(failure) => {
            let residual = failure.file.path().to_owned();
            match failure.file.close() {
                Ok(()) => Err(failure.error),
                Err(cleanup) => Err(io::Error::other(format!(
                    "publish: {}; cleanup {}: {cleanup}",
                    failure.error,
                    residual.display()
                ))),
            }
        }
    }
}

/// Reads the result a worker left in `work_dir`, if any.
pub fn read_response(work_dir: &Path, result: &Path) -> Option<Result<Response, String>> {
    let bytes = std::fs::read(work_dir.join(result)).ok()?;
    Some(
        serde_json::from_slice::<Response>(&bytes)
            .map_err(|e| format!("弹幕转换进程的结果无法识别：{e}"))
            .and_then(|response| {
                let protocol = match &response {
                    Response::Converted { protocol, .. } | Response::Failed { protocol, .. } => {
                        *protocol
                    }
                };
                if protocol == PROTOCOL {
                    Ok(response)
                } else {
                    Err("弹幕转换进程版本不一致，请重启 biliup".into())
                }
            }),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// Unit tests run in the libtest binary, which cannot take the worker
    /// flag. It starts itself filtered to [`worker_process`], which runs the
    /// request named by this variable.
    const HARNESS_REQUEST: &str = "BILIUP_TEST_DANMAKU_WORKER_REQUEST";
    const HARNESS_PARENT_ROOT: &str = "BILIUP_TEST_DANMAKU_PARENT_ROOT";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Hook {
        SleepMs(u64),
        Abort,
        /// Writes PID and working directory, then waits to be killed.
        Pause {
            ready: PathBuf,
            after_publish: bool,
        },
        MissingAss,
        WrongProtocol,
        MalformedResponse,
        FailAfterResponse,
        ResultUnwritable,
        CleanupFailure,
        PauseBeforeAccept {
            ready: PathBuf,
        },
    }

    pub(super) fn harness_command(work_dir: &Path, request: &Path) -> tokio::process::Command {
        let mut command = crate::tools::command(std::env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "danmaku_worker::tests::worker_process",
                "--nocapture",
                "--test-threads=1",
                "-q",
            ])
            .env(HARNESS_REQUEST, request)
            .current_dir(work_dir);
        command
    }

    pub(crate) fn parent_proxy_command(root: &Path, attempt: &Path) -> tokio::process::Command {
        let mut command = crate::tools::command(std::env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "danmaku_worker::tests::crashed_parent_proxy",
                "--nocapture",
                "-q",
            ])
            .env(HARNESS_PARENT_ROOT, root)
            .current_dir(attempt);
        command
    }

    pub(super) fn run_hook(hook: &Hook) {
        match hook {
            Hook::SleepMs(ms) => std::thread::sleep(std::time::Duration::from_millis(*ms)),
            Hook::Abort => {
                let _ = writeln!(std::io::stderr(), "test hook: aborting");
                std::process::abort();
            }
            Hook::Pause {
                ready,
                after_publish: false,
            } => pause(ready),
            _ => {}
        }
    }

    fn pause(ready: &Path) {
        let marker = serde_json::json!({
            "pid": std::process::id(),
            "work": std::env::current_dir().unwrap(),
        });
        std::fs::write(ready, serde_json::to_vec(&marker).unwrap()).unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    pub(super) fn after_convert(request: &Request, response: &mut Response) {
        match &request.test_hook {
            Some(Hook::Pause {
                ready,
                after_publish: true,
            }) => pause(ready),
            Some(Hook::MissingAss) => {
                std::fs::remove_file(&request.output).unwrap();
            }
            Some(Hook::WrongProtocol) => match response {
                Response::Converted { protocol, .. } | Response::Failed { protocol, .. } => {
                    *protocol += 1
                }
            },
            Some(Hook::ResultUnwritable) => {
                std::fs::create_dir(&request.result).unwrap();
            }
            _ => {}
        }
    }

    #[test]
    fn worker_process() {
        if let Some(request) = std::env::var_os(HARNESS_REQUEST) {
            std::process::exit(run(Path::new(&request)));
        }
    }

    #[test]
    fn crashed_parent_proxy() {
        if let Some(root) = std::env::var_os(HARNESS_PARENT_ROOT) {
            let _lease = acquire_root_lease(Path::new(&root)).unwrap();
            let mut command = harness_command(
                &std::env::current_dir().unwrap(),
                Path::new("danmaku-request.json"),
            );
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            // std Child has no kill-on-drop behavior: killing this parent
            // deliberately leaves the independently leased worker alive.
            let _worker = command.as_std_mut().spawn().unwrap();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        }
    }

    #[test]
    fn only_the_exact_flag_selects_worker_mode() {
        assert_eq!(run_if_requested(["biliup"]), None);
        assert_eq!(run_if_requested(["biliup", "server"]), None);
        assert_eq!(run_if_requested(["biliup", "--help"]), None);
        assert_eq!(run_if_requested(Vec::<OsString>::new()), None);
        assert_eq!(run_if_requested(["biliup", WORKER_FLAG]), Some(EXIT_USAGE));
        assert_eq!(
            run_if_requested(["biliup", WORKER_FLAG, "a", "b"]),
            Some(EXIT_USAGE)
        );
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        assert_eq!(
            run_if_requested([OsString::from("biliup"), WORKER_FLAG.into(), missing.into()]),
            Some(EXIT_BAD_REQUEST)
        );
        let wrong = dir.path().join("wrong.json");
        std::fs::write(
            &wrong,
            r#"{"protocol":999,"input":"a","output":"b","result":"c","options":{}}"#,
        )
        .unwrap();
        assert_eq!(
            run_if_requested([OsString::from("biliup"), WORKER_FLAG.into(), wrong.into()]),
            Some(EXIT_BAD_REQUEST)
        );
    }

    #[test]
    fn python_launcher_loads_the_parent_module_file() {
        let dir = tempfile::tempdir().unwrap();
        let module = dir.path().join("stream_gears.abi3.so");
        std::fs::write(&module, b"").unwrap();
        let python = PathBuf::from("/opt/venv/bin/python3");
        let Launcher::Program { program, args } =
            Launcher::python(Some(python.clone()), Some(module.clone()))
        else {
            panic!("expected a program");
        };
        assert_eq!(program, python);
        assert_eq!(args[0], "-c");
        assert!(
            args[1]
                .to_str()
                .unwrap()
                .contains("spec_from_file_location")
        );
        assert_eq!(args[2], module.as_os_str());
        assert!(matches!(
            Launcher::python(None, Some(module.clone())),
            Launcher::Unavailable(_)
        ));
        assert!(matches!(
            Launcher::python(Some(PathBuf::new()), Some(module)),
            Launcher::Unavailable(_)
        ));
        assert!(matches!(
            Launcher::python(Some(python), Some(dir.path().join("missing.so"))),
            Launcher::Unavailable(_)
        ));
    }
}
