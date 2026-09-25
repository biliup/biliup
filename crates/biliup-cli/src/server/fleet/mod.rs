//! biliup Fleet：一台控制面管理多台录播节点（#1744）。
//!
//! 控制面（`biliup server --controller`）与节点之间只有 iroh QUIC 连接，连接一律由节点发起，
//! 两端都经控制面内嵌的 relay 中转、能打洞时转直连。单机模式（不带 `--controller`、
//! 也没有 `data/node.json`）不创建 iroh 端点、不开任何端口、不写任何 Fleet 文件。
//!
//! 这一阶段只做加入、心跳与在线状态：不分派房间、不下发配置。

pub mod store;
pub mod ticket;

use sqlx::migrate::Migrator;

/// 控制面独立库的迁移，与主库 `migrations/` 各自编号
pub static FLEET_MIGRATOR: Migrator = sqlx::migrate!("./fleet_migrations");

pub const FLEET_DB: &str = "data/fleet.sqlite3";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    /// 已发布的 Fleet 迁移同样是只读历史，规矩与主库的 `shipped_migration_checksums_are_frozen` 相同：
    /// 修 SQL 只能新增迁移文件。
    #[test]
    fn shipped_fleet_migration_checksums_are_frozen() {
        const FROZEN: &[(i64, &str)] = &[(
            1,
            "ffd740cff5fdaeb33e832d290112ce69dbc46d8993f753d7d98acb6bd9b634cc929890bae23a4385280f18dafaabe8c1",
        )];
        let embedded: Vec<(i64, String)> = FLEET_MIGRATOR
            .iter()
            .map(|migration| (migration.version, store::hex(&migration.checksum)))
            .collect();
        let frozen: Vec<(i64, String)> = FROZEN
            .iter()
            .map(|(version, checksum)| (*version, (*checksum).to_string()))
            .collect();
        assert_eq!(embedded, frozen);
    }

    #[tokio::test]
    async fn fleet_database_is_separate_and_numbered_from_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(versions, [1]);
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'fleet_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            tables,
            ["fleet_identity", "fleet_join_tokens", "fleet_nodes"]
        );
        // 主库的表一张都不在这里
        let foreign: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('livestreamers', 'web_users', 'clips')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(foreign, 0);
        pool.close().await;
        // 重开不会重复迁移
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
