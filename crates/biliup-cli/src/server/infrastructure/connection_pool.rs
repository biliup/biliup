use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::path::Path;
use tracing::{info, warn};

/// SQLite连接池类型别名
pub type ConnectionPool = Pool<Sqlite>;

/// 一条曾经被就地改写过的迁移。
///
/// `legacy_checksums` 是历史版本里该文件内容的 SHA-384（sqlx 记在 `_sqlx_migrations`
/// 里的那一列），`repair` 是把旧内容的执行效果补成新内容所需的语句。
struct SupersededMigration {
    version: i64,
    /// 旧内容在 LF 与 CRLF 检出下的摘要都要登记：`.gitattributes` 把迁移统一成 LF 之前，
    /// Windows 发布包（`windows-latest` 上以 `core.autocrlf=true` 检出）和 Windows 本地
    /// 构建记进库里的都是 CRLF 摘要。
    legacy_checksums: &'static [&'static str],
    /// 必须幂等：老库可能已经处于新内容期望的状态，也可能重复启动多次。
    repair: &'static [&'static str],
}

/// 迁移文件是只读历史：sqlx 会用文件内容的 SHA-384 校验每条已应用的迁移，一旦
/// 就地改写，所有老库都会在启动时以
/// `migration N was previously applied but has been modified` 整体失败 —— 服务起不来，
/// 容器无限重启（#1701 / #1702）。修 SQL 的正确做法始终是新增一个迁移文件。
///
/// 这张表只给**已经发生过**的改写兜底，不是继续改写的许可。登记一条的前提是：
/// 对已经跑过旧内容的库来说，新旧内容的差异能用下面这组幂等语句补齐。
const SUPERSEDED_MIGRATIONS: &[SupersededMigration] = &[
    // 迁移 2 原本写 `UPDATE uploadstreamers SET tags = null`，而该列是 JSON NOT NULL，
    // 从 Python 版升级且 tags 为空串的实例会卡死在这一步；v1.2.5 把它改成了
    // `SET tags = '[]'`（#1684）。改动本身是对的，但让 v1.2.4 及更早版本建立的库
    // 全部校验失配。两版内容的唯一差异就是这一条 UPDATE。
    SupersededMigration {
        version: 2,
        legacy_checksums: &[
            // LF 检出：Linux / macOS / Docker 构建
            "fcc6436a889297e5c28f2a0f12196e5eee5975ba352032c8201dfa61fb1b9c3fd01ddc41ddfa71bd7423999521b895eb",
            // CRLF 检出：v1.1.8 ~ v1.2.4 的 Windows 发布包内嵌的就是这个，Windows 本地构建同理
            "f636cd5b7b62ae53f422ee22daa1c957ddb9d932a9b1c8061fda58adb921032fae46f7896f55ea561158ff963b8ea038",
        ],
        repair: &["UPDATE uploadstreamers SET tags = '[]' WHERE tags = '' OR tags = 'null'"],
    },
];

/// 连接管理器
/// 负责管理SQLite数据库连接池的创建和配置
pub struct ConnectionManager;

impl ConnectionManager {
    /// 创建新的数据库连接池
    ///
    /// # 参数
    /// * `path` - 数据库文件路径
    ///
    /// # 返回
    /// 返回配置好的SQLite连接池
    pub async fn new_pool(path: &str) -> AppResult<ConnectionPool> {
        let pool = Self::connect(path).await?;

        // 运行数据库迁移，确保数据库结构是最新的
        let migrator = sqlx::migrate!();
        Self::reconcile_applied_migrations(&pool, &migrator).await?;

        info!("migrations enabled, running...");
        migrator.run(&pool).await.change_context(AppError::Custom(
            "error while running database migrations".to_string(),
        ))?;

        Ok(pool)
    }

