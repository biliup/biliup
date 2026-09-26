//! 节点端应用控制面下发的配置（F3）。
//!
//! 下发的是「Fleet 全局 ⊕ 本节点覆盖」里的白名单键，这里再叠上本机的密钥（名单外的键保持本机值），
//! 先在内存里做完全部检查，通过了才交给 `apply_config` 落库并立即生效（池大小、ffmpeg、日志级别）。
//! 检查不过就不动本机配置，把原因放进 `Ack` 报给控制面。
//!
//! 最近一次的结果记在 `data/fleet-state.json` 的 `config` 里：控制面连不上时照常按本机库里的配置跑，
//! 重启后若库里的配置与记录不一致（例如库被还原过）再套一遍记录。离开或被移除时删掉这份记录，
//! 当时生效的配置就是本机配置，本机又能改了。

use super::layers::{self, Object};
use super::protocol::{ConfigAck, DesiredConfig};
use crate::server::config::Config;
use crate::server::errors::AppError;
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::services::configuration::{ApplyConfigError, apply_config};
use error_stack::{FrameKind, Report};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// `fleet-state.json` 的 `config`：控制面在管这台节点的配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ManagedConfig {
    /// 最近一次收到的全局配置版本，0 表示控制面还没保存过全局配置
    #[serde(default)]
    pub global_version: i64,
    /// 第一次收到配置时本机的白名单投影：下发里没有的键（没覆盖的按节点键）按它恢复
    #[serde(default)]
    pub base: Object,
    /// 此刻生效的白名单投影；本机改配置时按它判断改没改到控制面管的键
    #[serde(default)]
    pub applied: Object,
    /// 最近一次没能应用的原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ManagedConfig {
    fn starting_from(live: &Config) -> Self {
        let projected = layers::project(live);
        ManagedConfig {
            base: projected.clone(),
            applied: projected,
            ..ManagedConfig::default()
        }
    }
}

/// 内部错误报给控制面的原因：错误链最底层的那一条（例如 SQLite 的 database is locked），
/// 顶层多半只是 `AppError::Unknown`，看不出发生了什么
fn root_cause(report: &Report<AppError>) -> String {
    report
        .frames()
        .filter_map(|frame| match frame.kind() {
            FrameKind::Context(context) => Some(context.to_string()),
            FrameKind::Attachment(_) => None,
        })
        .last()
        .unwrap_or_else(|| report.current_context().to_string())
}

fn live(services: &ServiceRegister) -> Config {
    services.config.read().unwrap().clone()
}

/// 合并、检查，与本机现有配置不同才落库生效；返回此刻生效的配置
async fn apply_merged(
    services: &ServiceRegister,
    base: &Object,
    values: &Object,
) -> Result<Config, String> {
    let current = live(services);
    let merged = layers::merge(&current, base, values)?;
    layers::validate(&merged)?;
    if merged == current {
        return Ok(current);
    }
    apply_config(
        &services.config,
        &services.pool,
        &services.managers,
        &services.log_handle,
        merged,
    )
    .await
    .map_err(|e| match e {
        ApplyConfigError::Invalid(reason) => reason,
        ApplyConfigError::Internal(report) => format!("保存配置失败：{}", root_cause(&report)),
    })
}

/// 处理期望状态里的配置。`None`（控制面不下发配置，例如 F2 控制面）时放弃托管，本机配置保持现状。
pub async fn apply(
    services: &ServiceRegister,
    state: &mut Option<ManagedConfig>,
    desired: Option<DesiredConfig>,
) -> Option<ConfigAck> {
    let Some(desired) = desired else {
        if state.take().is_some() {
            info!("fleet controller no longer manages the configuration, keeping it as local");
        }
        return None;
    };
    let managed = state.get_or_insert_with(|| ManagedConfig::starting_from(&live(services)));
    managed.global_version = desired.global_version;
    match apply_merged(services, &managed.base, &desired.values).await {
        Ok(config) => {
            let applied = layers::project(&config);
            if applied != managed.applied {
                info!(
                    global_version = desired.global_version,
                    changed = ?layers::changed_keys(&managed.applied, &applied),
                    "fleet configuration applied"
                );
            }
            managed.applied = applied;
            managed.error = None;
            Some(ConfigAck::applied())
        }
        Err(reason) => {
            warn!(%reason, "fleet configuration rejected, keeping the current one");
            managed.applied = layers::project(&live(services));
            managed.error = Some(reason.clone());
            Some(ConfigAck::failed(reason))
        }
    }
}

