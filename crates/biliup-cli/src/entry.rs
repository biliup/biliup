//! Shared entry point for the native `biliup` binary and the Python
//! `stream_gears.main_loop` wrapper, so both parse, log and dispatch the same way.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;

use biliup::uploader::util::SubmitOption;
use clap::Parser;
use error_stack::ResultExt;
use time::macros::format_description;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, reload};

use crate::LogHandle;
use crate::cli::{Cli, Commands, expand_path};
use crate::downloader::{download, generate_json};
use crate::server::errors::{AppError, AppResult};
use crate::uploader::{
    append, comments, list, login, renew, reply, show, upload_by_command, upload_by_config,
};

pub const DEFAULT_LOG_FILTER: &str = "tower_http=debug,info";
pub const LOG_FILE: &str = "ds_update.log";

static LOG_HANDLE: OnceLock<LogHandle> = OnceLock::new();

/// Fills in the default subcommand: no subcommand, or the legacy `start`, means `server`.
pub fn with_default_command<I, T>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match args.len() {
        0 => args.extend(["biliup".into(), "server".into()]),
        1 => args.push("server".into()),
        _ if args[1] == "start" => args[1] = "server".into(),
        _ => {}
    }
    args
}

/// Parses command-line arguments (including `argv[0]`) after applying [`with_default_command`].
pub fn parse<I, T>(args: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    Cli::try_parse_from(with_default_command(args))
}

/// An explicit `--rust-log` wins over `RUST_LOG`, which wins over [`DEFAULT_LOG_FILTER`].
pub fn log_filter_directives(cli_value: Option<&str>, env_value: Option<&str>) -> String {
    cli_value
        .or(env_value.filter(|value| !value.trim().is_empty()))
        .unwrap_or(DEFAULT_LOG_FILTER)
        .to_owned()
}

pub fn writes_log_file(command: &Commands) -> bool {
    matches!(command, Commands::Server { .. })
}

/// Keeps the log file writer alive; drop it only after the command has finished.
pub struct Logging {
    handle: LogHandle,
    _file_guard: Option<WorkerGuard>,
}

impl Logging {
    pub fn handle(&self) -> LogHandle {
        self.handle.clone()
    }
}

/// Initialises logging for `cli`: console output, plus `ds_update.log` in the
/// working directory for the `server` command.
pub fn init_tracing(cli: &Cli) -> Logging {
    let env = std::env::var("RUST_LOG").ok();
    init_tracing_with(
        &log_filter_directives(cli.rust_log.as_deref(), env.as_deref()),
        writes_log_file(&cli.command),
    )
}

/// Installs the global subscriber on the first call. Later calls in the same
/// process (e.g. `main_loop` called twice) keep the existing layers and only
/// swap the filter.
pub fn init_tracing_with(filter: &str, log_file: bool) -> Logging {
    if let Some(handle) = LOG_HANDLE.get() {
        let _ = handle.modify(|current| *current = EnvFilter::new(filter));
        return Logging {
            handle: handle.clone(),
            _file_guard: None,
        };
    }

    let timer = tracing_subscriber::fmt::time::LocalTime::new(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ));
    let (filter_layer, handle) = reload::Layer::new(EnvFilter::new(filter));

    let (file_layer, file_guard) = if log_file {
        let file_appender = tracing_appender::rolling::never(".", LOG_FILE);
        let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_timer(timer.clone())
            .with_writer(file_writer)
            .with_ansi(false);
        (Some(file_layer), Some(file_guard))
    } else {
        (None, None)
    };

    let installed = tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_timer(timer))
        .with(file_layer)
        .try_init()
        .is_ok();
    if installed {
        let _ = LOG_HANDLE.set(handle.clone());
    }
    Logging {
        handle,
        _file_guard: file_guard,
    }
}

/// Initialises logging and runs the command described by `cli`.
pub async fn run(cli: Cli) -> AppResult<()> {
    let logging = init_tracing(&cli);
    dispatch(cli, logging.handle()).await
}

