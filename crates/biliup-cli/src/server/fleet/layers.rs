//! 分层配置（D13）：Fleet 全局 ⊕ 节点覆盖 ⊕ 节点本地密钥。
//!
//! - **可下发的键**只有 [`redact::VISIBLE_CONFIG_KEYS`] 里的那些（D11）。Cookie、密码、主播列表
//!   这些名单外的键不进控制面的库、不上连接，节点合并时一律保留本机的值。
//! - **按节点的键**（[`PER_NODE_KEYS`]：池大小、ffmpeg 路径、边录边传目录、最低可用空间、日志级别）
//!   跟机器走，全局配置不管它们；只有写进节点覆盖才下发，否则节点保留自己的值。
//! - **全局配置**存全部共享键（白名单减按节点的键），`null` 就是「未设置」，同样会下发、覆盖节点。
//! - **节点覆盖**是 `ConfigPatch` 语义：只存设置了的键，`null` 与空字符串都当没设置；
//!   唯一能显式清空的是 `file_size`（写 `null`）。其余字段覆盖不能清空（沿用 WEB-23）。
//!   与 `ConfigPatch` 不同的是 `ffmpeg_path` / `preview_max_minutes` 也能覆盖（`ConfigPatch` 跳过它们）。
//!
//! 节点收到的是「全局 ⊕ 覆盖」（[`delivered`]），自己再按 [`merge`] 叠到本机配置上：
//! 下发里有的键用下发的值，没有的用接入时记下的本机原值（`base`），白名单外的键保持本机现值。

use crate::server::api::redact::VISIBLE_CONFIG_KEYS;
use crate::server::config::Config;
use serde_json::{Map, Value};
use tracing_subscriber::EnvFilter;

/// 跟机器走的键：全局配置不下发，只能在节点覆盖里设置
pub const PER_NODE_KEYS: &[&str] = &[
    "pool1_size",
    "pool2_size",
    "ffmpeg_path",
    "sync_save_dir",
    "min_free_space",
    "loggers_level",
];

/// 覆盖里唯一可以写 `null` 表示「显式清空」的键
const CLEARABLE_KEY: &str = "file_size";

pub type Object = Map<String, Value>;

/// 能随 Fleet 下发的键（白名单）
pub fn is_deliverable(key: &str) -> bool {
    VISIBLE_CONFIG_KEYS.contains(&key)
}

pub fn is_per_node(key: &str) -> bool {
    PER_NODE_KEYS.contains(&key)
}

/// 全局配置管的键：白名单里、不按节点
pub fn is_shared(key: &str) -> bool {
    is_deliverable(key) && !is_per_node(key)
}

/// 整理控制面收到的配置时出的错
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// 白名单外的键带了值（多半是 Cookie / 密码）：控制面不存，只能在节点本机填
    Rejected(Vec<String>),
    /// 类型不对或取值不合法，原因可以直接展示
    Invalid(String),
}

impl std::fmt::Display for LayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerError::Rejected(keys) => write!(
                f,
                "这些字段不随 Fleet 下发，控制面不保存：{}。Cookie、密码等请到各节点本机的「空间配置」里填写",
                keys.join("、")
            ),
            LayerError::Invalid(reason) => f.write_str(reason),
        }
    }
}

/// 整理后的配置与被丢掉的键
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Normalized {
    pub config: Object,
    /// 没有保存的键：全局配置里的按节点键，以及值为 `null` 的白名单外键
    pub ignored: Vec<String>,
}

/// 配置的白名单投影：只留可下发的键
pub fn project(config: &Config) -> Object {
    project_value(serde_json::to_value(config).unwrap_or(Value::Null))
}

fn project_value(value: Value) -> Object {
    match value {
        Value::Object(map) => map.into_iter().filter(|(k, _)| is_deliverable(k)).collect(),
        _ => Object::new(),
    }
}

/// 两份投影里取值不同的白名单键，按名单顺序
pub fn changed_keys(a: &Object, b: &Object) -> Vec<String> {
    VISIBLE_CONFIG_KEYS
        .iter()
        .filter(|key| a.get(**key) != b.get(**key))
        .map(|key| key.to_string())
        .collect()
}

