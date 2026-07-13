use crate::server::core::downloader::DownloaderType;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::models::hook_step::HookStep;
use biliup::bilibili::Credit;
use error_stack::{ResultExt, bail};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, path::PathBuf};
use struct_patch::Patch;

/// 全局配置结构体
#[derive(bon::Builder, Debug, PartialEq, Clone, Serialize, Deserialize, Patch)]
#[patch(attribute(derive(Debug, Clone, Default, Deserialize, Serialize)))]
pub struct Config {
    // ===== 全局录播与上传设置 =====
    /// 下载器类型：streamlink | ffmpeg | stream-gears | 自定义
    #[serde(default)]
    pub downloader: Option<DownloaderType>,

    /// 文件大小限制（字节）
    #[patch(attribute(serde(default, deserialize_with = "deserialize_option_patch")))]
    #[serde(default = "default_file_size")]
    pub file_size: Option<u64>,

    /// 分段时间，格式如 "00:00:00"，保留为字符串以保持直观
    #[serde(default)]
    pub segment_time: Option<String>,

    /// 过滤阈值（MB）
    #[builder(default = default_filtering_threshold())]
    #[serde(default = "default_filtering_threshold")]
    pub filtering_threshold: u64,

    /// 文件名前缀
    #[serde(default)]
    pub filename_prefix: Option<String>,

    /// 分段处理器是否并行执行
    #[serde(default)]
    pub segment_processor_parallel: Option<bool>,

    /// 上传器类型：Noop | bili_web | biliup-rs | 其他
    #[serde(default)]
    pub uploader: Option<String>,

    /// 提交API类型：web | client
    #[serde(default)]
    pub submit_api: Option<String>,

    /// 上传线路：AUTO | alia | bda2 | bldsa | qn | tx | txa
    #[builder(default = default_lines())]
    #[serde(default = "default_lines")]
    pub lines: String,

    /// 上传线程数
    #[builder(default = default_threads())]
    #[serde(default = "default_threads")]
    pub threads: u32,

    /// 延迟时间（秒）
    #[builder(default = default_delay())]
    #[serde(default = "default_delay")]
    pub delay: u64,

    /// 事件循环间隔（秒）
    #[builder(default = default_event_loop_interval())]
    #[serde(default = "default_event_loop_interval")]
    pub event_loop_interval: u64,

    /// 检查器休眠时间（秒）
    #[builder(default = default_checker_sleep())]
    #[serde(default = "default_checker_sleep")]
    pub checker_sleep: u64,

    /// 连接池1大小
    #[builder(default = default_pool1_size())]
    #[serde(default = "default_pool1_size")]
    pub pool1_size: u32,

    /// 连接池2大小
    #[builder(default = default_pool2_size())]
    #[serde(default = "default_pool2_size")]
    pub pool2_size: u32,

    // ===== 各平台录播设置 =====
    /// 是否使用直播封面
    #[serde(default)]
    pub use_live_cover: Option<bool>,

    // 斗鱼平台设置
    /// 斗鱼CDN节点
    #[serde(default)]
    pub douyu_cdn: Option<String>,
    /// 斗鱼强制 hs 流使用构造链接
    #[serde(default)]
    pub douyu_force_hs: Option<bool>,
    /// 斗鱼弹幕录制
    #[serde(default)]
    pub douyu_danmaku: Option<bool>,
    /// 斗鱼码率
    #[serde(default)]
    pub douyu_rate: Option<u32>,
    /// 斗鱼互动游戏运行时跳过录制
    #[serde(default)]
    pub douyu_disable_interactive_game: Option<bool>,

