use crate::danmaku::PyDanmakuClient;
use async_trait::async_trait;
use biliup::uploader::util::SubmitOption;
use biliup_cli::cli::{Cli, Commands};
use biliup_cli::downloader::generate_json;
use biliup_cli::server::app::ApplicationController;
use biliup_cli::server::common::util::media_ext_from_url;
use biliup_cli::server::config::Config;
use biliup_cli::server::core::download_manager::DownloadManager;
use biliup_cli::server::core::downloader::DanmakuClient;
use biliup_cli::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use biliup_cli::server::errors::{AppError, AppResult};
use biliup_cli::server::infrastructure::connection_pool::ConnectionManager;
use biliup_cli::server::infrastructure::context::{Context, Worker};
use biliup_cli::server::infrastructure::models::StreamerInfo;
use biliup_cli::server::infrastructure::repositories;
use biliup_cli::server::infrastructure::repositories::get_upload_config;
use biliup_cli::server::infrastructure::service_register::ServiceRegister;
use biliup_cli::uploader::{append, list, login, renew, show, upload_by_command, upload_by_config};
use chrono::Utc;
use clap::Parser;
use error_stack::{FutureExt, Report, ResultExt};
use fancy_regex::Regex;
use pyo3::prelude::PyDictMethods;
use pyo3::prelude::{PyAnyMethods, PyListMethods, PyModule};
use pyo3::types::PyDict;
use pyo3::types::{PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult, Python};
use pyo3::{pyclass, pyfunction, pymethods};
use pythonize::pythonize;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::ops::Deref;
use std::sync::{Arc, LazyLock, RwLock};
use time::macros::format_description;
use tracing::{debug, info, warn};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug)]
pub struct PyPlugin {
    plugin: Arc<Py<PyType>>,
    pattern: Regex,
    name: String,
}

impl PyPlugin {
    pub fn from_pytype(class: &Bound<PyType>) -> PyResult<Self> {
        let pattern = class.getattr("VALID_URL_BASE")?.to_string();
        // info!("{pattern}");
        let re = Regex::new(&pattern).unwrap();
        let plugin = class.clone();
        let name: String = class.getattr("__name__")?.extract()?;
        // info!("发现插件类: {name}");
        Ok(Self {
            plugin: Arc::new(plugin.unbind()),
            pattern: re,
            name,
        })
    }
}

#[derive(Debug)]
pub struct PyDownloader {
    plugin: Arc<Py<PyType>>,
    url: String,
    remark: String,
    danmaku: Option<Arc<Py<PyAny>>>,
}

impl PyDownloader {
    fn new(plugin: Arc<Py<PyType>>, url: String, remark: String) -> Self {
        Self {
            plugin,
            url: url.clone(),
            remark: remark.clone(),
            danmaku: None,
        }
    }

    async fn call_via_threads(&mut self) -> AppResult<Option<StreamInfoExt>> {
        let url = self.url.clone();
        let remark = self.remark.clone();
        let obj = self.plugin.clone();
        Ok(
            match tokio::task::spawn_blocking(move || {
                Python::attach(
                    |py| -> PyResult<Option<(StreamInfoExt, Option<Py<PyAny>>)>> {
                        // 从 biliup.util 获取 loop（按你项目里真实的名字来取）
                        let util = PyModule::import(py, "biliup.common.util")?;
                        // 下面两行二选一（取决于 biliup.util 的 API）：
                        // let loop_obj: Py<PyAny> = util.getattr("loop")?.into_py(py);
                        // 或：
                        // let loop_obj: Py<PyAny> = util.call_method0("get_loop")?.into_py(py);

                        // 这里假设是直接暴露了 util.loop
                        let loop_obj = util.getattr("loop")?;

                        let asyncio = PyModule::import(py, "asyncio")?;

                        // 生成协程 self.acheck_stream()
                        let instance = obj.bind(py).call1((remark, url))?;
                        let coro = instance.call_method0("acheck_stream")?;

                        // 调度到指定 loop
                        let fut = asyncio
                            .getattr("run_coroutine_threadsafe")?
                            .call1((coro, loop_obj))?;

                        let res = fut.call_method0("result")?;
                        let is_live = res.unbind().extract(py)?;
                        if is_live {
                            let self_obj = &instance;
                            // 从 self 上获取属性并抽取为 Rust 类型
                            let name: String = self_obj.getattr("fname")?.extract()?;
                            let url: String = self_obj.getattr("url")?.extract()?;
                            let raw_stream_url: String =
                                self_obj.getattr("raw_stream_url")?.extract()?;
                            let title: String = self_obj.getattr("room_title")?.extract()?;
                            let live_cover_path: Option<String> =
                                self_obj.getattr("live_cover_path")?.extract()?;
                            let _is_download: bool = self_obj.getattr("is_download")?.extract()?;
                            let platform: String = self_obj.getattr("platform")?.extract()?;

                            let stream_headers: HashMap<String, String> = if platform == "Huya" {
                                let stream_headers = self_obj.getattr("stream_headers")?;
                                self_obj.call_method1("update_headers", (&stream_headers,))?;
                                stream_headers.extract()?
                            } else {
                                self_obj.getattr("stream_headers")?.extract()?
                            };

                            let danmaku_init = self_obj.call_method0("danmaku_init")?;
                            // let platform: Option<PyAny> = self_obj.getattr("danmaku")?.extract()?;
                            // danmaku 可能在条件下没有设置（比如 bilibili_danmaku 为 False）
                            let self_danmaku = self_obj.getattr("danmaku")?;
                            let danmaku = if !self_danmaku.is_none() {
                                Some(self_danmaku.unbind())
                            } else {
                                None
                            };

                            Ok(Some((
                                StreamInfoExt {
                                    streamer_info: StreamerInfo {
                                        id: 0,
                                        name,
                                        url,
                                        title,
                                        date: Utc::now(),
                                        live_cover_path: live_cover_path.unwrap_or_default(),
                                    },
                                    suffix: media_ext_from_url(&raw_stream_url)
                                        .unwrap_or("flv".to_string()),
                                    raw_stream_url,
                                    platform,
                                    stream_headers,
                                },
                                danmaku,
                            )))
                        } else {
                            Ok(None)
                        }
                    },
                )
            })
            .await
            .change_context(AppError::Unknown)?
            .change_context(AppError::Unknown)?
            {
                Some((info, Some(danmaku))) => {
                    self.danmaku = Some(Arc::new(danmaku));
                    Some(info)
                }
                Some((info, None)) => Some(info),
                None => None,
            },
        )
    }
}