/// 把白名单外的键分成「丢掉」（值为 `null`，例如回传脱敏后的配置）与「拒绝」（带了值）
fn split_foreign(body: Object) -> Result<(Object, Vec<String>), LayerError> {
    let mut kept = Object::new();
    let mut ignored = Vec::new();
    let mut rejected = Vec::new();
    for (key, value) in body {
        if is_deliverable(&key) {
            kept.insert(key, value);
        } else if value.is_null() {
            ignored.push(key);
        } else {
            rejected.push(key);
        }
    }
    if rejected.is_empty() {
        Ok((kept, ignored))
    } else {
        rejected.sort();
        Err(LayerError::Rejected(rejected))
    }
}

fn parse(object: Object) -> Result<Config, LayerError> {
    let mut config: Config = serde_json::from_value(Value::Object(object))
        .map_err(|e| LayerError::Invalid(format!("配置格式不对：{e}")))?;
    config.normalize_segment_limits();
    Ok(config)
}

/// 下发前后都要过的取值检查：池大小至少为 1，日志级别能解析。
/// 单机的 `apply_config` 在落库之后才检查日志级别，Fleet 在应用前就拦下。
pub fn validate(config: &Config) -> Result<(), String> {
    config.validate_pool_sizes()?;
    if let Some(level) = &config.loggers_level {
        EnvFilter::try_new(level).map_err(|e| format!("日志级别（loggers_level）格式不对：{e}"))?;
    }
    Ok(())
}

/// 这个键能不能取 `null`（`Option` 字段能；`delay` 这类非 `Option` 的数字不能）
fn accepts_null(key: &str) -> bool {
    let mut probe = Object::new();
    probe.insert(key.to_string(), Value::Null);
    serde_json::from_value::<Config>(Value::Object(probe)).is_ok()
}

/// 整理 `PUT /v1/fleet/configuration` 的请求体：与单机保存一样是整份替换，缺的键取默认值；
/// 不接受 `null` 的键写 `null` 也取默认值（界面清空数字框时键仍在、值为 `null`）。
/// 按节点的键不保存，白名单外带值的键整体拒绝。返回全部共享键。
pub fn normalize_global(body: Object) -> Result<Normalized, LayerError> {
    let (kept, mut ignored) = split_foreign(body)?;
    let mut shared = Object::new();
    for (key, value) in kept {
        if is_per_node(&key) {
            ignored.push(key);
        } else if !value.is_null() || accepts_null(&key) {
            shared.insert(key, value);
        }
    }
    let config = parse(shared)?;
    let config = project(&config)
        .into_iter()
        .filter(|(key, _)| is_shared(key))
        .collect();
    ignored.sort();
    Ok(Normalized { config, ignored })
}

/// 整理 `PUT /v1/fleet/nodes/{id}/config` 的请求体（整份替换这台节点的覆盖）：
/// `null` 与空白字符串当没设置（`file_size: null` 除外），白名单外带值的键整体拒绝。
/// 取值叠在默认配置上检查，`pool1_size = 0` 这类值在这里就拒掉。
pub fn normalize_override(body: Object) -> Result<Normalized, LayerError> {
    let (kept, mut ignored) = split_foreign(body)?;
    let patch: Object = kept
        .into_iter()
        .filter(|(key, value)| match value {
            Value::Null => key == CLEARABLE_KEY,
            Value::String(text) => !text.trim().is_empty(),
            _ => true,
        })
        .collect();
    let mut candidate = project(&Config::default());
    candidate.extend(patch.clone());
    let config = parse(candidate)?;
    validate(&config).map_err(LayerError::Invalid)?;
    ignored.sort();
    Ok(Normalized {
        config: patch,
        ignored,
    })
}

