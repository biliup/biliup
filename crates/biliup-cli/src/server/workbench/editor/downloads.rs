//! Durable downloads reused by assistant media_download/media_import and the normal importer.
//! Reference: Uncut agentDownload.ts and tools/ask.ts (MIT).
use super::{ConnectionPool, Document, Error, invalid};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqliteConnection};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio_util::{io::ReaderStream, sync::CancellationToken};
pub const MAX_BYTES: u64 = 300 * 1024 * 1024;
pub const TURN_BYTES: u64 = 1024 * 1024 * 1024;
#[derive(Default)]
pub struct Budget {
    pub count: usize,
    pub bytes: u64,
}
pub async fn initialize(pool: &ConnectionPool, root: &Path) -> sqlx::Result<()> {
    sqlx::query("UPDATE editor_downloads SET phase=CASE WHEN result_json IS NULL THEN 'failed' ELSE 'ready' END,error='服务重启，下载已停止；已有结果保留',updated_at=? WHERE phase='running'").bind(super::now_ms()).execute(pool).await?;
    // This directory owns only temporary transfers and pending font results. Keep ready fonts;
    // remove crash leftovers and results whose project was deleted before startup.
    if let Ok(mut entries) = tokio::fs::read_dir(root.join("editor-downloads")).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if !entry.file_type().await.is_ok_and(|kind| kind.is_file()) {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let owned = if let Some((id, ext)) = name.rsplit_once('.') {
                id.len()==32&&id.bytes().all(|c|c.is_ascii_hexdigit())&&is_font(&name)&&!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM editor_downloads WHERE id=? AND phase='ready' AND json_extract(result_json,'$.font')=1 AND json_extract(result_json,'$.extension')=?)").bind(id).bind(ext).fetch_one(pool).await?
            } else {
                false
            };
            if owned || name.starts_with(".tmp") {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }
    Ok(())
}
pub async fn get(pool: &ConnectionPool, project: i64, id: &str) -> Result<Value, Error> {
    let row = sqlx::query("SELECT * FROM editor_downloads WHERE id=? AND project_id=?")
        .bind(id)
        .bind(project)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(
        json!({"id":row.try_get::<String,_>("id")?,"owner":row.try_get::<Option<String>,_>("agent_turn_id")?,"source":row.try_get::<String,_>("source")?,"name":row.try_get::<Option<String>,_>("name")?,"phase":row.try_get::<String,_>("phase")?,"bytes":row.try_get::<i64,_>("bytes")?,"error":row.try_get::<Option<String>,_>("error")?,"result":row.try_get::<Option<String>,_>("result_json")?.map(|s|serde_json::from_str::<Value>(&s)).transpose()?,"startedAt":super::agent::store::timestamp(row.try_get("created_at")?)}),
    )
}
pub async fn list(pool: &ConnectionPool, project: i64) -> Result<Vec<Value>, Error> {
    let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM editor_downloads WHERE project_id=? ORDER BY created_at DESC,id DESC LIMIT 40").bind(project).fetch_all(pool).await?;
    let mut out = vec![];
    for id in ids {
        let mut value = get(pool, project, &id).await?;
        value.as_object_mut().unwrap().remove("owner");
        value.as_object_mut().unwrap().remove("result");
        out.push(value);
    }
    Ok(out)
}
pub async fn prepare(
    pool: &ConnectionPool,
    project: i64,
    turn: &str,
    source: Option<&str>,
    name: Option<&str>,
    job: Option<&str>,
    retry: bool,
) -> Result<Value, Error> {
    if name.is_some_and(|name| {
        name.trim().is_empty()
            || name
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    }) {
        return Err(invalid("素材名称无效"));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
    if !active {
        return Err(invalid("助手已停止"));
    }
    let key = format!(
        "{:x}",
        Sha256::digest(format!("{}\n{}", source.unwrap_or(""), name.unwrap_or("")))
    );
    let existing = if let Some(id) = job {
        Some(id.to_string())
    } else {
        sqlx::query_scalar::<_,String>("SELECT id FROM editor_downloads WHERE project_id=? AND agent_turn_id=? AND request_key=? ORDER BY created_at DESC LIMIT 1").bind(project).bind(turn).bind(&key).fetch_optional(&mut *tx).await?
    };
    let now = super::now_ms();
    let id = if let Some(id) = existing {
        let row = sqlx::query(
            "SELECT phase,agent_turn_id FROM editor_downloads WHERE id=? AND project_id=?",
        )
        .bind(&id)
        .bind(project)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let owner: Option<String> = row.try_get("agent_turn_id")?;
        if let Some(owner) = owner.filter(|owner| owner != turn) {
            let active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')",
            )
            .bind(owner)
            .fetch_one(&mut *tx)
            .await?;
            if active {
                return Err(invalid("另一个助手轮次正在处理此下载"));
            }
        }
        let phase: String = row.try_get("phase")?;
        if phase == "running" {
            return Err(invalid("下载仍在处理中"));
        }
        if retry && matches!(phase.as_str(), "failed" | "cancelled") {
            sqlx::query("UPDATE editor_downloads SET phase=CASE WHEN result_json IS NULL THEN 'queued' ELSE 'ready' END,error=NULL,agent_turn_id=?,updated_at=? WHERE id=?").bind(turn).bind(now).bind(&id).execute(&mut *tx).await?;
        } else {
            sqlx::query("UPDATE editor_downloads SET agent_turn_id=?,updated_at=? WHERE id=?")
                .bind(turn)
                .bind(now)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
        }
        id
    } else {
        let source = source.ok_or_else(|| invalid("请提供素材地址或已有下载 job"))?;
        // Drop only old terminal records; pending results remain recoverable by ID.
        sqlx::query("DELETE FROM editor_downloads WHERE project_id=? AND phase IN ('done','failed','cancelled') AND id NOT IN (SELECT id FROM editor_downloads WHERE project_id=? ORDER BY created_at DESC,id DESC LIMIT 200)").bind(project).bind(project).execute(&mut *tx).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM editor_downloads WHERE project_id=?")
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 500 {
            return Err(invalid("待处理下载过多，请先处理已有下载结果"));
        }
        let id = super::generate::random_id().map_err(Error::Invalid)?;
        sqlx::query("INSERT INTO editor_downloads(id,project_id,agent_turn_id,request_key,source,name,phase,created_at,updated_at) VALUES(?,?,?,?,?,?,'queued',?,?)").bind(&id).bind(project).bind(turn).bind(key).bind(source).bind(name).bind(now).bind(now).execute(&mut *tx).await?;
        id
    };
    tx.commit().await?;
    get(pool, project, &id).await
}
pub async fn acknowledge(
    conn: &mut SqliteConnection,
    project: i64,
    document: &Document,
    id: &str,
    turn: &str,
) -> Result<(), Error> {
    let raw:Option<String>=sqlx::query_scalar("SELECT result_json FROM editor_downloads WHERE id=? AND project_id=? AND agent_turn_id=? AND phase='ready'").bind(id).bind(project).bind(turn).fetch_optional(&mut *conn).await?.flatten();
    let result: Value =
        serde_json::from_str(&raw.ok_or_else(|| invalid("下载结果已处理，请重新读取"))?)?;
    let media = &result["media"];
    let key = media["id"]
        .as_str()
        .ok_or_else(|| invalid("下载素材标识无效"))?;
    let actual = serde_json::to_value(document.sources.get(key))?;
    if !document.project["media"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == key && item["path"] == media["path"])
    }) || [
        "kind",
        "assetId",
        "durationMs",
        "sessionId",
        "segmentId",
        "viewSessionId",
        "viewStartMs",
        "sourceStartMs",
    ]
    .iter()
    .any(|key| actual[key] != result["source"][key])
    {
        return Err(invalid("工程未包含下载结果或其素材来源不匹配"));
    }
    sqlx::query("UPDATE editor_downloads SET phase='done',error=NULL,updated_at=? WHERE id=?")
        .bind(super::now_ms())
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
fn extension(value: &str) -> &str {
    value.rsplit('.').next().unwrap_or("")
}
fn mime_extension(mime: &str) -> Option<&'static str> {
    super::resources::importer::mime_extension(mime).or_else(|| {
        Some(match mime {
            "image/avif" => "avif",
            "video/mp4" => "mp4",
            "video/quicktime" => "mov",
            "video/webm" => "webm",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
            "audio/mp4" | "audio/x-m4a" => "m4a",
            "audio/aac" => "aac",
            "audio/ogg" => "ogg",
            "audio/flac" => "flac",
            _ => return None,
        })
    })
}
pub fn is_font(name: &str) -> bool {
    matches!(extension(name), "woff" | "woff2" | "ttf" | "otf")
}
fn allowed(ext: &str) -> bool {
    matches!(
        ext,
        "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "avif"
            | "mp4"
            | "mov"
            | "webm"
            | "mp3"
            | "wav"
            | "m4a"
            | "aac"
            | "ogg"
            | "flac"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
    )
}
pub fn staged_path(root: &Path, id: &str, ext: &str) -> Result<PathBuf, Error> {
    if id.is_empty()
        || id.len() > 100
        || !id.bytes().all(|c| c.is_ascii_hexdigit())
        || !allowed(ext)
    {
        return Err(invalid("下载文件标识无效"));
    }
    Ok(root.join("editor-downloads").join(format!("{id}.{ext}")))
}
pub async fn transfer(
    super::Storage { pool, root }: super::Storage<'_>,
    project: i64,
    job: &Value,
    budget: &mut Budget,
    cancel: &CancellationToken,
    online: &super::online_library::Library,
    mut progress: impl FnMut(u64, Option<u64>) + Send + 'static,
) -> Result<Value, Error> {
    if budget.count >= 20 || budget.bytes >= TURN_BYTES {
        return Err(invalid("本轮下载已达到 20 个文件或 1 GiB 上限"));
    }
    budget.count += 1;
    let id = job["id"].as_str().unwrap();
    let source = job["source"].as_str().unwrap();
    let preferred = job["name"].as_str();
    if let Some(recent) = source.strip_prefix("recent:") {
        return super::recent_media::import_pinned(pool, project, recent).await;
    }
    if let Some(item) = source.strip_prefix("library:") {
        let bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let meter = bytes.clone();
        let mut result = super::library::add(
            super::Storage { pool, root },
            online,
            project,
            item,
            (TURN_BYTES - budget.bytes).min(MAX_BYTES) as usize,
            cancel,
            move |count, total| {
                meter.store(count, std::sync::atomic::Ordering::Relaxed);
                progress(count, total)
            },
        )
        .await;
        let size = bytes.load(std::sync::atomic::Ordering::Relaxed);
        budget.bytes += size;
        if let Ok(value) = &mut result {
            value["bytes"] = json!(size);
        }
        return result;
    }
    let directory = root.join("editor-downloads");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let temp = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| Error::Invalid(e.to_string()))?
        .into_temp_path();
    let mut output = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let (name, ext, total, mut body) = if source.starts_with("https://") {
        let response = super::public_http::get(source, None, None)
            .await
            .map_err(|e| Error::Invalid(e.replace("音源", "素材")))?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(Error::Invalid(format!(
                "下载失败（{}）",
                response.status().as_u16()
            )));
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let raw = response
            .url()
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap_or("download");
        let decoded = urlencoding::decode(raw).map_err(|_| invalid("文件地址编码无效"))?;
        let mut candidate = extension(&decoded).to_ascii_lowercase();
        if !allowed(&candidate) {
            candidate = preferred.map(extension).unwrap_or("").to_ascii_lowercase();
        }
        let ext = mime_extension(&mime)
            .or_else(|| {
                (matches!(
                    mime.as_str(),
                    "" | "application/octet-stream" | "binary/octet-stream"
                ) && allowed(&candidate))
                .then_some(candidate.as_str())
            })
            .ok_or_else(|| {
                invalid("链接未返回支持的媒体或字体文件；网页请先用 web_fetch 查找文件直接链接")
            })?
            .to_string();
        let stem = preferred.unwrap_or(&decoded);
        let stem = if allowed(extension(stem)) {
            stem.rsplit_once('.').map(|v| v.0).unwrap_or(stem)
        } else {
            stem
        };
        let stem = stem
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '?' | '*' | '"' | '<' | '>' | '|')
                {
                    '-'
                } else {
                    c
                }
            })
            .take(80)
            .collect::<String>();
        let stem = stem.trim_matches(['.', ' ', '-']);
        let stem = if stem.is_empty() { "download" } else { stem };
        let name = format!("{stem}.{ext}");
        let total = response.content_length();
        (
            name,
            ext,
            total,
            axum::body::Body::from_stream(super::public_http::stream(response, MAX_BYTES as usize))
                .into_data_stream(),
        )
    } else {
        let files = super::resources::files(pool, project).await?;
        let file = files
            .iter()
            .find(|f| f["url"] == source)
            .ok_or_else(|| invalid("请使用已登记的品牌资源链接；本机文件请在素材面板导入"))?;
        let original = file["name"].as_str().unwrap();
        let path = super::resources::file(
            pool,
            root,
            project,
            file["scope"].as_str().unwrap(),
            file["kit"].as_str().unwrap(),
            original,
        )
        .await?;
        let input = tokio::fs::File::open(path)
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let size = input
            .metadata()
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?
            .len();
        let ext = extension(original).to_ascii_lowercase();
        let name = preferred
            .map(|n| format!("{}.{}", n.trim_end_matches(&format!(".{ext}")), ext))
            .unwrap_or_else(|| original.to_string());
        (
            name,
            ext,
            Some(size),
            axum::body::Body::from_stream(ReaderStream::new(input)).into_data_stream(),
        )
    };
    let max = if is_font(&name) {
        super::project_fonts::MAX_BYTES as u64
    } else {
        MAX_BYTES
    };
    if total.is_some_and(|n| n > max || n > TURN_BYTES - budget.bytes) {
        return Err(invalid("文件超过本次下载大小限制"));
    }
    use futures::StreamExt;
    let mut size = 0u64;
    let mut head = Vec::new();
    while let Some(chunk) = body.next().await {
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止下载"));
        }
        let chunk = chunk.map_err(|_| invalid("素材下载中断"))?;
        size += chunk.len() as u64;
        budget.bytes += chunk.len() as u64;
        if size > max || budget.bytes > TURN_BYTES {
            return Err(invalid("素材下载超过大小限制"));
        }
        if head.len() < 16 {
            head.extend_from_slice(&chunk[..chunk.len().min(16 - head.len())]);
        }
        output
            .write_all(&chunk)
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;
        progress(size, total);
    }
    output
        .flush()
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    drop(output);
    if size == 0 {
        return Err(invalid("不能导入空文件"));
    }
    if ext == "svg" {
        let bytes = super::resources::importer::read(&temp, super::svg::MAX_BYTES).await?;
        super::resources::importer::validate(&bytes, &ext)?;
    } else {
        sniff(&head, &ext)?;
    }
    let result = if is_font(&name) {
        let path = staged_path(root, id, &ext)?;
        // A failed database write can leave this job's unregistered font behind. A queued retry
        // owns this exact file; ready jobs skip transfer and retain their original result.
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| Error::Invalid(e.to_string()))?;
        }
        temp.persist_noclobber(&path)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        json!({"font":true,"name":name,"extension":ext,"bytes":size})
    } else {
        let input = tokio::fs::File::open(&temp)
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let mut value = super::imports::upload(
            pool,
            root,
            project,
            &name,
            None,
            axum::body::Body::from_stream(ReaderStream::new(input)),
        )
        .await
        .map_err(Error::Invalid)?;
        value["media"]["source"] =
            json!({"module":"import.download","kind":"download","url":source});
        value["bytes"] = json!(size);
        value
    };
    Ok(result)
}
fn sniff(head: &[u8], ext: &str) -> Result<(), Error> {
    let at = |offset: usize, sig: &[u8]| head.get(offset..offset + sig.len()) == Some(sig);
    let valid = match ext {
        "avif" | "m4a" => at(4, b"ftyp"),
        "mp3" => {
            at(0, b"ID3")
                || head.first() == Some(&255) && head.get(1).is_some_and(|v| v & 0xe0 == 0xe0)
        }
        "wav" => at(0, b"RIFF") && at(8, b"WAVE"),
        "aac" => head.first() == Some(&255) && head.get(1).is_some_and(|v| v & 0xf0 == 0xf0),
        "ogg" => at(0, b"OggS"),
        "flac" => at(0, b"fLaC"),
        _ => return super::resources::importer::validate(head, ext),
    };
    if valid {
        Ok(())
    } else {
        Err(invalid("下载内容与媒体格式不符，可能返回了网页或播放列表"))
    }
}
