//! 自动切片的全局配置（`Config::auto_clip`）、API key 的掩码回显与保存时的保留规则。
//!
//! 配置里的 key 明文存在 `configuration` 表（与各平台 Cookie 同一处），但所有返回配置的接口
//! 只给掩码（`sk-…abcd`）。前端原样把掩码交回来时保留库里的原值；为了不让掩码成为把原 key
//! 发往别处的通道，只有这把 key 要发往的地址没有变多时才保留，否则要求重新填写完整的 key。

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::Duration;

/// 覆盖 chat 的 `api_key`（Docker 用 secret 注入时不必把 key 写进配置）。
pub const API_KEY_ENV: &str = "BILIUP_AUTO_CLIP_API_KEY";

pub const DEFAULT_MAX_ASR_MINUTES: u64 = 300;
pub const DEFAULT_MAX_CHAT_TOKENS: u64 = 300_000;
pub const DEFAULT_CHAT_TIMEOUT_SECS: u64 = 180;
pub const DEFAULT_ASR_TIMEOUT_SECS: u64 = 300;

/// 掩码里的省略号。真实的 API key 不会含这个字符，所以收到含它的值就当作「没改」。
const MASK_MARK: char = '…';

/// 缩图开关：`auto` 按连通性测试的结果，模型能看图才开。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Thumbnails {
    #[default]
    Auto,
    On,
    Off,
}

/// `[auto_clip]`：缺省或 `enabled = false` 时什么都不跑。`Debug` 只打 key 的掩码。
#[derive(bon::Builder, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AutoClipConfig {
    /// 总开关
    #[builder(default)]
    #[serde(default)]
    pub enabled: bool,
    /// OpenAI 兼容接口的地址，一般以 `/v1` 结尾
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub chat_model: Option<String>,
    /// 转写服务的地址；空 = 同 `base_url`
    #[serde(default)]
    pub asr_base_url: Option<String>,
    /// 转写服务的 key；空 = 同 `api_key`
    #[serde(default)]
    pub asr_api_key: Option<String>,
    #[serde(default)]
    pub asr_model: Option<String>,
    /// 传给 transcriptions 的 `language`；空 = 让服务自己识别
    #[serde(default)]
    pub asr_language: Option<String>,
    /// 传给 transcriptions 的 `prompt`（热词：主播名、游戏名、梗）
    #[serde(default)]
    pub asr_prompt: Option<String>,
    #[serde(default)]
    pub thumbnails: Option<Thumbnails>,
    /// 每场送转写的分钟数上限；空 = 300
    #[serde(default)]
    pub max_asr_minutes: Option<u64>,
    /// 每场 chat 的 token 上限（输入 + 输出）；空 = 30 万
    #[serde(default)]
    pub max_chat_tokens: Option<u64>,
    /// 单次 chat 请求的超时秒数；空 = 180
    #[serde(default)]
    pub chat_timeout_secs: Option<u64>,
    /// 单次转写请求的超时秒数；空 = 300
    #[serde(default)]
    pub asr_timeout_secs: Option<u64>,
}

impl std::fmt::Debug for AutoClipConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = self.masked();
        f.debug_struct("AutoClipConfig")
            .field("enabled", &masked.enabled)
            .field("base_url", &masked.base_url)
            .field("api_key", &masked.api_key)
            .field("chat_model", &masked.chat_model)
            .field("asr_base_url", &masked.asr_base_url)
            .field("asr_api_key", &masked.asr_api_key)
            .field("asr_model", &masked.asr_model)
            .field("asr_language", &masked.asr_language)
            .field("asr_prompt", &masked.asr_prompt)
            .field("thumbnails", &masked.thumbnails)
            .field("max_asr_minutes", &masked.max_asr_minutes)
            .field("max_chat_tokens", &masked.max_chat_tokens)
            .field("chat_timeout_secs", &masked.chat_timeout_secs)
            .field("asr_timeout_secs", &masked.asr_timeout_secs)
            .finish()
    }
}

