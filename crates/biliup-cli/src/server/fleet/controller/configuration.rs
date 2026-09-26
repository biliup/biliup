//! 控制面的分层配置：生成每台节点要下发的配置，汇总节点应用配置的情况给界面看。
//! 存取在 [`crate::server::fleet::config_store`]，合并规则在 [`crate::server::fleet::layers`]。

use super::{Controller, LiveNode};
use crate::server::errors::AppResult;
use crate::server::fleet::config_store;
use crate::server::fleet::layers;
use crate::server::fleet::protocol::{CONFIG_SINCE, DesiredConfig, PROTOCOL_MINOR};
use serde::Serialize;

/// 控制面自己的版本，节点比它旧时界面提示升级
pub const CONTROLLER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `GET /v1/fleet/nodes` 每台节点的 `config`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NodeConfigState {
    /// 在线节点的配置同步情况：`unsupported`（次版本 < 2，只收房间不收配置）、`pending`（已下发、
    /// 还没确认）、`applied`、`failed`；离线为 `None`
    pub sync: Option<&'static str>,
    /// `failed` 时节点报的原因；节点保持原来的配置
    pub error: Option<String>,
    /// 节点的 biliup 版本或协议次版本比控制面旧
    pub outdated: bool,
    /// 这台节点覆盖了哪些键
    pub override_keys: Vec<String>,
}

impl NodeConfigState {
    pub(super) fn of(live: Option<&LiveNode>, version: Option<&str>) -> Self {
        let outdated = version.is_some_and(|version| version_older(version, CONTROLLER_VERSION))
            || live.is_some_and(|node| node.proto < PROTOCOL_MINOR);
        let Some(node) = live else {
            return NodeConfigState {
                outdated,
                ..NodeConfigState::default()
            };
        };
        let (sync, error) = if node.proto < CONFIG_SINCE {
            ("unsupported", None)
        } else if node.pushed_version.is_none() || node.pushed_version != node.acked_version {
            ("pending", None)
        } else {
            match &node.config_ack {
                Some(ack) if ack.applied => ("applied", None),
                Some(ack) => ("failed", ack.error.clone()),
                None => ("pending", None),
            }
        };
        NodeConfigState {
            sync: Some(sync),
            error,
            outdated,
            ..NodeConfigState::default()
        }
    }
}

/// 按数字比较 `主.次.修订`，`-` / `+` 之后的后缀不看；解析不了时不算旧
pub fn version_older(version: &str, than: &str) -> bool {
    fn parts(version: &str) -> Option<Vec<u64>> {
        let core = version
            .trim()
            .trim_start_matches('v')
            .split(['-', '+'])
            .next()?;
        core.split('.').map(|part| part.parse().ok()).collect()
    }
    match (parts(version), parts(than)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    }
}

impl Controller {
    /// 这台节点此刻应收到的配置：最新的全局配置 ⊕ 它的覆盖
    pub async fn desired_config(&self, node: i64) -> AppResult<DesiredConfig> {
        let global = config_store::latest_config(&self.pool).await?;
        let patch = config_store::node_override(&self.pool, node)
            .await?
            .unwrap_or_default();
        Ok(DesiredConfig {
            global_version: global.as_ref().map_or(0, |global| global.version),
            values: layers::delivered(global.as_ref().map(|global| &global.config), &patch),
        })
    }

    /// 一台节点的配置同步情况；不在线时只有版本是否过旧
    pub fn node_config_state(&self, node: i64, version: Option<&str>) -> NodeConfigState {
        let live = self.live.lock().unwrap();
        let live = live.get(&node);
        let version = live.map(|node| node.version.as_str()).or(version);
        NodeConfigState::of(live, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically_and_ignore_suffixes() {
        assert!(version_older("1.2.8", "1.2.10"));
        assert!(version_older("1.2", "1.2.1"));
        assert!(version_older("v0.9.9", "1.0.0"));
        assert!(!version_older("1.2.10", "1.2.8"));
        assert!(!version_older("1.2.8", "1.2.8"));
        assert!(!version_older("1.2.8-beta.1", "1.2.8"));
        assert!(!version_older("dev", "1.2.8"));
        assert!(!version_older("1.2.8", "dev"));
    }
}
