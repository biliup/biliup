//! `data/fleet.sqlite3` 的读写。表结构见 `fleet_migrations/`。

use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use iroh::SecretKey;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::ticket::{SECRET_LEN, TOKEN_ID_LEN};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct NodeRow {
    pub id: i64,
    pub name: String,
    pub endpoint_id: String,
    pub labels: String,
    pub allow_hooks: bool,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
    pub last_version: Option<String>,
    pub last_summary: Option<String>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TokenRow {
    pub id: String,
    #[serde(skip)]
    pub secret_hash: String,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub expires_at: i64,
    pub used_at: Option<i64>,
    pub used_by_node: Option<i64>,
}

/// join 秘密校验的结果
#[derive(Debug, PartialEq, Eq)]
pub enum Redeem {
    /// 节点已登记，令牌已作废
    Joined(NodeRow),
    /// 令牌不存在、已用过、已过期或秘密不对
    InvalidToken,
    /// 这个公钥之前被移除过
    Revoked,
}

fn db_error(what: &'static str) -> AppError {
    AppError::Custom(format!("fleet database: {what}"))
}

pub fn hash_secret(secret: &[u8]) -> String {
    hex(&Sha256::digest(secret))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

pub fn parse_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
        out[index] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(out)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// 控制面的 iroh 私钥；首次以 `--controller` 启动时生成并落库。
pub async fn identity(pool: &ConnectionPool, now: i64) -> AppResult<SecretKey> {
    let existing: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT secret_key FROM fleet_identity WHERE id = 1")
            .fetch_optional(pool)
            .await
            .change_context(db_error("read identity"))?;
    if let Some(bytes) = existing {
        let bytes: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| db_error("identity key has the wrong length"))?;
        return Ok(SecretKey::from_bytes(&bytes));
    }
    let key = SecretKey::generate();
    sqlx::query("INSERT INTO fleet_identity (id, secret_key, created_at) VALUES (1, ?, ?)")
        .bind(key.to_bytes().to_vec())
        .bind(now)
        .execute(pool)
        .await
        .change_context(db_error("write identity"))?;
    Ok(key)
}

fn random_token_id() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = [0u8; TOKEN_ID_LEN];
    OsRng.fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

/// 生成一张一次性令牌，返回令牌行与明文秘密（只在这一次返回，库里只存摘要）。
pub async fn create_token(
    pool: &ConnectionPool,
    created_by: Option<i64>,
    now: i64,
    expires_at: i64,
) -> AppResult<(TokenRow, [u8; SECRET_LEN])> {
    let mut secret = [0u8; SECRET_LEN];
    OsRng.fill_bytes(&mut secret);
    let secret_hash = hash_secret(&secret);
    // 36^6 ≈ 2.2e9，撞上已有 id 的概率可以忽略，撞上了就换一个
    for _ in 0..8 {
        let id = random_token_id();
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO fleet_join_tokens (id, secret_hash, created_by, created_at, expires_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&secret_hash)
        .bind(created_by)
        .bind(now)
        .bind(expires_at)
        .execute(pool)
        .await
        .change_context(db_error("create join token"))?
        .rows_affected();
        if inserted == 1 {
            let row = TokenRow {
                id,
                secret_hash,
                created_by,
                created_at: now,
                expires_at,
                used_at: None,
                used_by_node: None,
            };
            return Ok((row, secret));
        }
    }
    Err(db_error("could not allocate a join token id").into())
}

pub async fn list_tokens(pool: &ConnectionPool) -> AppResult<Vec<TokenRow>> {
    sqlx::query_as("SELECT * FROM fleet_join_tokens ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .change_context(db_error("list join tokens"))
}

/// 作废（删掉）一张令牌；不存在时返回 `false`。
pub async fn delete_token(pool: &ConnectionPool, id: &str) -> AppResult<bool> {
    let affected = sqlx::query("DELETE FROM fleet_join_tokens WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .change_context(db_error("delete join token"))?
        .rows_affected();
    Ok(affected > 0)
}

/// relay 准入用：令牌存在、没用过、没过期。只看 id，不校验秘密（秘密只在 iroh 加密连接里出示）。
pub async fn token_is_open(pool: &ConnectionPool, id: &str, now: i64) -> AppResult<bool> {
    let open: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM fleet_join_tokens WHERE id = ? AND used_at IS NULL AND expires_at > ?",
    )
    .bind(id)
    .bind(now)
    .fetch_optional(pool)
    .await
    .change_context(db_error("check join token"))?;
    Ok(open.is_some())
}

/// 校验 join 秘密并登记节点；校验、登记与作废令牌在同一个事务里，同一张票据只能成功一次。
pub async fn redeem_token(
    pool: &ConnectionPool,
    token: &str,
    secret: &[u8],
    endpoint_id: &str,
    name: &str,
    allow_hooks: bool,
    now: i64,
) -> AppResult<Redeem> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let row: Option<TokenRow> = sqlx::query_as(
        "SELECT * FROM fleet_join_tokens WHERE id = ? AND used_at IS NULL AND expires_at > ?",
    )
    .bind(token)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .change_context(db_error("read join token"))?;
    let Some(row) = row else {
        return Ok(Redeem::InvalidToken);
    };
    if !constant_time_eq(hash_secret(secret).as_bytes(), row.secret_hash.as_bytes()) {
        return Ok(Redeem::InvalidToken);
    }

    let existing: Option<NodeRow> =
        sqlx::query_as("SELECT * FROM fleet_nodes WHERE endpoint_id = ?")
            .bind(endpoint_id)
            .fetch_optional(&mut *tx)
            .await
            .change_context(db_error("read node"))?;
    let node = match existing {
        Some(node) if node.revoked_at.is_some() => return Ok(Redeem::Revoked),
        // 同一把钥匙重复 join（例如上次写 node.json 失败）：沿用原来的节点
        Some(node) => node,
        None => sqlx::query_as(
            "INSERT INTO fleet_nodes (name, endpoint_id, allow_hooks, created_at, last_seen_at) \
             VALUES (?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(name)
        .bind(endpoint_id)
        .bind(allow_hooks)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .change_context(db_error("insert node"))?,
    };
    sqlx::query("UPDATE fleet_join_tokens SET used_at = ?, used_by_node = ? WHERE id = ?")
        .bind(now)
        .bind(node.id)
        .bind(token)
        .execute(&mut *tx)
        .await
        .change_context(db_error("consume join token"))?;
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(Redeem::Joined(node))
}

pub async fn node_by_endpoint(
    pool: &ConnectionPool,
    endpoint_id: &str,
) -> AppResult<Option<NodeRow>> {
    sqlx::query_as("SELECT * FROM fleet_nodes WHERE endpoint_id = ?")
        .bind(endpoint_id)
        .fetch_optional(pool)
        .await
        .change_context(db_error("read node"))
}

/// 未被移除的节点，按加入顺序
pub async fn list_nodes(pool: &ConnectionPool) -> AppResult<Vec<NodeRow>> {
    sqlx::query_as("SELECT * FROM fleet_nodes WHERE revoked_at IS NULL ORDER BY id")
        .fetch_all(pool)
        .await
        .change_context(db_error("list nodes"))
}

/// 移除节点：只写 `revoked_at`，公钥之后再连上来也会被拒。返回被移除节点的公钥。
pub async fn revoke_node(pool: &ConnectionPool, id: i64, now: i64) -> AppResult<Option<String>> {
    sqlx::query_scalar(
        "UPDATE fleet_nodes SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL RETURNING endpoint_id",
    )
    .bind(now)
    .bind(id)
    .fetch_optional(pool)
    .await
    .change_context(db_error("revoke node"))
}

/// 节点连上时按它 `node.json` 里的 `allow_hooks` 更新：是否接受钩子是节点自己的意愿
pub async fn set_allow_hooks(pool: &ConnectionPool, id: i64, allow_hooks: bool) -> AppResult<()> {
    sqlx::query("UPDATE fleet_nodes SET allow_hooks = ? WHERE id = ?")
        .bind(allow_hooks)
        .bind(id)
        .execute(pool)
        .await
        .change_context(db_error("update node allow_hooks"))?;
    Ok(())
}

pub async fn record_seen(
    pool: &ConnectionPool,
    id: i64,
    seen_at: i64,
    version: &str,
    summary: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE fleet_nodes SET last_seen_at = ?, last_version = ?, \
         last_summary = COALESCE(?, last_summary) WHERE id = ?",
    )
    .bind(seen_at)
    .bind(version)
    .bind(summary)
    .bind(id)
    .execute(pool)
    .await
    .change_context(db_error("record node heartbeat"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    async fn pool() -> (tempfile::TempDir, ConnectionPool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn identity_is_generated_once_and_then_reused() {
        let (_dir, pool) = pool().await;
        let first = identity(&pool, 1).await.unwrap();
        let second = identity(&pool, 2).await.unwrap();
        assert_eq!(first.public(), second.public());
    }

    #[tokio::test]
    async fn a_token_redeems_exactly_once() {
        let (_dir, pool) = pool().await;
        let (token, secret) = create_token(&pool, Some(1), 1_000, 10_000).await.unwrap();
        assert!(super::super::ticket::is_token_id(&token.id));
        assert!(token_is_open(&pool, &token.id, 2_000).await.unwrap());

        let joined = redeem_token(&pool, &token.id, &secret, "aa", "nas", false, 2_000)
            .await
            .unwrap();
        let Redeem::Joined(node) = joined else {
            panic!("{joined:?}")
        };
        assert_eq!(node.name, "nas");
        assert!(!token_is_open(&pool, &token.id, 2_000).await.unwrap());

        let again = redeem_token(&pool, &token.id, &secret, "bb", "pi", false, 2_001)
            .await
            .unwrap();
        assert_eq!(again, Redeem::InvalidToken);
        assert_eq!(list_nodes(&pool).await.unwrap().len(), 1);

        let tokens = list_tokens(&pool).await.unwrap();
        assert_eq!(tokens[0].used_by_node, Some(node.id));
    }

    #[tokio::test]
    async fn wrong_secret_and_expired_tokens_are_rejected() {
        let (_dir, pool) = pool().await;
        let (token, mut secret) = create_token(&pool, None, 1_000, 10_000).await.unwrap();
        let expired = redeem_token(&pool, &token.id, &secret, "aa", "n", false, 10_000)
            .await
            .unwrap();
        assert_eq!(expired, Redeem::InvalidToken);
        assert!(!token_is_open(&pool, &token.id, 10_000).await.unwrap());

        secret[0] ^= 1;
        let wrong = redeem_token(&pool, &token.id, &secret, "aa", "n", false, 2_000)
            .await
            .unwrap();
        assert_eq!(wrong, Redeem::InvalidToken);
        // 秘密不对不会把令牌用掉
        assert!(token_is_open(&pool, &token.id, 2_000).await.unwrap());
    }

    #[tokio::test]
    async fn revoked_keys_cannot_join_again() {
        let (_dir, pool) = pool().await;
        let (token, secret) = create_token(&pool, None, 1_000, 10_000).await.unwrap();
        let Redeem::Joined(node) = redeem_token(&pool, &token.id, &secret, "aa", "n", false, 2_000)
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            revoke_node(&pool, node.id, 3_000).await.unwrap().as_deref(),
            Some("aa")
        );
        assert_eq!(revoke_node(&pool, node.id, 3_001).await.unwrap(), None);
        assert!(list_nodes(&pool).await.unwrap().is_empty());

        let (token, secret) = create_token(&pool, None, 4_000, 10_000).await.unwrap();
        let again = redeem_token(&pool, &token.id, &secret, "aa", "n", false, 5_000)
            .await
            .unwrap();
        assert_eq!(again, Redeem::Revoked);
    }

    #[tokio::test]
    async fn deleting_a_token_voids_it() {
        let (_dir, pool) = pool().await;
        let (token, _) = create_token(&pool, None, 1_000, 10_000).await.unwrap();
        assert!(delete_token(&pool, &token.id).await.unwrap());
        assert!(!delete_token(&pool, &token.id).await.unwrap());
        assert!(!token_is_open(&pool, &token.id, 2_000).await.unwrap());
    }

    #[tokio::test]
    async fn heartbeat_summary_is_kept_when_absent() {
        let (_dir, pool) = pool().await;
        let (token, secret) = create_token(&pool, None, 1_000, 10_000).await.unwrap();
        let Redeem::Joined(node) = redeem_token(&pool, &token.id, &secret, "aa", "n", true, 2_000)
            .await
            .unwrap()
        else {
            panic!()
        };
        record_seen(&pool, node.id, 3_000, "1.2.8", Some("{\"rooms\":1}"))
            .await
            .unwrap();
        record_seen(&pool, node.id, 4_000, "1.2.8", None)
            .await
            .unwrap();
        let row = node_by_endpoint(&pool, "aa").await.unwrap().unwrap();
        assert_eq!(row.last_seen_at, Some(4_000));
        assert_eq!(row.last_summary.as_deref(), Some("{\"rooms\":1}"));
        assert!(row.allow_hooks);
    }

    #[test]
    fn hex_round_trips() {
        let bytes = [0u8, 1, 0xab, 0xff];
        assert_eq!(hex(&bytes), "0001abff");
        assert_eq!(parse_hex::<4>("0001abff"), Some(bytes));
        assert_eq!(parse_hex::<4>("0001abf"), None);
        assert_eq!(parse_hex::<4>("0001abfg"), None);
    }
}
