//! One import implementation for editor library controls and assistant library_add.
use super::{ConnectionPool, Error, invalid};
use rust_embed::Embed;
use serde_json::{Value, json};
use std::sync::LazyLock;
use tokio_util::sync::CancellationToken;
#[derive(Embed)]
#[folder = "../../public/editor-sfx/"]
struct Sounds;
static CATALOG: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/sfx/library.json"
    )))
    .expect("bundled sound catalog")
});
// Same sfxCredit fields as core/audio/library.ts, from the same bundled CC0 manifest.
fn bundled(id: &str) -> Result<Value, Error> {
    let sound = CATALOG["sounds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .ok_or_else(|| invalid("内置音效不存在"))?;
    let title = sound["title"]
        .as_str()
        .ok_or_else(|| invalid("音效标题无效"))?;
    let author = sound["author"]
        .as_str()
        .ok_or_else(|| invalid("音效署名无效"))?;
    let license = "https://creativecommons.org/publicdomain/zero/1.0/";
    Ok(
        json!({"file":sound["file"],"title":title,"source":{"module":"library.bundled","libraryId":format!("sfx:{id}"),"title":title,"creator":author,"license":"cc0","licenseVersion":"1.0","licenseUrl":license,"landingUrl":sound["source"],"provider":sound["provider"],"attribution":format!("\"{title}\" by {author} is marked with CC0 1.0. To view the terms, visit {license}")}}),
    )
}
async fn identity(pool: &ConnectionPool, project: i64, mut entry: Value) -> Result<Value, Error> {
    let saved = super::get(pool, project).await?;
    let (media, _) = super::imports::prepare_media(&saved.document.project, &entry["media"])?;
    entry["media"] = media;
    Ok(entry)
}

pub async fn add(
    super::Storage { pool, root }: super::Storage<'_>,
    online: &super::online_library::Library,
    project: i64,
    id: &str,
    max: usize,
    cancel: &CancellationToken,
    mut progress: impl FnMut(u64, Option<u64>) + Send + 'static,
) -> Result<Value, Error> {
    if id.len() > 300 {
        return Err(invalid("音源标识过长"));
    }
    if cancel.is_cancelled() {
        return Err(invalid("音源导入已取消"));
    }
    let saved = super::get(pool, project).await?;
    if let Some(media) = saved.document.project["media"]
        .as_array()
        .and_then(|all| all.iter().find(|m| m["source"]["libraryId"] == id))
    {
        let key = media["id"]
            .as_str()
            .ok_or_else(|| invalid("素材标识无效"))?;
        if let Some(source) = saved.document.sources.get(key) {
            let path = super::media::source_path(pool, source).await?;
            if tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file()) {
                return Ok(json!({"media":media,"source":source}));
            }
        }
    }
    if let Some(id) = id.strip_prefix("folder:") {
        let (mut entry, path) = super::sound_library::sound(pool, project, id).await?;
        if !tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file()) {
            return Err(invalid("音频文件不存在"));
        }
        entry["media"]["source"] =
            json!({"module":"library.folder","libraryId":format!("folder:{id}")});
        return identity(pool, project, entry).await;
    }
    if let Some(id) = id.strip_prefix("sfx:") {
        let spec = bundled(id)?;
        let file = spec["file"]
            .as_str()
            .ok_or_else(|| invalid("内置音效路径无效"))?;
        let bytes = Sounds::get(file)
            .ok_or_else(|| invalid("安装包缺少此内置音效"))?
            .data;
        if bytes.len() > max {
            return Err(invalid("音效超过本轮导入大小限制"));
        }
        progress(bytes.len() as u64, Some(bytes.len() as u64));
        let mut entry = super::imports::upload(
            pool,
            root,
            project,
            file,
            None,
            axum::body::Body::from(bytes.into_owned()),
        )
        .await
        .map_err(Error::Invalid)?;
        entry["media"]["name"] = spec["title"].clone();
        entry["media"]["source"] = spec["source"].clone();
        return identity(pool, project, entry).await;
    }
    let (source, id) = id
        .split_once(':')
        .ok_or_else(|| invalid("请使用素材库搜索返回的音源标识"))?;
    if !matches!(source, "openverse" | "freesound") {
        return Err(invalid("音源类型无效"));
    }
    let entry = online
        .add_limited(project, source, id, max, progress)
        .await
        .map_err(Error::Invalid)?;
    identity(pool, project, entry).await
}