    /// 打开一个独立于主库、用自己那套迁移的 SQLite 库（如控制面的 `data/fleet.sqlite3`）。
    ///
    /// 这套迁移从一开始就是 LF、从未被就地改写过，所以不做主库那套历史校验和对齐：
    /// 任何失配都直接交给 sqlx 报错。
    pub async fn new_pool_with(path: &str, migrator: &Migrator) -> AppResult<ConnectionPool> {
        let pool = Self::connect(path).await?;
        migrator
            .run(&pool)
            .await
            .change_context(AppError::Custom(format!(
                "error while running database migrations for {path}"
            )))?;
        Ok(pool)
    }

    async fn connect(path: &str) -> AppResult<ConnectionPool> {
        // 创建所有父级目录（如果不存在）
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .change_context(AppError::Unknown)
                .attach_with(|| path.to_string())?; // 创建 data/ 目录
        }

        let db_url = format!("sqlite://{path}");

        // 创建数据库文件（如果不存在）
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .change_context(AppError::Unknown)?;

        // 创建连接池，最大连接数设为2
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await
            .change_context(AppError::Custom(
                "error while initializing the database connection pool".to_string(),
            ))?;
        Ok(pool)
    }

    /// 在 sqlx 校验之前，把两类可以放行的校验和失配对齐到当前二进制内嵌的摘要，让老库
    /// 还能继续升级：
    ///
    /// - 内容没变、只是换行不同：`.gitattributes` 把迁移统一成 LF 之前，Windows 发布包和
    ///   Windows 本地构建记进库里的都是 CRLF 检出的摘要。只改写校验和，不补跑任何语句。
    /// - 记录**恰好等于** `SUPERSEDED_MIGRATIONS` 里登记过的历史摘要：补丁语句与校验和
    ///   改写在同一个事务里，中途崩溃不会留下「补丁没跑但校验和已对齐」的中间态。
    ///
    /// 其余任何失配都原样留给 sqlx 报错，免得把用户自己改过的迁移悄悄放行。
    async fn reconcile_applied_migrations(
        pool: &ConnectionPool,
        migrator: &Migrator,
    ) -> AppResult<()> {
        let bookkeeping_exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
        )
        .fetch_optional(pool)
        .await
        .change_context(AppError::Custom(
            "error while inspecting database migration state".to_string(),
        ))?;
        // 全新安装：连记账表都还没有，没有任何历史需要对齐。
        if bookkeeping_exists.is_none() {
            return Ok(());
        }

        // 没跑过（含跑失败留下的记录）的迁移由 sqlx 正常应用，不需要对齐。
        let applied: Vec<(i64, Vec<u8>)> = sqlx::query_as(
            "SELECT version, checksum FROM _sqlx_migrations WHERE success = TRUE ORDER BY version",
        )
        .fetch_all(pool)
        .await
        .change_context(AppError::Custom(
            "error while inspecting database migration state".to_string(),
        ))?;

        for (version, checksum) in applied {
            let Some(current) = migrator
                .iter()
                .find(|m| m.version == version && !m.migration_type.is_down_migration())
            else {
                continue;
            };
            if checksum == current.checksum.as_ref() {
                continue;
            }

            if checksum == crlf_checksum(current) {
                sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                    .bind(current.checksum.to_vec())
                    .bind(version)
                    .execute(pool)
                    .await
                    .change_context(AppError::Custom(
                        "error while reconciling database migration state".to_string(),
                    ))?;
                info!(
                    version,
                    "该迁移记录的是 CRLF 检出下的摘要，内容与当前一致，已对齐校验和"
                );
                continue;
            }

            let applied_checksum = hex_lower(&checksum);
            let Some(entry) = SUPERSEDED_MIGRATIONS.iter().find(|entry| {
                entry.version == version
                    && entry.legacy_checksums.contains(&applied_checksum.as_str())
            }) else {
                warn!(
                    version,
                    "已应用的迁移既不是当前内容也不是已知的历史内容，不做自动对齐"
                );
                continue;
            };

            let mut tx = pool.begin().await.change_context(AppError::Custom(
                "error while reconciling database migration state".to_string(),
            ))?;
            for statement in entry.repair {
                sqlx::query(statement)
                    .execute(&mut *tx)
                    .await
                    .change_context(AppError::Custom(
                        "error while reconciling database migration state".to_string(),
                    ))?;
            }
            sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                .bind(current.checksum.to_vec())
                .bind(version)
                .execute(&mut *tx)
                .await
                .change_context(AppError::Custom(
                    "error while reconciling database migration state".to_string(),
                ))?;
            tx.commit().await.change_context(AppError::Custom(
                "error while reconciling database migration state".to_string(),
            ))?;

            info!(version, "该迁移在新版本中被修正，已补跑差异并对齐校验和");
        }

        Ok(())
    }
}

