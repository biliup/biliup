//! Bounded temporal evidence. The reference's pure scan runs on FFmpeg area-scaled pixels.
use super::super::{self as editor, ConnectionPool, Error};
use crate::server::common::processing::engine;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
fn invalid(value: impl Into<String>) -> Error {
    Error::Invalid(value.into())
}
pub struct Evidence {
    pub analysis: Value,
    pub images: Vec<(String, Vec<u8>)>,
}
async fn pixels(
    directory: &Path,
    pictures: &[editor::export::frames::Picture],
    plan: &Value,
    cancel: &CancellationToken,
) -> Result<Value, Error> {
    let _permit = crate::server::common::processing::transcode::acquire(cancel)
        .await
        .map_err(invalid)?;
    let input = directory.join("motion.mjpeg");
    let mut file = tokio::fs::File::create(&input)
        .await
        .map_err(|e| invalid(e.to_string()))?;
    let mut bytes = 0usize;
    for p in pictures {
        bytes += p.jpeg.len();
        if bytes > 64 * 1024 * 1024 {
            return Err(invalid("运动分析图片批次过大"));
        }
        file.write_all(&p.jpeg)
            .await
            .map_err(|e| invalid(e.to_string()))?;
    }
    file.flush().await.map_err(|e| invalid(e.to_string()))?;
    drop(file);
    let mut result = json!({});
    for (name, dimension, format, channels) in [
        ("gray", "size", "gray", 1),
        ("bare", "empty", "gray", 1),
        ("fine", "fine", "gray", 1),
        ("rgb", "size", "rgb24", 3),
    ] {
        let size = &plan[dimension];
        let width = size["w"].as_u64().ok_or_else(|| invalid("扫描宽度无效"))?;
        let height = size["h"].as_u64().ok_or_else(|| invalid("扫描高度无效"))?;
        let expected = width * height * channels * pictures.len() as u64;
        if width == 0 || height == 0 || width > 2048 || height > 2048 || expected > 64 * 1024 * 1024
        {
            return Err(invalid("运动分析尺寸超限"));
        }
        let output = directory.join(format!("{name}.raw"));
        let mut command = crate::tools::ffmpeg_command();
        command
            .args([
                "-y",
                "-v",
                "error",
                "-threads",
                "1",
                "-filter_threads",
                "1",
                "-f",
                "image2pipe",
                "-c:v",
                "mjpeg",
                "-i",
            ])
            .arg(&input)
            .args([
                "-vf",
                &format!("scale={width}:{height}:flags=area,format={format}"),
                "-frames:v",
                &pictures.len().to_string(),
                "-f",
                "rawvideo",
                "-pix_fmt",
                format,
            ])
            .arg(&output);
        engine::run_process(command, cancel, Duration::from_secs(30), |_, _| {})
            .await
            .map_err(invalid)?;
        if tokio::fs::metadata(&output)
            .await
            .map_err(|e| invalid(e.to_string()))?
            .len()
            != expected
        {
            return Err(invalid("运动分析解码输出不完整"));
        }
        result[name] = json!(
            STANDARD.encode(
                tokio::fs::read(output)
                    .await
                    .map_err(|e| invalid(e.to_string()))?
            )
        );
    }
    Ok(result)
}
pub async fn collect(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    document: &editor::Document,
    prepared: &Value,
    session: &mut editor::export::frames::Session,
    cancel: &CancellationToken,
) -> Result<Evidence, Error> {
    let plan = &prepared["temporal"];
    let order: Vec<u64> = serde_json::from_value(plan["order"].clone())?;
    if order.len() > 320 {
        return Err(invalid("运动分析帧数超限"));
    }
    let width = plan["width"]
        .as_u64()
        .ok_or_else(|| invalid("运动扫描宽度无效"))? as u32;
    let directory = tempfile::Builder::new()
        .prefix("editor-review-motion-")
        .tempdir()
        .map_err(|e| invalid(e.to_string()))?;
    let mut analyzer = editor::export::browser::Browser::launch(cancel)
        .await
        .map_err(invalid)?;
    let result = async {
        analyzer
            .eval("BiliupExport.agentMotionBegin()")
            .await
            .map_err(invalid)?;
        let started = Instant::now();
        let budget = Duration::from_secs(40);
        let mut pictures = BTreeMap::new();
        let mut picture_bytes = 0usize;
        for chunk in order.chunks(24) {
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                break;
            }
            let rendered = session
                .render_budget(
                    pool,
                    root,
                    project,
                    editor::export::frames::FrameRequest {
                        document: document.clone(),
                        frames: chunk,
                        width,
                        motion_blur: false,
                        census: false,
                    },
                    Some(remaining.as_millis() as u64),
                )
                .await
                .map_err(invalid)?;
            if rendered.is_empty() {
                break;
            }
            let decoded = pixels(directory.path(), &rendered, plan, cancel).await?;
            let frames: Vec<_> = rendered
                .iter()
                .map(|p| p.evidence["frame"].clone())
                .collect();
            analyzer
                .eval(&format!(
                    "BiliupExport.agentMotionPixels({prepared},{},{decoded})",
                    json!(frames)
                ))
                .await
                .map_err(invalid)?;
            for p in rendered {
                picture_bytes += p.jpeg.len();
                if picture_bytes > 64 * 1024 * 1024 {
                    return Err(invalid("运动扫描图片超过 64 MiB"));
                }
                pictures.insert(p.evidence["frame"].as_u64().unwrap(), p.jpeg);
            }
        }
        for (kind, budget_ms) in [("jumps", 20000u64), ("empty", 12000u64)] {
            let started = Instant::now();
            let mut state = analyzer
                .eval(&format!(
                    "BiliupExport.agentMotionRefineStart({prepared},{},{budget_ms})",
                    json!(kind)
                ))
                .await
                .map_err(invalid)?;
            let mut finished = false;
            for _ in 0..64 {
                if state["done"] == true {
                    finished = true;
                    break;
                }
                if let Some(error) = state["error"].as_str() {
                    return Err(invalid(error));
                }
                let requested: Vec<u64> = serde_json::from_value(state["frames"].clone())?;
                if requested.len() > 1024 {
                    return Err(invalid("运动细化请求超限"));
                }
                for chunk in requested.chunks(24) {
                    let remaining =
                        Duration::from_millis(budget_ms).saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        break;
                    }
                    let rendered = session
                        .render_budget(
                            pool,
                            root,
                            project,
                            editor::export::frames::FrameRequest {
                                document: document.clone(),
                                frames: chunk,
                                width,
                                motion_blur: false,
                                census: false,
                            },
                            Some(remaining.as_millis() as u64),
                        )
                        .await
                        .map_err(invalid)?;
                    if rendered.is_empty() {
                        break;
                    }
                    let decoded = pixels(directory.path(), &rendered, plan, cancel).await?;
                    let frames: Vec<_> = rendered
                        .iter()
                        .map(|p| p.evidence["frame"].clone())
                        .collect();
                    analyzer
                        .eval(&format!(
                            "BiliupExport.agentMotionPixels({prepared},{},{decoded})",
                            json!(frames)
                        ))
                        .await
                        .map_err(invalid)?;
                }
                state = analyzer
                    .eval("BiliupExport.agentMotionRefineNext()")
                    .await
                    .map_err(invalid)?;
            }
            if !finished {
                return Err(invalid("运动细化操作次数超限"));
            }
        }
        let analysis = analyzer
            .eval(&format!("BiliupExport.agentMotionFinish({prepared})"))
            .await
            .map_err(invalid)?;
        let mut images = vec![];
        for strip in analysis["strips"].as_array().unwrap() {
            let frames: Vec<u64> = serde_json::from_value(strip["frames"].clone())?;
            let cells = Value::Array(
                frames
                    .iter()
                    .filter_map(|f| {
                        pictures
                            .get(f)
                            .map(|jpeg| json!({"frame":f,"jpeg":STANDARD.encode(jpeg)}))
                    })
                    .collect(),
            );
            let jpeg = editor::export::frames::sheet(&cells, &plan["fps"], &strip["grid"], cancel)
                .await
                .map_err(invalid)?;
            images.push((
                format!(
                    "frame strip around {}; frames {frames:?} in order",
                    strip["label"]
                ),
                jpeg,
            ));
        }
        for strip in analysis["blur"].as_array().unwrap() {
            let frames: Vec<u64> = serde_json::from_value(strip["frames"].clone())?;
            let rendered = session
                .render(
                    pool,
                    root,
                    project,
                    editor::export::frames::FrameRequest {
                        document: document.clone(),
                        frames: &frames,
                        width: 360,
                        motion_blur: true,
                        census: false,
                    },
                )
                .await
                .map_err(invalid)?;
            let cells = Value::Array(
                rendered
                    .iter()
                    .map(|p| json!({"frame":p.evidence["frame"],"jpeg":STANDARD.encode(&p.jpeg)}))
                    .collect(),
            );
            let jpeg = editor::export::frames::sheet(&cells, &plan["fps"], &strip["grid"], cancel)
                .await
                .map_err(invalid)?;
            images.push((strip["label"].as_str().unwrap().to_owned(), jpeg));
        }
        Ok(Evidence { analysis, images })
    }
    .await;
    analyzer.close().await;
    result
}