    // 虎牙平台设置
    /// 虎牙CDN节点
    #[serde(default)]
    pub huya_cdn: Option<String>,
    /// 虎牙CDN回退
    #[serde(default)]
    pub huya_cdn_fallback: Option<bool>,
    /// 虎牙弹幕录制
    #[serde(default)]
    pub huya_danmaku: Option<bool>,
    /// 虎牙最大比率
    #[serde(default)]
    pub huya_max_ratio: Option<u32>,
    /// 虎牙 Flv or Hls
    #[serde(default)]
    pub huya_protocol: Option<String>,
    /// 虎牙是否保留 imgplus 流名
    #[serde(default)]
    pub huya_imgplus: Option<bool>,
    /// 虎牙走小程序 API 获取房间信息
    #[serde(default)]
    pub huya_mobile_api: Option<bool>,
    /// 虎牙编码参数
    #[serde(default)]
    pub huya_codec: Option<String>,

    // 抖音平台设置
    /// 抖音弹幕录制
    #[serde(default)]
    pub douyin_danmaku: Option<bool>,
    /// 抖音画质
    #[serde(default)]
    pub douyin_quality: Option<String>,
    /// 抖音直播协议：flv 或 hls
    #[serde(default)]
    pub douyin_protocol: Option<String>,
    /// 双屏直播录制方式
    #[serde(default)]
    pub douyin_double_screen: Option<bool>,
    /// 抖音真原画
    #[serde(default)]
    pub douyin_true_origin: Option<bool>,

    // 快手平台设置
    /// 快手Cookie
    #[serde(default)]
    pub kuaishou_cookie: Option<String>,

    // 网易 CC 平台设置
    /// 直播协议：hls 或 flv
    #[serde(default)]
    pub cc_protocol: Option<String>,

    // Kilakila 平台设置
    /// 直播协议：hls 或 flv
    #[serde(default)]
    pub kila_protocol: Option<String>,

    // 哔哩哔哩平台设置
    /// B站弹幕录制
    #[serde(default)]
    pub bilibili_danmaku: Option<bool>,
    /// B站弹幕详细信息
    #[serde(default)]
    pub bilibili_danmaku_detail: Option<bool>,
    /// B站弹幕原始数据
    #[serde(default)]
    pub bilibili_danmaku_raw: Option<bool>,
    /// B站协议类型：stream | hls_ts | hls_fmp4
    #[serde(default)]
    pub bili_protocol: Option<String>,
    /// B站CDN节点列表
    #[serde(default)]
    pub bili_cdn: Option<Vec<String>>,
    /// B站强制原画
    #[serde(default)]
    pub bili_force_source: Option<bool>,
    /// B站直播API
    #[serde(default)]
    pub bili_liveapi: Option<String>,
    /// B站回退API
    #[serde(default)]
    pub bili_fallback_api: Option<String>,
    /// B站CDN回退
    #[serde(default)]
    pub bili_cdn_fallback: Option<bool>,
    /// B站 hls_fmp4 转码等待时间（秒）
    #[serde(default)]
    pub bili_hls_transcode_timeout: Option<u64>,
    /// B站cn01节点替换
    #[serde(default)]
    pub bili_replace_cn01: Option<Vec<String>>,
    /// B站画质编号
    #[serde(default)]
    pub bili_qn: Option<u32>,
    /// B站免登录原画
    #[serde(default)]
    pub bili_anonymous_origin: Option<bool>,

    // YouTube平台设置
    /// YouTube首选视频编码
    #[serde(default)]
    pub youtube_prefer_vcodec: Option<String>,
    /// YouTube首选音频编码
    #[serde(default)]
    pub youtube_prefer_acodec: Option<String>,
    /// YouTube最大分辨率
    #[serde(default)]
    pub youtube_max_resolution: Option<u32>,
    /// YouTube最大视频大小
    #[serde(default)]
    pub youtube_max_videosize: Option<String>,
    /// YouTube开始日期
    #[serde(default)]
    pub youtube_after_date: Option<String>,
    /// YouTube结束日期
    #[serde(default)]
    pub youtube_before_date: Option<String>,
    /// YouTube启用直播下载
    #[serde(default)]
    pub youtube_enable_download_live: Option<bool>,
    /// YouTube启用回放下载
    #[serde(default)]
    pub youtube_enable_download_playback: Option<bool>,
    /// YouTube弹幕录制
    #[serde(default)]
    pub youtube_danmaku: Option<bool>,
    /// 兼容旧版配置的 YouTube 弹幕录制字段
    #[serde(default)]
    pub ytb_danmaku: Option<bool>,