/// 同一份迁移内容以 CRLF 检出时 sqlx 会记下的校验和。
///
/// 内嵌的 SQL 一定是 LF（`build.rs` 在编译期把关），把每个 `\n` 换成 `\r\n` 就是
/// `core.autocrlf=true` 检出的字节；摘要交给 sqlx 自己的 `Migration::new` 算，与它记账的
/// 算法保持一致。
fn crlf_checksum(migration: &Migration) -> Vec<u8> {
    Migration::new(
        migration.version,
        migration.description.clone(),
        migration.migration_type,
        Cow::Owned(migration.sql.replace('\n', "\r\n")),
        migration.no_tx,
    )
    .checksum
    .into_owned()
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::{ConnectionManager, ConnectionPool, crlf_checksum, hex_lower};

    /// v1.1.8 ~ v1.2.4 发布包给迁移 2（改写前的内容）记下的摘要：Linux / macOS / Docker
    /// 包是 LF 检出，Windows 包是在 `windows-latest` 上以 CRLF 检出后编的。
    const V1_2_4_MIGRATION_2_LF: &str = "fcc6436a889297e5c28f2a0f12196e5eee5975ba352032c8201dfa61fb1b9c3fd01ddc41ddfa71bd7423999521b895eb";
    const V1_2_4_MIGRATION_2_CRLF: &str = "f636cd5b7b62ae53f422ee22daa1c957ddb9d932a9b1c8061fda58adb921032fae46f7896f55ea561158ff963b8ea038";

    fn decode_hex(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    async fn set_checksum(pool: &ConnectionPool, version: i64, checksum: &[u8]) {
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
            .bind(checksum.to_vec())
            .bind(version)
            .execute(pool)
            .await
            .unwrap();
    }

    /// 把一个全新库回退成 v1.2.4 那一代的样子：迁移 4/5 尚未应用，迁移 2 记的是
    /// 改写前的校验和。
    async fn rewind_to_v1_2_4(pool: &ConnectionPool, migration_2_checksum: &[u8]) {
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version IN (4, 5, 6)")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE web_users")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE uploadstreamers DROP COLUMN tid_v2")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 2")
            .bind(migration_2_checksum.to_vec())
            .execute(pool)
            .await
            .unwrap();
    }

    /// 已发布迁移的内容摘要，逐条钉死。
    ///
    /// 这个断言挂掉说明有人改动了一条**已经发布过**的迁移 —— 所有老库都会在升级时
    /// 以校验和失配启动失败（#1701 就是这么来的）。正确做法是把改动挪进一个新的迁移
    /// 文件；确实只能就地改的话，把旧内容在 LF / CRLF 两种检出下的摘要连同幂等补丁
    /// 登记进 `SUPERSEDED_MIGRATIONS`，再更新这里。
    #[test]
    fn shipped_migration_checksums_are_frozen() {
        const FROZEN: &[(i64, &str)] = &[
            (
                1,
                "f0dbce08e61eb0722d99ea5415742b6786a00904aa87bf1d9c21eceabced7484e340c4b51185546cb0578106bbb0c865",
            ),
            (
                2,
                "aef99c14523e93ceed5602d79b6c40c61891c794ea7f5fe8583ec1858315f57afeebfcaf9167a108ba7589745d7e34bf",
            ),
            (
                3,
                "bda5d2a2757539c071a1887ffba044a2660dd03200680d9f8537546680556c987047422c074bd438ecb7ef25a73be51b",
            ),
            (
                4,
                "8c93f71ec95e78418443f93395375eaeb9e0f16cf666fec555aa3165e73faa75e9a336794e292556d8d717a979085aaa",
            ),
            (
                5,
                "f6f913ecbbdf1f4d437bf2ce4778b8d0317353056c4b240795fc9347b4eeac7f71e719d4c799ac2df72b06802ee064c7",
            ),
            (
                6,
                "11d73df52e3a459d5256d4f5d624cbc5e891d14c358d1ef4a63debae235c2fd24ce2fe8745a85ac95312ad4773475ca2",
            ),
            (
                7,
                "66fd3bd6b5d4aab82389bbe03e190ff869f274aa8fdc74ee9110f277ff1dcae237a4b6a2088c84f4d31e997c34badc03",
            ),
            (
                8,
                "edff6c311dbece198eec1e0e4aa55983c56996d9b5ea2a7b352b479dd71149ccfe52145a2178ed52b33fc4ddc4c5ea3e",
            ),
            (
                9,
                "50962387ef63bab0ce70701139798580f8e6d82cfc8fbd80fc601c638e6649b25763b439c636da84bc9c7a142642ccfa",
            ),
            (
                10,
                "6a37aaac20f18ea3e0a9cdb1a6bb971882eb843b5569ea26897a61f328d0813176efebf788d4e8b188f4cf3dd4db2fc6",
            ),
            (
                11,
                "d0b03f1d85267c90184bb260970affe744114d40a5f06e020604bca39307cf13cceebc6401c8d1b8cb47d7162040846e",
            ),
            (
                12,
                "53b87607fc5effda086366a3aa1023e7c273895c363229074fc77b915159838ff8ca90d24cb929536d9c87003c66f7fa",
            ),
        ];
        let embedded = sqlx::migrate!();
        let actual: Vec<(i64, String)> = embedded
            .iter()
            .map(|m| (m.version, hex_lower(&m.checksum)))
            .collect();
        let expected: Vec<(i64, String)> =
            FROZEN.iter().map(|(v, c)| (*v, (*c).to_string())).collect();
        assert_eq!(actual, expected);
    }

    /// #1701：v1.2.5 就地改写了迁移 2，所有 v1.2.4 及更早版本建立的库都会以
    /// `migration 2 was previously applied but has been modified` 启动失败。
    /// 升级必须照常完成：待应用的迁移要跑完，既有数据不能丢。
    #[tokio::test]
    async fn upgrade_from_pre_1_2_5_database_reconciles_rewritten_migration() {
        assert_upgrades_from_v1_2_4(V1_2_4_MIGRATION_2_LF, false).await;
    }

    /// 同一代的库，但由 Windows 发布包建立：迁移 1/3 内容没变、记的是 CRLF 摘要，迁移 2
    /// 记的是改写前内容的 CRLF 摘要。两种失配都得在同一次启动里对齐。
    #[tokio::test]
    async fn upgrade_from_pre_1_2_5_windows_release_database() {
        assert_upgrades_from_v1_2_4(V1_2_4_MIGRATION_2_CRLF, true).await;
    }

    async fn assert_upgrades_from_v1_2_4(migration_2_checksum: &str, crlf_checkout: bool) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        rewind_to_v1_2_4(&pool, &decode_hex(migration_2_checksum)).await;
        if crlf_checkout {
            let embedded = sqlx::migrate!();
            for migration in embedded.iter().filter(|m| m.version == 1 || m.version == 3) {
                set_checksum(&pool, migration.version, &crlf_checksum(migration)).await;
            }
        }
        sqlx::query(
            "INSERT INTO uploadstreamers (id, template_name, tags) VALUES \
             (1, 'from-python', ''), (2, 'normal', '[\"直播录像\"]')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO livestreamers (url, remark, override) VALUES \
             ('https://live.bilibili.com/1', 'placeholder', '{\"file_size\":null}')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .expect("v1.2.4 建立的库必须能升级上来");

        let migrations: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            migrations.iter().map(|m| m.0).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            "待应用的迁移必须补齐"
        );
        let embedded = sqlx::migrate!();
        for (version, checksum) in &migrations {
            let expected = embedded.iter().find(|m| m.version == *version).unwrap();
            assert_eq!(
                hex_lower(checksum),
                hex_lower(&expected.checksum),
                "迁移 {version} 的校验和必须对齐到当前内嵌的摘要"
            );
        }

        let tags: Vec<(i64, String)> =
            sqlx::query_as("SELECT id, tags FROM uploadstreamers ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(tags[0].1, "[]", "空 tags 必须被补成空数组，而不是 NULL");
        assert_eq!(tags[1].1, "[\"直播录像\"]", "正常数据原样保留");

        // 迁移 4/5 确实跑过：占位 null 被清掉，tid_v2 列已存在。
        let override_json: String = sqlx::query_scalar("SELECT override FROM livestreamers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(!override_json.contains("file_size"), "{override_json}");
        sqlx::query("SELECT tid_v2 FROM uploadstreamers")
            .fetch_all(&pool)
            .await
            .expect("迁移 5 必须已应用");
    }

    /// v1.2.5 起的 Windows 发布包（以及 Windows 本地构建）建立的库：内容与当前完全一致，
    /// 只是每条记录都是 CRLF 摘要。只改写校验和，不能借机补跑任何语句。
    #[tokio::test]
    async fn database_recorded_from_crlf_checkout_is_realigned_without_replaying() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let embedded = sqlx::migrate!();
        for migration in embedded.iter() {
            set_checksum(&pool, migration.version, &crlf_checksum(migration)).await;
        }
        // 迁移 2 的补丁会把空串 tags 改成 '[]'，只是换行不同时不该碰到它。
        sqlx::query(
            "INSERT INTO uploadstreamers (id, template_name, tags) VALUES (1, 'untouched', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .expect("CRLF 检出下建立的库必须能直接启动");

        let recorded: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            recorded
                .iter()
                .map(|(version, checksum)| (*version, hex_lower(checksum)))
                .collect::<Vec<_>>(),
            embedded
                .iter()
                .map(|m| (m.version, hex_lower(&m.checksum)))
                .collect::<Vec<_>>(),
        );
        let tags: String = sqlx::query_scalar("SELECT tags FROM uploadstreamers WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tags, "", "只是换行不同，不能补跑迁移 2 的补丁");
    }

    /// `crlf_checksum` 必须复现 Windows 发布包真实记下的摘要。两个值取自官方 v1.2.7
    /// Windows 包（`biliupR-v1.2.7-x86_64-windows.zip`）内嵌的迁移表。
    #[test]
    fn crlf_checksum_matches_windows_release_builds() {
        let embedded = sqlx::migrate!();
        let crlf = |version| {
            hex_lower(&crlf_checksum(
                embedded.iter().find(|m| m.version == version).unwrap(),
            ))
        };
        assert_eq!(
            crlf(1),
            "582585a2cecf6e9b4d1ae9edb1a2fac80b080106a9384b2e4d9be4c158dedd32f6c3c0c5c396634e7f5679d4221c3878"
        );
        assert_eq!(
            crlf(2),
            "10f4a226e937d35e0e49aa2995f3637150135dda5bfee0a4d9caeb7df4f4c1674cb18416901dfd87a4fadb6dc6577414"
        );
    }

    /// 自愈只针对登记在册的历史内容。用户自己改过的迁移仍然要报错，不能被悄悄放行。
    #[tokio::test]
    async fn unknown_migration_checksum_is_not_silently_reconciled() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 2")
            .bind(vec![0u8; 48])
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        assert!(
            ConnectionManager::new_pool(db.to_str().unwrap())
                .await
                .is_err()
        );
    }

    /// 旧版本落库的覆写里 `file_size: null` 只是占位，迁移后必须消失（跟随全局），
    /// 其它显式设置的字段与真正的数值原样保留。
    #[tokio::test]
    async fn migration_strips_placeholder_null_file_size_from_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 4")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO livestreamers (url, remark, override) VALUES \
             ('https://live.bilibili.com/1', 'placeholder', '{\"downloader\":\"sync-downloader\",\"file_size\":null,\"bili_qn\":null}'), \
             ('https://live.bilibili.com/2', 'explicit-size', '{\"file_size\":52428800}'), \
             ('https://live.bilibili.com/3', 'no-override', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT remark, override FROM livestreamers ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();

        let placeholder: serde_json::Value =
            serde_json::from_str(rows[0].1.as_deref().unwrap()).unwrap();
        assert!(
            placeholder.get("file_size").is_none(),
            "占位 null 必须被移除: {placeholder}"
        );
        assert_eq!(placeholder["downloader"], "sync-downloader");
        assert!(
            placeholder["bili_qn"].is_null(),
            "其它字段的 null 无害，保持原样"
        );

        let explicit: serde_json::Value =
            serde_json::from_str(rows[1].1.as_deref().unwrap()).unwrap();
        assert_eq!(explicit["file_size"], 52_428_800);
        assert!(rows[2].1.is_none());
    }

    /// v1.2.8（迁移 1–6）建立的库升级：`streamerinfo` 改名为 `stream_sessions`，数据、外键、
    /// 用户自己加的索引 / 视图 / 触发器都跟着改名，`/v1/streamer-info` 读到的内容不变。
    #[tokio::test]
    async fn upgrade_from_v1_2_8_renames_streamerinfo_to_stream_sessions() {
        use crate::server::infrastructure::models::{FileItem, StreamerInfo};
        use chrono::DateTime;
        use ormlite::Model;

        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let legacy = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite://{}?mode=rwc", db.display()))
            .await
            .unwrap();
        let mut v1_2_8 = sqlx::migrate!();
        v1_2_8.migrations = v1_2_8
            .migrations
            .iter()
            .filter(|m| m.version <= 6)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        v1_2_8.run(&legacy).await.unwrap();
        let rfc3339 = "2026-09-23T10:00:00.123456789+00:00";
        let python = "2024-01-02 03:04:05.678000";
        for sql in [
            // Python 版建的库没有强制外键，可能留有孤儿 filelist 行
            "PRAGMA foreign_keys = OFF",
            "INSERT INTO livestreamers (id, url, remark) VALUES \
             (1, 'https://www.douyu.com/9999', 'douyu'), (2, 'https://live.bilibili.com/6', 'bili')",
            &format!(
                "INSERT INTO streamerinfo (id, name, url, title, date, live_cover_path) VALUES \
                 (1, 'douyu', 'https://www.douyu.com/9999', '第一场', '{rfc3339}', ''), \
                 (2, 'bili', 'https://live.bilibili.com/6', '老数据', '{python}', 'cover.jpg'), \
                 (3, 'gone', 'https://www.huya.com/gone', '主播已删', '{rfc3339}', '')"
            ),
            "INSERT INTO filelist (id, file, streamer_info_id) VALUES \
             (1, 'a.flv', 1), (2, 'b.flv', 1), (3, 'c.mp4', 2), (4, 'orphan.flv', 99)",
            "CREATE INDEX idx_user_filelist ON filelist (streamer_info_id)",
            "CREATE VIEW v_user AS SELECT s.title, f.file FROM streamerinfo s \
             JOIN filelist f ON f.streamer_info_id = s.id",
            "CREATE TRIGGER trg_user AFTER UPDATE OF title ON streamerinfo BEGIN \
             UPDATE filelist SET file = file WHERE streamer_info_id = new.id; END",
        ] {
            sqlx::query(sql).execute(&legacy).await.unwrap();
        }
        legacy.close().await;

        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .expect("v1.2.8 建立的库必须能升级上来");

        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN \
             ('streamerinfo', 'stream_sessions', 'session_streamerinfo', 'segments') ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(tables, vec!["segments", "stream_sessions"]);

        // 外键跟着表名、列名走，ON DELETE CASCADE 保留
        let fk: (String, String, String) = sqlx::query_as(
            "SELECT \"table\", \"from\", on_delete FROM pragma_foreign_key_list('filelist')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            fk,
            (
                "stream_sessions".into(),
                "session_id".into(),
                "CASCADE".into()
            )
        );
        for (name, needles) in [
            ("idx_user_filelist", &["session_id"][..]),
            ("v_user", &["stream_sessions", "session_id"][..]),
            ("trg_user", &["stream_sessions", "session_id"][..]),
        ] {
            let sql: String = sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE name = ?")
                .bind(name)
                .fetch_one(&pool)
                .await
                .unwrap();
            for needle in needles {
                assert!(sql.contains(needle), "{name}: {sql}");
            }
            assert!(!sql.contains("streamer_info_id"), "{name}: {sql}");
        }
        let joined: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v_user")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(joined, 3);
        sqlx::query("UPDATE stream_sessions SET title = title WHERE id = 1")
            .execute(&pool)
            .await
            .expect("触发器改名后仍可执行");

        // 老数据：内容原样；按 url 找回主播，找不到为 NULL；结束时间记为开播时间
        let history = StreamerInfo::select().fetch_all(&pool).await.unwrap();
        assert_eq!(
            history
                .iter()
                .map(|h| (h.id, h.title.as_str(), h.live_cover_path.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (1, "第一场", ""),
                (2, "老数据", "cover.jpg"),
                (3, "主播已删", "")
            ]
        );
        let rfc3339_ms = DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .timestamp_millis();
        let python_ms = chrono::NaiveDateTime::parse_from_str(python, "%F %T%.f")
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(history[0].date.timestamp_millis(), rfc3339_ms);
        type Backfill = (Option<i64>, Option<i64>, Option<i64>, Option<i64>);
        let backfill: Vec<Backfill> = sqlx::query_as(
            "SELECT streamer_id, started_at, ended_at, retain_until FROM stream_sessions ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            backfill,
            vec![
                (Some(1), None, Some(rfc3339_ms), None),
                (Some(2), None, Some(python_ms), None),
                (None, None, Some(rfc3339_ms), None),
            ]
        );
        let json = serde_json::to_value(&history[1]).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            keys,
            vec!["date", "id", "live_cover_path", "name", "title", "url"],
            "/v1/streamer-info 的字段不变"
        );

        let files = FileItem::select()
            .where_("session_id = ?")
            .bind(1)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(
            files.iter().map(|f| f.file.as_str()).collect::<Vec<_>>(),
            vec!["a.flv", "b.flv"]
        );
        assert_eq!(
            serde_json::to_value(&files[0]).unwrap(),
            serde_json::json!({"id": 1, "file": "a.flv", "streamer_info_id": 1}),
            "/v1/streamer-info/files 的字段不变"
        );
        let orphans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM filelist WHERE id = 4")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orphans, 1, "迁移不检查、也不删除孤儿行");

        sqlx::query("DELETE FROM stream_sessions WHERE id = 2")
            .execute(&pool)
            .await
            .unwrap();
        let left: Vec<i64> = sqlx::query_scalar("SELECT id FROM filelist ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(left, vec![1, 2, 4], "删场次级联删文件记录");
        sqlx::query("DELETE FROM livestreamers WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        let streamer: Option<i64> =
            sqlx::query_scalar("SELECT streamer_id FROM stream_sessions WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(streamer, None, "删主播只清空场次的主播外键");
    }

    #[tokio::test]
    async fn identity_migration_fails_closed_on_multiple_existing_administrators() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("DROP INDEX uq_configuration_biliup_identity")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 3")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO configuration (key, value) VALUES ('biliup', 'first'), ('biliup', 'second')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        assert!(
            ConnectionManager::new_pool(db.to_str().unwrap())
                .await
                .is_err()
        );

        let options = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite://{}", db.display()))
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'biliup'")
                .fetch_one(&options)
                .await
                .unwrap();
        assert_eq!(
            count, 2,
            "migration must not pick an administrator implicitly"
        );
    }
}