#[async_trait]
impl DownloadPlugin for PyPlugin {
    fn matches(&self, url: &str) -> bool {
        if self.pattern.is_match(url).unwrap() {
            // 找到匹配的部分
            if let Some(mat) = self.pattern.find(url).unwrap() {
                debug!("  匹配内容: {}", mat.as_str());
                return true;
            }
        }
        false
    }

    fn create_downloader(&self, ctx: &mut Context) -> Box<dyn DownloadBase> {
        let url = ctx.worker.live_streamer.url.to_string();
        let remark = ctx.worker.live_streamer.remark.to_string();
        Box::new(PyDownloader::new(self.plugin.clone(), url, remark))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl DownloadBase for PyDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        match self.call_via_threads().await? {
            Some(info) => Ok(StreamStatus::Live {
                stream_info: Box::new(info),
            }),
            None => Ok(StreamStatus::Offline),
        }
    }

    fn danmaku_init(&self) -> Option<Arc<dyn DanmakuClient + Send + Sync>> {
        if let Some(danmaku) = &self.danmaku {
            let danmaku = Arc::new(PyDanmakuClient::new(danmaku.clone()))
                as Arc<dyn DanmakuClient + Send + Sync>;
            // ctx.extension.insert(danmaku);
            Some(danmaku)
        } else {
            None
        }
    }
}

pub fn from_py() -> PyResult<Vec<PyPlugin>> {
    let classes: Vec<PyPlugin> = Python::attach(|py| -> PyResult<Vec<PyPlugin>> {
        let plugins = py.import("biliup.plugins")?;
        let decorators = py.import("biliup.engine.decorators")?;
        // 获取 Plugin 类
        let plugin_class = decorators.getattr("Plugin")?;

        let _instance = plugin_class.call1((plugins,))?;

        // 如果要获取类属性（而不是实例属性）
        let bound = plugin_class.getattr("download_plugins")?;
        let plugin_list: &Bound<PyList> = bound.cast()?;

        plugin_list
            .iter()
            .map(|x| {
                let download = x.cast::<PyType>()?;
                let py_plugin = PyPlugin::from_pytype(download)?;
                Ok(py_plugin)
            })
            .collect::<PyResult<_>>()
        // 基类 DownloadBase
        // py.import("biliup.engine.download")?.getattr("DownloadBase")
    })?;
    // println!("类属性值: {:?}", class_attr);

    // let download_base: &Bound<PyType> = download_base.downcast()?;

    Ok(classes)
}

#[pyclass]
struct ConfigState {
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

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        // 控制台输出
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_timer(local_time.clone())
                .with_file(true) // 打印文件名
                .with_line_number(true)
                .with_thread_ids(true),
        )
        // 文件输出
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_timer(local_time)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false), // .json() // 可选：使用 JSON 格式便于解析
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
        Commands::DumpFlv { file_name } => generate_json(file_name)?,
        Commands::Download {
            url,
            output,
            split_size,
            split_time,
        } => biliup_cli::downloader::download(&url, output, split_size, split_time).await?,
        Commands::Server { bind, port, auth } => {
            info!(
                "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
            );
            let conn_pool = ConnectionManager::new_pool("data/data.sqlite3")
                .await
                .expect("could not initialize the database connection pool");

            *CONFIG.write().unwrap() = repositories::get_config(&conn_pool).await?;
            let download_manager = DownloadManager::new(
                CONFIG.read().unwrap().pool1_size,
                CONFIG.read().unwrap().pool2_size,
                conn_pool.clone(),
            );
            let vec = from_py().unwrap();

            for v in vec {
                download_manager.add_plugin(Arc::new(v));
            }

            let service_register =
                ServiceRegister::new(conn_pool.clone(), CONFIG.clone(), download_manager);

            let all_streamer = repositories::get_all_streamer(&conn_pool).await?;

            for streamer in all_streamer {
                // workers.push(Arc::new(Worker::new(streamer.id, service_register.pool.clone())));
                let upload_config = get_upload_config(&conn_pool, streamer.id).await?;
                let url = streamer.url.clone();
                let worker = Worker::new(
                    streamer,
                    upload_config,
                    CONFIG.clone(),
                    service_register.client.clone(),
                );
                if service_register.managers.add_room(worker).await.is_none() {
                    warn!(url = url, "Could not add room to manager");
                }
            }

            info!("migrations successfully ran, initializing axum server...");
            let addr = (bind, port);
            let addr = addr
                .to_socket_addrs()
                .change_context(AppError::Unknown)?
                .next()
                .unwrap();
            ApplicationController::serve(&addr, auth, service_register)
                .await
                .attach("could not initialize application routes")?;
            // biliup_cli::run((&bind, port)).await?
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
