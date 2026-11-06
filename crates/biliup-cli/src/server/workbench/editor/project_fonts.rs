//! Portable project fonts, also used when the browser shares an installed font's bytes.
//! No host font-directory assumptions. Registered immutable files use the existing brand asset store.
use super::{ConnectionPool, Error, invalid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::Path;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
#[derive(Deserialize)]
pub struct Upload {
    pub name: String,
    pub family: Option<String>,
    pub weight: Option<String>,
    pub style: Option<String>,
}
#[derive(Serialize)]
pub struct Font {
    pub hash: String,
    pub extension: String,
    pub family: String,
    pub weight: String,
    pub style: String,
    pub name: String,
    pub bytes: i64,
}
fn u16be(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32be(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_be_bytes(bytes.get(at..at.checked_add(4)?)?.try_into().ok()?) as usize)
}
fn table<'a>(bytes: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    if !bytes.starts_with(&[0, 1, 0, 0]) && !bytes.starts_with(b"OTTO") {
        return None;
    }
    let count = u16be(bytes, 4)? as usize;
    if count > 4096 {
        return None;
    }
    for i in 0..count {
        let pos = 12 + i * 16;
        if bytes.get(pos..pos + 4) == Some(tag) {
            let start = u32be(bytes, pos + 8)?;
            let len = u32be(bytes, pos + 12)?;
            return bytes.get(start..start.checked_add(len)?);
        }
    }
    None
}
fn own_family(bytes: &[u8]) -> Option<String> {
    let data = table(bytes, b"name")?;
    let strings = u16be(data, 4)? as usize;
    let count = u16be(data, 2)? as usize;
    let mut candidates = Vec::new();
    for i in 0..count.min(4096) {
        let p = 6 + i * 12;
        let platform = u16be(data, p)?;
        let language = u16be(data, p + 4)?;
        let name = u16be(data, p + 6)?;
        if !matches!(name, 1 | 16) {
            continue;
        }
        let size = u16be(data, p + 8)? as usize;
        let start = strings.checked_add(u16be(data, p + 10)? as usize)?;
        let Some(raw) = data.get(start..start.checked_add(size)?) else {
            continue;
        };
        let value = if matches!(platform, 0 | 3) {
            String::from_utf16(
                &raw.chunks_exact(2)
                    .map(|b| u16::from_be_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .ok()
        } else if platform == 1 && raw.is_ascii() {
            String::from_utf8(raw.to_vec()).ok()
        } else {
            None
        };
        if let Some(value) = value {
            let value = value.trim_matches('\0').trim();
            if super::fonts::family(value) {
                let score = (if name == 16 { 4 } else { 0 })
                    + if language == 0x0409 { 2 } else { 0 }
                    + if platform == 3 { 1 } else { 0 };
                candidates.push((score, value.to_string()));
            }
        }
    }
    candidates.sort_by_key(|(score, _)| *score);
    candidates.pop().map(|(_, family)| family)
}
fn style_weight(bytes: &[u8], name: &str) -> (String, String) {
    let os2 = table(bytes, b"OS/2");
    let italic = os2
        .and_then(|b| u16be(b, 62))
        .is_some_and(|flags| flags & 1 != 0)
        || name.to_lowercase().contains("italic");
    let weight = if table(bytes, b"fvar").is_some() || name.to_lowercase().contains("variable") {
        "100 1000".into()
    } else if let Some(weight) = os2
        .and_then(|b| u16be(b, 4))
        .filter(|w| (100..=1000).contains(w))
    {
        weight.to_string()
    } else {
        let name = name.to_lowercase();
        let weights = [
            ("thin", 100),
            ("extralight", 200),
            ("ultralight", 200),
            ("light", 300),
            ("medium", 500),
            ("semibold", 600),
            ("demibold", 600),
            ("extrabold", 800),
            ("ultrabold", 800),
            ("black", 900),
            ("heavy", 900),
            ("bold", 700),
        ];
        weights
            .iter()
            .find(|(part, _)| name.replace('-', "").contains(part))
            .map(|(_, w)| w.to_string())
            .unwrap_or_else(|| "400".into())
    };
    (weight, if italic { "italic" } else { "normal" }.into())
}
pub async fn list(pool: &ConnectionPool, project: i64) -> Result<Vec<Font>, Error> {
    if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?)")
        .bind(project)
        .fetch_one(pool)
        .await?
    {
        return Err(Error::NotFound);
    }
    sqlx::query("SELECT * FROM editor_project_fonts WHERE project_id=? ORDER BY family,weight,style,created_at").bind(project).fetch_all(pool).await?.into_iter().map(|r|Ok(Font{hash:r.try_get("hash")?,extension:r.try_get("extension")?,family:r.try_get("family")?,weight:r.try_get("weight")?,style:r.try_get("style")?,name:r.try_get("name")?,bytes:r.try_get("bytes")?})).collect()
}
pub async fn upload(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    input: Upload,
    bytes: &[u8],
) -> Result<Vec<Font>, Error> {
    upload_inner(pool,root,project,input,bytes,None).await
}
pub(crate) async fn upload_agent(pool:&ConnectionPool,root:&Path,project:i64,input:Upload,bytes:&[u8],turn:&str,job:&str)->Result<Vec<Font>,Error>{
    upload_inner(pool,root,project,input,bytes,Some((turn,job))).await
}
async fn upload_inner(pool:&ConnectionPool,root:&Path,project:i64,input:Upload,bytes:&[u8],owner:Option<(&str,&str)>)->Result<Vec<Font>,Error>{
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(invalid("字体文件必须小于 2 MiB"));
    }
    if input.name.is_empty()
        || input.name.len() > 240
        || input
            .name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(invalid("字体文件名称无效"));
    }
    let ext = if bytes.starts_with(b"wOF2") {
        "woff2"
    } else if bytes.starts_with(b"wOFF") {
        "woff"
    } else if bytes.starts_with(b"OTTO") {
        "otf"
    } else if bytes.starts_with(&[0, 1, 0, 0]) {
        "ttf"
    } else {
        return Err(invalid("请选择 TTF、OTF、WOFF 或 WOFF2 字体"));
    };
    let (weight, style) = style_weight(bytes, &input.name);
    let stem = input
        .name
        .rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(&input.name);
    let family = input
        .family
        .or_else(|| own_family(bytes))
        .unwrap_or_else(|| stem.split('-').next().unwrap_or(stem).replace('_', " "));
    let family = family.trim().to_string();
    if !super::fonts::family(&family) {
        return Err(invalid("字体名称无效"));
    }
    let weight = input.weight.unwrap_or(weight);
    let values = weight
        .split(' ')
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("字体字重无效"))?;
    if values.is_empty()
        || values.len() > 2
        || values.iter().any(|n| !(100..=1000).contains(n))
        || values.len() == 2 && values[0] > values[1]
    {
        return Err(invalid("字体字重无效"));
    }
    let style = input.style.unwrap_or(style);
    if !matches!(style.as_str(), "normal" | "italic") {
        return Err(invalid("字体样式无效"));
    }
    let hash = format!("{:x}", Sha256::digest(bytes));
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some((turn,job))=owner{
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_downloads d JOIN editor_agent_turns t ON t.id=d.agent_turn_id WHERE d.id=? AND d.project_id=? AND d.agent_turn_id=? AND d.phase='ready' AND t.status='running' AND json_extract(d.result_json,'$.font')=1 AND json_extract(d.result_json,'$.name')=?)").bind(job).bind(project).bind(turn).bind(&input.name).fetch_one(&mut *tx).await?;
        if !active{return Err(invalid("助手已停止或下载结果已处理"))}
    }
    if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?)")
        .bind(project)
        .fetch_one(&mut *tx)
        .await?
    {
        return Err(Error::NotFound);
    }
    let (count,size):(i64,i64)=sqlx::query_as("SELECT COUNT(*),COALESCE(SUM(bytes),0) FROM editor_project_fonts WHERE project_id=? AND NOT (hash=? AND family=? AND weight=? AND style=?)").bind(project).bind(&hash).bind(&family).bind(&weight).bind(&style).fetch_one(&mut *tx).await?;
    if count >= 64 || size + bytes.len() as i64 > 16 * 1024 * 1024 {
        return Err(invalid(
            "每个工程最多导入 64 个字体文件，总大小不能超过 16 MiB",
        ));
    }
    let folder = root.join("editor-brand-assets");
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|_| invalid("无法创建字体存储目录"))?;
    let path = folder.join(format!("{hash}.{ext}"));
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        let temp = tempfile::NamedTempFile::new_in(&folder)
            .map_err(|_| invalid("无法保存字体"))?
            .into_temp_path();
        tokio::fs::write(&temp, bytes)
            .await
            .map_err(|_| invalid("无法保存字体"))?;
        temp.persist_noclobber(&path)
            .or_else(|e| {
                if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|_| invalid("无法保存字体"))?;
    }
    sqlx::query("INSERT OR IGNORE INTO editor_project_fonts(project_id,hash,extension,family,weight,style,name,bytes,created_at) VALUES(?,?,?,?,?,?,?,?,?)").bind(project).bind(hash).bind(ext).bind(family).bind(weight).bind(style).bind(input.name).bind(bytes.len() as i64).bind(super::now_ms()).execute(&mut *tx).await?;
    if let Some((_,job))=owner{sqlx::query("UPDATE editor_downloads SET phase='done',error=NULL,updated_at=? WHERE id=?").bind(super::now_ms()).bind(job).execute(&mut *tx).await?;}
    tx.commit().await?;
    list(pool, project).await
}
