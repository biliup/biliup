//! Project graphics use the same sandbox and render(t, info) clock as preview.
pub(super) struct Scene<'a> {
    pub(super) graphics: &'a [Graphic],
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) project: &'a Value,
    pub(super) paths: &'a BTreeMap<String, String>,
}

use super::{browser::Browser, embed_images};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Instant};
use tokio_util::sync::CancellationToken;
#[derive(Deserialize)]
pub(super) struct Graphic {
    index: usize,
    doc: String,
    frames: Vec<f64>,
    samples: Option<Vec<Vec<f64>>>,
    info: Value,
}
impl Graphic {
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
pub(super) async fn render(
    browser: &mut Browser,
    Scene {
        graphics,
        width,
        height,
        project,
        paths,
    }: Scene<'_>,
    root: &Path,
    cancel: &CancellationToken,
    mut report: impl FnMut(),
) -> Result<BTreeMap<usize, Value>, String> {
    let mut outputs = BTreeMap::new();
    let mut images = BTreeMap::new();
    let mut disk_bytes = 0u64;
    browser.media(project, paths);
    for graphic in graphics {
        let doc = embed_images(&graphic.doc, project, paths, &mut images).await?;
        browser
            .eval(&format!(
                "BiliupExport.loadGraphicDocument({},{width},{height})",
                json!(doc)
            ))
            .await?;
        browser
            .eval(&format!("BiliupExport.resetGraphicBlur({width},{height})"))
            .await?;
        let fps = graphic.info["fps"]
            .as_f64()
            .filter(|v| v.is_finite() && *v > 0.)
            .ok_or("图形帧率无效")?;
        for (index, frame) in graphic.frames.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err("导出已取消".into());
            }
            let mut info = graphic.info.clone();
            info["frame"] = json!(frame);
            let initial = graphic
                .samples
                .as_ref()
                .and_then(|s| s.get(index))
                .cloned()
                .unwrap_or_else(|| vec![frame / fps]);
            if initial.is_empty() || initial.len() > 64 {
                return Err("图形采样参数无效".into());
            }
            if initial.len() == 1 {
                browser.eval("BiliupExport.showGraphicSource()").await?;
                browser
                    .eval(&format!("BiliupExport.drawGraphic({},{info})", initial[0]))
                    .await?;
                let png = browser.png().await?;
                disk_bytes += png.len() as u64;
                if disk_bytes > 16 * 1024 * 1024 * 1024 {
                    return Err("动态图形临时帧超过 16 GiB".into());
                }
                tokio::fs::write(
                    root.join(format!("graphic-{}-{index:08}.png", graphic.index)),
                    png,
                )
                .await
                .map_err(|e| e.to_string())?;
                report();
                continue;
            }
            let started = Instant::now();
            let mut times = initial.clone();
            let mut per_sub = 0.;
            loop {
                let pass = async {
                    browser
                        .eval(
                            "BiliupExport.beginGraphicSamples(); BiliupExport.showGraphicSource()",
                        )
                        .await?;
                    for time in &times {
                        if cancel.is_cancelled() {
                            return Err("导出已取消".to_string());
                        }
                        browser
                            .eval(&format!("BiliupExport.drawGraphic({time},{info})"))
                            .await?;
                        let png = browser.png().await?;
                        browser
                            .eval(&format!(
                                "BiliupExport.addGraphicSample({})",
                                json!(STANDARD.encode(png))
                            ))
                            .await?;
                    }
                    browser.eval("BiliupExport.finishGraphicSamples()").await?;
                    Ok::<(), String>(())
                }
                .await;
                if let Err(error) = pass {
                    if cancel.is_cancelled() || times == initial {
                        return Err(format!("图形 {} 第 {index} 帧：{error}", graphic.index));
                    }
                    tracing::warn!("图形运动模糊保留上一采样级别：{error}");
                    break;
                }
                if times == initial {
                    per_sub = started.elapsed().as_secs_f64() / times.len() as f64
                }
                let next: Vec<f64> = serde_json::from_value(
                    browser
                        .eval(&format!(
                            "BiliupExport.nextGraphicSamples({})",
                            json!(times)
                        ))
                        .await?,
                )
                .map_err(|e| e.to_string())?;
                if next.is_empty()
                    || started.elapsed().as_secs_f64() + per_sub * next.len() as f64 > 12.
                {
                    break;
                }
                times = next;
            }
            browser.eval("BiliupExport.showGraphicMean()").await?;
            let png = browser.png().await?;
            disk_bytes += png.len() as u64;
            if disk_bytes > 16 * 1024 * 1024 * 1024 {
                return Err("动态图形临时帧超过 16 GiB".into());
            }
            tokio::fs::write(
                root.join(format!("graphic-{}-{index:08}.png", graphic.index)),
                png,
            )
            .await
            .map_err(|e| e.to_string())?;
            report();
        }
        outputs.insert(
            graphic.index,
            json!({"kind":"sequence","pattern":format!("graphic-{}-%08d.png",graphic.index)}),
        );
    }
    Ok(outputs)
}
