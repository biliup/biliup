//! The internal danmaku worker: XML -> ASS with the DanmakuFactory core that
//! is compiled into this program (`danmaku-factory`), run in a child process
//! started from the same program.
//!
//! The C core cannot be interrupted, and a memory error in it would take the
//! whole server down. Each conversion therefore runs in its own process,
//! which a timeout or a cancelled export kills and reaps, exactly as the
//! separate DanmakuFactory executable used to be handled.
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
//! line or reaches the C code.

use danmaku_factory::{ConvertError, ConvertOptions, ConvertStats};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

/// `argv[1]` that selects worker mode. Not a subcommand: it is checked before
/// command-line parsing, logging, the runtime and the configuration.
pub const WORKER_FLAG: &str = "--biliup-internal-danmaku-worker";
/// Bumped when [`Request`] or [`Response`] change, so a parent never talks to
/// a different build (e.g. after the binary was replaced while running).
pub const PROTOCOL: u32 = 1;

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
    Program { program: PathBuf, args: Vec<OsString> },
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
            return Self::Unavailable("无法启动弹幕转换进程：Python 没有提供解释器路径（sys.executable 为空）".into());
        };
        let Some(module) = module.filter(|path| path.is_file()) else {
            return Self::Unavailable("无法启动弹幕转换进程：找不到 stream_gears 扩展模块文件".into());
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
        Err(error) => Launcher::Unavailable(format!("无法定位当前程序，弹幕转换进程无法启动：{error}")),
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
    let request: Request = match std::fs::read(request_path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
    {
        Ok(request) => request,
        Err(error) => {
            eprintln!("danmaku worker: cannot read {}: {error}", request_path.display());
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
    let (response, code) =
        match danmaku_factory::convert_xml_to_ass(&request.input, &request.output, &request.options) {
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
    // Written under a temporary name and renamed, so a parent never reads a
    // half-written result of a worker that was killed.
    let partial = request.result.with_extension("partial");
    let written = serde_json::to_vec(&response)
        .map_err(std::io::Error::other)
        .and_then(|json| std::fs::write(&partial, json))
        .and_then(|()| std::fs::rename(&partial, &request.result));
    if let Err(error) = written {
        eprintln!(
            "danmaku worker: cannot write {}: {error}",
            request.result.display()
        );
        return EXIT_RESULT_UNWRITABLE;
    }
    code
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Hook {
        SleepMs(u64),
        Abort,
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

    pub(super) fn run_hook(hook: &Hook) {
        match hook {
            Hook::SleepMs(ms) => std::thread::sleep(std::time::Duration::from_millis(*ms)),
            Hook::Abort => {
                let _ = writeln!(std::io::stderr(), "test hook: aborting");
                std::process::abort();
            }
        }
    }

    #[test]
    fn worker_process() {
        if let Some(request) = std::env::var_os(HARNESS_REQUEST) {
            std::process::exit(run(Path::new(&request)));
        }
    }

    #[test]
    fn only_the_exact_flag_selects_worker_mode() {
        assert_eq!(run_if_requested(["biliup"]), None);
        assert_eq!(run_if_requested(["biliup", "server"]), None);
        assert_eq!(run_if_requested(["biliup", "--help"]), None);
        assert_eq!(run_if_requested(Vec::<OsString>::new()), None);
        assert_eq!(run_if_requested(["biliup", WORKER_FLAG]), Some(EXIT_USAGE));
        assert_eq!(run_if_requested(["biliup", WORKER_FLAG, "a", "b"]), Some(EXIT_USAGE));
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        assert_eq!(
            run_if_requested([OsString::from("biliup"), WORKER_FLAG.into(), missing.into()]),
            Some(EXIT_BAD_REQUEST)
        );
        let wrong = dir.path().join("wrong.json");
        std::fs::write(&wrong, r#"{"protocol":999,"input":"a","output":"b","result":"c","options":{}}"#).unwrap();
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
        let Launcher::Program { program, args } = Launcher::python(Some(python.clone()), Some(module.clone()))
        else {
            panic!("expected a program");
        };
        assert_eq!(program, python);
        assert_eq!(args[0], "-c");
        assert!(args[1].to_str().unwrap().contains("spec_from_file_location"));
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
