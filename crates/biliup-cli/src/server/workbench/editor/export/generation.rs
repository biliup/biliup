//! Uncut's markup renderer using the existing isolated Chromium/FFmpeg worker.
use super::browser::Browser;
use crate::server::common::processing::{engine, transcode};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
pub const MAX_MARKUP: usize = 512 * 1024;
pub const SIZES: &[&str] = &["canvas", "1920x1080", "1080x1920", "1080x1350", "1024x1024"];
pub const SYSTEM: &str = "Write exactly one self-contained SVG or HTML graphic, with markup only, no explanation or code fences. No external links, fonts, images, scripts or other network resources. Images may use data URLs. Use system fonts. A still is an SVG with xmlns=\"http://www.w3.org/2000/svg\" and a viewBox of the requested size, or a self-contained HTML page. An animation is an SVG using CSS/SMIL or an HTML page defining window.render=(t,info)=>{...}. Compute each frame from t alone, in seconds, including out-of-order seeks. Do not use timers or transitions. Fill the animation frame with a background colour. Do not use external tools or ask the user questions.";
pub fn extract(answer: &str) -> Result<String, String> {
    let fenced = regex::Regex::new(r"(?is)```(?:svg|html|xml)?\s*\n(.*?)```").unwrap();
    let content = fenced
        .captures(answer)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or(answer)
        .trim();
    let html = regex::Regex::new(r"(?is)(?:<!doctype html[^>]*>\s*)?<html\b.*</html>").unwrap();
    let svg = regex::Regex::new(r"(?is)<svg\b.*</svg>").unwrap();
    let content = html
        .find(content)
        .or_else(|| svg.find(content))
        .map(|m| m.as_str())
        .unwrap_or(content);
    if content.len() > MAX_MARKUP {
        return Err("图形内容不能超过 512 KiB".into());
    }
    if !regex::Regex::new(
        r"(?is)^\s*<(?:!doctype html|html|head|body|div|canvas|svg|style|script|main|section)\b",
    )
    .unwrap()
    .is_match(content)
    {
        return Err("模型没有返回有效的 SVG 或 HTML".into());
    }
    Ok(content.to_string())
}
pub fn pixels(size: &str, canvas: &serde_json::Value) -> Result<(u32, u32), String> {
    let (w, h) = if size == "canvas" {
        (
            canvas["width"].as_u64().unwrap_or(0),
            canvas["height"].as_u64().unwrap_or(0),
        )
    } else {
        let (w, h) = size.split_once('x').ok_or("图形尺寸无效")?;
        (
            w.parse::<u64>().map_err(|_| "图形尺寸无效")?,
            h.parse::<u64>().map_err(|_| "图形尺寸无效")?,
        )
    };
    if !(16..=4096).contains(&w) || !(16..=4096).contains(&h) {
        return Err("图形尺寸必须在 16 到 4096 像素之间".into());
    }
    Ok((w as u32, h as u32))
}
pub async fn render(
    markup: &str,
    video: bool,
    (width, height): (u32, u32),
    seconds: f64,
    root: &Path,
    cancel: &CancellationToken,
    mut status: impl FnMut(String),
) -> Result<(tempfile::TempDir, PathBuf), String> {
    let directory = root.join("editor-generation");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| e.to_string())?;
    let temp = tempfile::Builder::new()
        .prefix("graphic-")
        .tempdir_in(directory)
        .map_err(|e| e.to_string())?;
    if !video && markup.trim_start().starts_with("<svg") {
        let mut markup = markup.to_string();
        let opening = markup.split('>').next().unwrap_or("");
        if !regex::Regex::new(r"(?i)\b(?:viewBox|width)\s*=")
            .unwrap()
            .is_match(opening)
        {
            markup = markup.replacen(
                "<svg",
                &format!("<svg width=\"{width}\" height=\"{height}\""),
                1,
            )
        }
        super::super::svg::size(&markup)?;
        let path = temp.path().join("image.svg");
        tokio::fs::write(&path, markup)
            .await
            .map_err(|e| e.to_string())?;
        return Ok((temp, path));
    }
    let _permit = transcode::acquire(cancel).await?;
    let mut browser = Browser::launch(cancel).await?;
    let result = async {
        browser.viewport(width, height).await?;
        browser
            .eval(&format!(
                "BiliupExport.loadGenerated({},{width},{height})",
                json!(markup)
            ))
            .await?;
        let frames = if video {
            (seconds * 30.).round().clamp(1., 360.) as u32
        } else {
            1
        };
        let mut written = 0usize;
        for frame in 0..frames {
            if cancel.is_cancelled() {
                return Err("已取消".into());
            }
            if frame % 15 == 0 {
                status(format!("正在绘制 {}/{} 帧", frame + 1, frames))
            }
            browser
                .eval(&format!("BiliupExport.drawGenerated({frame},{frames},30)"))
                .await?;
            let bytes = browser.png().await?;
            written = written.saturating_add(bytes.len());
            if written > 2 * 1024 * 1024 * 1024usize {
                return Err("动画中间帧超过 2 GiB 限制".into());
            }
            tokio::fs::write(temp.path().join(format!("frame-{frame:06}.png")), bytes)
                .await
                .map_err(|e| e.to_string())?;
        }
        if !video {
            return Ok(temp.path().join("frame-000000.png"));
        }
        status("正在编码动画".into());
        let path = temp.path().join("animation.mp4");
        let mut command = crate::tools::ffmpeg_command();
        command
            .args([
                "-nostdin",
                "-hide_banner",
                "-y",
                "-protocol_whitelist",
                "file,pipe",
                "-framerate",
                "30",
                "-i",
            ])
            .arg(temp.path().join("frame-%06d.png"))
            .args([
                "-vf",
                &format!(
                    "scale={}:{}:flags=lanczos,format=yuv420p",
                    width.div_ceil(2) * 2,
                    height.div_ceil(2) * 2
                ),
                "-c:v",
                "libx264",
                "-crf",
                "18",
                "-movflags",
                "+faststart",
            ])
            .arg(&path);
        engine::run_process(command, cancel, Duration::from_secs(240), |_, _| {}).await?;
        Ok(path)
    }
    .await;
    browser.close().await;
    result.map(|path| (temp, path))
}