/// chat 的 key 从哪来，给状态接口用（不含 key 本身）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    Env,
    Config,
}

impl AutoClipConfig {
    /// 空白字符串当作没填，地址去掉结尾的 `/`；整份都是默认值时返回 `None`，
    /// 这样没配置过的人保存一次空间配置也不会多出 `auto_clip` 这个键。
    pub fn normalized(mut self) -> Option<Self> {
        for value in [
            &mut self.api_key,
            &mut self.chat_model,
            &mut self.asr_api_key,
            &mut self.asr_model,
            &mut self.asr_language,
            &mut self.asr_prompt,
        ] {
            trim_blank(value);
        }
        for url in [&mut self.base_url, &mut self.asr_base_url] {
            trim_blank(url);
            if let Some(value) = url {
                *value = normalize_url(value);
            }
        }
        (self != Self::default()).then_some(self)
    }

    pub fn thumbnails(&self) -> Thumbnails {
        self.thumbnails.unwrap_or_default()
    }

    pub fn max_asr_minutes(&self) -> u64 {
        self.max_asr_minutes.unwrap_or(DEFAULT_MAX_ASR_MINUTES)
    }

    pub fn max_chat_tokens(&self) -> u64 {
        self.max_chat_tokens.unwrap_or(DEFAULT_MAX_CHAT_TOKENS)
    }

    pub fn chat_timeout(&self) -> Duration {
        Duration::from_secs(
            self.chat_timeout_secs
                .filter(|secs| *secs > 0)
                .unwrap_or(DEFAULT_CHAT_TIMEOUT_SECS),
        )
    }

    pub fn asr_timeout(&self) -> Duration {
        Duration::from_secs(
            self.asr_timeout_secs
                .filter(|secs| *secs > 0)
                .unwrap_or(DEFAULT_ASR_TIMEOUT_SECS),
        )
    }

    /// 转写用的地址：没单独填就用 chat 的。
    pub fn asr_base_url(&self) -> Option<&str> {
        self.asr_base_url.as_deref().or(self.base_url.as_deref())
    }

    /// 实际生效的 chat key 与来源：环境变量优先。
    pub fn chat_key(&self) -> Option<(String, KeySource)> {
        self.chat_key_with(std::env::var(API_KEY_ENV).ok())
    }

    pub fn chat_key_with(&self, env_key: Option<String>) -> Option<(String, KeySource)> {
        match env_key.filter(|key| !key.trim().is_empty()) {
            Some(key) => Some((key.trim().to_string(), KeySource::Env)),
            None => self.api_key.clone().map(|key| (key, KeySource::Config)),
        }
    }

    /// 实际生效的转写 key：没单独填就用 chat 的（含环境变量）。
    pub fn asr_key_with(&self, env_key: Option<String>) -> Option<String> {
        self.asr_api_key
            .clone()
            .or_else(|| self.chat_key_with(env_key).map(|(key, _)| key))
    }

    /// 用于回显：两把 key 换成掩码。
    pub fn masked(&self) -> Self {
        let mut masked = self.clone();
        for value in [&mut masked.api_key, &mut masked.asr_api_key]
            .into_iter()
            .flatten()
        {
            *value = mask(value);
        }
        masked
    }

    /// chat key 会被发往的地址：chat 的地址，以及没单独填转写 key 时的转写地址。
    fn chat_key_destinations(&self) -> BTreeSet<String> {
        let mut urls = BTreeSet::new();
        urls.extend(self.base_url.clone());
        if self.asr_api_key.is_none() {
            urls.extend(self.asr_base_url().map(str::to_string));
        }
        urls
    }

    fn asr_key_destinations(&self) -> BTreeSet<String> {
        self.asr_base_url()
            .map(str::to_string)
            .into_iter()
            .collect()
    }
}

