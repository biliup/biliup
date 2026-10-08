//! Mask completed recordings before making them eligible for upload.

use super::config::{MosaicConfig, MosaicRegion};
use super::ffmpeg_filter::build_filter_graph;
use crate::server::config::{Config, ConfigPatch};
use crate::server::core::downloader::SegmentInfo;
use crate::server::errors::{AppError, AppResult};
use crate::server::plugins::plugin_api::{ProcessResult, SegmentProcessorPlugin};
use crate::tools;
use async_trait::async_trait;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{debug, info};

// Encoding must not grow without bound when several rooms close a segment.
// This also prevents two pipelines from modifying the same segment concurrently.
static ENCODING_SLOT: Semaphore = Semaphore::const_new(1);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const PROCESS_TIMEOUT: Duration = Duration::from_secs(3600);
const STDERR_TAIL: usize = 8192;

fn config_value<'a>(
    config: &'a Config,
    override_cfg: &'a Option<ConfigPatch>,
) -> Option<&'a serde_json::Value> {
    match override_cfg
        .as_ref()
        .and_then(|patch| patch.mosaic_config.as_ref())
    {
        Some(value) => value.as_ref(),
        None => config.mosaic_config.as_ref(),
    }
}

/// An explicit malformed setting is protected too: parse/validation failures must
/// be reported by the plugin rather than silently disabling requested masking.
pub fn masking_required(config: &Config, override_cfg: &Option<ConfigPatch>) -> bool {
    config_value(config, override_cfg).is_some_and(|value| {
        value.get("enabled").and_then(serde_json::Value::as_bool) != Some(false)
    })
}

pub fn is_unmasked(path: &Path) -> bool {
    let path = if path.extension().is_some_and(|ext| ext == "part") {
        path.with_file_name(path.file_stem().expect("part has a stem"))
    } else {
        path.to_path_buf()
    };
    path.extension().is_some_and(|ext| ext == "unmasked")
        || path.file_stem().is_some_and(|stem| {
            Path::new(stem)
                .extension()
                .is_some_and(|ext| ext == "unmasked")
        })
}

pub fn masked_path(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    while path
        .extension()
        .is_some_and(|ext| ext == "part" || ext == "unmasked")
    {
        path = path.with_file_name(path.file_stem().expect("marker has a stem"));
    }
    if let (Some(stem), Some(ext)) = (path.file_stem(), path.extension()) {
        let base = Path::new(stem);
        if base.extension().is_some_and(|ext| ext == "unmasked") {
            let mut name = base
                .file_stem()
                .expect("unmasked marker has a stem")
                .to_os_string();
            name.push(".");
            name.push(ext);
            return path.with_file_name(name);
        }
    }
    path
}

/// Mark the real file, not merely its event path. Fail if the quarantine filename
/// already exists so a duplicate/colliding event cannot overwrite preserved footage.
pub fn quarantine(path: &Path) -> std::io::Result<PathBuf> {
    if is_unmasked(path) {
        return Ok(path.to_path_buf());
    }
    let mut name = path.as_os_str().to_os_string();
    name.push(".unmasked");
    let protected = PathBuf::from(name);
    if protected.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "unmasked recording already exists",
        ));
    }
    std::fs::rename(path, &protected)?;
    Ok(protected)
}

pub struct MosaicPlugin;

impl MosaicPlugin {
    pub fn new() -> Self {
        Self
    }