    // Twitch平台设置
    /// Twitch弹幕录制
    #[serde(default)]
    pub twitch_danmaku: Option<bool>,
    /// Twitch禁用广告
    #[serde(default)]
    pub twitch_disable_ads: Option<bool>,

    // TwitCasting平台设置
    /// TwitCasting弹幕录制
    #[serde(default)]
    pub twitcasting_danmaku: Option<bool>,
    /// TwitCasting密码
    #[serde(default)]
    pub twitcasting_password: Option<String>,
    /// TwitCasting画质 high | medium | low
    #[serde(default)]
    pub twitcasting_quality: Option<String>,

    /// 录制主播配置映射
    #[serde(default)]
    pub streamers: HashMap<String, StreamerConfig>,

    /// 用户Cookie配置
    #[serde(default)]
    pub user: Option<UserConfig>,

    pub loggers_level: Option<String>,
}

/// 主播配置结构体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreamerConfig {
    /// 直播间URL列表
    pub url: Vec<String>,

    /// 视频标题
    #[serde(default)]
    pub title: Option<String>,

    /// 分区ID
    #[serde(default)]
    pub tid: Option<u32>,

    /// 版权类型
    #[serde(default)]
    pub copyright: Option<u8>,

    /// 转载来源
    #[serde(default)]
    pub copyright_source: Option<String>,

    /// 封面路径
    #[serde(default)]
    pub cover_path: Option<PathBuf>,

    /// 视频描述（保留缩进和多行格式）
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
    pub up_selection_reply: Option<u8>,

    #[serde(default)]
    pub up_close_reply: Option<u8>,

    #[serde(default)]
    pub up_close_danmu: Option<u8>,

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
    pub extra_fields: Option<String>,

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

/// 用户配置结构体
#[derive(bon::Builder, PartialEq, Debug, Clone, Serialize, Deserialize, Default, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize)))]
pub struct UserConfig {
    // B站配置
    /// B站Cookie字符串
    #[serde(default)]
    pub bili_cookie: Option<String>,
    /// B站Cookie文件路径
    #[serde(default)]
    pub bili_cookie_file: Option<PathBuf>,

    // 抖音配置
    /// 抖音Cookie
    #[serde(default)]
    pub douyin_cookie: Option<String>,

    // Twitch配置
    /// Twitch Cookie
    #[serde(default)]
    pub twitch_cookie: Option<String>,

    // TwitCasting配置
    /// TwitCasting Cookie
    #[serde(default)]
    pub twitcasting_cookie: Option<String>,

    // YouTube配置
    /// YouTube Cookie文件路径
    #[serde(default)]
    pub youtube_cookie: Option<PathBuf>,

    // Niconico配置（使用rename保持与配置文件一致）
    /// Niconico邮箱
    #[serde(rename = "niconico-email", default)]
    pub niconico_email: Option<String>,
    /// Niconico密码
    #[serde(rename = "niconico-password", default)]
    pub niconico_password: Option<String>,
    /// Niconico用户会话
    #[serde(rename = "niconico-user-session", default)]
    pub niconico_user_session: Option<String>,
    /// Niconico清除凭据
    #[serde(rename = "niconico-purge-credentials", default)]
    pub niconico_purge_credentials: Option<String>,

    // AfreecaTV配置
    /// AfreecaTV用户名
    #[serde(default)]
    pub afreecatv_username: Option<String>,
    /// AfreecaTV密码
    #[serde(default)]
    pub afreecatv_password: Option<String>,
}

/// 默认文件大小：2.5GB
fn default_file_size() -> Option<u64> {
    Some(2_621_440_000)
}

fn deserialize_option_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// 默认分段时间：不启用时长分段
pub fn default_segment_time() -> Option<String> {
    None
}

/// 默认过滤阈值：20MB
fn default_filtering_threshold() -> u64 {
    20
}

