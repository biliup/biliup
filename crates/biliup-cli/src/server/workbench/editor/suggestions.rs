//! Suggestions become normal editor clips; acceptance and source retention share the save transaction.
use super::{ConnectionPool, Document, Error, Result, invalid};
use crate::server::{auto_clip::suggestions, workbench::retention};
use serde::{Deserialize, Serialize};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Acceptance {
    pub id: i64,
    pub items: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Use {
    suggestion_id: i64,
    project_id: Option<i64>,
    project_name: Option<String>,
    revision: i64,
    items: Vec<String>,
}

pub async fn list(pool: &ConnectionPool, session: i64) -> Result<Vec<Use>> {
    let rows = sqlx::query("SELECT u.*,p.name FROM editor_suggestion_uses u JOIN clip_suggestions s ON s.id=u.suggestion_id LEFT JOIN editor_projects p ON p.id=u.project_id WHERE s.session_id=? OR s.session_id IN (SELECT source_session_id FROM session_group_members WHERE group_session_id=?)")
        .bind(session).bind(session).fetch_all(pool).await?;
    rows.iter().map(|row| Ok(Use {
        suggestion_id: row.try_get("suggestion_id")?,
        project_id: row.try_get("project_id")?,
        project_name: row.try_get("name")?,
        revision: row.try_get("revision")?,
        items: serde_json::from_str(row.try_get("item_ids_json")?)?,
    })).collect()
}

pub async fn accept(
    tx: &mut Transaction<'_, Sqlite>, project: i64, revision: i64,
    document: &Document, request: &Acceptance, now: i64,
) -> Result<()> {
    if request.id < 1 || request.items.is_empty() || request.items.len() > 1000 {
        return Err(invalid("剪辑建议或片段数量无效"));
    }
    let ids: HashSet<&str> = request.items.iter().map(String::as_str).collect();
    if ids.len() != request.items.len() { return Err(invalid("建议片段 ID 重复")); }
    let row = sqlx::query("SELECT session_id,in_ms,out_ms,state,created_at FROM clip_suggestions WHERE id=?")
        .bind(request.id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    if row.try_get::<&str,_>("state")? != "pending" {
        return Err(invalid("剪辑建议已处理，请刷新列表"));
    }
    if row.try_get::<i64,_>("created_at")?.saturating_add(suggestions::EXPIRE_AFTER_MS) <= now {
        return Err(invalid("剪辑建议已过期，请重新生成"));
    }
    let owner: i64 = row.try_get("session_id")?;
    let from = row.try_get::<i64,_>("in_ms")? as f64;
    let to = row.try_get::<i64,_>("out_ms")? as f64;
    let fps = document.project["canvas"]["fps"].as_f64().ok_or_else(|| invalid("工程帧率无效"))?;
    let tolerance = 2.0 * 1000.0 / fps;
    let previous: String = sqlx::query_scalar("SELECT document_json FROM editor_revisions WHERE project_id=? AND revision=?")
        .bind(project).bind(revision - 1).fetch_one(&mut **tx).await?;
    let previous: Document = serde_json::from_str(&previous)?;
    let old_ids: HashSet<&str> = previous.project["tracks"].as_array().into_iter().flatten()
        .flat_map(|track| track["items"].as_array().into_iter().flatten())
        .filter_map(|item| item["id"].as_str()).collect();
    let mut spans = Vec::new();
    for track in document.project["tracks"].as_array().into_iter().flatten() {
        for item in track["items"].as_array().into_iter().flatten() {
            let Some(id) = item["id"].as_str().filter(|id| ids.contains(id)) else { continue };
            if old_ids.contains(id) || track["role"] != "main" || item["type"] != "clip"
                || item["loop"].as_bool() == Some(true) || !item["hold"].is_null()
                || item["speed"].as_f64().unwrap_or(1.0) != 1.0 {
                return Err(invalid("建议必须作为新的普通视频片段加入主轨道"));
            }
            let source = item["media"].as_str().and_then(|id| document.sources.get(id))
                .and_then(super::MediaSource::recording).ok_or_else(|| invalid("建议片段缺少录像来源"))?;
            if source.session_id != owner || !matches!(source.state.as_str(), "recording" | "finished" | "pending_delete") {
                return Err(invalid("剪辑建议的录像不可用或来源不匹配"));
            }
            let start = source.source_start_ms as f64 + item["in"].as_f64().unwrap_or(0.0) * 1000.0 / fps;
            let end = start + item["duration"].as_f64().ok_or_else(|| invalid("片段时长无效"))? * 1000.0 / fps;
            spans.push((start, end));
        }
    }
    if spans.len() != ids.len() { return Err(invalid("建议片段未完整加入工程")); }
    spans.sort_by(|a,b| a.0.total_cmp(&b.0));
    let mut end = from;
    for (start, next) in spans {
        if (start - end).abs() > tolerance || next <= start || next > to + tolerance {
            return Err(invalid("建议片段范围不匹配或录像区间缺失"));
        }
        end = next;
    }
    if (end - to).abs() > tolerance { return Err(invalid("建议片段未覆盖完整选段")); }
    sqlx::query("UPDATE clip_suggestions SET state='accepted',updated_at=? WHERE id=? AND state='pending'")
        .bind(now).bind(request.id).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO editor_suggestion_uses(suggestion_id,project_id,revision,item_ids_json,created_at) VALUES(?,?,?,?,?)")
        .bind(request.id).bind(project).bind(revision).bind(serde_json::to_string(&request.items)?).bind(now).execute(&mut **tx).await?;
    retention::unpin(tx, &suggestions::pin_owner(request.id)).await?;
    Ok(())
}
