//! 控制面上的房间与投稿模板，以及它们下发到节点时的形状。
//!
//! 房间与主库 `livestreamers` 同形，模板与 `uploadstreamers` 同形，只是模板的 `user_cookie`
//! （节点本地的凭据文件路径）换成了 `account_mid`。节点把它们转成本地行时走 serde，
//! 不手写 `InsertLiveStreamer` / `InsertUploadStreamer` 的结构体字面量，主库模型加了可空字段也不用改这里。

use crate::server::config::ConfigPatch;
use crate::server::infrastructure::models::hook_step::HookStep;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 一个房间的录制设置（不含分派状态）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomSpec {
    pub url: String,
    pub remark: String,
    #[serde(default)]
    pub filename_prefix: Option<String>,
    #[serde(default)]
    pub time_range: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default, rename = "override")]
    pub override_cfg: Option<ConfigPatch>,
    #[serde(default)]
    pub preprocessor: Option<Vec<HookStep>>,
    #[serde(default)]
    pub segment_processor: Option<Vec<HookStep>>,
    #[serde(default)]
    pub downloaded_processor: Option<Vec<HookStep>>,
    #[serde(default)]
    pub postprocessor: Option<Vec<HookStep>>,
    #[serde(default)]
    pub opt_args: Option<Value>,
    #[serde(default)]
    pub excluded_keywords: Option<Value>,
}

impl RoomSpec {
    /// 去掉首尾空白、把空串当成没填，与本地表单保存时的处理一致
    pub fn normalized(mut self) -> Self {
        fn clean(value: Option<String>) -> Option<String> {
            value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }
        self.url = self.url.trim().to_string();
        self.remark = self.remark.trim().to_string();
        self.filename_prefix = clean(self.filename_prefix);
        self.time_range = clean(self.time_range);
        self.format = clean(self.format);
        for value in [&mut self.opt_args, &mut self.excluded_keywords] {
            if matches!(value, Some(Value::Null)) {
                *value = None;
            }
        }
        self
    }

    /// 是否带钩子：`override`，或任何一个处理器里有 `rm` 以外的步骤。
    ///
    /// 钩子走 `sh -c`、`override` 能改任意配置，下发它们等于控制面能在节点上执行命令，
    /// 所以只派给加入时带了 `--allow-hooks` 的节点（D12）。只有 `rm` 的后处理是表单默认值，不算钩子。
    pub fn has_hooks(&self) -> bool {
        let override_set = self.override_cfg.as_ref().is_some_and(|patch| {
            serde_json::to_value(patch)
                .ok()
                .and_then(|value| value.as_object().cloned())
                .is_some_and(|fields| fields.values().any(|v| !v.is_null()))
        });
        let processors = [
            &self.preprocessor,
            &self.segment_processor,
            &self.downloaded_processor,
            &self.postprocessor,
        ];
        override_set
            || processors.iter().any(|steps| {
                steps.as_ref().is_some_and(|steps| {
                    steps
                        .iter()
                        .any(|step| !matches!(step, HookStep::Remove(cmd) if cmd == "rm"))
                })
            })
    }
}

/// 一个投稿模板（不含 id 与时间戳）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateSpec {
    pub template_name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tid: Option<u16>,
    #[serde(default)]
    pub tid_v2: Option<u32>,
    #[serde(default)]
    pub copyright: Option<u8>,
    #[serde(default)]
    pub copyright_source: Option<String>,
    #[serde(default)]
    pub cover_path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dynamic: Option<String>,
    #[serde(default)]
    pub dtime: Option<u32>,
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
    /// 投稿用的 B 站账号；节点在本地已登记的凭据文件里按 `token_info.mid` 找
    #[serde(default)]
    pub account_mid: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub credits: Option<Value>,
    #[serde(default)]
    pub up_selection_reply: Option<u8>,
    #[serde(default)]
    pub up_close_reply: Option<u8>,
    #[serde(default)]
    pub up_close_danmu: Option<u8>,
    #[serde(default)]
    pub extra_fields: Option<String>,
}

impl TemplateSpec {
    pub fn is_noop_uploader(&self) -> bool {
        crate::server::infrastructure::models::upload_streamer::is_noop_uploader(
            self.uploader.as_deref(),
        )
    }

    pub fn normalized(mut self) -> Self {
        self.template_name = self.template_name.trim().to_string();
        if matches!(self.credits, Some(Value::Null)) {
            self.credits = None;
        }
        self
    }
}

/// 分派状态里「这台节点应该录这个房间」的那一份：下发到节点的房间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesiredRoom {
    pub id: i64,
    pub epoch: i64,
    #[serde(default)]
    pub paused: bool,
    /// 用哪个 Fleet 模板投稿；`None` 表示不投稿
    #[serde(default)]
    pub template_id: Option<i64>,
    #[serde(flatten)]
    pub spec: RoomSpec,
}

/// 下发到节点的投稿模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesiredTemplate {
    pub id: i64,
    #[serde(flatten)]
    pub spec: TemplateSpec,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(json: Value) -> RoomSpec {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn only_rm_postprocessing_is_not_a_hook() {
        assert!(!room(serde_json::json!({ "url": "u", "remark": "r" })).has_hooks());
        assert!(
            !room(serde_json::json!({ "url": "u", "remark": "r", "postprocessor": ["rm"] }))
                .has_hooks()
        );
        assert!(
            room(serde_json::json!({ "url": "u", "remark": "r", "postprocessor": [{ "run": "echo" }] }))
                .has_hooks()
        );
        assert!(
            room(serde_json::json!({ "url": "u", "remark": "r", "postprocessor": ["rm", { "mv": "backup/" }] }))
                .has_hooks()
        );
        assert!(
            room(serde_json::json!({ "url": "u", "remark": "r", "preprocessor": [{ "run": "echo" }] }))
                .has_hooks()
        );
        // 空的 override 不算，设了任意一项就算
        assert!(
            !room(serde_json::json!({ "url": "u", "remark": "r", "override": {} })).has_hooks()
        );
        assert!(
            room(serde_json::json!({ "url": "u", "remark": "r", "override": { "downloader": "ffmpeg" } }))
                .has_hooks()
        );
    }

    #[test]
    fn specs_round_trip_through_the_wire_shape() {
        let desired = DesiredRoom {
            id: 3,
            epoch: 2,
            paused: true,
            template_id: Some(7),
            spec: room(serde_json::json!({
                "url": "https://live.example/1",
                "remark": "主播",
                "override": { "segment_time": "01:00:00" },
                "excluded_keywords": ["回放"],
            })),
        };
        let wire = serde_json::to_value(&desired).unwrap();
        assert_eq!(wire["override"]["segment_time"], "01:00:00");
        assert_eq!(wire["url"], "https://live.example/1");
        let back: DesiredRoom = serde_json::from_value(wire).unwrap();
        assert_eq!(back.epoch, 2);
        assert!(back.paused);
        assert_eq!(back.spec.remark, "主播");

        let template: TemplateSpec = serde_json::from_value(serde_json::json!({
            "template_name": " 模板 ",
            "account_mid": 42,
            "tags": ["a"],
            "user_cookie": "cookies.json",
        }))
        .unwrap();
        let template = template.normalized();
        assert_eq!(template.template_name, "模板");
        let wire = serde_json::to_value(DesiredTemplate {
            id: 1,
            spec: template,
        })
        .unwrap();
        // 凭据路径不在线上的形状里，传进来也会被丢掉
        assert!(wire.get("user_cookie").is_none());
        assert_eq!(wire["account_mid"], 42);
    }
}
