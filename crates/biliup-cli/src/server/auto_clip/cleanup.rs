//! 随分段清理任务每分钟跑一次的收尾：候选过期。
//!
//! 分析数据目录（`auto_clip/`）不存在时什么都不做：没用过自动切片的库不多一次查询。

use super::runner;
use super::suggestions;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use std::path::Path;
use tracing::{info, warn};

/// 一轮收尾做了什么。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swept {
    /// 过期的候选数
    pub expired: u64,
}

pub async fn sweep(pool: &ConnectionPool, now: i64) -> Swept {
    sweep_in(pool, &runner::root(), now).await
}

pub async fn sweep_in(pool: &ConnectionPool, root: &Path, now: i64) -> Swept {
    let mut swept = Swept::default();
    if !tokio::fs::try_exists(root).await.unwrap_or(false) {
        return swept;
    }
    match suggestions::expire(pool, now).await {
        Ok(0) => {}
        Ok(n) => {
            info!(
                suggestions = n,
                "自动切片：72 小时没处理的候选已过期，撤销素材引用"
            );
            swept.expired = n;
        }
        Err(error) => warn!(%error, "自动切片：候选过期处理失败"),
    }
    swept
}