    fn load_config(
        config: &Config,
        override_cfg: &Option<ConfigPatch>,
    ) -> Result<Option<MosaicConfig>, String> {
        let Some(value) = config_value(config, override_cfg) else {
            return Ok(None);
        };
        if value.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
            return Ok(Some(MosaicConfig::default()));
        }
        serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| format!("解析 mosaic_config 失败: {e}"))
    }

    fn dimensions_from_stderr(stderr: &str) -> Option<(u32, u32)> {
        static RESOLUTION: OnceLock<Regex> = OnceLock::new();
        let regex = RESOLUTION
            .get_or_init(|| Regex::new(r"Video:.*?,\s*(\d+)x(\d+)(?:\s|,|\[|$)").unwrap());
        let captures = regex.captures(stderr)?;
        let width = captures[1].parse::<u32>().ok()?;
        let height = captures[2].parse::<u32>().ok()?;
        (width > 0 && height > 0).then_some((width, height))
    }

    async fn detect_video_dimensions(path: &Path) -> AppResult<(u32, u32)> {
        // Reuse the configured/bundled ffmpeg binary rather than requiring a second
        // ffprobe install. Zero output frames probes headers without decoding a whole
        // recording (the old code decoded the complete file just to find its size).
        let mut cmd = tools::ffmpeg_command();
        cmd.args(["-nostdin", "-hide_banner", "-i"])
            .arg(path)
            .args(["-map", "0:v:0", "-frames:v", "0", "-an", "-f", "null", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let output = tokio::time::timeout(PROBE_TIMEOUT, cmd.output())
            .await
            .map_err(|_| AppError::Custom("检测视频分辨率超时".into()))?
            .map_err(|e| AppError::Custom(format!("执行 FFmpeg 探测失败: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Custom("FFmpeg 无法读取录像的视频流".into()).into());
        }
        Self::dimensions_from_stderr(&String::from_utf8_lossy(&output.stderr))
            .ok_or_else(|| AppError::Custom("无法检测视频分辨率".into()).into())
    }

    fn output_format(path: &Path) -> AppResult<&'static str> {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("flv") => Ok("flv"),
            Some("ts") => Ok("mpegts"),
            Some("mp4" | "m4s") => Ok("mp4"),
            Some("mkv") => Ok("matroska"),
            Some("mov") => Ok("mov"),
            Some("avi") => Ok("avi"),
            _ => Err(AppError::Custom("画面遮挡不支持这个录像容器".into()).into()),
        }
    }

    fn build_ffmpeg_command(input: &Path, output: &Path, filter: &str, format: &str) -> Command {
        let mut cmd = tools::low_priority_ffmpeg_command();
        cmd.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
            .arg(input)
            .args([
                "-filter_complex_threads",
                "1",
                "-filter_complex",
                filter,
                "-map",
                "[masked]",
                "-map",
                "0:a?",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "20",
                "-c:a",
                "copy",
            ]);
        // Fragmented MP4 is understood by the existing workbench indexer.
        if matches!(format, "mp4" | "mov") {
            cmd.args(["-movflags", "+frag_keyframe+empty_moov+default_base_moof"]);
        }
        cmd.args(["-f", format, "-y"])
            .arg(output)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd
    }

    async fn process_file(&self, input: &Path, regions: &[MosaicRegion]) -> AppResult<PathBuf> {
        let _slot = ENCODING_SLOT
            .acquire()
            .await
            .map_err(|_| AppError::Custom("画面遮挡处理队列已关闭".into()))?;
        let destination = masked_path(input);
        // Files from new downloads are already quarantined. Protect legacy/HA
        // retry inputs too, so an encoding error never leaves an uploadable raw file.
        let input =
            quarantine(input).map_err(|e| AppError::Custom(format!("无法隔离未遮挡录像: {e}")))?;
        let format = Self::output_format(&destination)?;
        let (width, height) = Self::detect_video_dimensions(&input).await?;
        let pixels: Vec<_> = regions.iter().map(|r| r.to_pixel(width, height)).collect();
        let filter = build_filter_graph(&pixels, width, height);
        debug!(width, height, filter, "生成画面遮挡滤镜");
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let temp = tempfile::Builder::new()
            .prefix(".biliup-mosaic-")
            .suffix(".tmp")
            .tempfile_in(parent)
            .map_err(|e| AppError::Custom(format!("创建遮挡临时文件失败: {e}")))?;
        let mut child = Self::build_ffmpeg_command(&input, temp.path(), &filter, format)
            .spawn()
            .map_err(|e| AppError::Custom(format!("启动 FFmpeg 失败: {e}")))?;
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let capture = async move {
            let mut tail = Vec::new();
            let mut buffer = [0u8; 4096];
            while let Ok(len) = stderr.read(&mut buffer).await {
                if len == 0 {
                    break;
                }
                tail.extend_from_slice(&buffer[..len]);
                if tail.len() > STDERR_TAIL {
                    tail.drain(..tail.len() - STDERR_TAIL);
                }
            }
            String::from_utf8_lossy(&tail).into_owned()
        };
        let result = tokio::time::timeout(PROCESS_TIMEOUT, async {
            tokio::join!(child.wait(), capture)
        })
        .await;
        let (status, stderr) = match result {
            Ok((status, stderr)) => (
                status.map_err(|e| AppError::Custom(format!("FFmpeg 进程错误: {e}")))?,
                stderr,
            ),
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(AppError::Custom("FFmpeg 画面遮挡处理超时".into()).into());
            }
        };
        if !status.success() {
            return Err(AppError::Custom(format!(
                "FFmpeg 画面遮挡失败 ({:?}): {stderr}",
                status.code()
            ))
            .into());
        }
        if temp.as_file().metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Err(AppError::Custom("FFmpeg 没有生成遮挡后的视频".into()).into());
        }
        // Publishing is atomic and occurs only after a successful encode. Keep the
        // quarantined original until the destination is durable and ready.
        temp.persist_noclobber(&destination)
            .map_err(|e| AppError::Custom(format!("保存遮挡后的视频失败: {e}")))?;
        // An old byte-offset index must never be reused for newly encoded media.
        crate::server::workbench::index::remove(&destination);
        crate::server::workbench::index::remove(&input);
        // The upload pipeline removes the raw source only after persisting the
        // new workbench/filelist paths. A database error must retain both files.
        info!(path = %destination.display(), regions = regions.len(), "画面遮挡处理完成");
        Ok(destination)
    }
}