/// 发给节点的配置：全局配置的共享键，再叠上节点覆盖。控制面还没保存过全局配置时只有覆盖。
pub fn delivered(global: Option<&Object>, patch: &Object) -> Object {
    let mut values: Object = global
        .into_iter()
        .flatten()
        .filter(|(key, _)| is_shared(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    values.extend(
        patch
            .iter()
            .filter(|(key, _)| is_deliverable(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    values
}

/// 节点侧合并：白名单键取下发值，下发里没有的取 `base`（接入时本机的原值），
/// 名单外的键保持 `live` 的值。只做合并与格式检查，取值检查见 [`validate`]。
pub fn merge(live: &Config, base: &Object, delivered: &Object) -> Result<Config, String> {
    let Value::Object(mut merged) =
        serde_json::to_value(live).map_err(|e| format!("读不出本机配置：{e}"))?
    else {
        return Err("读不出本机配置".to_string());
    };
    for key in VISIBLE_CONFIG_KEYS {
        if let Some(value) = delivered.get(*key).or_else(|| base.get(*key)) {
            merged.insert(key.to_string(), value.clone());
        }
    }
    parse(merged).map_err(|e| match e {
        LayerError::Invalid(reason) => reason,
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::UserConfig;
    use serde_json::json;

    fn object(value: Value) -> Object {
        value.as_object().unwrap().clone()
    }

    fn node_config() -> Config {
        Config {
            pool1_size: 2,
            pool2_size: 1,
            ffmpeg_path: Some("/opt/ffmpeg".into()),
            segment_time: Some("00:30:00".into()),
            kuaishou_cookie: Some("ks-secret".into()),
            douyu_device_id: Some("did-secret".into()),
            user: Some(UserConfig {
                bili_cookie: Some("SESSDATA=secret".into()),
                ..Default::default()
            }),
            ..Config::default()
        }
    }

    #[test]
    fn key_sets_are_consistent_with_the_whitelist_and_config() {
        let fields = project_value(serde_json::to_value(Config::default()).unwrap());
        for key in VISIBLE_CONFIG_KEYS {
            assert!(fields.contains_key(*key), "{key} is not a Config field");
        }
        for key in PER_NODE_KEYS {
            assert!(is_deliverable(key), "{key} must be in the whitelist");
            assert!(!is_shared(key));
        }
        for secret in [
            "user",
            "streamers",
            "kuaishou_cookie",
            "douyu_deviceId",
            "twitcasting_password",
        ] {
            assert!(!is_deliverable(secret), "{secret}");
        }
        assert!(is_shared("segment_time") && is_shared("filename_prefix"));
    }

    #[test]
    fn projection_drops_every_secret() {
        let projected = project(&node_config());
        let text = Value::Object(projected.clone()).to_string();
        for secret in ["ks-secret", "did-secret", "SESSDATA"] {
            assert!(!text.contains(secret), "{secret} leaked: {text}");
        }
        assert_eq!(projected["pool1_size"], 2);
        assert_eq!(projected.len(), VISIBLE_CONFIG_KEYS.len());
    }

    #[test]
    fn global_keeps_all_shared_keys_and_drops_per_node_ones() {
        let normalized = normalize_global(object(json!({
            "segment_time": "01:00:00",
            "filename_prefix": "{streamer}%Y",
            "pool1_size": 9,
            "ffmpeg_path": "/usr/bin/ffmpeg",
            "user": null,
            "kuaishou_cookie": null,
        })))
        .unwrap();
        assert_eq!(normalized.config["segment_time"], "01:00:00");
        assert_eq!(normalized.config["filename_prefix"], "{streamer}%Y");
        // 缺的共享键取默认值，与单机整份保存一致
        assert_eq!(normalized.config["delay"], 300);
        assert_eq!(normalized.config["file_size"], 2_621_440_000u64);
        assert!(normalized.config.keys().all(|key| is_shared(key)));
        assert_eq!(
            normalized.config.len(),
            VISIBLE_CONFIG_KEYS.len() - PER_NODE_KEYS.len()
        );
        assert_eq!(
            normalized.ignored,
            ["ffmpeg_path", "kuaishou_cookie", "pool1_size", "user"]
        );
    }

    #[test]
    fn global_normalizes_blank_segment_time_and_keeps_explicit_nulls() {
        let normalized =
            normalize_global(object(json!({"segment_time": " ", "file_size": null}))).unwrap();
        assert_eq!(normalized.config["segment_time"], Value::Null);
        assert_eq!(normalized.config["file_size"], Value::Null);
    }

    #[test]
    fn global_nulls_on_non_optional_keys_fall_back_to_defaults() {
        let normalized = normalize_global(object(
            json!({"delay": null, "threads": null, "lines": null}),
        ))
        .unwrap();
        let defaults = project(&Config::default());
        for key in ["delay", "threads", "lines"] {
            assert_eq!(normalized.config[key], defaults[key], "{key}");
        }
        assert!(accepts_null("segment_time") && accepts_null("file_size"));
        assert!(!accepts_null("delay") && !accepts_null("retention_hours"));
    }

    /// 前端 `app/lib/fleet-config.ts` 手写了可下发键与按节点键，两边必须一致
    #[test]
    fn frontend_key_lists_match() {
        fn ts_list(source: &str, name: &str) -> Vec<String> {
            let head = format!("export const {name} = [");
            let start = source.find(&head).expect("前端常量不见了") + head.len();
            let end = start + source[start..].find(']').unwrap();
            source[start..end]
                .split(',')
                .map(|item| item.trim().trim_matches('\'').to_string())
                .filter(|item| !item.is_empty())
                .collect()
        }
        let source = include_str!("../../../../../app/lib/fleet-config.ts");
        assert_eq!(
            ts_list(source, "DELIVERABLE_KEYS"),
            VISIBLE_CONFIG_KEYS.to_vec()
        );
        assert_eq!(ts_list(source, "PER_NODE_KEYS"), PER_NODE_KEYS.to_vec());
    }

    #[test]
    fn secrets_with_values_are_rejected_by_both_layers() {
        let body = object(json!({
            "segment_time": "01:00:00",
            "user": {"bili_cookie": "SESSDATA=x"},
            "kuaishou_cookie": "ks",
        }));
        let expected = LayerError::Rejected(vec!["kuaishou_cookie".into(), "user".into()]);
        assert_eq!(normalize_global(body.clone()), Err(expected.clone()));
        assert_eq!(normalize_override(body), Err(expected.clone()));
        assert!(expected.to_string().contains("kuaishou_cookie、user"));
    }

    #[test]
    fn wrong_types_are_invalid() {
        let Err(LayerError::Invalid(reason)) = normalize_global(object(json!({"delay": "soon"})))
        else {
            panic!()
        };
        assert!(reason.starts_with("配置格式不对"), "{reason}");
        assert!(matches!(
            normalize_override(object(json!({"pool1_size": -1}))),
            Err(LayerError::Invalid(_))
        ));
    }

    #[test]
    fn override_keeps_only_set_keys_with_file_size_clearable() {
        let normalized = normalize_override(object(json!({
            "pool1_size": 1,
            "segment_time": null,
            "filename_prefix": "  ",
            "file_size": null,
            "douyu_cdn": "hw-h5",
            "user": null,
        })))
        .unwrap();
        assert_eq!(
            normalized.config,
            object(json!({"pool1_size": 1, "file_size": null, "douyu_cdn": "hw-h5"}))
        );
        assert_eq!(normalized.ignored, ["user"]);
        assert_eq!(
            normalize_override(Object::new()).unwrap().config,
            Object::new()
        );
    }

    #[test]
    fn override_rejects_invalid_values_before_saving() {
        assert_eq!(
            normalize_override(object(json!({"pool1_size": 0}))),
            Err(LayerError::Invalid(
                "下载线程池大小（pool1_size）至少为 1".into()
            ))
        );
        let Err(LayerError::Invalid(reason)) =
            normalize_override(object(json!({"loggers_level": "biliup=notalevel"})))
        else {
            panic!()
        };
        assert!(reason.contains("loggers_level"), "{reason}");
    }

    #[test]
    fn override_can_set_fields_config_patch_skips() {
        let normalized = normalize_override(object(json!({
            "ffmpeg_path": "/opt/ff",
            "preview_max_minutes": 5,
        })))
        .unwrap();
        assert_eq!(normalized.config.len(), 2);
    }

    #[test]
    fn delivered_is_global_shared_keys_then_override() {
        let global = object(json!({
            "segment_time": "01:00:00",
            "delay": 300,
            "file_size": 1000,
            // 老数据里即便混进来也不下发
            "pool1_size": 8,
            "user": {"bili_cookie": "x"},
        }));
        let patch = object(json!({"pool1_size": 2, "file_size": null, "delay": 60}));
        assert_eq!(
            delivered(Some(&global), &patch),
            object(json!({
                "segment_time": "01:00:00",
                "delay": 60,
                "file_size": null,
                "pool1_size": 2,
            }))
        );
        assert_eq!(delivered(None, &patch), patch);
        assert_eq!(delivered(None, &Object::new()), Object::new());
    }

    #[test]
    fn merge_applies_delivered_keys_and_keeps_local_secrets() {
        let live = node_config();
        let base = project(&live);
        let values = object(json!({
            "segment_time": "02:00:00",
            "filename_prefix": "{title}",
            // 下发里混进名单外的键也不理
            "kuaishou_cookie": "from-controller",
        }));
        let merged = merge(&live, &base, &values).unwrap();
        assert_eq!(merged.segment_time.as_deref(), Some("02:00:00"));
        assert_eq!(merged.filename_prefix.as_deref(), Some("{title}"));
        assert_eq!(merged.kuaishou_cookie.as_deref(), Some("ks-secret"));
        assert_eq!(merged.douyu_device_id.as_deref(), Some("did-secret"));
        assert_eq!(merged.user, live.user);
        assert_eq!(merged.streamers, live.streamers);
        // 没下发的按节点键保留本机值
        assert_eq!(merged.pool1_size, 2);
        assert_eq!(merged.ffmpeg_path.as_deref(), Some("/opt/ffmpeg"));
    }

    #[test]
    fn keys_missing_from_delivered_fall_back_to_base() {
        let original = node_config();
        let base = project(&original);
        let overridden = merge(&original, &base, &object(json!({"pool1_size": 7}))).unwrap();
        assert_eq!(overridden.pool1_size, 7);
        // 覆盖撤掉后回到接入时的本机值
        let restored = merge(&overridden, &base, &Object::new()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn global_nulls_override_node_values_and_file_size_can_be_cleared() {
        let live = node_config();
        let base = project(&live);
        let merged = merge(
            &live,
            &base,
            &object(json!({"segment_time": null, "file_size": null})),
        )
        .unwrap();
        assert_eq!(merged.segment_time, None);
        assert_eq!(merged.file_size, None);
    }

    #[test]
    fn merge_reports_type_errors_without_touching_anything() {
        let live = node_config();
        let reason =
            merge(&live, &project(&live), &object(json!({"threads": "many"}))).unwrap_err();
        assert!(reason.starts_with("配置格式不对"), "{reason}");
    }

    #[test]
    fn validate_checks_pool_sizes_and_log_filters() {
        assert_eq!(validate(&Config::default()), Ok(()));
        let zero = Config {
            pool2_size: 0,
            ..Config::default()
        };
        assert_eq!(
            validate(&zero),
            Err("上传线程池大小（pool2_size）至少为 1".into())
        );
        let bad_log = Config {
            loggers_level: Some("biliup=notalevel".into()),
            ..Config::default()
        };
        assert!(validate(&bad_log).unwrap_err().contains("loggers_level"));
    }

    #[test]
    fn changed_keys_follow_whitelist_order() {
        let a = project(&Config::default());
        let mut b = a.clone();
        b.insert("pool2_size".into(), json!(9));
        b.insert("segment_time".into(), json!("01:00:00"));
        b.insert("user".into(), json!("ignored"));
        assert_eq!(changed_keys(&a, &b), ["segment_time", "pool2_size"]);
        assert!(changed_keys(&a, &a).is_empty());
    }
}
