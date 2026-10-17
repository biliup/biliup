//! Service activity, derived from the business records at read time.
//!
//! 事件不落库：每次读取时从场次、分段、压制任务和上传任务推导。写入路径因此不用登记活动，
//! 代价是历史不再独立于业务记录 —— 被删掉的记录（保留策略、手动删除）连同它的事件一起消失，
//! 同一个对象的多次状态变化只留下当前状态。
use crate::server::infrastructure::connection_pool::ConnectionPool;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ActivityEvent {
    pub id: i64,
    pub kind: String,
    /// Unix milliseconds; clients display both date and time.
    pub ts: i64,
    pub text: String,
    pub sub: Option<String>,
    pub session_id: Option<i64>,
    pub entity_id: Option<i64>,
}

/// 每个来源一段 SELECT，再按时间取最近的若干条。
///
/// - `id` 用「来源段号 × 10^12 + 该行 id」拼出来：派生事件没有自己的主键，但客户端拿它当列表
///   key，必须稳定、且跨来源不重复。
/// - 每段各取 `limit` 条再合并：全局最近 `limit` 条一定落在各段最近的 `limit` 条里，结果不变，
///   但每张表只需要扫自己那一段（`stream_sessions`、`segments` 上没有排序列的索引）。
/// - 场次的时间以 `started_at` 为准，Python 版留下的只有 `date` 的老录像退回按日期算 —— 与升级
///   时的回填口径一致；分段事件沿用回填的保守口径，只在场次有 `started_at` 时才有时间基准。
const DERIVED_EVENTS: &str = r#"
WITH recent_limit(n) AS (SELECT ?)
SELECT id,kind,ts,text,sub,session_id,entity_id FROM (
    SELECT 1000000000000 + id AS id, 'start' AS kind,
        COALESCE(started_at,CAST(ROUND((julianday(date)-2440587.5)*86400000) AS INTEGER)) AS ts,
        name || ' 开始录制' AS text, NULL AS sub, id AS session_id, id AS entity_id
    FROM stream_sessions
    WHERE kind='source'
        AND COALESCE(started_at,CAST(ROUND((julianday(date)-2440587.5)*86400000) AS INTEGER)) > 0
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) UNION ALL SELECT * FROM (
    SELECT 2000000000000 + id AS id, 'end' AS kind, ended_at AS ts,
        name || ' 结束录制' AS text, NULL AS sub, id AS session_id, id AS entity_id
    FROM stream_sessions
    WHERE kind='source' AND ended_at > 0
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) UNION ALL SELECT * FROM (
    SELECT 3000000000000 + g.id AS id, 'file' AS kind, s.started_at + g.end_ms AS ts,
        '录制分段已完成' AS text, g.path AS sub, g.session_id AS session_id, g.id AS entity_id
    FROM segments g JOIN stream_sessions s ON s.id=g.session_id
    WHERE s.kind='source' AND g.state='finished' AND s.started_at > 0 AND g.end_ms IS NOT NULL
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) UNION ALL SELECT * FROM (
    SELECT 4000000000000 + j.id AS id, 'render' AS kind, j.updated_at AS ts,
        '压制任务 #' || j.id AS text,
        CASE j.state WHEN 'queued' THEN '等待压制' WHEN 'running' THEN '正在压制'
            WHEN 'ready' THEN '压制完成' WHEN 'failed' THEN '压制失败'
            WHEN 'cancelled' THEN '压制已取消' END AS sub,
        j.session_id AS session_id, j.id AS entity_id
    FROM render_jobs j
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) UNION ALL SELECT * FROM (
    SELECT 5000000000000 + t.rowid AS id, 'upload_queued' AS kind, t.created_at AS ts,
        json_extract(t.snapshot,'$.title') AS text, '上传任务已登记' AS sub,
        json_extract(t.snapshot,'$.session_id') AS session_id, NULL AS entity_id
    FROM upload_observation_tasks t
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) UNION ALL SELECT * FROM (
    SELECT 6000000000000 + t.rowid AS id, 'upload_' || t.state AS kind, t.updated_at AS ts,
        json_extract(t.snapshot,'$.title') AS text,
        CASE t.state WHEN 'queued' THEN '等待上传' WHEN 'running' THEN '正在处理上传'
            WHEN 'retrying' THEN '上传重试' WHEN 'submitting' THEN '正在提交稿件'
            WHEN 'succeeded' THEN '投稿成功' WHEN 'partial' THEN '部分文件投稿成功'
            WHEN 'failed' THEN '上传失败' WHEN 'cancelled' THEN '上传取消'
            WHEN 'interrupted' THEN '上传中断' WHEN 'unknown' THEN '投稿结果未知'
            WHEN 'paused' THEN '上传暂停' WHEN 'skipped' THEN '未执行投稿' END AS sub,
        json_extract(t.snapshot,'$.session_id') AS session_id, NULL AS entity_id
    FROM upload_observation_tasks t
    ORDER BY ts DESC LIMIT (SELECT n FROM recent_limit)
) ORDER BY ts DESC,id DESC LIMIT (SELECT n FROM recent_limit)
"#;

async fn recent(pool: &ConnectionPool, limit: i64) -> sqlx::Result<Vec<ActivityEvent>> {
    sqlx::query(DERIVED_EVENTS)
        .bind(limit)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| {
            Ok(ActivityEvent {
                id: row.try_get("id")?,
                kind: row.try_get("kind")?,
                ts: row.try_get("ts")?,
                text: row.try_get("text")?,
                sub: row.try_get("sub")?,
                session_id: row.try_get("session_id")?,
                entity_id: row.try_get("entity_id")?,
            })
        })
        .collect()
}

pub async fn list(State(pool): State<ConnectionPool>, Query(query): Query<ListQuery>) -> Response {
    let limit = query.limit.unwrap_or(12);
    if !(1..=100).contains(&limit) {
        return (StatusCode::BAD_REQUEST, "事件数量须在 1–100 之间").into_response();
    }
    match recent(&pool, limit).await {
        Ok(events) => Json(serde_json::json!({ "events": events })).into_response(),
        Err(error) => {
            tracing::warn!(%error, "推导活动事件失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "读取最近事件失败，请查看服务日志",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests;