/// 节点代理启动时：库里的配置与最近一次应用的记录对不上就再套一遍记录
pub async fn resume(services: &ServiceRegister, managed: &mut ManagedConfig) {
    if layers::project(&live(services)) == managed.applied {
        return;
    }
    match apply_merged(services, &managed.applied, &Object::new()).await {
        Ok(config) => {
            info!("fleet configuration restored from the state file");
            managed.applied = layers::project(&config);
        }
        Err(reason) => {
            warn!(%reason, "could not restore the fleet configuration from the state file");
            managed.applied = layers::project(&live(services));
            managed.error = Some(reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::UserConfig;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};
    use tracing_subscriber::{EnvFilter, Registry, reload};

    struct Fixture {
        _dir: tempfile::TempDir,
        services: ServiceRegister,
        /// 句柄只持有弱引用，过滤层被丢弃后换不了日志级别
        _filter: reload::Layer<EnvFilter, Registry>,
    }

    #[test]
    fn internal_errors_report_their_root_cause() {
        let report = Report::new(sqlx::Error::PoolTimedOut).change_context(AppError::Unknown);
        assert_eq!(root_cause(&report), sqlx::Error::PoolTimedOut.to_string());
        assert_eq!(
            root_cause(&Report::new(AppError::Unknown)),
            AppError::Unknown.to_string()
        );
    }

    async fn fixture(config: Config) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (filter, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        let services =
            ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await;
        Fixture {
            _dir: dir,
            services,
            _filter: filter,
        }
    }

    impl Fixture {
        fn config(&self) -> Config {
            live(&self.services)
        }

        async fn saved(&self) -> Vec<Config> {
            let rows: Vec<String> =
                sqlx::query_scalar("SELECT value FROM configuration WHERE key = 'config'")
                    .fetch_all(&self.services.pool)
                    .await
                    .unwrap();
            rows.iter()
                .map(|row| serde_json::from_str(row).unwrap())
                .collect()
        }

        fn log_filter(&self) -> String {
            self.services
                .log_handle
                .with_current(|filter| filter.to_string())
                .unwrap()
        }
    }

    fn node_config() -> Config {
        Config {
            pool1_size: 2,
            segment_time: Some("00:30:00".into()),
            kuaishou_cookie: Some("ks-secret".into()),
            user: Some(UserConfig {
                bili_cookie: Some("SESSDATA=secret".into()),
                ..Default::default()
            }),
            ..Config::default()
        }
    }

    fn desired(global_version: i64, values: Value) -> Option<DesiredConfig> {
        Some(DesiredConfig {
            global_version,
            values: values.as_object().unwrap().clone(),
        })
    }

    #[tokio::test]
    async fn delivered_config_is_merged_with_local_secrets_and_takes_effect() {
        let f = fixture(node_config()).await;
        let mut state = None;
        let ack = apply(
            &f.services,
            &mut state,
            desired(
                3,
                json!({"segment_time": "01:00:00", "pool1_size": 4, "loggers_level": "debug"}),
            ),
        )
        .await;
        assert_eq!(ack, Some(ConfigAck::applied()));

        let config = f.config();
        assert_eq!(config.segment_time.as_deref(), Some("01:00:00"));
        assert_eq!(config.kuaishou_cookie.as_deref(), Some("ks-secret"));
        assert_eq!(config.user, node_config().user);
        assert_eq!(f.services.managers.download_pool_size(), 4);
        assert_eq!(f.log_filter(), "debug");
        let saved = f.saved().await;
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0], config);

        let managed = state.unwrap();
        assert_eq!(managed.global_version, 3);
        assert_eq!(managed.base, layers::project(&node_config()));
        assert_eq!(managed.applied, layers::project(&config));
        assert_eq!(managed.error, None);
        let recorded = serde_json::to_string(&managed).unwrap();
        for secret in ["ks-secret", "SESSDATA"] {
            assert!(!recorded.contains(secret), "{secret} leaked: {recorded}");
        }
    }

    #[tokio::test]
    async fn nothing_is_written_when_the_config_already_matches() {
        let f = fixture(node_config()).await;
        let mut state = None;
        let ack = apply(&f.services, &mut state, desired(0, json!({}))).await;
        assert_eq!(ack, Some(ConfigAck::applied()));
        assert!(f.saved().await.is_empty());
        assert_eq!(f.config(), node_config());
        assert!(state.is_some(), "控制面下发了配置（哪怕是空的）就算托管");
    }

    #[tokio::test]
    async fn an_invalid_config_is_reported_and_nothing_changes() {
        let f = fixture(node_config()).await;
        let mut state = None;
        for values in [
            json!({"pool1_size": 0, "segment_time": "02:00:00"}),
            json!({"loggers_level": "biliup=notalevel", "segment_time": "02:00:00"}),
            json!({"threads": "many"}),
        ] {
            let ack = apply(&f.services, &mut state, desired(1, values))
                .await
                .unwrap();
            assert!(!ack.applied);
            assert!(ack.error.is_some());
            assert_eq!(f.config(), node_config());
            assert!(f.saved().await.is_empty());
            assert_eq!(f.services.managers.download_pool_size(), 2);
            assert_eq!(f.log_filter(), "info");
            let managed = state.as_ref().unwrap();
            assert_eq!(managed.error, ack.error);
            assert_eq!(managed.applied, layers::project(&node_config()));
        }
        // 下一份合法的配置照常生效，错误清掉
        let ack = apply(
            &f.services,
            &mut state,
            desired(2, json!({"pool1_size": 3})),
        )
        .await;
        assert_eq!(ack, Some(ConfigAck::applied()));
        assert_eq!(state.unwrap().error, None);
    }

    #[tokio::test]
    async fn a_broken_local_log_level_is_repaired_by_an_override() {
        let f = fixture(Config {
            loggers_level: Some("biliup=notalevel".into()),
            ..node_config()
        })
        .await;
        let mut state = None;
        let ack = apply(&f.services, &mut state, desired(1, json!({"delay": 60})))
            .await
            .unwrap();
        assert!(ack.error.unwrap().contains("loggers_level"));
        assert_eq!(f.config().delay, 300);

        let ack = apply(
            &f.services,
            &mut state,
            desired(1, json!({"delay": 60, "loggers_level": "info"})),
        )
        .await;
        assert_eq!(ack, Some(ConfigAck::applied()));
        assert_eq!(f.config().delay, 60);
        assert_eq!(f.config().loggers_level.as_deref(), Some("info"));
    }

    #[tokio::test]
    async fn dropping_an_override_restores_the_node_value() {
        let f = fixture(node_config()).await;
        let mut state = None;
        apply(
            &f.services,
            &mut state,
            desired(1, json!({"pool1_size": 7})),
        )
        .await;
        assert_eq!(f.services.managers.download_pool_size(), 7);
        apply(&f.services, &mut state, desired(1, json!({}))).await;
        assert_eq!(f.config(), node_config());
        assert_eq!(f.services.managers.download_pool_size(), 2);
    }

    #[tokio::test]
    async fn a_controller_that_stops_sending_config_releases_it() {
        let f = fixture(node_config()).await;
        let mut state = None;
        apply(&f.services, &mut state, desired(1, json!({"delay": 60}))).await;
        let ack = apply(&f.services, &mut state, None).await;
        assert_eq!(ack, None);
        assert!(state.is_none());
        // 生效的配置留作本机配置
        assert_eq!(f.config().delay, 60);
    }

    #[tokio::test]
    async fn resume_reapplies_the_recorded_config_when_the_database_differs() {
        let f = fixture(node_config()).await;
        let mut state = None;
        apply(&f.services, &mut state, desired(1, json!({"delay": 60}))).await;
        let mut managed = state.unwrap();

        // 与记录一致：什么都不做
        let before = managed.clone();
        resume(&f.services, &mut managed).await;
        assert_eq!(managed, before);

        // 库被换回旧配置后重启
        *f.services.config.write().unwrap() = node_config();
        resume(&f.services, &mut managed).await;
        assert_eq!(f.config().delay, 60);
        assert_eq!(f.config().kuaishou_cookie.as_deref(), Some("ks-secret"));
        assert_eq!(managed.applied, before.applied);
    }
}
