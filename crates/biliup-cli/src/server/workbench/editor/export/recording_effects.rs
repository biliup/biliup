//! Recorded comments are rendered as transparent media; picture masks stay in the editor export graph.
pub(super) struct Preparation<'a> {
    pub(super) document: &'a mut editor::Document,
    pub(super) range: &'a Value,
    pub(super) paths: &'a mut BTreeMap<String, String>,
    pub(super) directory: &'a Path,
}

use super::*;
use crate::server::{
    common::processing::{engine::SourceSpan, model::RenderRecipe},
    infrastructure::connection_pool::ConnectionPool,
    workbench::{editor, store},
};
use std::path::PathBuf;

#[derive(Deserialize)]
struct DanmakuWindow {
    item: String,
    from: u64,
    to: u64,
    #[serde(rename = "in")]
    offset: u64,
    #[serde(rename = "loop")]
    loops: bool,
}

/// SVG sources and image covers use Chromium, just as the live editor does.
/// Only job-local paths change; the saved media identity and project stay intact.
pub(super) async fn prepare_mask_images(
    browser: &mut browser::Browser,
    project: &Value,
    range: &Value,
    paths: &mut BTreeMap<String, String>,
    directory: &Path,
) -> Result<(), String> {
    let visible: std::collections::BTreeSet<String> = serde_json::from_value(
        browser
            .eval(&format!("BiliupExport.pictureItemIds({project},{range})",))
            .await?,
    )
    .map_err(|error| format!("画面素材计划无效：{error}"))?;
    let mut needed = std::collections::BTreeSet::new();
    for track in project["tracks"].as_array().into_iter().flatten() {
        if track["hidden"] == true || track["kind"] == "audio" {
            continue;
        }
        for item in track["items"].as_array().into_iter().flatten() {
            if !item["id"].as_str().is_some_and(|id| visible.contains(id)) {
                continue;
            }
            for effect in item["effects"].as_array().into_iter().flatten() {
                if effect["module"] != "effect.biliup-mask" || effect["bypass"] == true {
                    continue;
                }
                let regions = effect["params"]["regions"]
                    .as_array()
                    .ok_or("遮挡区域无效")?;
                if regions.is_empty() {
                    continue;
                }
                if let Some(id) = item["media"].as_str() {
                    needed.insert(id.to_owned());
                }
                for region in regions {
                    if region["effect_type"] == "image" {
                        needed.insert(
                            region["image"]["media"]
                                .as_str()
                                .ok_or("遮挡图片引用无效")?
                                .to_owned(),
                        );
                    }
                }
            }
        }
    }
    let mut images = BTreeMap::new();
    for media in project["media"].as_array().into_iter().flatten() {
        let Some(id) = media["id"].as_str() else {
            continue;
        };
        if !needed.contains(id)
            || media["kind"] != "image"
            || !media["path"]
                .as_str()
                .is_some_and(|path| path.to_ascii_lowercase().ends_with(".svg"))
        {
            continue;
        }
        let width = media["probe"]["width"].as_u64().ok_or("遮挡图片宽度缺失")?;
        let height = media["probe"]["height"]
            .as_u64()
            .ok_or("遮挡图片高度缺失")?;
        if width == 0
            || height == 0
            || width > 16384
            || height > 16384
            || width * height > 16_777_216
        {
            return Err("遮挡图片尺寸超出限制".into());
        }
        let html = format!(
            "<img src=\"biliup-media:{id}\" style=\"position:absolute;inset:0;width:100%;height:100%\">"
        );
        let html = embed_images(&html, project, paths, &mut images).await?;
        browser.viewport(width as u32, height as u32).await?;
        browser
            .eval(&format!("BiliupExport.draw({},{{}})", json!(html)))
            .await?;
        let output = directory.join(format!("mask-image-{:016x}.png", rand::random::<u64>()));
        tokio::fs::write(&output, browser.transparent_png().await?)
            .await
            .map_err(|e| e.to_string())?;
        paths.insert(id.to_owned(), output.to_string_lossy().into_owned());
    }
    Ok(())
}

