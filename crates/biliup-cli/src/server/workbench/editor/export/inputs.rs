//! Resolve the shared planner's dependencies, never every file in the media library.
use super::browser::Browser;
use crate::server::{
    common::processing::transcode,
    workbench::editor::{self, ConnectionPool, Document, Error},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
struct Selection {
    project: Value,
    media: BTreeSet<String>,
    graphics: bool,
}

/// Reuse the browser and select required files without trimming the timeline.
/// Playback phases, transition handles and retry snapshots keep their original coordinates.
pub(super) async fn prepare(
    pool: &ConnectionPool,
    document: &mut Document,
    range: &Value,
    purpose: &str,
    cancel: &CancellationToken,
) -> Result<(Browser, BTreeMap<String, String>), String> {
    let permit = transcode::acquire(cancel).await?;
    prepare_in_slot(pool, document, range, purpose, cancel, &permit).await
}

/// A caller doing preparation and audio processing together keeps one slot for both.
pub(super) async fn prepare_in_slot(
    pool: &ConnectionPool,
    document: &mut Document,
    range: &Value,
    purpose: &str,
    cancel: &CancellationToken,
    _permit: &tokio::sync::OwnedSemaphorePermit,
) -> Result<(Browser, BTreeMap<String, String>), String> {
    let mut browser = Browser::launch(cancel).await?;
    let selected: Selection = serde_json::from_value(browser.eval(&format!(
        "BiliupExport.selectRenderInputs({},{range},{})", document.project, json!(purpose),
    )).await?).map_err(|e| format!("导出素材计划无效：{e}"))?;
    document.project = selected.project;
    let mut paths = BTreeMap::new();
    for id in &selected.media {
        if cancel.is_cancelled() { return Err("素材准备已取消".into()); }
        let source = document.sources.get(id).ok_or("导出引用了未登记素材")?;
        let path = source_path(pool, source).await.map_err(|error| {
            let name = document.project["media"].as_array().into_iter().flatten()
                .find(|media| media["id"] == *id)
                .and_then(|media| media["name"].as_str()).unwrap_or(id);
            tracing::warn!(media=%id, %error, "读取导出所需素材失败");
            format!("导出所需素材不可读取：{name}")
        })?;
        paths.insert(id.clone(), std::path::absolute(path).map_err(|e|e.to_string())?.to_string_lossy().into_owned());
    }
    if selected.graphics {
        // A graphic can request registered media dynamically. Expose available files
        // without probing them; a missing file only matters if the graphic asks for it.
        for (id, source) in &document.sources {
            if cancel.is_cancelled() { return Err("素材准备已取消".into()); }
            if paths.contains_key(id) { continue; }
            match source_path(pool, source).await {
                Ok(path) => { paths.insert(id.clone(), std::path::absolute(path).map_err(|e|e.to_string())?.to_string_lossy().into_owned()); }
                Err(Error::NotFound) => {}
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    Ok((browser, paths))
}

async fn source_path(pool:&ConnectionPool,source:&editor::MediaSource)->Result<PathBuf,Error>{
    let path=editor::media::source_path(pool,source).await?;
    match tokio::fs::metadata(&path).await {
        Ok(info) if info.is_file()=>Ok(path),
        Ok(_)=>Err(Error::NotFound),
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>Err(Error::NotFound),
        Err(_)=>Err(Error::Invalid("无法读取素材文件信息".into())),
    }
}
