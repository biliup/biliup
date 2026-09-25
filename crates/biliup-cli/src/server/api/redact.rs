//! 给非超管看的配置与主播数据脱敏。
//!
//! 配置按字段**白名单**输出：不在名单里的键一律置 `null`，所以新增的配置字段默认不外露，
//! 要让操作员 / 只读用户看到必须显式加进来。

use serde_json::Value;

/// 非超管可见的全局配置字段：都是录制与投稿参数，不含凭据、路径形式的凭据或钩子。
const VISIBLE_CONFIG_KEYS: &[&str] = &[
    "downloader",
    "sync_save_dir",
    "ffmpeg_path",
    "file_size",
    "segment_time",
    "filtering_threshold",
    "filename_prefix",
    "segment_processor_parallel",
    "uploader",
    "submit_api",
    "lines",
    "threads",
    "delay",
    "event_loop_interval",
    "checker_sleep",
    "pool1_size",
    "pool2_size",
    "live_merge_minutes",
    "retention_hours",
    "min_free_space",
    "preview_transport",
    "use_live_cover",
    "douyu_cdn",
    "douyu_force_hs",
    "douyu_danmaku",
    "douyu_rate",
    "douyu_disable_interactive_game",
    "huya_cdn",
    "huya_cdn_fallback",
    "huya_danmaku",
    "huya_max_ratio",
    "huya_protocol",
    "huya_imgplus",
    "huya_mobile_api",
    "huya_codec",
    "douyin_danmaku",
    "douyin_quality",
    "douyin_protocol",
    "douyin_double_screen",
    "douyin_true_origin",
    "cc_protocol",
    "kila_protocol",
    "bilibili_danmaku",
    "bilibili_danmaku_detail",
    "bilibili_danmaku_raw",
    "bili_protocol",
    "bili_cdn",
    "bili_force_source",
    "bili_liveapi",
    "bili_fallback_api",
    "bili_cdn_fallback",
    "bili_hls_transcode_timeout",
    "bili_replace_cn01",
    "bili_qn",
    "bili_anonymous_origin",
    "youtube_prefer_vcodec",
    "youtube_prefer_acodec",
    "youtube_max_resolution",
    "youtube_max_videosize",
    "youtube_after_date",
    "youtube_before_date",
    "youtube_enable_download_live",
    "youtube_enable_download_playback",
    "youtube_danmaku",
    "ytb_danmaku",
    "twitch_danmaku",
    "twitch_disable_ads",
    "twitcasting_danmaku",
    "twitcasting_quality",
    "loggers_level",
];

/// 主播上等同服务器 shell 的字段：没有 `streamer.hooks` 的人看不到，保存时也改不动。
pub const STREAMER_HOOK_KEYS: &[&str] = &[
    "override",
    "preprocessor",
    "segment_processor",
    "downloaded_processor",
    "postprocessor",
];

pub fn config(config: &impl serde::Serialize) -> Value {
    let mut value = serde_json::to_value(config).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut value {
        for (key, field) in map.iter_mut() {
            if !VISIBLE_CONFIG_KEYS.contains(&key.as_str()) {
                *field = Value::Null;
            }
        }
    }
    value
}

pub fn streamer_hooks(streamer: &mut Value) {
    if let Value::Object(map) = streamer {
        for key in STREAMER_HOOK_KEYS {
            if let Some(field) = map.get_mut(*key) {
                *field = Value::Null;
            }
        }
    }
}

pub fn upload_template(template: &mut Value) {
    if let Value::Object(map) = template
        && let Some(field) = map.get_mut("user_cookie")
    {
        *field = Value::Null;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::{Config, UserConfig};

    #[test]
    fn secrets_are_hidden_and_ordinary_settings_are_kept() {
        let config = Config {
            kuaishou_cookie: Some("ks-secret".into()),
            twitcasting_password: Some("tc-secret".into()),
            preview_transport: Some(crate::server::config::PreviewTransport::Direct),
            user: Some(UserConfig {
                bili_cookie: Some("SESSDATA=secret".into()),
                douyin_cookie: Some("dy-secret".into()),
                niconico_password: Some("nico-secret".into()),
                ..Default::default()
            }),
            ..Config::default()
        };
        let redacted = super::config(&config);
        let text = redacted.to_string();
        for secret in [
            "ks-secret",
            "tc-secret",
            "SESSDATA",
            "dy-secret",
            "nico-secret",
        ] {
            assert!(!text.contains(secret), "{secret} leaked: {text}");
        }
        assert_eq!(redacted["user"], Value::Null);
        assert_eq!(redacted["streamers"], Value::Null);
        assert_eq!(redacted["preview_transport"], "direct");
        assert_eq!(redacted["pool1_size"], 5);
        // 键本身保留，前端按同一套字段渲染
        assert!(
            redacted
                .as_object()
                .unwrap()
                .contains_key("kuaishou_cookie")
        );
    }

    #[test]
    fn unknown_fields_default_to_hidden() {
        let redacted = super::config(&serde_json::json!({
            "pool1_size": 5,
            "some_future_token": "secret"
        }));
        assert_eq!(redacted["pool1_size"], 5);
        assert_eq!(redacted["some_future_token"], Value::Null);
    }

    #[test]
    fn streamer_hooks_are_removed() {
        let mut streamer = serde_json::json!({
            "id": 1,
            "remark": "a",
            "override": {"downloader": "ffmpeg"},
            "postprocessor": [{"run": "rclone copy"}],
        });
        streamer_hooks(&mut streamer);
        assert_eq!(streamer["override"], Value::Null);
        assert_eq!(streamer["postprocessor"], Value::Null);
        assert_eq!(streamer["remark"], "a");
    }
}
