use crate::construct_headers;
use crate::server::core::download_manager::{ActorHandle, DownloadManager};
use crate::server::core::downloader::ffmpeg_downloader::FfmpegDownloader;
use crate::server::core::downloader::stream_gears::StreamGears;
use crate::server::core::downloader::{DownloadConfig, Downloader, DownloaderType};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Worker;
use crate::server::util::{Recorder, media_ext_from_url, parse_time};
use async_trait::async_trait;
use biliup::downloader::util::Segmentable;
use error_stack::{Report, ResultExt};
use fancy_regex::Regex;
use pyo3::prelude::{PyAnyMethods, PyListMethods, PyModule};
use pyo3::types::{PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult, Python};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{debug, info};

// Stream information structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub name: String,
    pub url: String,
    pub raw_stream_url: String,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime, // 保存 Python 的时间对象（如 time.struct_time）
    // pub end_time: PyObject,   // 同上
    pub live_cover_path: Option<String>,
    pub platform: String,
    pub stream_headers: HashMap<String, String>,
    pub suffix: String,
}

impl StreamInfo {
    /// 从 Python 的 `self`（Bound<PyAny>）与 start_time / end_time 构造 StreamInfo
    /// end_time 的语义与 Python 中一致：若未提供或为“假值”，则使用 time.localtime()
    pub fn from_py(py: Python<'_>, self_obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        // 从 self 上获取属性并抽取为 Rust 类型
        let name: String = self_obj.getattr("fname")?.extract()?;
        let url: String = self_obj.getattr("url")?.extract()?;
        let raw_stream_url: String = self_obj.getattr("raw_stream_url")?.extract()?;
        let title: String = self_obj.getattr("room_title")?.extract()?;
        let live_cover_path: Option<String> = self_obj.getattr("live_cover_path")?.extract()?;
        let is_download: bool = self_obj.getattr("is_download")?.extract()?;
        let platform: String = self_obj.getattr("platform")?.extract()?;
        let stream_headers: HashMap<String, String> =
            self_obj.getattr("stream_headers")?.extract()?;
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

        Ok(Self {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamStatus {
    Live { stream_info: StreamInfo },
    Offline,
    Unknown,
}

#[async_trait]
pub trait DownloadBase: Send + Sync {
    async fn check_stream(&self) -> Result<bool, Report<AppError>>;
    async fn get_stream_info(&self) -> Result<StreamInfo, Report<AppError>>;
    async fn download(&self, output_path: impl AsRef<Path>) -> Result<(), Report<AppError>>;
    fn should_record(&self, room_title: &str) -> bool;
    fn get_platform_name(&self) -> &'static str;
}

#[async_trait]
pub trait DownloadPlugin {
    fn matches(&self, url: &str) -> bool;
    async fn check_status(&self, url: &str) -> Result<StreamStatus, Report<AppError>>;
    async fn create_downloader(
        &self,
        stream_info: &StreamInfo,
        worker: &Worker,
    ) -> AppResult<Box<dyn Downloader>>;

    fn name(&self) -> &str;
}

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
                Some(StreamInfo::from_py(py, &instance)?)
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
