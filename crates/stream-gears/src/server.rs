use biliup::uploader::util::SubmitOption;
use biliup_cli::cli::{Cli, Commands};
use biliup_cli::downloader::generate_json;
use biliup_cli::server::config::Config;
use biliup_cli::server::errors::AppResult;
use biliup_cli::uploader::{
    append, comments, list, login, renew, reply, show, upload_by_command, upload_by_config,
};
use clap::Parser;
use pyo3::prelude::PyAnyMethods;
use pyo3::prelude::PyDictMethods;
use pyo3::types::PyDict;
use pyo3::{Bound, PyAny, PyResult, Python};
use pyo3::{pyclass, pyfunction, pymethods};
use pythonize::pythonize;
use std::ops::Deref;
use std::sync::{Arc, LazyLock, RwLock};
use time::macros::format_description;
use tracing::info;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, reload};

#[pyclass]
#[derive(Debug, Clone)]
struct OnceConfig {
    // 用 PyObject 存，方便保持任意 Python 对象
    map: Config,
}

#[pymethods]
impl OnceConfig {
    /// 获取：config.get("k", default=None)
    /// - 若 key 存在，返回保存的对象
    /// - 若不存在，返回 default（默认 None）
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let guard = &self.map;
        // serde_json::to_value(guard.deref())
        if let Some(bound) = pythonize(py, guard)?
            .extract::<Bound<PyDict>>()?
            .get_item(key)?
        {
            if bound.is_none()
                && let Some(d) = default
            {
                return Ok(d);
            }
            // 尝试转换为字典并过滤
            return match bound.cast::<PyDict>() {
                Ok(dict) => {
                    let filtered = PyDict::new(py);
                    dict.iter()
                        .filter(|(_, v)| !v.is_none())
                        .try_for_each(|(k, v)| filtered.set_item(k, v))?;
                    Ok(filtered.into_any())
                }
                Err(_) => Ok(bound), // 不是字典，直接返回
            };
        };
        let Some(default) = default else {
            return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
                "object has no attribute '{key}'"
            )));
        };
        Ok(default)
    }
}

#[pyclass]
pub struct ConfigState {
    // 用 PyObject 存，方便保持任意 Python 对象
    map: Arc<RwLock<Config>>,
}

#[pymethods]
impl ConfigState {
    /// 获取：config.get("k", default=None)
    /// - 若 key 存在，返回保存的对象
    /// - 若不存在，返回 default（默认 None）
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let guard = self.map.read().unwrap();
        // serde_json::to_value(guard.deref())
        if let Some(bound) = pythonize(py, guard.deref())?
            .extract::<Bound<PyDict>>()?
            .get_item(key)?
        {
            if bound.is_none()
                && let Some(d) = default
            {
                return Ok(d);
            }
            // 尝试转换为字典并过滤
            return match bound.cast::<PyDict>() {
                Ok(dict) => {
                    let filtered = PyDict::new(py);
                    dict.iter()
                        .filter(|(_, v)| !v.is_none())
                        .try_for_each(|(k, v)| filtered.set_item(k, v))?;
                    Ok(filtered.into_any())
                }
                Err(_) => Ok(bound), // 不是字典，直接返回
            };
        };
        let Some(default) = default else {
            return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
                "object has no attribute '{key}'"
            )));
        };
        Ok(default)
    }
}

#[pyfunction]
pub fn config_bindings() -> PyResult<ConfigState> {
    let state = ConfigState {
        map: cfg_arc().clone(),
    };
    // pythonize(py, &config)
    Ok(state)
}

// 进程级全局单例（安全）：OnceLock + Arc + RwLock
pub static CONFIG: LazyLock<Arc<RwLock<Config>>> = LazyLock::new(|| {
    Arc::new(RwLock::new(
        Config::builder().streamers(Default::default()).build(),
    ))
});

fn cfg_arc() -> &'static Arc<RwLock<Config>> {
    &CONFIG
}

#[tokio::main]
pub(crate) async fn _main(args: &[String]) -> AppResult<()> {
    let cli = match Cli::try_parse_from(args) {
        Ok(res) => res,
        Err(e) => e.exit(),
    };

    let local_time = tracing_subscriber::fmt::time::LocalTime::new(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ));

    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_timer(local_time.clone())
        .with_file(true) // 打印文件名
        .with_line_number(true)
        .with_thread_ids(true);

    // 按日期滚动，每天创建新文件
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // rotate log files once every hour
        .rotation(Rotation::NEVER) // rotate log files once every hour
        .filename_prefix("biliup") // log file names will be prefixed with `myapp.`
        .filename_prefix("download") // log file names will be prefixed with `myapp.`
        .filename_suffix("log") // log file names will be suffixed with `.log`
        // .max_log_files(3)
        // .build("logs") // try to build an appender that stores log files in `/var/log`
        .build("") // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");
    // 或者按小时滚动
    // let file_appender = tracing_appender::rolling::hourly("logs", "upload.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_timer(local_time)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer) // 这个是“总开关”，所有 layer 都会被它过滤
        // 控制台输出
        .with(console_layer)
        // 文件输出
        .with(
            file_layer, // .json() // 可选：使用 JSON 格式便于解析
        );

    subscriber.init();

    info!("Tracing initialized with daily rotation");

    match cli.command {
        Commands::Login => login(cli.user_cookie, cli.proxy.as_deref()).await?,
        Commands::Renew => {
            renew(cli.user_cookie, cli.proxy.as_deref()).await?;
        }
        Commands::Upload {
            video_path,
            config: None,
            line,
            limit,
            studio,
            submit,
        } => {
            upload_by_command(
                studio,
                cli.user_cookie,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::Upload {
            video_path: _,
            config: Some(config),
            submit,
            ..
        } => {
            upload_by_config(config, cli.user_cookie, submit, cli.proxy.as_deref()).await?;
        }
        Commands::Append {
            video_path,
            vid,
            line,
            limit,
            studio: _,
            submit,
        } => {
            append(
                cli.user_cookie,
                vid,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::Show { vid } => show(cli.user_cookie, vid, cli.proxy.as_deref()).await?,
        Commands::Comments { vid, sort, pn, ps } => {
            comments(cli.user_cookie, vid, sort, pn, ps, cli.proxy.as_deref()).await?
        }
        Commands::Reply {
            vid,
            rpid,
            message,
            execute,
        } => {
            reply(
                cli.user_cookie,
                vid,
                rpid,
                message,
                execute,
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::DumpFlv { file_name } => generate_json(file_name)?,
        Commands::Download {
            url,
            output,
            split_size,
            split_time,
        } => biliup_cli::downloader::download(&url, output, split_size, split_time).await?,
        Commands::Server {
            bind,
            port,
            auth,
            config,
        } => {
            biliup_cli::run((&bind, port), auth, reload_handle, config).await?;
        }
        Commands::List {
            is_pubing,
            pubed,
            not_pubed,
            from_page,
            max_pages,
        } => {
            list(
                cli.user_cookie,
                is_pubing,
                pubed,
                not_pubed,
                cli.proxy.as_deref(),
                from_page,
                max_pages,
            )
            .await?
        }
    };

    Ok(())
}
