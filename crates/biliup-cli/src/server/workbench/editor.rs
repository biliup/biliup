//! Portable editor documents. Only registered recordings and imported assets are admitted as media.
//! Saves, revision checks and media references commit together. Saved history holds
//! sources until the project is deleted, including during disk-pressure cleanup.

use super::{recorder::now_ms, retention::MEDIA_LIFECYCLE};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqliteConnection};
use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/editor/project.schema.json"
    )))
    .expect("bundled project schema");
    jsonschema::validator_for(&schema).expect("valid bundled project schema")
});

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("工程、场次或素材不存在")]
    NotFound,
    #[error("工程已有新版本，请刷新工程状态后重试")]
    Conflict,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, Error>;
pub(crate) mod agent;
pub(crate) mod analysis;
pub(crate) mod downloads;
pub(crate) mod export;
pub(crate) mod fonts;
pub(crate) mod generate;
pub(crate) mod graphic_media;
pub(crate) mod graphics;
pub(crate) mod history;
pub(crate) mod imports;
pub(crate) mod keys;
pub(crate) mod library;
pub(crate) mod media;
mod metadata;
pub(crate) mod online_library;
pub(crate) mod project_fonts;
pub(crate) mod proxies;
pub(crate) mod public_http;
pub(crate) mod recent_media;
pub(crate) mod resources;
pub(crate) mod shaders;
pub(crate) mod sound_library;
pub(crate) mod suggestions;
pub(crate) mod svg;
pub(crate) mod text_model;
pub(crate) mod waveform;
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    pub session_id: Option<i64>,
    pub in_ms: i64,
    pub out_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordingSource {
    pub kind: String,
    pub session_id: i64,
    pub segment_id: i64,
    pub view_session_id: i64,
    pub view_start_ms: i64,
    pub source_start_ms: i64,
    pub duration_ms: i64,
    pub file_name: String,
    pub state: String,
    pub has_danmaku: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileSource {
    pub kind: String,
    pub asset_id: String,
    pub duration_ms: Option<i64>,
    pub file_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MediaSource {
    Recording(RecordingSource),
    File(FileSource),
}
impl MediaSource {
    pub fn recording(&self) -> Option<&RecordingSource> {
        match self {
            Self::Recording(source) => Some(source),
            Self::File(_) => None,
        }
    }
    pub fn duration_ms(&self) -> Option<i64> {
        match self {
            Self::Recording(source) => Some(source.duration_ms),
            Self::File(source) => source.duration_ms,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMarker {
    pub marker_id: i64,
    pub session_id: i64,
    pub at_ms: i64,
    pub view_at_ms: i64,
    pub project_frame: i64,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub project: Value,
    pub sources: BTreeMap<String, MediaSource>,
    pub origin: Origin,
    pub source_markers: Vec<SourceMarker>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Summary {
    pub id: i64,
    pub session_id: Option<i64>,
    pub name: String,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Serialize)]
pub struct Saved {
    #[serde(flatten)]
    pub summary: Summary,
    pub document: Document,
    pub history: history::cursor::Info,
}

// A persisted project is data, never a source of JS prototype properties.
fn safe_json(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(k, v)| {
            !matches!(k.as_str(), "__proto__" | "constructor" | "prototype") && safe_json(v)
        }),
        Value::Array(items) => items.iter().all(safe_json),
        Value::Number(n) => n
            .as_f64()
            .is_some_and(|v| v.abs() <= 9_007_199_254_740_991.),
        _ => true,
    }
}

fn validate(doc: &Document) -> Result<()> {
    if doc.format != "biliup.editor/1"
        || match doc.origin.session_id {
            Some(id) => id <= 0 || doc.origin.in_ms < 0 || doc.origin.out_ms <= doc.origin.in_ms,
            // A project created from the root navigation has no entry-point session.
            // It can still reuse recordings; resolve_sources validates and retains each one.
            None => doc.origin.in_ms != 0 || doc.origin.out_ms != 0,
        }
    {
        return Err(invalid("工程格式或导入范围无效"));
    }
    if !safe_json(&doc.project) {
        return Err(invalid("工程含不安全的属性或超出精度的数值"));
    }
    if let Err(e) = SCHEMA.validate(&doc.project) {
        // Do not echo the invalid instance (it can contain local paths or huge strings).
        return Err(Error::Invalid(format!(
            "工程结构无效：{}",
            e.instance_path()
        )));
    }
    if doc.project["name"]
        .as_str()
        .is_none_or(|n| n.trim().is_empty() || n.chars().count() > 256)
    {
        return Err(invalid("工程名称需为 1–256 个字符"));
    }
    let media = doc.project["media"].as_array().unwrap();
    if media.len() != doc.sources.len() {
        return Err(invalid("素材与来源记录不一致"));
    }
    let mut ids = HashSet::new();
    for item in media {
        if !ids.insert(item["id"].as_str().unwrap()) {
            return Err(invalid("素材 ID 重复"));
        }
    }
    let mut main_count = 0;
    for track in doc.project["tracks"].as_array().unwrap() {
        if !ids.insert(track["id"].as_str().unwrap()) {
            return Err(invalid("轨道 ID 重复"));
        }
        if track["role"] == "main" {
            main_count += 1;
        }
        for item in track["items"].as_array().unwrap() {
            if !ids.insert(item["id"].as_str().unwrap()) {
                return Err(invalid("片段 ID 重复"));
            }
            if let Some(id) = item["media"].as_str()
                && !doc.sources.contains_key(id)
            {
                return Err(invalid("片段引用了未登记素材"));
            }
            for effect in item["effects"].as_array().into_iter().flatten() {
                if effect["module"] != "effect.biliup-mask" {
                    continue;
                }
                for region in effect["params"]["regions"].as_array().into_iter().flatten() {
                    if region["effect_type"] != "image" && region["image"].is_null() {
                        continue;
                    }
                    let id = region["image"]["media"]
                        .as_str()
                        .ok_or_else(|| invalid("遮挡图片引用无效"))?;
                    if region["effect_type"] != "image"
                        || !media
                            .iter()
                            .any(|entry| entry["id"] == id && entry["kind"] == "image")
                        || !doc.sources.contains_key(id)
                    {
                        return Err(invalid("遮挡图片需引用工程中的图片素材"));
                    }
                }
            }
        }
    }
    if main_count > 1 {
        return Err(invalid("工程只能有一条主轨道"));
    }
    let fps = doc.project["canvas"]["fps"].as_f64().unwrap();
    for track in doc.project["tracks"].as_array().unwrap() {
        for item in track["items"].as_array().unwrap() {
            if let Some(source) = item["media"].as_str().and_then(|id| doc.sources.get(id)) {
                let Some(duration_ms) = source.duration_ms() else {
                    continue;
                };
                let end = item["in"].as_f64().unwrap_or(0.)
                    + item["duration"].as_f64().unwrap() * item["speed"].as_f64().unwrap_or(1.);
                let limit = (duration_ms as f64 * fps / 1000.).ceil() + 1.;
                if item["loop"] != true && item["hold"] != true && end > limit {
                    return Err(invalid("片段超出了素材的时长"));
                }
            }
        }
    }
    Ok(())
}

/// Source metadata is authoritative in SQLite. Reject forged IDs/times/paths and
/// refresh display metadata (filenames, availability) without exposing disk paths.
async fn resolve_sources(conn: &mut SqliteConnection, doc: &mut Document) -> Result<Vec<i64>> {
    if let Some(origin) = doc.origin.session_id {
        let origin_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stream_sessions WHERE id=?)")
                .bind(origin)
                .fetch_one(&mut *conn)
                .await?;
        if !origin_exists {
            return Err(Error::NotFound);
        }
    }
    let mut segments = Vec::new();
    for media in doc.project["media"].as_array().unwrap() {
        let id = media["id"].as_str().unwrap();
        let source = doc
            .sources
            .get_mut(id)
            .ok_or_else(|| invalid("缺少素材来源"))?;
        let source = match source {
            MediaSource::File(source) => {
                imports::validate_source(conn, id, media, source).await?;
                continue;
            }
            MediaSource::Recording(source) => source,
        };
        let row = sqlx::query("SELECT * FROM segments WHERE id=?")
            .bind(source.segment_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(Error::NotFound)?;
        let owner: i64 = row.try_get("session_id")?;
        let start: i64 = row.try_get("start_ms")?;
        let mut end: Option<i64> = row.try_get("end_ms")?;
        if row.try_get::<&str, _>("state")? == "recording" {
            let segment = super::store::SegmentRow::from_row(&row)?;
            if let (Ok(index), Ok(meta)) = (
                super::segment_index(&segment).await,
                tokio::fs::metadata(&segment.path).await,
            ) {
                end = start.checked_add(super::readable_ms(&index, meta.len()));
            }
        }
        let container: String = row.try_get("container")?;
        let offset = if owner == source.view_session_id {
            0
        } else {
            sqlx::query_scalar::<_, i64>("SELECT offset_ms FROM session_group_members WHERE group_session_id=? AND source_session_id=?")
                .bind(source.view_session_id).bind(owner).fetch_optional(&mut *conn).await?
                .ok_or_else(|| invalid("素材不属于此场次"))?
        };
        if source.kind != "recording"
            || source.session_id != owner
            || source.source_start_ms != start
            || start.checked_add(offset) != Some(source.view_start_ms)
            || source.duration_ms <= 0
            || end.is_none_or(|end| source.duration_ms > end.saturating_sub(start))
            // The media ID may be preserved by a replacement or allocated by a re-import.
            // Its immutable recording path still names the verified owning session/segment.
            || media["path"] != format!("recordings/recording-{owner}-{}.{container}",source.segment_id)
            || media["kind"] != "video"
        {
            return Err(invalid("录像来源、范围或素材路径不匹配"));
        }
        let path: String = row.try_get("path")?;
        source.file_name = std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        source.state = row.try_get("state")?;
        source.has_danmaku = row.try_get::<Option<String>, _>("danmaku_path")?.is_some();
        segments.push(source.segment_id);
    }
    Ok(segments)
}

pub async fn list(
    pool: &ConnectionPool,
    session: Option<i64>,
    before: Option<i64>,
) -> Result<Vec<Summary>> {
    list_filtered(pool, session, before, "", 100).await
}

pub async fn list_filtered(
    pool: &ConnectionPool,
    session: Option<i64>,
    before: Option<i64>,
    search: &str,
    limit: i64,
) -> Result<Vec<Summary>> {
    if search.chars().count() > 256 {
        return Err(invalid("搜索内容过长"));
    }
    Ok(sqlx::query_as(
        "SELECT id,session_id,name,revision,created_at,updated_at FROM editor_projects
        WHERE (?1 IS NULL OR session_id=?1) AND (?2 IS NULL OR id<?2)
        AND (?3='' OR instr(lower(name),lower(?3))>0) ORDER BY id DESC LIMIT ?4",
    )
    .bind(session)
    .bind(before)
    .bind(search.trim())
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &ConnectionPool, id: i64) -> Result<Saved> {
    let row = sqlx::query("SELECT p.*,h.head_revision,h.branch_revision,h.past_json,h.future_json FROM editor_projects p JOIN editor_history_cursor h ON h.project_id=p.id WHERE p.id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;
    let summary = Summary {
        id,
        session_id: row.try_get("session_id")?,
        name: row.try_get("name")?,
        revision: row.try_get("revision")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    };
    let history = history::cursor::Cursor::from_row(&row, summary.revision)?.info();
    Ok(Saved {
        summary,
        history,
        document: serde_json::from_str(row.try_get::<&str, _>("document_json")?)?,
    })
}

pub async fn revision(pool: &ConnectionPool, id: i64, rev: i64) -> Result<Document> {
    let text: String = sqlx::query_scalar(
        "SELECT document_json FROM editor_revisions WHERE project_id=? AND revision=?",
    )
    .bind(id)
    .bind(rev)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(serde_json::from_str(&text)?)
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct RevisionInfo {
    revision: i64,
    created_at: i64,
}
pub async fn revisions(pool: &ConnectionPool, id: i64) -> Result<Vec<RevisionInfo>> {
    get(pool, id).await?;
    Ok(sqlx::query_as("SELECT revision,created_at FROM editor_revisions WHERE project_id=? ORDER BY revision DESC LIMIT 200")
        .bind(id).fetch_all(pool).await?)
}

/// `previous` is absent only for creation. Updates always compare-and-swap.
pub async fn save(
    pool: &ConnectionPool,
    previous: Option<(i64, i64)>,
    document: Document,
) -> Result<Saved> {
    save_generated(pool, previous, document, &[]).await
}
pub async fn save_generated(
    pool: &ConnectionPool,
    previous: Option<(i64, i64)>,
    document: Document,
    generated: &[String],
) -> Result<Saved> {
    save_inner(pool, previous, document, generated, None, None, None).await
}
pub async fn save_user_results(
    pool: &ConnectionPool,
    previous: (i64, i64),
    document: Document,
    generated: &[String],
    note: Option<&history::UserNote>,
    suggestion: Option<&suggestions::Acceptance>,
) -> Result<Saved> {
    save_inner(
        pool,
        Some(previous),
        document,
        generated,
        None,
        note.map(agent::store::Journal::User),
        suggestion,
    )
    .await
}
pub(crate) async fn save_agent(
    pool: &ConnectionPool,
    previous: (i64, i64),
    document: Document,
    journal: agent::store::Journal<'_>,
) -> Result<Saved> {
    save_inner(
        pool,
        Some(previous),
        document,
        &[],
        None,
        Some(journal),
        None,
    )
    .await
}
pub(crate) async fn save_agent_generated(
    pool: &ConnectionPool,
    previous: (i64, i64),
    document: Document,
    generated: &[String],
    journal: agent::store::Journal<'_>,
) -> Result<Saved> {
    save_inner(
        pool,
        Some(previous),
        document,
        generated,
        None,
        Some(journal),
        None,
    )
    .await
}
pub(crate) async fn save_agent_download(
    pool: &ConnectionPool,
    previous: (i64, i64),
    document: Document,
    job: &str,
    journal: agent::store::Journal<'_>,
) -> Result<Saved> {
    save_inner(
        pool,
        Some(previous),
        document,
        &[],
        Some(job),
        Some(journal),
        None,
    )
    .await
}
async fn save_inner(
    pool: &ConnectionPool,
    previous: Option<(i64, i64)>,
    mut document: Document,
    generated: &[String],
    download: Option<&str>,
    journal: Option<agent::store::Journal<'_>>,
    suggestion: Option<&suggestions::Acceptance>,
) -> Result<Saved> {
    validate(&document)?;
    let _lifecycle = MEDIA_LIFECYCLE.read().await;
    metadata::prepare(pool, previous, &mut document).await?;
    // SQLite serializes writers before any reads: no deferred read-to-write upgrade race.
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some((id, revision)) = previous {
        let saved: (Option<i64>, i64) =
            sqlx::query_as("SELECT session_id,revision FROM editor_projects WHERE id=?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        if saved.1 != revision {
            return Err(Error::Conflict);
        }
        if saved.0.is_some() && saved.0 != document.origin.session_id {
            return Err(invalid("不能更改工程来源场次"));
        }
    }
    let segments = resolve_sources(&mut tx, &mut document).await?;
    for src in graphics::references(&document.project) {
        let (id, _) = previous.ok_or_else(|| invalid("请先创建工程再添加动态图形"))?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM editor_graphics WHERE project_id=? AND src=?)",
        )
        .bind(id)
        .bind(src)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(invalid("动态图形不存在或不属于当前工程"));
        }
    }
    if generated.len() > 10 {
        return Err(invalid("一次最多导入 10 个生成结果"));
    }
    for job in generated {
        let (id, _) = previous.ok_or_else(|| invalid("生成结果需要关联已保存的工程"))?;
        let turn = match &journal {
            Some(agent::store::Journal::Edit { turn, .. }) => Some(*turn),
            _ => None,
        };
        generate::acknowledge_import(&mut tx, id, &document, job, turn).await?;
    }
    if let Some(job) = download {
        let (id, _) = previous.ok_or_else(|| invalid("下载结果需要关联已保存的工程"))?;
        let turn = match &journal {
            Some(agent::store::Journal::Edit { turn, .. }) => *turn,
            _ => return Err(invalid("下载结果缺少助手记录")),
        };
        downloads::acknowledge(&mut tx, id, &document, job, turn).await?;
    }
    let text = serde_json::to_string(&document)?;
    if text.len() > 2 * 1024 * 1024 {
        return Err(invalid("工程数据超过 2 MiB"));
    }
    let name = document.project["name"].as_str().unwrap();
    let now = now_ms();
    let (id, rev) = match previous {
        Some((id, revision)) => {
            analysis::restore::record_changes(&mut tx, id, &document, now).await?;
            let next = revision
                .checked_add(1)
                .ok_or_else(|| invalid("版本号超出范围"))?;
            sqlx::query("UPDATE editor_projects SET name=?,revision=?,document_json=?,updated_at=?,session_id=? WHERE id=? AND revision=?")
                .bind(name).bind(next).bind(&text).bind(now).bind(document.origin.session_id).bind(id).bind(revision).execute(&mut *tx).await?;
            (id, next)
        }
        None => {
            let id = sqlx::query_scalar("INSERT INTO editor_projects(session_id,name,revision,document_json,created_at,updated_at) VALUES(?,?,1,?,?,?) RETURNING id")
                .bind(document.origin.session_id).bind(name).bind(&text).bind(now).bind(now).fetch_one(&mut *tx).await?;
            (id, 1)
        }
    };
    sqlx::query("INSERT INTO editor_revisions(project_id,revision,document_json,created_at) VALUES(?,?,?,?)")
        .bind(id).bind(rev).bind(&text).bind(now).execute(&mut *tx).await?;
    for segment in segments {
        sqlx::query("INSERT OR IGNORE INTO editor_sources(project_id,segment_id) VALUES(?,?)")
            .bind(id)
            .bind(segment)
            .execute(&mut *tx)
            .await?;
    }
    let created_at = sqlx::query_scalar("SELECT created_at FROM editor_projects WHERE id=?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if let Some(suggestion) = suggestion {
        suggestions::accept(&mut tx, id, rev, &document, suggestion, now).await?;
    }
    recent_media::index_document(&mut tx, id, &document, now).await?;
    let history = history::cursor::advance(
        &mut tx,
        id,
        previous.map(|(_, revision)| revision),
        rev,
        journal.as_ref(),
    )
    .await?;
    if let Some(journal) = journal {
        agent::store::commit(
            &mut tx,
            id,
            previous.ok_or_else(|| invalid("助手编辑需要已有工程"))?.1,
            rev,
            journal,
            now,
        )
        .await?;
    } else {
        history::write(
            &mut tx,
            id,
            rev,
            history::Note {
                kind: if rev == 1 { "initial" } else { "edit" },
                author: "user",
                label: if rev == 1 {
                    "创建工程"
                } else {
                    "保存工程"
                },
                lines: &[],
                turn: None,
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Saved {
        summary: Summary {
            id,
            session_id: document.origin.session_id,
            name: name.into(),
            revision: rev,
            created_at,
            updated_at: now,
        },
        document,
        history,
    })
}

pub async fn delete(pool: &ConnectionPool, id: i64, revision: i64) -> Result<()> {
    let _lifecycle = MEDIA_LIFECYCLE.read().await;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let current: i64 = sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    if current != revision {
        return Err(Error::Conflict);
    }
    let exporting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_exports WHERE project_id=? AND phase IN ('queued','preparing','rendering'))").bind(id).fetch_one(&mut *tx).await?;
    if exporting {
        return Err(invalid("工程正在导出，请先取消或等待完成"));
    }
    let generating:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_generation_jobs WHERE project_id=? AND phase='running')").bind(id).fetch_one(&mut *tx).await?;
    if generating {
        return Err(invalid("工程正在生成素材，请先取消或等待完成"));
    }
    let downloading: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM editor_downloads WHERE project_id=? AND phase='running')",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if downloading {
        return Err(invalid("工程正在下载素材，请先停止助手或等待完成"));
    }
    sqlx::query("DELETE FROM editor_projects WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests;

pub struct Storage<'a> {
    pub pool: &'a ConnectionPool,
    pub root: &'a std::path::Path,
}
