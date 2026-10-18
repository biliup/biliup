//! 分层配置在 `data/fleet.sqlite3` 里的读写：`fleet_config`（全局，按版本追加）与
//! `fleet_nodes.config_override`（节点覆盖）。表结构见 `fleet_migrations/3_config.sql`。
//!
//! 这里只管存取，不做校验；写进来的 JSON 由调用方保证只含白名单键（见 [`super::layers`]）。

use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use serde::Serialize;
use serde_json::{Map, Value};

/// 全局配置保留的历史版本数；更早的在保存新版本时删掉
pub const KEEP_VERSIONS: i64 = 20;

/// `fleet_config` 的一行
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ConfigVersion {
    pub version: i64,
    pub config: Map<String, Value>,
    pub updated_at: i64,
    pub updated_by: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct ConfigRow {
    version: i64,
    config: String,
    updated_at: i64,
    updated_by: Option<i64>,
}

impl ConfigRow {
    fn decode(self) -> AppResult<ConfigVersion> {
        Ok(ConfigVersion {
            version: self.version,
            config: parse_object(&self.config).attach("fleet_config.config")?,
            updated_at: self.updated_at,
            updated_by: self.updated_by,
        })
    }
}

fn db_error(what: &'static str) -> AppError {
    AppError::Custom(format!("fleet database: {what}"))
}

fn parse_object(text: &str) -> AppResult<Map<String, Value>> {
    serde_json::from_str(text).change_context(db_error("stored config is not a JSON object"))
}

fn encode(object: &Map<String, Value>) -> AppResult<String> {
    serde_json::to_string(object).change_context(db_error("encode config"))
}

/// 当前生效的全局配置；从没保存过时为 `None`
pub async fn latest_config(pool: &ConnectionPool) -> AppResult<Option<ConfigVersion>> {
    let row: Option<ConfigRow> =
        sqlx::query_as("SELECT * FROM fleet_config ORDER BY version DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .change_context(db_error("read fleet config"))?;
    row.map(ConfigRow::decode).transpose()
}

/// 保存一版全局配置，返回新版本；同一事务里删掉超出 [`KEEP_VERSIONS`] 的旧版本
pub async fn save_config(
    pool: &ConnectionPool,
    config: &Map<String, Value>,
    updated_by: Option<i64>,
    now: i64,
) -> AppResult<ConfigVersion> {
    let text = encode(config)?;
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let row: ConfigRow = sqlx::query_as(
        "INSERT INTO fleet_config (config, updated_at, updated_by) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(&text)
    .bind(now)
    .bind(updated_by)
    .fetch_one(&mut *tx)
    .await
    .change_context(db_error("insert fleet config"))?;
    sqlx::query("DELETE FROM fleet_config WHERE version <= ?")
        .bind(row.version - KEEP_VERSIONS)
        .execute(&mut *tx)
        .await
        .change_context(db_error("prune fleet config history"))?;
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    row.decode()
}

/// Conditional global write used by the configuration editor.  The version
/// check and insert share one write transaction, so two editors based on the
/// same version cannot silently overwrite one another.
pub async fn save_config_if_version(
    pool: &ConnectionPool,
    expected_version: i64,
    config: &Map<String, Value>,
    updated_by: Option<i64>,
    now: i64,
) -> AppResult<ConditionalGlobal> {
    let text = encode(config)?;
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .change_context(db_error("begin conditional transaction"))?;
    let current: Option<ConfigRow> =
        sqlx::query_as("SELECT * FROM fleet_config ORDER BY version DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .change_context(db_error("read fleet config version"))?;
    if current.as_ref().map_or(0, |current| current.version) != expected_version {
        tx.rollback()
            .await
            .change_context(db_error("rollback stale fleet config"))?;
        return Ok(ConditionalGlobal::Conflict);
    }
    if let Some(current) = current {
        let current = current.decode()?;
        if current.config == *config {
            tx.rollback()
                .await
                .change_context(db_error("rollback unchanged fleet config"))?;
            return Ok(ConditionalGlobal::Unchanged(current));
        }
    }
    let row: ConfigRow = sqlx::query_as(
        "INSERT INTO fleet_config (config, updated_at, updated_by) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(&text)
    .bind(now)
    .bind(updated_by)
    .fetch_one(&mut *tx)
    .await
    .change_context(db_error("insert conditional fleet config"))?;
    sqlx::query("DELETE FROM fleet_config WHERE version <= ?")
        .bind(row.version - KEEP_VERSIONS)
        .execute(&mut *tx)
        .await
        .change_context(db_error("prune conditional fleet config history"))?;
    tx.commit()
        .await
        .change_context(db_error("commit conditional fleet config"))?;
    Ok(ConditionalGlobal::Saved(row.decode()?))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionalGlobal {
    Saved(ConfigVersion),
    Unchanged(ConfigVersion),
    Conflict,
}

/// 保留着的全局配置版本，新的在前
pub async fn config_history(pool: &ConnectionPool) -> AppResult<Vec<ConfigVersion>> {
    let rows: Vec<ConfigRow> = sqlx::query_as("SELECT * FROM fleet_config ORDER BY version DESC")
        .fetch_all(pool)
        .await
        .change_context(db_error("read fleet config history"))?;
    rows.into_iter().map(ConfigRow::decode).collect()
}

/// 节点覆盖；节点不存在或已移除时为 `None`
pub async fn node_override(
    pool: &ConnectionPool,
    node_id: i64,
) -> AppResult<Option<Map<String, Value>>> {
    let text: Option<String> = sqlx::query_scalar(
        "SELECT config_override FROM fleet_nodes WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .change_context(db_error("read node config override"))?;
    text.as_deref()
        .map(parse_object)
        .transpose()
        .attach("fleet_nodes.config_override")
}

/// 未移除节点的覆盖，节点列表用
pub async fn node_overrides(
    pool: &ConnectionPool,
) -> AppResult<std::collections::HashMap<i64, Map<String, Value>>> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, config_override FROM fleet_nodes WHERE revoked_at IS NULL")
            .fetch_all(pool)
            .await
            .change_context(db_error("list node config overrides"))?;
    rows.into_iter()
        .map(|(id, text)| Ok((id, parse_object(&text)?)))
        .collect()
}

/// 整份替换节点覆盖；节点不存在或已移除时返回 `false`
pub async fn set_node_override(
    pool: &ConnectionPool,
    node_id: i64,
    patch: &Map<String, Value>,
) -> AppResult<bool> {
    let affected = sqlx::query(
        "UPDATE fleet_nodes SET config_override = ? WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(encode(patch)?)
    .bind(node_id)
    .execute(pool)
    .await
    .change_context(db_error("write node config override"))?
    .rows_affected();
    Ok(affected > 0)
}

/// Result of a conditional node override write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalOverride {
    Saved,
    Conflict,
    NotFound,
}

/// Compare the parsed JSON object and replace the override in one write
/// transaction.  Comparing decoded objects avoids rejecting a valid match
/// merely because SQLite stored object keys in a different order.
pub async fn set_node_override_if_current(
    pool: &ConnectionPool,
    node_id: i64,
    expected: &Map<String, Value>,
    expected_global_version: i64,
    patch: &Map<String, Value>,
) -> AppResult<ConditionalOverride> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .change_context(db_error("begin conditional override transaction"))?;
    let current: Option<String> = sqlx::query_scalar(
        "SELECT config_override FROM fleet_nodes WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(node_id)
    .fetch_optional(&mut *tx)
    .await
    .change_context(db_error("read conditional node override"))?;
    let Some(current) = current else {
        tx.rollback()
            .await
            .change_context(db_error("rollback missing node override"))?;
        return Ok(ConditionalOverride::NotFound);
    };
    if parse_object(&current)? != *expected {
        tx.rollback()
            .await
            .change_context(db_error("rollback stale node override"))?;
        return Ok(ConditionalOverride::Conflict);
    }
    let current_global: Option<i64> =
        sqlx::query_scalar("SELECT version FROM fleet_config ORDER BY version DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .change_context(db_error("read conditional override global version"))?;
    if current_global.unwrap_or(0) != expected_global_version {
        tx.rollback()
            .await
            .change_context(db_error("rollback stale override global version"))?;
        return Ok(ConditionalOverride::Conflict);
    }
    let affected = sqlx::query(
        "UPDATE fleet_nodes SET config_override = ? WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(encode(patch)?)
    .bind(node_id)
    .execute(&mut *tx)
    .await
    .change_context(db_error("write conditional node override"))?
    .rows_affected();
    tx.commit()
        .await
        .change_context(db_error("commit conditional node override"))?;
    Ok(if affected > 0 {
        ConditionalOverride::Saved
    } else {
        ConditionalOverride::NotFound
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::fleet::store;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use serde_json::json;

    async fn pool() -> (tempfile::TempDir, ConnectionPool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        (dir, pool)
    }

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    async fn join(pool: &ConnectionPool, endpoint: &str) -> store::NodeRow {
        let (token, secret) = store::create_token(pool, None, 1, 10_000).await.unwrap();
        match store::redeem_token(pool, &token.id, &secret, endpoint, "n", false, 2)
            .await
            .unwrap()
        {
            store::Redeem::Joined(node) => node,
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn global_config_versions_increase_and_old_ones_are_pruned() {
        let (_dir, pool) = pool().await;
        assert_eq!(latest_config(&pool).await.unwrap(), None);
        assert!(config_history(&pool).await.unwrap().is_empty());

        let first = save_config(
            &pool,
            &object(json!({"segment_time": "01:00:00"})),
            Some(7),
            10,
        )
        .await
        .unwrap();
        assert_eq!(first.version, 1);
        assert_eq!(first.updated_by, Some(7));
        assert_eq!(latest_config(&pool).await.unwrap(), Some(first));

        for n in 0..KEEP_VERSIONS + 5 {
            save_config(&pool, &object(json!({ "delay": n })), None, 20 + n)
                .await
                .unwrap();
        }
        let history = config_history(&pool).await.unwrap();
        assert_eq!(history.len() as i64, KEEP_VERSIONS);
        let latest = latest_config(&pool).await.unwrap().unwrap();
        assert_eq!(latest.version, KEEP_VERSIONS + 6);
        assert_eq!(latest.config, object(json!({ "delay": KEEP_VERSIONS + 4 })));
        assert_eq!(history[0], latest);
        assert_eq!(history.last().unwrap().version, 7);
    }

    #[tokio::test]
    async fn versions_are_not_reused_after_pruning() {
        let (_dir, pool) = pool().await;
        for n in 0..KEEP_VERSIONS + 1 {
            save_config(&pool, &Map::new(), None, n).await.unwrap();
        }
        sqlx::query("DELETE FROM fleet_config")
            .execute(&pool)
            .await
            .unwrap();
        let next = save_config(&pool, &Map::new(), None, 99).await.unwrap();
        assert_eq!(next.version, KEEP_VERSIONS + 2);
    }

    #[tokio::test]
    async fn conditional_global_save_is_compare_and_swap() {
        let (_dir, pool) = pool().await;
        let initial = save_config(&pool, &object(json!({"delay": 1})), None, 1)
            .await
            .unwrap();
        let first = save_config_if_version(
            &pool,
            initial.version,
            &object(json!({"delay": 2})),
            None,
            2,
        )
        .await
        .unwrap();
        let ConditionalGlobal::Saved(first) = first else {
            panic!("expected a saved conditional config")
        };
        assert_eq!(first.version, initial.version + 1);
        assert!(matches!(
            save_config_if_version(&pool, first.version, &first.config, None, 3)
                .await
                .unwrap(),
            ConditionalGlobal::Unchanged(_)
        ));
        assert_eq!(
            save_config_if_version(
                &pool,
                initial.version,
                &object(json!({"delay": 3})),
                None,
                3,
            )
            .await
            .unwrap(),
            ConditionalGlobal::Conflict
        );
        assert_eq!(latest_config(&pool).await.unwrap(), Some(first));
    }

    #[tokio::test]
    async fn concurrent_conditional_global_writers_have_one_winner() {
        let (_dir, pool) = pool().await;
        let initial = save_config(&pool, &object(json!({"delay": 1})), None, 1)
            .await
            .unwrap();
        let first_config = object(json!({"delay": 2}));
        let second_config = object(json!({"delay": 3}));
        let (first, second) = tokio::join!(
            save_config_if_version(&pool, initial.version, &first_config, None, 2,),
            save_config_if_version(&pool, initial.version, &second_config, None, 3,),
        );
        let results = [first.unwrap(), second.unwrap()];
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, ConditionalGlobal::Saved(_)))
                .count(),
            1,
            "{results:?}"
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, ConditionalGlobal::Conflict))
                .count(),
            1,
            "{results:?}"
        );
    }

    #[tokio::test]
    async fn node_override_defaults_to_empty_and_is_replaced_whole() {
        let (_dir, pool) = pool().await;
        let node = join(&pool, "aa").await;
        assert_eq!(
            node_override(&pool, node.id).await.unwrap(),
            Some(Map::new())
        );
        assert_eq!(node_override(&pool, node.id + 1).await.unwrap(), None);

        let patch = object(json!({"pool1_size": 2, "file_size": null}));
        assert!(set_node_override(&pool, node.id, &patch).await.unwrap());
        assert_eq!(
            node_override(&pool, node.id).await.unwrap(),
            Some(patch.clone())
        );
        assert_eq!(
            node_overrides(&pool).await.unwrap(),
            std::collections::HashMap::from([(node.id, patch)])
        );
        assert!(
            !set_node_override(&pool, node.id + 1, &Map::new())
                .await
                .unwrap()
        );

        // NodeRow 用 SELECT * 读，多出来的列不影响
        assert_eq!(
            store::node(&pool, node.id).await.unwrap().unwrap().id,
            node.id
        );

        store::revoke_node(&pool, node.id, 3).await.unwrap();
        assert_eq!(node_override(&pool, node.id).await.unwrap(), None);
        assert!(
            !set_node_override(&pool, node.id, &Map::new())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn conditional_node_override_rejects_a_stale_writer() {
        let (_dir, pool) = pool().await;
        let node = join(&pool, "conditional-override").await;
        let expected = object(json!({}));
        let first_override = object(json!({"pool1_size": 2}));
        let second_override = object(json!({"pool1_size": 3}));
        let (first, second) = tokio::join!(
            set_node_override_if_current(&pool, node.id, &expected, 0, &first_override,),
            set_node_override_if_current(&pool, node.id, &expected, 0, &second_override,),
        );
        let results = [first.unwrap(), second.unwrap()];
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == ConditionalOverride::Saved)
                .count(),
            1,
            "{results:?}"
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == ConditionalOverride::Conflict)
                .count(),
            1,
            "{results:?}"
        );
        // Formatting and key order in stored JSON do not affect the expected-state comparison.
        let mut expected = node_override(&pool, node.id).await.unwrap().unwrap();
        let pool1_size = expected["pool1_size"].as_i64().unwrap();
        expected.insert("file_size".into(), json!(10));
        sqlx::query("UPDATE fleet_nodes SET config_override = ? WHERE id = ?")
            .bind(format!(
                r#"{{ "file_size": 10, "pool1_size": {pool1_size} }}"#
            ))
            .bind(node.id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            set_node_override_if_current(&pool, node.id, &expected, 0, &Map::new())
                .await
                .unwrap(),
            ConditionalOverride::Saved
        );
        save_config(&pool, &object(json!({"delay": 4})), None, 4)
            .await
            .unwrap();
        assert_eq!(
            set_node_override_if_current(
                &pool,
                node.id,
                &Map::new(),
                0,
                &object(json!({"pool1_size": 4}))
            )
            .await
            .unwrap(),
            ConditionalOverride::Conflict
        );
    }
}
