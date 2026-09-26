//! `auto_clip_jobs` 的读写：按场次排队的自动切片任务。
//!
//! 状态只经这里改，所有「结束」都带 `state = 'running'` 条件：取消与任务自己收尾同时发生时，
//! 先落库的算数，另一方什么也不改。

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::retention;
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// 与迁移里 `auto_clip_jobs.trigger` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// 下播后按主播开关自动排的
    Auto,
    /// 界面上手动点的
    Manual,
}

impl Trigger {
    fn as_str(self) -> &'static str {
        match self {
            Trigger::Auto => "auto",
            Trigger::Manual => "manual",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "auto" => Trigger::Auto,
            _ => Trigger::Manual,
        }
    }
}

/// 与迁移里 `auto_clip_jobs.state` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    Canceled,
}

impl JobState {
    fn parse(value: &str) -> Self {
        match value {
            "queued" => JobState::Queued,
            "running" => JobState::Running,
            "done" => JobState::Done,
            "canceled" => JobState::Canceled,
            _ => JobState::Failed,
        }
    }
}

/// 与迁移里 `auto_clip_jobs.stage` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// 抽音频与静音检测
    Audio,
    /// 转写
    Asr,
    /// 弹幕密度与缩图（候选生成）
    Signals,
    /// 送 chat 模型分析（候选生成）
    Analyze,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Audio => "audio",
            Stage::Asr => "asr",
            Stage::Signals => "signals",
            Stage::Analyze => "analyze",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "audio" => Some(Stage::Audio),
            "asr" => Some(Stage::Asr),
            "signals" => Some(Stage::Signals),
            "analyze" => Some(Stage::Analyze),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Job {
    pub id: i64,
    pub session_id: i64,
    pub trigger: Trigger,
    pub state: JobState,
    pub stage: Option<Stage>,
    pub progress_done: i64,
    pub progress_total: i64,
    /// Unix 毫秒：到这个时刻才跑
    pub not_before: i64,
    pub error: Option<String>,
    /// 静音跳过后这次要送转写的秒数；还没抽完音频时为空
    pub asr_planned_seconds: Option<i64>,
    /// 实际已送转写的秒数
    pub asr_seconds: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub images: i64,
    /// 用的模型与服务的主机名，不含 key
    pub models: Option<Value>,
    /// 跳过的分段等提示
    pub warnings: Vec<String>,
    pub reuse_transcript: bool,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

impl Job {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let json = |column: &str| -> sqlx::Result<Option<Value>> {
            Ok(row
                .try_get::<Option<&str>, _>(column)?
                .and_then(|text| serde_json::from_str(text).ok()))
        };
        Ok(Job {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            trigger: Trigger::parse(row.try_get("trigger")?),
            state: JobState::parse(row.try_get("state")?),
            stage: row
                .try_get::<Option<&str>, _>("stage")?
                .and_then(Stage::parse),
            progress_done: row.try_get("progress_done")?,
            progress_total: row.try_get("progress_total")?,
            not_before: row.try_get("not_before")?,
            error: row.try_get("error")?,
            asr_planned_seconds: row.try_get("asr_planned_seconds")?,
            asr_seconds: row.try_get("asr_seconds")?,
            tokens_in: row.try_get("tokens_in")?,
            tokens_out: row.try_get("tokens_out")?,
            images: row.try_get("images")?,
            models: json("models")?,
            warnings: json("warnings")?
                .and_then(|value| serde_json::from_value(value).ok())
                .unwrap_or_default(),
            reuse_transcript: row.try_get("reuse_transcript")?,
            created_by: row.try_get("created_by")?,
            created_at: row.try_get("created_at")?,
            started_at: row.try_get("started_at")?,
            finished_at: row.try_get("finished_at")?,
        })
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, JobState::Queued | JobState::Running)
    }
}

const COLUMNS: &str = "id, session_id, trigger, state, stage, progress_done, progress_total, \
     not_before, error, asr_planned_seconds, asr_seconds, tokens_in, tokens_out, images, models, \
     warnings, reuse_transcript, created_by, created_at, started_at, finished_at";

#[derive(bon::Builder, Debug, Clone)]
pub struct NewJob {
    pub session_id: i64,
    pub trigger: Trigger,
    pub not_before: i64,
    #[builder(default = true)]
    pub reuse_transcript: bool,
    pub created_by: Option<i64>,
    pub created_at: i64,
}

/// 任务运行期间引用整场素材时用的名义。
pub fn pin_owner(id: i64) -> String {
    format!("autoclip-job:{id}")
}

async fn one<'q>(
    pool: &ConnectionPool,
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
) -> sqlx::Result<Option<Job>> {
    query
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(Job::from_row)
        .transpose()
}

/// 插一条排队任务；这一场已经有排队或运行中的任务时不插，返回 `None`。
pub async fn insert(pool: &ConnectionPool, job: &NewJob) -> sqlx::Result<Option<Job>> {
    let sql = format!(
        "INSERT INTO auto_clip_jobs (session_id, trigger, not_before, reuse_transcript, created_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING RETURNING {COLUMNS}"
    );
    one(
        pool,
        sqlx::query(&sql)
            .bind(job.session_id)
            .bind(job.trigger.as_str())
            .bind(job.not_before)
            .bind(job.reuse_transcript)
            .bind(job.created_by)
            .bind(job.created_at),
    )
    .await
}

