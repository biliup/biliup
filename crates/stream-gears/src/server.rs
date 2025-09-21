use crate::construct_headers;
use async_trait::async_trait;
use biliup::downloader::util::Segmentable;
use biliup_cli::server::app::ApplicationController;
use biliup_cli::server::config;
use biliup_cli::server::config::Config;
use biliup_cli::server::core::download_manager::{ActorHandle, DownloadManager};
use biliup_cli::server::core::downloader::ffmpeg_downloader::FfmpegDownloader;
use biliup_cli::server::core::downloader::stream_gears::StreamGears;
use biliup_cli::server::core::downloader::{DownloadConfig, Downloader, DownloaderType};
use biliup_cli::server::core::plugin::{DownloadPlugin, StreamInfo, StreamStatus};
use biliup_cli::server::errors::{AppError, AppResult};
use biliup_cli::server::infrastructure::connection_pool::ConnectionManager;
use biliup_cli::server::infrastructure::context::Worker;
use biliup_cli::server::infrastructure::repositories;
use biliup_cli::server::infrastructure::service_register::ServiceRegister;
use biliup_cli::server::util::{Recorder, media_ext_from_url, parse_time};
use error_stack::{Report, ResultExt};
use fancy_regex::Regex;
use futures::TryFutureExt;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::PyDictMethods;
use pyo3::prelude::{PyAnyMethods, PyListMethods, PyModule};
use pyo3::sync::OnceLockExt;
use pyo3::types::PyDict;
use pyo3::types::{PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult, Python};
use pyo3::{FromPyObject, pyclass, pyfunction, pymethods};
use pythonize::pythonize;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, RwLock};
use time::OffsetDateTime;
use tracing::{debug, info};

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

    async fn check_status(&self, url: &str) -> Result<StreamStatus, Report<AppError>> {
        info!("Checking status");
        match call_via_threads(self.plugin.clone(), url)
            .await
            .change_context(AppError::Unknown)?
        {
            Some(info) => Ok(StreamStatus::Live { stream_info: info }),
            None => Ok(StreamStatus::Offline),
        }
    }

    async fn create_downloader(
        &self,
        stream_info: &StreamInfo,
        worker: &Worker,
    ) -> AppResult<Box<dyn Downloader>> {
        let config = worker.get_config().await?;
        let streamer = worker.get_streamer().await?;
        // info!(stream_info=?stream_info, "Create downloader");
        let raw_stream_url = &stream_info.raw_stream_url;
        let suffix = streamer
            .format
            .unwrap_or_else(|| stream_info.suffix.clone());
        println!("suffix: {suffix}");
        let recorder = Recorder::new(
            streamer.filename_prefix,
            &streamer.remark,
            &stream_info.title,
            &suffix,
        );

        match config.downloader {
            Some(DownloaderType::Ffmpeg) => {
                let config = DownloadConfig {
                    format: suffix,
                    segment_time: config.segment_time,
                    file_size: Some(config.file_size), // 2GB
                    headers: stream_info.stream_headers.clone(),
                    extra_args: vec![],
                    downloader_type: DownloaderType::FfmpegInternal,
                    filename_prefix: recorder.generate_filename(),
                };

                Ok(Box::new(FfmpegDownloader::new(
                    raw_stream_url,
                    config,
                    PathBuf::from("./downloads"),
                )))
            }
            // Some(DownloaderType::StreamGears) => {
            //
            // },
            _ => Ok(Box::new(StreamGears::new(
                raw_stream_url,
                construct_headers(&stream_info.stream_headers),
                recorder.filename_template(),
                Segmentable::new(
                    config.segment_time.as_deref().map(parse_time),
                    Some(config.file_size),
                ),
                None,
            ))),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

async fn call_via_threads(obj: Arc<Py<PyType>>, url: &str) -> PyResult<Option<StreamInfo>> {
    let url = url.to_string();
    // obj.
    tokio::task::spawn_blocking(move || {
        Python::attach(|py| -> PyResult<Option<StreamInfo>> {
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
            let instance = obj.bind(py).call1(("fname", url))?;
            let coro = instance.call_method0("acheck_stream")?;

            // 调度到指定 loop
            let fut = asyncio
                .getattr("run_coroutine_threadsafe")?
                .call1((coro, loop_obj))?;

            let res = fut.call_method0("result")?;
            let is_live = res.unbind().extract(py)?;
            let info = if is_live {
                Some(stream_info_from_py(py, &instance)?)
            } else {
                None
            };
            Ok(info)
        })
    })
    .await
    .expect("spawn_blocking panicked")
}

pub fn from_py(actor_handle: Arc<ActorHandle>) -> PyResult<Vec<DownloadManager>> {
    let classes: Vec<DownloadManager> = Python::attach(|py| -> PyResult<Vec<DownloadManager>> {
        let plugins = py.import("biliup.plugins")?;
        let decorators = py.import("biliup.engine.decorators")?;
        // 获取 Plugin 类
        let plugin_class = decorators.getattr("Plugin")?;

        let instance = plugin_class.call1((plugins,))?;

        // 如果要获取类属性（而不是实例属性）
        let bound = plugin_class.getattr("download_plugins")?;
        let plugin_list: &Bound<PyList> = bound.downcast()?;

        plugin_list
            .iter()
            .map(|x| {
                let download = x.downcast::<PyType>()?;
                let py_plugin = PyPlugin::from_pytype(download)?;
                Ok(DownloadManager::new(py_plugin, actor_handle.clone()))
            })
            .collect::<PyResult<_>>()
        // 基类 DownloadBase
        // py.import("biliup.engine.download")?.getattr("DownloadBase")
    })?;
    // println!("类属性值: {:?}", class_attr);

    // let download_base: &Bound<PyType> = download_base.downcast()?;

    Ok(classes)
}

/// 从 Python 的 `self`（Bound<PyAny>）与 start_time / end_time 构造 StreamInfo
/// end_time 的语义与 Python 中一致：若未提供或为“假值”，则使用 time.localtime()
pub fn stream_info_from_py(py: Python<'_>, self_obj: &Bound<'_, PyAny>) -> PyResult<StreamInfo> {
    // 从 self 上获取属性并抽取为 Rust 类型
    let name: String = self_obj.getattr("fname")?.extract()?;
    let url: String = self_obj.getattr("url")?.extract()?;
    let raw_stream_url: String = self_obj.getattr("raw_stream_url")?.extract()?;
    let title: String = self_obj.getattr("room_title")?.extract()?;
    let live_cover_path: Option<String> = self_obj.getattr("live_cover_path")?.extract()?;
    let is_download: bool = self_obj.getattr("is_download")?.extract()?;
    let platform: String = self_obj.getattr("platform")?.extract()?;
    let stream_headers: HashMap<String, String> = self_obj.getattr("stream_headers")?.extract()?;
    self_obj.call_method1("update_headers", (&stream_headers,))?;

    // date 直接使用传入的 start_time（保留为 Python 对象）
    let date = OffsetDateTime::now_utc();
    // end_time: 若传入 None 或“假值”，则使用 time.localtime()
    // let end_time_obj: PyObject = match end_time {
    //     Some(et) if et.is_true()? => et.to_object(py),
    //     _ => {
    //         let time_mod = py.import("time")?;
    //         let lt = time_mod.getattr("localtime")?.call0()?;
    //         lt.to_object(py)
    //     }
    // };self.update_headers(self.stream_headers)

    Ok(StreamInfo {
        name,
        url,
        suffix: media_ext_from_url(&raw_stream_url).unwrap(),
        raw_stream_url,
        title,
        date,
        live_cover_path,
        platform,
        stream_headers,
    })
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
            return Ok(bound);
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
pub fn config_bindings(py: Python<'_>) -> PyResult<ConfigState> {
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
    &*CONFIG
}

#[pyfunction]
pub fn main_loop(py: Python<'_>) -> PyResult<()> {
    py.detach(_main)
        .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))

    // for item in plugin_list.iter() {
    //     // 每个元素都是一个类（type）
    //     let ty: &Bound<PyType> = item.downcast()?;
    //
    //     // 确认是否是 DownloadBase 的子类
    //     if ty.is_subclass(download_base)? {
    //         let name: String = ty.getattr("__name__")?.extract()?;
    //         println!("发现插件类: {name}");
    //
    //         // 如果要实例化（若不是抽象类且构造器无参）
    //         // let obj = ty.call0()?;
    //         // 或者需要参数：ty.call1((arg1, arg2,))?;
    //
    //         // 如果要调用方法（示例）
    //         // if let Ok(method) = obj.getattr("download") {
    //         //     let result = method.call0()?;
    //         //     println!("download() 返回: {}", result.repr()?.extract::<String>()?);
    //         // }
    //     }
    // }
}

#[tokio::main]
async fn _main() -> AppResult<()> {
    info!(
        "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
    );
    let conn_pool = ConnectionManager::new_pool("data/data.sqlite3")
        .await
        .expect("could not initialize the database connection pool");

    *CONFIG.write().unwrap() = repositories::get_config(&conn_pool).await?;
    let actor_handle = Arc::new(ActorHandle::new(
        CONFIG.read().unwrap().pool1_size,
        CONFIG.read().unwrap().pool2_size,
    ));
    let vec = from_py(actor_handle.clone()).unwrap();

    let service_register = ServiceRegister::new(conn_pool, CONFIG.clone(), actor_handle, vec);

    let all_streamer = repositories::get_all_streamer(&service_register.pool).await?;

    for streamer in all_streamer {
        // workers.push(Arc::new(Worker::new(streamer.id, service_register.pool.clone())));
        if let Some(manager) = service_register.get_manager(&streamer.url) {
            let monitor = manager.ensure_monitor();
            let _ = service_register
                .add_room(streamer.id, &streamer.url, monitor)
                .await?;
        };
    }

    info!("migrations successfully ran, initializing axum server...");
    let addr = ("0.0.0.0", 19159);
    let addr = addr
        .to_socket_addrs()
        .change_context(AppError::Unknown)?
        .next()
        .unwrap();
    ApplicationController::serve(&addr, service_register)
        .await
        .attach("could not initialize application routes")?;
    Ok(())
}