/// Runs the command described by `cli`. Logging must already be initialised;
/// `log_handle` lets the Web UI change the log level at runtime.
pub async fn dispatch(cli: Cli, log_handle: LogHandle) -> AppResult<()> {
    let user_cookie = expand_path(cli.user_cookie);
    let proxy = cli.proxy.as_deref();

    match cli.command {
        Commands::Login => login(user_cookie, proxy).await?,
        Commands::Renew => renew(user_cookie, proxy).await?,
        Commands::Upload {
            video_path,
            config: None,
            line,
            limit,
            studio,
            submit,
        } => {
            let video_path: Vec<_> = video_path.into_iter().map(expand_path).collect();
            upload_by_command(
                studio,
                user_cookie,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                proxy,
            )
            .await?
        }
        Commands::Upload {
            video_path: _,
            config: Some(config),
            submit,
            ..
        } => upload_by_config(expand_path(config), user_cookie, submit, proxy).await?,
        Commands::Append {
            video_path,
            vid,
            line,
            limit,
            studio: _,
            submit,
        } => {
            let video_path: Vec<_> = video_path.into_iter().map(expand_path).collect();
            append(
                user_cookie,
                vid,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                proxy,
            )
            .await?
        }
        Commands::Show { vid } => show(user_cookie, vid, proxy).await?,
        Commands::Comments { vid, sort, pn, ps } => {
            comments(user_cookie, vid, sort, pn, ps, proxy).await?
        }
        Commands::Reply {
            vid,
            rpid,
            message,
            execute,
        } => reply(user_cookie, vid, rpid, message, execute, proxy).await?,
        Commands::DumpFlv { file_name } => generate_json(expand_path(file_name))?,
        Commands::Download {
            url,
            output,
            split_size,
            split_time,
        } => download(&url, output, split_size, split_time).await?,
        Commands::Server {
            bind,
            port,
            auth,
            secure_session_cookie,
            config,
        } => {
            serve(ServeOptions {
                bind,
                port,
                auth,
                secure_session_cookie,
                config,
                user_cookie,
                work_dir: None,
                log_handle,
            })
            .await?
        }
        Commands::User { action } => crate::web_user_cli::run(action).await?,
        Commands::List {
            is_pubing,
            pubed,
            not_pubed,
            from_page,
            max_pages,
        } => {
            list(
                user_cookie,
                is_pubing,
                pubed,
                not_pubed,
                proxy,
                from_page,
                max_pages,
            )
            .await?
        }
    };
    Ok(())
}

/// Options for [`serve`]; the fields mirror `biliup server`.
pub struct ServeOptions {
    pub bind: String,
    pub port: u16,
    pub auth: bool,
    pub secure_session_cookie: bool,
    pub config: Option<PathBuf>,
    pub user_cookie: PathBuf,
    /// The server keeps `data/`, `ds_update.log` and recordings relative to the
    /// working directory. When set, the directory is created and the whole
    /// process switches to it before the server starts.
    pub work_dir: Option<PathBuf>,
    pub log_handle: LogHandle,
}

/// Runs the Web server until shutdown. It only needs a tokio runtime, so an
/// embedding host (e.g. the desktop app) can spawn it on its own runtime after
/// calling [`init_tracing_with`].
pub async fn serve(opts: ServeOptions) -> AppResult<()> {
    if let Some(dir) = &opts.work_dir {
        std::fs::create_dir_all(dir)
            .and_then(|()| std::env::set_current_dir(dir))
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not switch to work dir {}", dir.display()))?;
    }
    crate::run_with_cookie(
        (&opts.bind, opts.port),
        opts.auth,
        opts.secure_session_cookie,
        opts.log_handle,
        opts.config,
        opts.user_cookie,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalized(args: &[&str]) -> Vec<String> {
        with_default_command(args.iter().copied())
            .into_iter()
            .map(|arg| arg.into_string().unwrap())
            .collect()
    }

    #[test]
    fn default_command_is_server() {
        assert_eq!(normalized(&[]), ["biliup", "server"]);
        assert_eq!(normalized(&["biliup"]), ["biliup", "server"]);
        assert_eq!(
            normalized(&["biliup", "start", "--port", "1"]),
            ["biliup", "server", "--port", "1"]
        );
        assert_eq!(normalized(&["biliup", "renew"]), ["biliup", "renew"]);
        assert_eq!(normalized(&["biliup", "--help"]), ["biliup", "--help"]);
        assert_eq!(
            normalized(&["biliup", "-u", "start"]),
            ["biliup", "-u", "start"]
        );
    }

    #[test]
    fn parse_without_arguments_starts_server() {
        let cli = parse(["biliup"]).unwrap();
        assert!(matches!(cli.command, Commands::Server { port: 19159, .. }));
        assert!(cli.rust_log.is_none());
        let cli = parse(["biliup", "start", "--port", "0"]).unwrap();
        assert!(matches!(cli.command, Commands::Server { port: 0, .. }));
    }

    #[test]
    fn help_and_version_are_errors_with_zero_exit_code() {
        for flag in ["--help", "--version"] {
            let err = parse(["biliup", flag]).err().unwrap();
            assert_eq!(err.exit_code(), 0, "{flag}");
        }
        let err = parse(["biliup", "--bad-flag"]).err().unwrap();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn explicit_rust_log_overrides_env() {
        let cli = parse(["biliup", "--rust-log", "debug", "server"]).unwrap();
        assert_eq!(cli.rust_log.as_deref(), Some("debug"));
        assert_eq!(
            log_filter_directives(cli.rust_log.as_deref(), Some("warn")),
            "debug"
        );
    }

    #[test]
    fn env_rust_log_applies_without_flag() {
        assert_eq!(log_filter_directives(None, Some("warn")), "warn");
        assert_eq!(log_filter_directives(None, Some(" ")), DEFAULT_LOG_FILTER);
        assert_eq!(log_filter_directives(None, None), DEFAULT_LOG_FILTER);
    }

    #[test]
    fn log_file_is_written_only_by_the_server_command() {
        let server = parse(["biliup"]).unwrap();
        let renew = parse(["biliup", "renew"]).unwrap();
        let list = parse(["biliup", "list"]).unwrap();

        assert!(writes_log_file(&server.command));
        assert!(!writes_log_file(&renew.command));
        assert!(!writes_log_file(&list.command));
    }
}