/// 把界面交回来的配置和库里的配置合起来：值是掩码的 key 换回原值。
///
/// 掩码只在这把 key 要发往的地址没有多出来时才换回原值；改了地址（或清空了单独的转写 key，
/// 让 chat key 要发往新的转写地址）时返回一句能照着做的错误，不保存。
pub fn restore_masked_keys(
    stored: Option<&AutoClipConfig>,
    submitted: AutoClipConfig,
) -> Result<AutoClipConfig, String> {
    let mut merged = submitted;
    let empty = AutoClipConfig::default();
    let stored = stored.unwrap_or(&empty);
    // 先按提交的值算地址：掩码本身也算「填了」，决定 chat key 是否还会发往转写地址
    let chat_targets = merged.chat_key_destinations();
    let asr_targets = merged.asr_key_destinations();
    if is_masked(merged.api_key.as_deref()) {
        merged.api_key = keep(
            stored.api_key.as_ref(),
            &stored.chat_key_destinations(),
            &chat_targets,
            "API key",
        )?;
    }
    if is_masked(merged.asr_api_key.as_deref()) {
        merged.asr_api_key = keep(
            stored.asr_api_key.as_ref(),
            &stored.asr_key_destinations(),
            &asr_targets,
            "转写 API key",
        )?;
    }
    Ok(merged)
}

fn keep(
    original: Option<&String>,
    allowed: &BTreeSet<String>,
    targets: &BTreeSet<String>,
    label: &str,
) -> Result<Option<String>, String> {
    let Some(original) = original else {
        return Err(format!(
            "{label} 是掩码，但服务器上没有保存过 key：请粘贴完整的 key"
        ));
    };
    if !targets.is_subset(allowed) {
        return Err(format!(
            "改了接口地址后请重新填写完整的 {label}：已保存的 key 不会发往新地址"
        ));
    }
    Ok(Some(original.clone()))
}

pub fn is_masked(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.contains(MASK_MARK))
}

/// `sk-proj-abcdef…` → `sk-…cdef`；太短的 key 只给省略号，免得掩码几乎就是原文。
pub fn mask(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() < 12 {
        return MASK_MARK.to_string();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}{MASK_MARK}{tail}")
}

