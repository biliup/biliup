//! `ha_pair` 与 `ha_sessions` 在 `data/fleet.sqlite3` 里的读写，表结构见 `fleet_migrations/5_ha.sql`。

use super::params::{HaMode, HaParams};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

fn db_error(what: &'static str) -> AppError {
    AppError::Custom(format!("fleet database: {what}"))
}

fn seconds(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn stored(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// 主副配对
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    pub primary_node_id: i64,
    pub standby_node_id: i64,
    pub mode: HaMode,
    pub params: HaParams,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(sqlx::FromRow)]
struct PairRow {
    primary_node_id: i64,
    standby_node_id: i64,
    mode: i64,
    offline_grace: i64,
    standby_upload_delay: i64,
    upload_start_timeout: i64,
    upload_stall_timeout: i64,
    progress_interval: i64,
    manual_timeout: i64,
    delete_standby_copy: bool,
    created_at: i64,
    updated_at: i64,
}

impl PairRow {
    fn decode(self) -> AppResult<Pair> {
        let mode = u8::try_from(self.mode)
            .ok()
            .and_then(|mode| HaMode::try_from(mode).ok())
            .ok_or_else(|| error_stack::Report::new(db_error("ha_pair.mode")))?;
        Ok(Pair {
            primary_node_id: self.primary_node_id,
            standby_node_id: self.standby_node_id,
            mode,
            params: HaParams {
                offline_grace: seconds(self.offline_grace),
                standby_upload_delay: seconds(self.standby_upload_delay),
                upload_start_timeout: seconds(self.upload_start_timeout),
                upload_stall_timeout: seconds(self.upload_stall_timeout),
                progress_interval: seconds(self.progress_interval),
                manual_timeout: seconds(self.manual_timeout),
                delete_standby_copy: self.delete_standby_copy,
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

pub async fn pair(pool: &ConnectionPool) -> AppResult<Option<Pair>> {
    let row: Option<PairRow> = sqlx::query_as("SELECT * FROM ha_pair WHERE id = 1")
        .fetch_optional(pool)
        .await
        .change_context(db_error("read ha_pair"))?;
    row.map(PairRow::decode).transpose()
}

/// 指定或改掉主副配对；已有配对时保留 `created_at`
pub async fn save_pair(
    pool: &ConnectionPool,
    primary: i64,
    standby: i64,
    mode: HaMode,
    params: &HaParams,
    now: i64,
) -> AppResult<Pair> {
    let row: PairRow = sqlx::query_as(
        "INSERT INTO ha_pair (id, primary_node_id, standby_node_id, mode, offline_grace, \
         standby_upload_delay, upload_start_timeout, upload_stall_timeout, progress_interval, \
         manual_timeout, delete_standby_copy, created_at, updated_at) \
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (id) DO UPDATE SET primary_node_id = excluded.primary_node_id, \
         standby_node_id = excluded.standby_node_id, mode = excluded.mode, \
         offline_grace = excluded.offline_grace, standby_upload_delay = excluded.standby_upload_delay, \
         upload_start_timeout = excluded.upload_start_timeout, \
         upload_stall_timeout = excluded.upload_stall_timeout, \
         progress_interval = excluded.progress_interval, manual_timeout = excluded.manual_timeout, \
         delete_standby_copy = excluded.delete_standby_copy, updated_at = excluded.updated_at \
         RETURNING *",
    )
    .bind(primary)
    .bind(standby)
    .bind(i64::from(u8::from(mode)))
    .bind(stored(params.offline_grace))
    .bind(stored(params.standby_upload_delay))
    .bind(stored(params.upload_start_timeout))
    .bind(stored(params.upload_stall_timeout))
    .bind(stored(params.progress_interval))
    .bind(stored(params.manual_timeout))
    .bind(params.delete_standby_copy)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .change_context(db_error("save ha_pair"))?;
    row.decode()
}

/// 解除配对；本来就没有时返回 `false`
pub async fn clear_pair(pool: &ConnectionPool) -> AppResult<bool> {
    let done = sqlx::query("DELETE FROM ha_pair WHERE id = 1")
        .execute(pool)
        .await
        .change_context(db_error("delete ha_pair"))?;
    Ok(done.rows_affected() > 0)
}

/// 主机自己那份的进度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryState {
    /// 主机没有这一场（备机在主机离线期间录的）
    #[default]
    None,
    Recording,
    /// 录完了、还没开始投
    Recorded,
    Uploading,
    Uploaded,
    Failed,
    /// 被过滤或没有文件
    Skipped,
    /// 主机进程在录或投的途中退出了
    Interrupted,
    /// 交给备机负责（主机回来时备机已在录或在投）
    HandedOver,
}

impl PrimaryState {
    pub fn as_str(self) -> &'static str {
        match self {
            PrimaryState::None => "none",
            PrimaryState::Recording => "recording",
            PrimaryState::Recorded => "recorded",
            PrimaryState::Uploading => "uploading",
            PrimaryState::Uploaded => "uploaded",
            PrimaryState::Failed => "failed",
            PrimaryState::Skipped => "skipped",
            PrimaryState::Interrupted => "interrupted",
            PrimaryState::HandedOver => "handed_over",
        }
    }

    fn parse(text: &str) -> Self {
        match text {
            "recording" => PrimaryState::Recording,
            "recorded" => PrimaryState::Recorded,
            "uploading" => PrimaryState::Uploading,
            "uploaded" => PrimaryState::Uploaded,
            "failed" => PrimaryState::Failed,
            "skipped" => PrimaryState::Skipped,
            "interrupted" => PrimaryState::Interrupted,
            "handed_over" => PrimaryState::HandedOver,
            _ => PrimaryState::None,
        }
    }
}

/// 这一场由谁投
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Uploader {
    Primary,
    Standby,
}

impl Uploader {
    fn as_str(self) -> &'static str {
        match self {
            Uploader::Primary => "primary",
            Uploader::Standby => "standby",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "primary" => Some(Uploader::Primary),
            "standby" => Some(Uploader::Standby),
            _ => None,
        }
    }
}

/// `ha_sessions` 的一行
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SessionRecord {
    pub session_key: String,
    pub room_id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub primary_state: PrimaryState,
    pub standby_state: Option<String>,
    pub uploader: Option<Uploader>,
    pub bvid: Option<String>,
    pub upload_bytes: i64,
    pub progress_at: Option<i64>,
    pub reason: Option<String>,
    #[serde(skip)]
    pub local_session_id: Option<i64>,
    #[serde(skip)]
    pub unit_started_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_key: String,
    room_id: i64,
    started_at: i64,
    ended_at: Option<i64>,
    primary_state: String,
    standby_state: Option<String>,
    uploader: Option<String>,
    bvid: Option<String>,
    upload_bytes: i64,
    progress_at: Option<i64>,
    reason: Option<String>,
    local_session_id: Option<i64>,
    unit_started_at: Option<i64>,
    updated_at: i64,
}

impl From<SessionRow> for SessionRecord {
    fn from(row: SessionRow) -> Self {
        SessionRecord {
            session_key: row.session_key,
            room_id: row.room_id,
            started_at: row.started_at,
            ended_at: row.ended_at,
            primary_state: PrimaryState::parse(&row.primary_state),
            standby_state: row.standby_state,
            uploader: row.uploader.as_deref().and_then(Uploader::parse),
            bvid: row.bvid,
            upload_bytes: row.upload_bytes,
            progress_at: row.progress_at,
            reason: row.reason,
            local_session_id: row.local_session_id,
            unit_started_at: row.unit_started_at,
            updated_at: row.updated_at,
        }
    }
}

pub async fn session(pool: &ConnectionPool, key: &str) -> AppResult<Option<SessionRecord>> {
    let row: Option<SessionRow> = sqlx::query_as("SELECT * FROM ha_sessions WHERE session_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .change_context(db_error("read ha_sessions"))?;
    Ok(row.map(SessionRecord::from))
}

/// 整行写入（新增或覆盖）
pub async fn save_session(pool: &ConnectionPool, record: &SessionRecord) -> AppResult<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO ha_sessions (session_key, room_id, started_at, ended_at, \
         primary_state, standby_state, uploader, bvid, upload_bytes, progress_at, reason, \
         local_session_id, unit_started_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.session_key)
    .bind(record.room_id)
    .bind(record.started_at)
    .bind(record.ended_at)
    .bind(record.primary_state.as_str())
    .bind(&record.standby_state)
    .bind(record.uploader.map(Uploader::as_str))
    .bind(&record.bvid)
    .bind(record.upload_bytes)
    .bind(record.progress_at)
    .bind(&record.reason)
    .bind(record.local_session_id)
    .bind(record.unit_started_at)
    .bind(record.updated_at)
    .execute(pool)
    .await
    .change_context(db_error("save ha_sessions"))?;
    Ok(())
}

/// 最近的场次，新的在前
pub async fn recent_sessions(pool: &ConnectionPool, limit: i64) -> AppResult<Vec<SessionRecord>> {
    let rows: Vec<SessionRow> =
        sqlx::query_as("SELECT * FROM ha_sessions ORDER BY started_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await
            .change_context(db_error("read ha_sessions"))?;
    Ok(rows.into_iter().map(SessionRecord::from).collect())
}

/// `since` 之后更新过的场次（主机进程启动时载入）
pub async fn updated_since(pool: &ConnectionPool, since: i64) -> AppResult<Vec<SessionRecord>> {
    let rows: Vec<SessionRow> =
        sqlx::query_as("SELECT * FROM ha_sessions WHERE updated_at >= ? ORDER BY started_at")
            .bind(since)
            .fetch_all(pool)
            .await
            .change_context(db_error("read ha_sessions"))?;
    Ok(rows.into_iter().map(SessionRecord::from).collect())
}

/// `since` 之后更新过、已经有 bvid 的场次（主机重启、备机重连后重发 `Uploaded` 用）
pub async fn uploaded_since(pool: &ConnectionPool, since: i64) -> AppResult<Vec<SessionRecord>> {
    let rows: Vec<SessionRow> = sqlx::query_as(
        "SELECT * FROM ha_sessions WHERE bvid IS NOT NULL AND updated_at >= ? ORDER BY started_at",
    )
    .bind(since)
    .fetch_all(pool)
    .await
    .change_context(db_error("read ha_sessions"))?;
    Ok(rows.into_iter().map(SessionRecord::from).collect())
}

/// 主机自己那份处在这些状态的场次
pub async fn sessions_in_state(
    pool: &ConnectionPool,
    states: &[PrimaryState],
) -> AppResult<Vec<SessionRecord>> {
    let rows: Vec<SessionRow> = sqlx::query_as("SELECT * FROM ha_sessions ORDER BY started_at")
        .fetch_all(pool)
        .await
        .change_context(db_error("read ha_sessions"))?;
    Ok(rows
        .into_iter()
        .map(SessionRecord::from)
        .filter(|record| states.contains(&record.primary_state))
        .collect())
}

/// 删掉 `before` 之前就不再更新的记录
pub async fn prune_sessions(pool: &ConnectionPool, before: i64) -> AppResult<u64> {
    let done = sqlx::query("DELETE FROM ha_sessions WHERE updated_at < ?")
        .bind(before)
        .execute(pool)
        .await
        .change_context(db_error("prune ha_sessions"))?;
    Ok(done.rows_affected())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    pub(crate) async fn pool() -> (tempfile::TempDir, ConnectionPool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        (dir, pool)
    }

    pub(crate) async fn node(pool: &ConnectionPool, name: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO fleet_nodes (name, endpoint_id, created_at) VALUES (?, ?, 0) RETURNING id",
        )
        .bind(name)
        .bind(format!("endpoint-{name}"))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn the_pair_is_a_single_row_that_keeps_its_creation_time() {
        let (_dir, pool) = pool().await;
        let primary = node(&pool, "local").await;
        let standby = node(&pool, "standby").await;
        assert!(pair(&pool).await.unwrap().is_none());
        let params = HaParams {
            offline_grace: 5,
            delete_standby_copy: true,
            ..HaParams::default()
        };
        let saved = save_pair(&pool, primary, standby, HaMode::Takeover, &params, 100)
            .await
            .unwrap();
        assert_eq!(saved.params, params);
        assert_eq!(saved.mode, HaMode::Takeover);
        let changed = save_pair(
            &pool,
            primary,
            standby,
            HaMode::DualRecord,
            &HaParams::default(),
            200,
        )
        .await
        .unwrap();
        assert_eq!(changed.created_at, 100);
        assert_eq!(changed.updated_at, 200);
        assert_eq!(pair(&pool).await.unwrap(), Some(changed));
        // 主副不能是同一台
        assert!(
            save_pair(&pool, primary, primary, HaMode::DualRecord, &params, 300)
                .await
                .is_err()
        );
        assert!(clear_pair(&pool).await.unwrap());
        assert!(!clear_pair(&pool).await.unwrap());
        assert!(pair(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn sessions_round_trip_and_are_found_by_state_and_bvid() {
        let (_dir, pool) = pool().await;
        let recording = SessionRecord {
            session_key: "3:1000".into(),
            room_id: 3,
            started_at: 1000,
            primary_state: PrimaryState::Recording,
            local_session_id: Some(9),
            unit_started_at: Some(1000),
            updated_at: 1000,
            ..SessionRecord::default()
        };
        save_session(&pool, &recording).await.unwrap();
        assert_eq!(
            session(&pool, "3:1000").await.unwrap().as_ref(),
            Some(&recording)
        );
        let uploaded = SessionRecord {
            session_key: "4:2000".into(),
            room_id: 4,
            started_at: 2000,
            ended_at: Some(3000),
            primary_state: PrimaryState::Uploaded,
            uploader: Some(Uploader::Primary),
            bvid: Some("BV1xx".into()),
            upload_bytes: 42,
            updated_at: 5000,
            ..SessionRecord::default()
        };
        save_session(&pool, &uploaded).await.unwrap();
        let interrupted = sessions_in_state(&pool, &[PrimaryState::Recording])
            .await
            .unwrap();
        assert_eq!(interrupted, std::slice::from_ref(&recording));
        assert_eq!(
            uploaded_since(&pool, 4000).await.unwrap(),
            std::slice::from_ref(&uploaded)
        );
        assert!(uploaded_since(&pool, 6000).await.unwrap().is_empty());
        let recent = recent_sessions(&pool, 10).await.unwrap();
        assert_eq!(recent[0].session_key, "4:2000");
        assert_eq!(prune_sessions(&pool, 2000).await.unwrap(), 1);
        assert_eq!(recent_sessions(&pool, 10).await.unwrap(), [uploaded]);
    }
}
