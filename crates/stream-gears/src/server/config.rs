use crate::server::core::downloader::DownloaderType;
use crate::server::infrastructure::models::HookStep;
use anyhow::{Context, bail};
use pyo3::prelude::PyDictMethods;
use pyo3::sync::OnceLockExt;
use pyo3::types::{PyAnyMethods, PyDict};
use pyo3::{Bound, FromPyObject, PyAny, PyResult, Python, pyclass, pyfunction, pymethods};
use pythonize::pythonize;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::{Arc, OnceLock, RwLock};
use std::{collections::HashMap, path::Path, path::PathBuf};

#[derive(bon::Builder, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ===== 全局录播与上传设置 =====
    #[serde(default)]
    pub downloader: Option<DownloaderType>, // streamlink | ffmpeg | stream-gears | 自定义

    #[builder(default = default_file_size())]
    #[serde(default = "default_file_size")]
    pub file_size: u64, // Byte

    // 形如 "00:00:00"；保留为字符串以保持直观（你也可以替换为Duration并写自定义反序列化）
    #[serde(default)]
    pub segment_time: Option<String>,

    #[builder(default = default_filtering_threshold())]
    #[serde(default = "default_filtering_threshold")]
    pub filtering_threshold: u64, // MB

    #[serde(default)]
    pub filename_prefix: Option<String>,

    #[serde(default)]
    pub segment_processor_parallel: Option<bool>,

    #[serde(default)]
    pub uploader: Option<String>, // Noop | bili_web | biliup-rs | 其他

    #[serde(default)]
    pub submit_api: Option<String>, // web | client

    #[builder(default = default_lines())]
    #[serde(default = "default_lines")]
    pub lines: String, // AUTO | alia | bda2 | bldsa | qn | tx | txa

    #[builder(default = default_threads())]
    #[serde(default = "default_threads")]
    pub threads: u32,

    #[builder(default = default_delay())]
    #[serde(default = "default_delay")]
    pub delay: u64,

    #[builder(default = default_event_loop_interval())]
    #[serde(default = "default_event_loop_interval")]
    pub event_loop_interval: u64,

    #[builder(default = default_checker_sleep())]
    #[serde(default = "default_checker_sleep")]
    pub checker_sleep: u64,

    #[builder(default = default_pool1_size())]
    #[serde(default = "default_pool1_size")]
    pub pool1_size: u32,

    #[builder(default = default_pool2_size())]
    #[serde(default = "default_pool2_size")]
    pub pool2_size: u32,

    // ===== 各平台录播设置（顶层分散字段） =====
    #[serde(default)]
    pub use_live_cover: Option<bool>,

    // 斗鱼
    #[serde(default)]
    pub douyu_cdn: Option<String>,
    #[serde(default)]
    pub douyu_danmaku: Option<bool>,
    #[serde(default)]
    pub douyu_rate: Option<u32>,

    // 虎牙
    #[serde(default)]
    pub huya_cdn: Option<String>,
    #[serde(default)]
    pub huya_cdn_fallback: Option<bool>,
    #[serde(default)]
    pub huya_danmaku: Option<bool>,
    #[serde(default)]
    pub huya_max_ratio: Option<u32>,

    // 抖音
    #[serde(default)]
    pub douyin_danmaku: Option<bool>,
    #[serde(default)]
    pub douyin_quality: Option<String>,

    // 哔哩哔哩
    #[serde(default)]
    pub bilibili_danmaku: Option<bool>,
    #[serde(default)]
    pub bilibili_danmaku_detail: Option<bool>,
    #[serde(default)]
    pub bilibili_danmaku_raw: Option<bool>,
    #[serde(default)]
    pub bili_protocol: Option<String>, // stream | hls_ts | hls_fmp4
    #[serde(default)]
    pub bili_cdn: Option<Vec<String>>,
    #[serde(default)]
    pub bili_force_source: Option<bool>,
    #[serde(default)]
    pub bili_liveapi: Option<String>,
    #[serde(default)]
    pub bili_fallback_api: Option<String>,
    #[serde(default)]
    pub bili_cdn_fallback: Option<bool>,
    #[serde(default)]
    pub bili_replace_cn01: Option<Vec<String>>,
    #[serde(default)]
    pub bili_qn: Option<u32>,

    // YouTube
    #[serde(default)]
    pub youtube_prefer_vcodec: Option<String>,
    #[serde(default)]
    pub youtube_prefer_acodec: Option<String>,
    #[serde(default)]
    pub youtube_max_resolution: Option<u32>,
    #[serde(default)]
    pub youtube_max_videosize: Option<String>,
    #[serde(default)]
    pub youtube_after_date: Option<String>,
    #[serde(default)]
    pub youtube_before_date: Option<String>,
    #[serde(default)]
    pub youtube_enable_download_live: Option<bool>,
    #[serde(default)]
    pub youtube_enable_download_playback: Option<bool>,

    // Twitch
    #[serde(default)]
    pub twitch_danmaku: Option<bool>,
    #[serde(default)]
    pub twitch_disable_ads: Option<bool>,

    // TwitCasting
    #[serde(default)]
    pub twitcasting_danmaku: Option<bool>,
    #[serde(default)]
    pub twitcasting_password: Option<String>,

    // 录制主播设置
    #[serde(default)]
    pub streamers: HashMap<String, StreamerConfig>,

    // 用户 cookie
    #[serde(default)]
    pub user: Option<UserConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamerConfig {
    pub url: Vec<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub tid: Option<u32>,

    #[serde(default)]
    pub copyright: Option<u8>,

    #[serde(default)]
    pub cover_path: Option<PathBuf>,

    // 用字符串保留缩进和多行
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub credits: Option<Vec<Credit>>,

    #[serde(default)]
    pub dynamic: Option<String>,

    #[serde(default)]
    pub dtime: Option<u64>,

    #[serde(default)]
    pub dolby: Option<u8>,

    #[serde(default)]
    pub hires: Option<u8>,

    #[serde(default)]
    pub charging_pay: Option<u8>,

    #[serde(default)]
    pub no_reprint: Option<u8>,

    #[serde(default)]
    pub is_only_self: Option<u8>,

    #[serde(default)]
    pub uploader: Option<String>,

    #[serde(default)]
    pub filename_prefix: Option<String>,

    #[serde(default)]
    pub user_cookie: Option<String>,

    #[serde(default)]
    pub use_live_cover: Option<bool>,

    #[serde(default)]
    pub tags: Option<Vec<String>>,

    #[serde(default)]
    pub time_range: Option<String>,

    #[serde(default)]
    pub excluded_keywords: Option<Vec<String>>,

    #[serde(default)]
    pub preprocessor: Option<Vec<HookStep>>,

    #[serde(default)]
    pub segment_processor: Option<Vec<HookStep>>,

    #[serde(default)]
    pub downloaded_processor: Option<Vec<HookStep>>,

    #[serde(default)]
    pub postprocessor: Option<Vec<HookStep>>,

    #[serde(default)]
    pub format: Option<String>,

    #[serde(default)]
    pub opt_args: Option<Vec<String>>,

    // “override” 是字段名，这里改为 override_cfg 避免与保留字混淆
    #[serde(rename = "override", default)]
    pub override_cfg: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credit {
    pub username: String,
    pub uid: String,
}

#[derive(bon::Builder, FromPyObject, Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    // B 站
    #[serde(default)]
    pub bili_cookie: Option<String>,
    #[serde(default)]
    pub bili_cookie_file: Option<PathBuf>,

    // 抖音
    #[serde(default)]
    pub douyin_cookie: Option<String>,

    // Twitch
    #[serde(default)]
    pub twitch_cookie: Option<String>,

    // YouTube
    #[serde(default)]
    pub youtube_cookie: Option<PathBuf>,

    // NICO（使用了包含破折号的 key，使用 rename 保持一致）
    #[serde(rename = "niconico-email", default)]
    pub niconico_email: Option<String>,
    #[serde(rename = "niconico-password", default)]
    pub niconico_password: Option<String>,
    #[serde(rename = "niconico-user-session", default)]
    pub niconico_user_session: Option<String>,
    #[serde(rename = "niconico-purge-credentials", default)]
    pub niconico_purge_credentials: Option<String>,

    // AfreecaTV
    #[serde(default)]
    pub afreecatv_username: Option<String>,
    #[serde(default)]
    pub afreecatv_password: Option<String>,
}

fn default_file_size() -> u64 {
    2_621_440_000
}
fn default_filtering_threshold() -> u64 {
    20
}
fn default_lines() -> String {
    "AUTO".to_string()
}
fn default_threads() -> u32 {
    3
}
fn default_delay() -> u64 {
    300
}
fn default_event_loop_interval() -> u64 {
    30
}
fn default_checker_sleep() -> u64 {
    10
}
fn default_pool1_size() -> u32 {
    5
}
fn default_pool2_size() -> u32 {
    3
}
fn default_check_sourcecode() -> u64 {
    15
}

impl Config {
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        bail!("load_or_create: {:?}", path.as_ref().display())
    }
}

// 进程级全局单例（安全）：OnceLock + Arc + RwLock
pub(crate) static CONFIG: OnceLock<Arc<RwLock<Config>>> = OnceLock::new();

fn cfg_arc() -> &'static Arc<RwLock<Config>> {
    CONFIG.get().expect("Config not initialized")
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
