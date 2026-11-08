//! Paged revision metadata; snapshots remain in editor_revisions and restore through normal CAS saves.
pub struct Note<'a> {
    pub kind: &'a str,
    pub author: &'a str,
    pub label: &'a str,
    pub lines: &'a [String],
    pub turn: Option<&'a str>,
}

use super::{ConnectionPool, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
pub(crate) mod cursor;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserNote {
    pub label: String,
    #[serde(default)]
    pub kind: UserKind,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserKind {
    #[default]
    Edit,
    Jump,
}

pub async fn write(
    tx: &mut Transaction<'_, Sqlite>,
    project: i64,
    revision: i64,
    Note {
        kind,
        author,
        label,
        lines,
        turn,
    }: Note<'_>,
) -> Result<()> {
    sqlx::query("INSERT INTO editor_revision_notes(project_id,revision,kind,author,label,lines_json,turn_id) VALUES(?,?,?,?,?,?,?)")
        .bind(project).bind(revision).bind(kind).bind(author).bind(label).bind(serde_json::to_string(lines)?).bind(turn).execute(&mut **tx).await?;
    Ok(())
}

pub async fn page(pool: &ConnectionPool, project: i64, before: Option<i64>) -> Result<Value> {
    if before.is_some_and(|n| n < 1) {
        return Err(Error::Invalid("历史版本游标无效".into()));
    }
    // One read transaction keeps the head, total and page consistent with simultaneous saves.
    let mut tx = pool.begin().await?;
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
        .bind(project)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    let total:i64=sqlx::query_scalar("SELECT COUNT(*) FROM editor_revisions r JOIN editor_revision_notes n ON n.project_id=r.project_id AND n.revision=r.revision WHERE r.project_id=? AND n.kind NOT IN ('undo','redo')").bind(project).fetch_one(&mut *tx).await?;
    let mut rows=sqlx::query("WITH RECURSIVE lineage(revision) AS (SELECT ? UNION ALL SELECT l.parent_revision FROM editor_history_links l JOIN lineage p ON l.revision=p.revision WHERE l.project_id=? AND l.parent_revision IS NOT NULL AND l.parent_revision<l.revision) SELECT r.revision,r.created_at,n.kind,n.author,n.label,n.lines_json,n.turn_id,EXISTS(SELECT 1 FROM lineage a WHERE a.revision=r.revision) AS on_branch FROM editor_revisions r JOIN editor_revision_notes n ON n.project_id=r.project_id AND n.revision=r.revision WHERE r.project_id=? AND (? IS NULL OR r.revision<?) AND n.kind NOT IN ('undo','redo') ORDER BY r.revision DESC LIMIT 101")
        .bind(revision).bind(project).bind(project).bind(before).bind(before).fetch_all(&mut *tx).await?;
    let more = rows.len() > 100;
    rows.truncate(100);
    let entries=rows.iter().map(|r|{
        let rev:i64=r.try_get("revision")?;
        let mut entry=json!({"revision":rev,"at":super::agent::store::timestamp(r.try_get("created_at")?),"kind":r.try_get::<String,_>("kind")?,"author":r.try_get::<String,_>("author")?,"label":r.try_get::<String,_>("label")?,"lines":serde_json::from_str::<Value>(r.try_get::<&str,_>("lines_json")?)?,"current":r.try_get::<bool,_>("on_branch")?});
        if let Some(turn)=r.try_get::<Option<String>,_>("turn_id")?{entry["turnId"]=json!(turn);}
        Ok(entry)
    }).collect::<Result<Vec<_>>>()?;
    let next = if more {
        rows.last().map(|r| r.get::<i64, _>("revision"))
    } else {
        None
    };
    tx.commit().await?;
    Ok(
        json!({"dir":format!("biliup:{project}"),"revision":revision,"total":total,"entries":entries,"before":next}),
    )
}