impl Default for MosaicPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SegmentProcessorPlugin for MosaicPlugin {
    fn name(&self) -> &'static str {
        "mosaic"
    }

    async fn is_enabled(
        &self,
        _streamer_id: i64,
        config: &Config,
        override_cfg: &Option<ConfigPatch>,
    ) -> bool {
        masking_required(config, override_cfg)
    }

    async fn process_segment(
        &self,
        segment: &SegmentInfo,
        config: &Config,
        override_cfg: &Option<ConfigPatch>,
    ) -> AppResult<ProcessResult> {
        let mosaic = match Self::load_config(config, override_cfg) {
            Ok(Some(mosaic)) if mosaic.enabled => mosaic,
            Ok(_) if is_unmasked(&segment.prev_file_path) => {
                return Ok(ProcessResult::failed(
                    "未遮挡录像需要有效的画面遮挡配置，已阻止上传",
                ));
            }
            Ok(_) => return Ok(ProcessResult::skipped()),
            Err(error) => return Ok(ProcessResult::failed(error)),
        };
        if let Err(error) = mosaic.validate() {
            return Ok(ProcessResult::failed(error));
        }
        match self
            .process_file(&segment.prev_file_path, &mosaic.regions)
            .await
        {
            Ok(path) => Ok(ProcessResult::completed_with_path(path)),
            Err(error) => Ok(ProcessResult::failed(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn probes_real_resolution_without_matching_codec_or_pixel_format() {
        let stderr =
            "Stream #0:0: Video: h264 (High), yuv420p(tv), 1920x1080 [SAR 1:1 DAR 16:9], 30 fps";
        assert_eq!(
            MosaicPlugin::dimensions_from_stderr(stderr),
            Some((1920, 1080))
        );
        assert_eq!(
            MosaicPlugin::dimensions_from_stderr("Video: h264, yuv420p, 0x0, 30 fps"),
            None
        );
    }

    #[test]
    fn config_scope_and_malformed_enabled_settings_fail_closed() {
        let mut config = Config::default();
        config.mosaic_config = Some(json!({"enabled": true, "regions": []}));
        assert!(masking_required(&config, &None));
        assert!(
            MosaicPlugin::load_config(&config, &None)
                .unwrap()
                .unwrap()
                .validate()
                .is_err()
        );
        config.mosaic_config = Some(json!({"enabled": "true"}));
        assert!(masking_required(&config, &None));
        assert!(MosaicPlugin::load_config(&config, &None).is_err());
        let patch: ConfigPatch =
            serde_json::from_value(json!({"mosaic_config": {"enabled": false, "regions": []}}))
                .unwrap();
        assert!(!masking_required(&config, &Some(patch)));
    }

    #[test]
    fn quarantine_renames_the_file_and_never_overwrites_a_previous_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("name.with.dots.flv");
        std::fs::write(&path, b"raw").unwrap();
        let protected = quarantine(&path).unwrap();
        assert_eq!(protected, dir.path().join("name.with.dots.flv.unmasked"));
        assert!(!path.exists());
        assert_eq!(masked_path(&protected), path);
        std::fs::write(&path, b"another").unwrap();
        assert!(quarantine(&path).is_err());
        assert_eq!(std::fs::read(protected).unwrap(), b"raw");
        let marked = dir.path().join("a.unmasked.flv.part");
        assert!(is_unmasked(&marked));
        assert_eq!(masked_path(&marked), dir.path().join("a.flv"));
    }

    #[tokio::test]
    async fn ffmpeg_masks_mixed_regions_and_preserves_raw_on_failure() {
        if tools::ffmpeg_command()
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_err()
        {
            tools::note_skipped_test("FFmpeg unavailable");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.flv");
        let status = tools::ffmpeg_command()
            .args([
                "-nostdin",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=white:s=64x64:d=0.2",
                "-c:v",
                "libx264",
                "-f",
                "flv",
                "-y",
            ])
            .arg(&input)
            .status()
            .await
            .unwrap();
        if !status.success() {
            tools::note_skipped_test("FFmpeg libx264 encoder unavailable");
            return;
        }
        let region = |effect_type| MosaicRegion {
            id: "test".into(),
            x: 11.0 / 64.0,
            y: 13.0 / 64.0,
            width: 21.0 / 64.0,
            height: 15.0 / 64.0,
            effect_type,
            strength: 64,
            color: Some("#000000".into()),
        };
        let result = MosaicPlugin::new()
            .process_file(
                &input,
                &[
                    region(super::super::EffectType::Mosaic),
                    region(super::super::EffectType::Blur),
                    region(super::super::EffectType::Solid),
                ],
            )
            .await
            .unwrap();
        assert_eq!(result, input);
        let output = tools::ffmpeg_command()
            .args(["-nostdin", "-loglevel", "error", "-i"])
            .arg(&input)
            .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let pixel = |x: usize, y: usize| &output.stdout[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3];
        assert!(
            pixel(12, 14).iter().all(|c| *c < 30),
            "solid region must cover the mixed-effect graph"
        );
        assert!(
            pixel(40, 40).iter().all(|c| *c > 220),
            "outside the region should remain white"
        );
        assert!(dir.path().join("input.flv.unmasked").exists());

        let broken = dir.path().join("broken.flv");
        std::fs::write(&broken, b"not a video").unwrap();
        assert!(
            MosaicPlugin::new()
                .process_file(&broken, &[region(super::super::EffectType::Mosaic)])
                .await
                .is_err()
        );
        assert!(!broken.exists());
        assert_eq!(
            std::fs::read(dir.path().join("broken.flv.unmasked")).unwrap(),
            b"not a video"
        );
        assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".biliup-mosaic-")
        }));
    }
}
