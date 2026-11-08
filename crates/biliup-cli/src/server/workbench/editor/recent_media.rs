//! Uncut media/mediaIndex.ts (MIT), integrated with registered assets and recording retention.
//! No arbitrary disk paths leave the server; the index identifies sources independently of timeline IDs.
use super::{Document, Error, MediaSource, media};
use crate::server::{
    infrastructure::connection_pool::ConnectionPool, workbench::retention::MEDIA_LIFECYCLE,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use std::{collections::HashSet, path::PathBuf};

#[derive(Default, Deserialize)]
pub struct Query {
    query: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
    before: Option<String>,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn source_id(source: &MediaSource) -> String {
    match source {
        MediaSource::File(file) => format!("file-{}", file.asset_id),
        MediaSource::Recording(recording) => format!(
            "recording-{}-{}",
            recording.session_id, recording.segment_id
        ),
    }
}
fn origin(media: &Value, source: &MediaSource) -> Option<&'static str> {
    let module = media["source"]["module"].as_str().unwrap_or("");
    let root = module
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    if matches!(root, "library" | "music" | "sfx") {
        return None;
    }
    Some(
        if matches!(source, MediaSource::Recording(_)) || root == "capture" {
            "capture"
        } else if root == "generate" {
            "generated"
        } else {
            "import"
        },
    )
}
async fn index_one(
    conn: &mut SqliteConnection,
    project: i64,
    name: &str,
    media: &Value,
    source: &MediaSource,
    at: i64,
) -> Result<(), Error> {
    if !media["id"].is_string() {
        return Ok(());
    }
    if !matches!(media["kind"].as_str(), Some("video" | "image" | "audio")) {
        return Ok(());
    }
    let Some(origin) = origin(media, source) else {
        return Ok(());
    };
    // Replacing a file preserves its timeline ID. Keying the global index by that
    // ID would overwrite the original source and collide with other projects' takes.
    let id = source_id(source);
    let mut source = source.clone();
    // A reuse points at the original session, independently of a later deleted/merged group.
    if let MediaSource::Recording(recording) = &mut source {
        recording.view_session_id = recording.session_id;
        recording.view_start_ms = recording.source_start_ms;
    }
    let registered: Option<i64> = match &source {
        MediaSource::File(file) => {
            sqlx::query_scalar("SELECT created_at FROM editor_assets WHERE id=?")
                .bind(&file.asset_id)
                .fetch_optional(&mut *conn)
                .await?
        }
        MediaSource::Recording(recording) => sqlx::query_scalar::<_, Option<i64>>(
            "SELECT started_at FROM stream_sessions WHERE id=?",
        )
        .bind(recording.session_id)
        .fetch_optional(&mut *conn)
        .await?
        .flatten()
        .and_then(|start| start.checked_add(recording.source_start_ms)),
    };
    let created = media["source"]["created"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis())
        .or(registered)
        .unwrap_or(at)
        .clamp(0, super::now_ms());
    sqlx::query("INSERT INTO editor_recent_media(id,project_id,project_name,media_json,source_json,origin,created_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET project_name=excluded.project_name,media_json=excluded.media_json,source_json=excluded.source_json,origin=excluded.origin WHERE editor_recent_media.project_id=excluded.project_id AND (editor_recent_media.project_name!=excluded.project_name OR editor_recent_media.media_json!=excluded.media_json OR editor_recent_media.source_json!=excluded.source_json)")
        .bind(id).bind(project).bind(name).bind(media.to_string()).bind(serde_json::to_string(&source)?).bind(origin).bind(created).execute(&mut *conn).await?;
    Ok(())
}
async fn trim(conn: &mut SqliteConnection) -> Result<(), Error> {
    sqlx::query("DELETE FROM editor_recent_media WHERE id NOT IN (SELECT id FROM editor_recent_media ORDER BY created_at DESC,id LIMIT 500)").execute(conn).await?;
    Ok(())
}
/// Called inside the project revision transaction: undo/save failure cannot publish phantom media.
pub async fn index_document(
    conn: &mut SqliteConnection,
    project: i64,
    document: &Document,
    at: i64,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE editor_recent_media SET project_name=? WHERE project_id=? AND project_name!=?",
    )
    .bind(document.project["name"].as_str().unwrap_or(""))
    .bind(project)
    .bind(document.project["name"].as_str().unwrap_or(""))
    .execute(&mut *conn)
    .await?;
    for media in document.project["media"].as_array().into_iter().flatten() {
        if let Some(source) = media["id"].as_str().and_then(|id| document.sources.get(id)) {
            index_one(
                conn,
                project,
                document.project["name"].as_str().unwrap_or(""),
                media,
                source,
                at,
            )
            .await?;
        }
    }
    trim(conn).await
}
struct Entry {
    id: String,
    project: i64,
    project_name: String,
    media: Value,
    source: MediaSource,
    origin: String,
    created: i64,
}
impl Entry {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            project: row.try_get("project_id")?,
            project_name: row.try_get("project_name")?,
            media: serde_json::from_str(row.try_get("media_json")?)?,
            source: serde_json::from_str(row.try_get("source_json")?)?,
            origin: row.try_get("origin")?,
            created: row.try_get("created_at")?,
        })
    }
    fn shown(&self) -> Value {
        let mut value = json!({"id":self.id,"path":self.media["path"],"name":self.media["name"],"kind":self.media["kind"],"origin":self.origin,"project":{"dir":format!("biliup:{}",self.project),"name":self.project_name},"created":chrono::DateTime::from_timestamp_millis(self.created).unwrap_or_default().to_rfc3339(),"thumb":self.media["kind"]!="audio"});
        if let Some(ms) = self.source.duration_ms() {
            value["seconds"] = json!(ms as f64 / 1000.);
        }
        for field in ["width", "height"] {
            if let Some(n) = self.media["probe"][field].as_u64() {
                value[field] = json!(n);
            }
        }
        if let Some(device) = self.media["source"]["device"].as_str() {
            value["device"] = json!(device);
        }
        value
    }
    async fn path(&self, pool: &ConnectionPool) -> Result<PathBuf, Error> {
        let path = media::source_path(pool, &self.source).await?;
        if !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
            return Err(Error::NotFound);
        }
        if let MediaSource::Recording(source) = &self.source {
            let segment = crate::server::workbench::store::segment(pool, source.segment_id)
                .await?
                .ok_or(Error::NotFound)?;
            if matches!(
                segment.state,
                crate::server::workbench::store::SegmentState::Missing
                    | crate::server::workbench::store::SegmentState::Deleted
                    | crate::server::workbench::store::SegmentState::PendingDelete
            ) {
                return Err(Error::NotFound);
            }
        }
        Ok(path)
    }
}
pub async fn search(pool: &ConnectionPool, project: i64, query: Query) -> Result<Value, Error> {
    if query.query.as_ref().is_some_and(|q| q.len() > 512) {
        return Err(invalid("搜索内容过长"));
    }
    let kind = query.kind.as_deref().unwrap_or("");
    if !matches!(
        kind,
        "" | "video" | "image" | "audio" | "capture" | "import" | "generated"
    ) {
        return Err(invalid("素材类型无效"));
    }
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let before = query
        .before
        .as_deref()
        .map(|cursor| {
            let (created, id) = cursor
                .split_once(':')
                .ok_or_else(|| invalid("素材分页位置无效"))?;
            let created = created
                .parse::<i64>()
                .map_err(|_| invalid("素材分页位置无效"))?;
            if created < 0 || id.is_empty() || cursor.len() > 256 {
                return Err(invalid("素材分页位置无效"));
            }
            Ok((created, id))
        })
        .transpose()?;
    let _lifecycle = MEDIA_LIFECYCLE.read().await;
    let saved = super::get(pool, project).await?;
    let mut seen: HashSet<String> = saved.document.sources.values().map(source_id).collect();
    let rows=sqlx::query("SELECT * FROM editor_recent_media WHERE project_id!=? ORDER BY created_at DESC,id LIMIT 500").bind(project).fetch_all(pool).await?;
    let query = query.query.unwrap_or_default().to_lowercase();
    let words: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut entries = vec![];
    let mut last = None;
    let mut next = None;
    for row in rows {
        let entry = Entry::from_row(&row)?;
        // A cursor is a position in the immutable creation order, not a row offset.
        // Importing/removing a result or receiving a new one cannot skip the following page.
        if before.is_some_and(|(created, id)| {
            entry.created > created || entry.created == created && entry.id.as_str() <= id
        }) || seen.contains(&entry.id)
            || !kind.is_empty() && kind != entry.origin && entry.media["kind"] != kind
        {
            continue;
        }
        let aliases = match entry.origin.as_str() {
            "capture" => "录制 录像 采集 recording take screen camera microphone",
            "generated" => "生成 generation generated",
            _ => "导入 import",
        };
        let type_alias = match entry.media["kind"].as_str() {
            Some("video") => "视频 video",
            Some("audio") => "音频 audio",
            _ => "图片 image",
        };
        let hay = format!(
            "{} {} {} {} {aliases} {type_alias}",
            entry.media["name"].as_str().unwrap_or(""),
            entry.media["path"].as_str().unwrap_or(""),
            entry.project_name,
            entry.media["source"]["device"].as_str().unwrap_or("")
        )
        .to_lowercase();
        if !words.iter().all(|word| hay.contains(word)) || entry.path(pool).await.is_err() {
            continue;
        }
        if entries.len() >= limit {
            next = last;
            break;
        }
        last = Some(format!("{}:{}", entry.created, entry.id));
        seen.insert(entry.id.clone());
        entries.push(entry.shown());
    }
    Ok(json!({"entries":entries,"next":next}))
}
async fn entry(pool: &ConnectionPool, id: &str) -> Result<Entry, Error> {
    let row = sqlx::query("SELECT * FROM editor_recent_media WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;
    Entry::from_row(&row)
}
/// Only prepares a registered reference. The normal project save commits its retention pin.
pub async fn import(pool: &ConnectionPool, project: i64, id: &str) -> Result<Value, Error> {
    prepare_import(pool, project, id, false).await
}
pub(crate) async fn import_pinned(
    pool: &ConnectionPool,
    project: i64,
    id: &str,
) -> Result<Value, Error> {
    prepare_import(pool, project, id, true).await
}
async fn prepare_import(
    pool: &ConnectionPool,
    project: i64,
    id: &str,
    pin: bool,
) -> Result<Value, Error> {
    let _lifecycle = MEDIA_LIFECYCLE.read().await;
    let saved = super::get(pool, project).await?;
    let mut entry = entry(pool, id).await?;
    entry.path(pool).await?;
    if pin && let MediaSource::Recording(source) = &entry.source {
        sqlx::query("INSERT OR IGNORE INTO editor_sources(project_id,segment_id) VALUES(?,?)")
            .bind(project)
            .bind(source.segment_id)
            .execute(pool)
            .await?;
    }
    let fps = saved.document.project["canvas"]["fps"]
        .as_f64()
        .unwrap_or(30.);
    if let Some(ms) = entry.source.duration_ms() {
        entry.media["probe"]["frames"] = json!((ms as f64 * fps / 1000.).floor().max(1.) as i64);
    }
    // Reuse the registered asset and its authored metadata. Replacing source with a
    // reuse tag would discard generation prompts/settings and the native Again action.
    Ok(json!({"media":entry.media,"source":entry.source}))
}
pub async fn thumbnail_source(
    pool: &ConnectionPool,
    project: i64,
    id: &str,
) -> Result<(PathBuf, MediaSource), Error> {
    super::get(pool, project).await?;
    let entry = entry(pool, id).await?;
    if entry.media["kind"] == "audio" {
        return Err(Error::NotFound);
    }
    let path = entry.path(pool).await?;
    Ok((path, entry.source))
}