pub(super) async fn prepare(
    pool: &ConnectionPool,
    browser: &mut browser::Browser,
    Preparation {
        document,
        range,
        paths,
        directory,
    }: Preparation<'_>,

    cancel: &CancellationToken,
    mut report: impl FnMut(usize, usize) + Send,
) -> Result<(), String> {
    let windows: Vec<DanmakuWindow> = serde_json::from_value(
        browser
            .eval(&format!(
                "BiliupExport.danmakuRenderWindows({},{range})",
                document.project,
            ))
            .await?,
    )
    .map_err(|e| format!("弹幕源时间计划无效：{e}"))?;
    let windows: BTreeMap<String, DanmakuWindow> = windows
        .into_iter()
        .map(|window| (window.item.clone(), window))
        .collect();
    let media = document.project["media"]
        .as_array()
        .ok_or("缺少素材")?
        .clone();
    let fps = document.project["canvas"]["fps"]
        .as_f64()
        .ok_or("帧率无效")?;
    let tracks = document.project["tracks"]
        .as_array_mut()
        .ok_or("缺少轨道")?;
    let mut processed = Vec::new();
    let mut cache = BTreeMap::<String, String>::new();
    let total = windows.len();
    let mut done = 0;
    for track in tracks {
        if track["hidden"] == true || track["kind"] == "audio" {
            continue;
        }
        for item in track["items"].as_array_mut().ok_or("轨道内容无效")? {
            if item["type"] != "danmaku" || item["danmaku"]["settings"]["enabled"] != true {
                continue;
            }
            let Some(window) = windows.get(item["id"].as_str().ok_or("弹幕图层标识无效")?)
            else {
                continue;
            };
            let recipe = RenderRecipe {
                danmaku: serde_json::from_value(item["danmaku"]["settings"].clone())
                    .map_err(|e| e.to_string())?,
                ..Default::default()
            };
            recipe.validate()?;
            let media_id = item["media"].as_str().ok_or("片段素材无效")?.to_owned();
            let frames = window
                .to
                .checked_sub(window.from)
                .filter(|frames| *frames > 0)
                .ok_or("弹幕源时间范围无效")?;
            let window_ms = (frames as f64 * 1000. / fps).ceil().max(1.) as i64;
            let source = document
                .sources
                .get(&media_id)
                .and_then(|source| source.recording())
                .ok_or("此素材没有录制弹幕")?;
            let duration_ms = document.sources[&media_id]
                .duration_ms()
                .ok_or("录像时长缺失")?;
            let key = format!(
                "{media_id}:{}:{}:{}",
                window.from,
                window.to,
                serde_json::to_string(&recipe.danmaku).map_err(|e| e.to_string())?
            );
            let alias = if let Some(alias) = cache.get(&key) {
                alias.clone()
            } else {
                let row = store::segment(pool, source.segment_id)
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("原始分段不存在")?;
                let original = media
                    .iter()
                    .find(|media| media["id"] == media_id)
                    .ok_or("缺少素材定义")?;
                let alias = format!("comments-{:016x}", rand::random::<u64>());
                let output = directory.join(format!("{alias}.mkv"));
                let origin: Option<i64> =
                    sqlx::query_scalar("SELECT recorded_at_ms FROM segments WHERE id=?")
                        .bind(source.segment_id)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                let started_at: Option<i64> =
                    sqlx::query_scalar("SELECT started_at FROM stream_sessions WHERE id=?")
                        .bind(source.session_id)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                let span = SourceSpan {
                    segment_id: source.segment_id,
                    source_session_id: source.session_id,
                    path: PathBuf::from(paths.get(&media_id).ok_or("素材不可读")?),
                    danmaku_path: row.danmaku_path.map(PathBuf::from),
                    start_ms: source.source_start_ms,
                    end_ms: source.source_start_ms + duration_ms,
                    source_origin_ms: origin
                        .or_else(|| started_at.map(|time| time + source.source_start_ms)),
                    trim_start_ms: 0,
                    trim_end_ms: duration_ms,
                };
                engine::render_danmaku_layer(
                    engine::DanmakuLayer {
                        source: &span,
                        settings: &recipe.danmaku,
                        size: (
                            original["probe"]["width"].as_u64().ok_or("录像宽度缺失")? as u32,
                            original["probe"]["height"].as_u64().ok_or("录像高度缺失")? as u32,
                        ),
                        fps,
                        window: (window.from, window.to),
                        output: &output,
                    },
                    cancel,
                    |ms| {
                        report(
                            done * 1000
                                + (ms.max(0).saturating_mul(1000) / window_ms).min(1000) as usize,
                            total * 1000,
                        )
                    },
                )
                .await?;
                let mut replacement = original.clone();
                replacement["id"] = json!(&alias);
                replacement["probe"]["hasAudio"] = json!(false);
                replacement["probe"]["frames"] = json!(frames);
                replacement["probe"]["fps"] = json!(fps);
                replacement["path"] = json!(format!("{alias}.mkv"));
                processed.push(replacement);
                paths.insert(alias.clone(), output.to_string_lossy().into_owned());
                cache.insert(key, alias.clone());
                alias
            };
            item["media"] = json!(alias);
            item["in"] = json!(window.offset);
            item["loop"] = json!(window.loops);
            done += 1;
            report(done * 1000, total * 1000);
        }
    }
    document.project["media"]
        .as_array_mut()
        .unwrap()
        .extend(processed);
    Ok(())
}