/// 下播后排自动任务：这一场已经有排队中的自动任务（断流合并接上后又下播）就把它推迟到
/// `not_before`；有手动任务或正在跑就不动。返回排上或推迟了的任务。
pub async fn schedule_auto(
    pool: &ConnectionPool,
    session_id: i64,
    not_before: i64,
    now: i64,
) -> sqlx::Result<Option<Job>> {
    let job = NewJob::builder()
        .session_id(session_id)
        .trigger(Trigger::Auto)
        .not_before(not_before)
        .created_at(now)
        .build();
    if let Some(job) = insert(pool, &job).await? {
        return Ok(Some(job));
    }
    let sql = format!(
        "UPDATE auto_clip_jobs SET not_before = MAX(not_before, ?)
         WHERE session_id = ? AND state = 'queued' AND trigger = 'auto' RETURNING {COLUMNS}"
    );
    one(pool, sqlx::query(&sql).bind(not_before).bind(session_id)).await
}

pub async fn get(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<Job>> {
    let sql = format!("SELECT {COLUMNS} FROM auto_clip_jobs WHERE id = ?");
    one(pool, sqlx::query(&sql).bind(id)).await
}

/// 这一场最近的一个任务（不论状态）。
pub async fn latest(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Option<Job>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM auto_clip_jobs WHERE session_id = ? ORDER BY id DESC LIMIT 1"
    );
    one(pool, sqlx::query(&sql).bind(session_id)).await
}

/// 最早该跑的排队任务（可能还没到点）。
pub async fn next_queued(pool: &ConnectionPool) -> sqlx::Result<Option<Job>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM auto_clip_jobs WHERE state = 'queued'
         ORDER BY not_before, id LIMIT 1"
    );
    one(pool, sqlx::query(&sql)).await
}

/// 把排队任务转成运行中；已被取消或别处先取走时返回 `None`。
pub async fn claim(pool: &ConnectionPool, id: i64, now: i64) -> sqlx::Result<Option<Job>> {
    let sql = format!(
        "UPDATE auto_clip_jobs SET state = 'running', started_at = COALESCE(started_at, ?)
         WHERE id = ? AND state = 'queued' RETURNING {COLUMNS}"
    );
    one(pool, sqlx::query(&sql).bind(now).bind(id)).await
}

pub async fn state(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<JobState>> {
    let state: Option<String> = sqlx::query_scalar("SELECT state FROM auto_clip_jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(state.as_deref().map(JobState::parse))
}

pub async fn set_stage(
    pool: &ConnectionPool,
    id: i64,
    stage: Stage,
    done: i64,
    total: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE auto_clip_jobs SET stage = ?, progress_done = ?, progress_total = ?
         WHERE id = ? AND state = 'running'",
    )
    .bind(stage.as_str())
    .bind(done)
    .bind(total)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_progress(pool: &ConnectionPool, id: i64, done: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE auto_clip_jobs SET progress_done = ? WHERE id = ? AND state = 'running'")
        .bind(done)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 转写完一块：进度 + 已送秒数。
pub async fn record_chunk(
    pool: &ConnectionPool,
    id: i64,
    done: i64,
    asr_seconds: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE auto_clip_jobs SET progress_done = ?, asr_seconds = asr_seconds + ?
         WHERE id = ? AND state = 'running'",
    )
    .bind(done)
    .bind(asr_seconds)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_planned(pool: &ConnectionPool, id: i64, seconds: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE auto_clip_jobs SET asr_planned_seconds = ? WHERE id = ?")
        .bind(seconds)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_details(
    pool: &ConnectionPool,
    id: i64,
    models: &Value,
    warnings: &[String],
) -> sqlx::Result<()> {
    sqlx::query("UPDATE auto_clip_jobs SET models = ?, warnings = ? WHERE id = ?")
        .bind(models.to_string())
        .bind((!warnings.is_empty()).then(|| serde_json::json!(warnings).to_string()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 运行中的任务结束：`error` 为空记为完成，否则记为失败。已被取消的不改，返回 `false`。
pub async fn finish(
    pool: &ConnectionPool,
    id: i64,
    error: Option<&str>,
    now: i64,
) -> sqlx::Result<bool> {
    let state = if error.is_some() { "failed" } else { "done" };
    let updated = sqlx::query(
        "UPDATE auto_clip_jobs SET state = ?, error = ?, finished_at = ?
         WHERE id = ? AND state = 'running'",
    )
    .bind(state)
    .bind(error)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(updated > 0)
}

/// 取消这一场排队或运行中的任务。运行中的由调度器在一秒左右内发现并停下。
pub async fn cancel(pool: &ConnectionPool, session_id: i64, now: i64) -> sqlx::Result<Option<Job>> {
    let sql = format!(
        "UPDATE auto_clip_jobs SET state = 'canceled', finished_at = ?
         WHERE session_id = ? AND state IN ('queued', 'running') RETURNING {COLUMNS}"
    );
    one(pool, sqlx::query(&sql).bind(now).bind(session_id)).await
}

pub async fn delete(pool: &ConnectionPool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM auto_clip_jobs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 启动时：上次没跑完的任务回到排队，按 `stage` 与缓存续跑。返回几条。
pub async fn recover(pool: &ConnectionPool) -> sqlx::Result<u64> {
    Ok(
        sqlx::query("UPDATE auto_clip_jobs SET state = 'queued' WHERE state = 'running'")
            .execute(pool)
            .await?
            .rows_affected(),
    )
}

/// 任务运行期间引用整场，防止整场投稿后的删除在分析中途删掉素材。
pub async fn pin_session(pool: &ConnectionPool, job: &Job) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;
    retention::pin(&mut conn, &pin_owner(job.id), job.session_id, 0, i64::MAX).await
}

pub async fn unpin_session(pool: &ConnectionPool, id: i64) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;
    retention::unpin(&mut conn, &pin_owner(id)).await?;
    Ok(())
}