/// 默认上传线路：自动选择
fn default_lines() -> String {
    "AUTO".to_string()
}

/// 默认线程数：3
fn default_threads() -> u32 {
    3
}

/// 默认延迟：300秒
fn default_delay() -> u64 {
    300
}

/// 默认事件循环间隔：30秒
fn default_event_loop_interval() -> u64 {
    30
}

/// 默认检查器休眠时间：10秒
fn default_checker_sleep() -> u64 {
    10
}

/// 默认连接池1大小：5
fn default_pool1_size() -> u32 {
    5
}

/// 默认连接池2大小：3
fn default_pool2_size() -> u32 {
    3
}

impl Default for Config {
    fn default() -> Self {
        serde_json::from_value(serde_json::json!({})).expect("default config should deserialize")
    }
}

impl Config {
    pub fn validate_segment_limits(&self) -> AppResult<()> {
        Ok(())
    }

    pub fn normalize_segment_limits(&mut self) {
        if self
            .segment_time
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            self.segment_time = None;
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("read config {}", path.display()))?;
        let extension = path.extension().and_then(|ext| ext.to_str());
        let mut config: Config = match extension {
            Some("toml") => toml::from_str(&contents)
                .change_context(AppError::Unknown)
                .attach_with(|| format!("parse toml config {}", path.display()))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&contents)
                .change_context(AppError::Unknown)
                .attach_with(|| format!("parse yaml config {}", path.display()))?,
            _ => bail!(AppError::Custom(format!(
                "unsupported config file extension: {}",
                path.display()
            ))),
        };
        config.normalize_segment_limits();
        config.validate_segment_limits()?;
        Ok(config)
    }

    /// 从指定路径加载配置文件，如果不存在则创建默认配置
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        Self::load(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_missing_file_size_uses_default() {
        let config: Config = serde_json::from_str(r#"{}"#).unwrap();

        assert_eq!(config.file_size, default_file_size());
        assert_eq!(config.segment_time, None);
        assert!(config.validate_segment_limits().is_ok());
    }

    #[test]
    fn deserialize_null_file_size_keeps_none() {
        let config: Config =
            serde_json::from_str(r#"{"file_size": null, "segment_time": "01:00:00"}"#).unwrap();

        assert_eq!(config.file_size, None);
        assert_eq!(config.segment_time, Some("01:00:00".to_string()));
        assert!(config.validate_segment_limits().is_ok());
    }

    #[test]
    fn size_or_time_segment_limit_is_valid() {
        let mut size_only = Config {
            file_size: Some(1024),
            segment_time: None,
            ..Config::default()
        };
        assert!(size_only.validate_segment_limits().is_ok());

        let mut time_only = Config {
            file_size: None,
            segment_time: Some("01:00:00".to_string()),
            ..Config::default()
        };
        assert!(time_only.validate_segment_limits().is_ok());

        time_only.segment_time = Some("".to_string());
        time_only.normalize_segment_limits();
        assert_eq!(time_only.segment_time, None);
        assert!(time_only.validate_segment_limits().is_ok());

        size_only.segment_time = Some("00:30:00".to_string());
        assert!(size_only.validate_segment_limits().is_ok());
    }

    #[test]
    fn empty_size_and_time_disables_segmentation() {
        let mut config = Config {
            file_size: None,
            segment_time: Some("".to_string()),
            ..Config::default()
        };

        config.normalize_segment_limits();

        assert_eq!(config.file_size, None);
        assert_eq!(config.segment_time, None);
        assert!(config.validate_segment_limits().is_ok());
    }

    #[test]
    fn config_patch_can_clear_file_size() {
        let mut config = Config::default();
        let patch: ConfigPatch =
            serde_json::from_str(r#"{"file_size": null, "segment_time": "01:00:00"}"#).unwrap();

        config.apply(patch);

        assert_eq!(config.file_size, None);
        assert_eq!(config.segment_time, Some("01:00:00".to_string()));
        assert!(config.validate_segment_limits().is_ok());
    }
}