pub fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// 给人看的 `host[:port]`：自建服务常在同一台机器上用不同端口区分。
pub fn display_host(url: &str) -> Option<String> {
    let url = url::Url::parse(url).ok()?;
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

fn trim_blank(value: &mut Option<String>) {
    if let Some(text) = value {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            *value = None;
        } else if trimmed.len() != text.len() {
            *value = Some(trimmed.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-proj-0123456789abcdef";
    const ASR_KEY: &str = "gsk_asr_9876543210wxyz";

    fn stored() -> AutoClipConfig {
        AutoClipConfig::builder()
            .enabled(true)
            .base_url("https://api.example.com/v1".into())
            .api_key(KEY.into())
            .chat_model("chat-small".into())
            .asr_model("whisper-1".into())
            .build()
    }

    #[test]
    fn display_host_keeps_explicit_ports() {
        assert_eq!(
            display_host("http://127.0.0.1:8080/v1").as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            display_host("https://api.example.com:443/v1").as_deref(),
            Some("api.example.com")
        );
        assert_eq!(display_host("not a url"), None);
    }

    #[test]
    fn masks_show_only_the_ends() {
        assert_eq!(mask(KEY), "sk-…cdef");
        assert_eq!(mask("short-key"), "…");
        assert!(!mask(KEY).contains("0123456789"));
        assert!(is_masked(Some("sk-…cdef")));
        assert!(!is_masked(Some(KEY)));
        let masked = stored().masked();
        assert_eq!(masked.api_key.as_deref(), Some("sk-…cdef"));
        assert_eq!(masked.base_url, stored().base_url);
        let debug = format!("{:?}", stored());
        assert!(
            debug.contains("sk-…cdef") && !debug.contains(KEY),
            "{debug}"
        );
    }

    #[test]
    fn a_masked_key_keeps_the_stored_value() {
        let merged = restore_masked_keys(Some(&stored()), stored().masked()).unwrap();
        assert_eq!(merged, stored());
    }

    #[test]
    fn a_new_key_replaces_the_stored_one() {
        let submitted = AutoClipConfig {
            api_key: Some("sk-new-key-000000000000".into()),
            ..stored()
        };
        let merged = restore_masked_keys(Some(&stored()), submitted.clone()).unwrap();
        assert_eq!(merged, submitted);
    }

    #[test]
    fn a_masked_key_is_not_sent_to_a_new_address() {
        let moved = AutoClipConfig {
            base_url: Some("https://elsewhere.example/v1".into()),
            ..stored().masked()
        };
        let error = restore_masked_keys(Some(&stored()), moved).unwrap_err();
        assert!(error.contains("重新填写完整的 API key"), "{error}");

        // 清空单独的转写 key 时 chat key 会发往转写地址，那也是新地址
        let with_asr = AutoClipConfig {
            asr_base_url: Some("https://asr.example/v1".into()),
            asr_api_key: Some(ASR_KEY.into()),
            ..stored()
        };
        let asr_inherits = AutoClipConfig {
            asr_api_key: None,
            ..with_asr.masked()
        };
        assert!(restore_masked_keys(Some(&with_asr), asr_inherits).is_err());

        // 转写 key 单独保留：转写地址不变即可
        let merged = restore_masked_keys(Some(&with_asr), with_asr.masked()).unwrap();
        assert_eq!(merged, with_asr);
        let asr_moved = AutoClipConfig {
            asr_base_url: Some("https://other-asr.example/v1".into()),
            ..with_asr.masked()
        };
        let error = restore_masked_keys(Some(&with_asr), asr_moved).unwrap_err();
        assert!(error.contains("转写 API key"), "{error}");
    }

    #[test]
    fn a_mask_without_a_stored_key_is_rejected() {
        let error = restore_masked_keys(None, stored().masked()).unwrap_err();
        assert!(error.contains("没有保存过 key"), "{error}");
    }

    #[test]
    fn blank_values_are_unset_and_an_empty_section_disappears() {
        let normalized = AutoClipConfig {
            base_url: Some(" https://api.example.com/v1/ ".into()),
            api_key: Some("  ".into()),
            ..AutoClipConfig::default()
        }
        .normalized()
        .unwrap();
        assert_eq!(
            normalized.base_url.as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(normalized.api_key, None);

        let untouched = AutoClipConfig {
            chat_model: Some(String::new()),
            ..AutoClipConfig::default()
        };
        assert_eq!(untouched.normalized(), None);
    }

    #[test]
    fn the_environment_key_wins_and_asr_falls_back_to_chat() {
        let config = stored();
        assert_eq!(
            config.chat_key_with(Some("sk-from-env".into())),
            Some(("sk-from-env".into(), KeySource::Env))
        );
        assert_eq!(
            config.chat_key_with(Some(" ".into())),
            Some((KEY.into(), KeySource::Config))
        );
        assert_eq!(config.asr_key_with(None).as_deref(), Some(KEY));
        assert_eq!(config.asr_base_url(), Some("https://api.example.com/v1"));
        let separate = AutoClipConfig {
            asr_api_key: Some(ASR_KEY.into()),
            ..config
        };
        assert_eq!(
            separate.asr_key_with(Some("sk-from-env".into())).as_deref(),
            Some(ASR_KEY)
        );
    }

    #[test]
    fn limits_and_timeouts_have_defaults() {
        let config = AutoClipConfig::default();
        assert_eq!(config.max_asr_minutes(), 300);
        assert_eq!(config.max_chat_tokens(), 300_000);
        assert_eq!(config.chat_timeout(), Duration::from_secs(180));
        assert_eq!(config.asr_timeout(), Duration::from_secs(300));
        assert_eq!(config.thumbnails(), Thumbnails::Auto);
        let parsed: AutoClipConfig =
            toml::from_str("enabled = true\nthumbnails = \"off\"\nchat_timeout_secs = 0").unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.thumbnails(), Thumbnails::Off);
        assert_eq!(parsed.chat_timeout(), Duration::from_secs(180));
    }
}
