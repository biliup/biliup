//! Shared and project-scoped editorial resources; no arbitrary server file paths.
use super::{ConnectionPool, Error, Result, invalid};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Path, PathBuf};
pub mod importer;
pub mod brand;
pub mod skills;
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn text(value: &Value, key: &str, max: usize) -> bool {
    value[key].as_str().is_some_and(|s| s.len() <= max)
}
fn strings(value: &Value, key: &str, max: usize, length: usize) -> bool {
    value[key].as_array().is_some_and(|items| {
        items.len() <= max
            && items
                .iter()
                .all(|s| s.as_str().is_some_and(|s| s.len() <= length))
    })
}
fn check_kind(kind: &str) -> Result<()> {
    if !matches!(kind, "kit" | "skill" | "template" | "active") {
        return Err(invalid("未知的编辑器资源类型"));
    }
    Ok(())
}
async fn exists(pool: &ConnectionPool, project: i64) -> Result<()> {
    if sqlx::query_scalar::<_, i64>("SELECT id FROM editor_projects WHERE id=?")
        .bind(project)
        .fetch_optional(pool)
        .await?
        .is_none()
    {
        return Err(Error::NotFound);
    }
    Ok(())
}
fn scope(project: i64, value: &str) -> Result<(String, Option<i64>)> {
    match value {
        "global" => Ok(("global".into(), None)),
        "project" => Ok((project.to_string(), Some(project))),
        _ => Err(invalid("资源范围无效")),
    }
}
fn normalize(kind: &str, id: &str, scope: &str, mut value: Value) -> Result<Value> {
    if kind == "skill" && skills::contains(id) {
        return Err(invalid("请复制内置技能后编辑"));
    }
    if !identifier(id) || value.to_string().len() > 256 * 1024 || !value.is_object() {
        return Err(invalid("资源内容或标识无效"));
    }
    let valid = match kind {
        "active" => {
            scope == "project"
                && id == "active"
                && (value["kit"].is_null() || value["kit"].as_str().is_some_and(identifier))
                && strings(&value, "skills", 100, 64)
        }
        "template" => {
            scope == "global"
                && text(&value, "name", 320)
                && text(&value, "description", 1200)
                && text(&value, "body", 160000)
        }
        "skill" => {
            text(&value, "name", 320)
                && text(&value, "description", 1200)
                && text(&value, "body", 80000)
        }
        "kit" => {
            text(&value, "name", 320)
                && text(&value, "tone", 16000)
                && strings(&value, "taglines", 20, 800)
                && strings(&value, "urls", 20, 1200)
                && strings(&value, "dos", 100, 2000)
                && strings(&value, "donts", 100, 2000)
                && strings(&value, "examples", 24, 1200)
                && value["logos"].as_object().is_some_and(|logos| {
                    logos.len() <= 3
                        && logos.iter().all(|(slot, name)| {
                            matches!(slot.as_str(), "light" | "dark" | "mark")
                                && name.as_str().is_some_and(|s| s.len() <= 512)
                        })
                })
                && value["colors"].as_array().is_some_and(|colors| {
                    colors.len() <= 32
                        && colors.iter().all(|c| {
                            text(c, "hex", 7)
                                && text(c, "role", 200)
                                && c["name"].as_str().is_none_or(|s| s.len() <= 320)
                        })
                })
                && value["fonts"].as_array().is_some_and(|fonts| {
                    fonts.len() <= 8
                        && fonts.iter().all(|f| {
                            text(f, "family", 400)
                                && text(f, "role", 200)
                                && f["file"].as_str().is_none_or(|s| s.len() <= 512)
                        })
                })
        }
        _ => false,
    };
    if !valid {
        return Err(invalid("资源字段不完整或超出限制"));
    }
    let object = value.as_object_mut().unwrap();
    object.remove("builtin");
    if kind != "active" {
        object.insert("id".into(), json!(id));
        object.insert("updatedAt".into(), json!(chrono::Utc::now().to_rfc3339()));
        if kind != "template" {
            object.insert("scope".into(), json!(scope));
        }
    }
    Ok(value)
}
pub async fn list(pool: &ConnectionPool, project: i64, kind: &str) -> Result<Vec<Value>> {
    check_kind(kind)?;
    exists(pool, project).await?;
    let mut result = if kind == "skill" {
        skills::all().iter().map(|skill| json!({"id":skill["id"],"scope":"global","value":skill})).collect::<Vec<_>>()
    } else {
        vec![]
    };
    for row in sqlx::query("SELECT scope,id,value_json FROM editor_resources WHERE kind=? AND (project_id IS NULL OR project_id=?) ORDER BY scope='global' DESC,id")
        .bind(kind).bind(project).fetch_all(pool).await? {
        let scope: String = row.try_get("scope")?;
        let id: String = row.try_get("id")?;
        let value: Value = serde_json::from_str(row.try_get::<&str,_>("value_json")?)?;
        result.push(json!({"id":id,"scope":if scope=="global"{"global"}else{"project"},"value":value}));
    }
    Ok(result)
}
pub async fn save(
    pool: &ConnectionPool,
    project: i64,
    kind: &str,
    id: &str,
    target: &str,
    value: Value,
) -> Result<Vec<Value>> {
    check_kind(kind)?;
    exists(pool, project).await?;
    let value = normalize(kind, id, target, value)?;
    let (scope, owner) = scope(project, target)?;
    sqlx::query("INSERT INTO editor_resources(scope,project_id,kind,id,value_json,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(scope,kind,id) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at")
        .bind(scope).bind(owner).bind(kind).bind(id).bind(value.to_string()).bind(chrono::Utc::now().timestamp_millis()).execute(pool).await?;
    list(pool, project, kind).await
}
pub async fn delete(
    pool: &ConnectionPool,
    project: i64,
    kind: &str,
    id: &str,
    target: &str,
) -> Result<Vec<Value>> {
    check_kind(kind)?;
    exists(pool, project).await?;
    if !identifier(id) || kind == "active" || (kind == "skill" && skills::contains(id)) {
        return Err(invalid("不能删除此资源"));
    }
    let (scope, _) = scope(project, target)?;
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM editor_resources WHERE scope=? AND kind=? AND id=?")
        .bind(&scope)
        .bind(kind)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    if kind == "kit" {
        sqlx::query("DELETE FROM editor_brand_files WHERE scope=? AND kit=?")
            .bind(&scope)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    list(pool, project, kind).await
}
fn file_extension(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 512
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '\0'))
    {
        return Err(invalid("文件名无效"));
    }
    let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "gif"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "mp4"
            | "mov"
            | "webm"
    ) {
        return Err(invalid("请选择图片、字体或视频文件"));
    }
    Ok(extension)
}
pub async fn files(pool: &ConnectionPool, project: i64) -> Result<Vec<Value>> {
    exists(pool, project).await?;
    sqlx::query("SELECT scope,kit,name FROM editor_brand_files WHERE project_id IS NULL OR project_id=? ORDER BY scope='global' DESC,kit,name").bind(project).fetch_all(pool).await?.into_iter().map(|row|{let scope:String=row.try_get("scope")?;let kit:String=row.try_get("kit")?;let name:String=row.try_get("name")?;let target=if scope=="global"{"global"}else{"project"};Ok(json!({"scope":target,"kit":kit,"name":name,"url":format!("/v1/editor/projects/{project}/brand/{target}/{kit}/files/{}",urlencoding::encode(&name))}))}).collect()
}
pub async fn add_file(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    target: &str,
    kit: &str,
    name: &str,
    bytes: &[u8],
) -> Result<String> {
    exists(pool, project).await?;
    if !identifier(kit) {
        return Err(invalid("品牌标识无效"));
    }
    let (scope, owner) = scope(project, target)?;
    let extension = file_extension(name)?;
    if bytes.is_empty() || bytes.len() > 200 * 1024 * 1024 {
        return Err(invalid("品牌文件必须小于 200 MB"));
    }
    importer::validate(bytes,&extension)?;
    let hash = format!("{:x}", Sha256::digest(bytes));
    let directory = root.join("editor-brand-assets");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let path = directory.join(format!("{hash}.{extension}"));
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        let temp = tempfile::NamedTempFile::new_in(&directory)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .into_temp_path();
        tokio::fs::write(&temp, bytes)
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;
        temp.persist(&path)
            .map_err(|e| Error::Invalid(e.to_string()))?;
    }
    // A name already in a kit is immutable from a user's viewpoint; retain both versions.
    let mut name = name.to_owned();
    let original = name.clone();
    let mut suffix = 1;
    loop {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT hash FROM editor_brand_files WHERE scope=? AND kit=? AND name=?",
        )
        .bind(&scope)
        .bind(kit)
        .bind(&name)
        .fetch_optional(pool)
        .await?;
        if existing.as_deref() == Some(&hash) {
            return Ok(name);
        }
        if existing.is_none() {
            let inserted=sqlx::query("INSERT INTO editor_brand_files(scope,project_id,kit,name,hash,extension) VALUES(?,?,?,?,?,?) ON CONFLICT(scope,kit,name) DO NOTHING")
                .bind(&scope).bind(owner).bind(kit).bind(&name).bind(&hash).bind(&extension).execute(pool).await?;
            if inserted.rows_affected()==1{return Ok(name)}
            // A concurrent import claimed the name; compare content before allocating a suffix.
            continue;
        }
        suffix += 1;
        let stem = original
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&original);
        name = format!("{stem}-{suffix}.{extension}");
        if suffix > 1000 {
            return Err(invalid("同名品牌文件过多"));
        }
    }
}
pub async fn file(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    target: &str,
    kit: &str,
    name: &str,
) -> Result<PathBuf> {
    exists(pool, project).await?;
    let (scope, _) = scope(project, target)?;
    let row = sqlx::query(
        "SELECT hash,extension FROM editor_brand_files WHERE scope=? AND kit=? AND name=?",
    )
    .bind(scope)
    .bind(kit)
    .bind(name)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)?;
    let hash: String = row.try_get("hash")?;
    let extension: String = row.try_get("extension")?;
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid("品牌文件记录损坏"));
    }
    file_extension(&format!("file.{extension}"))?;
    Ok(root
        .join("editor-brand-assets")
        .join(format!("{hash}.{extension}")))
}
