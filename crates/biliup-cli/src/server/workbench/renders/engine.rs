//! Immutable source recordings -> one encode per segment -> a copy-only MP4 join.
use super::encoding::{self, EncoderCapability, SelectedEncoder};
use super::model::{EffectType, MaskRegion, RenderRecipe, RenderSource};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

const PROCESS_STALL: Duration = Duration::from_secs(180);
const MAX_DIAGNOSTICS: usize = 16 * 1024;

pub type SourceSpan = RenderSource;

#[derive(Debug, Clone)]
pub struct EngineRequest {
    pub sources: Vec<SourceSpan>,
    pub settings: RenderRecipe,
    /// Keys are decimal asset ids; paths are resolved by the asset store.
    pub assets: HashMap<String, PathBuf>,
    pub output_path: PathBuf,
    /// Frozen font asset belonging to this task, when supplied by the queue.
    pub font_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct EngineOutput {
    pub output_path: PathBuf,
    pub output_bytes: i64,
    pub duration_ms: i64,
    pub encoder: Option<String>,
    pub encoder_device: Option<String>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineProgress {
    pub phase: String,
    pub ratio: Option<f64>,
    pub encoder: Option<String>,
    pub encoder_device: Option<String>,
    pub fallback_reason: Option<String>,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub processed_ms: Option<i64>,
    pub total_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssPreview {
    pub ass: String,
    pub width: u32,
    pub height: u32,
    /// ASS clock = segment-local clock - this origin; negative origins preserve
    /// already-scrolling comments at the first recorded frame.
    pub time_origin_ms: i64,
    pub estimated_timing: bool,
    pub comment_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderCapabilities {
    pub available: bool,
    pub ffmpeg: bool,
    pub libass: bool,
    pub libx264: bool,
    /// The built-in DanmakuFactory converter ran in a worker process. The
    /// name is kept for API compatibility; no external program is involved.
    pub danmaku_factory: bool,
    pub font: bool,
    pub encoders: Vec<EncoderCapability>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct FfmpegProgress {
    processed_ms: Option<i64>,
    frames: Option<i64>,
    fps: Option<f64>,
    speed: Option<f64>,
}

impl FfmpegProgress {
    fn parse(block: &str) -> Self {
        let mut stats = Self::default();
        for line in block.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let number = || {
                value
                    .trim()
                    .trim_end_matches('x')
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite() && *n >= 0.)
            };
            match key {
                "frame" => stats.frames = value.trim().parse::<i64>().ok().filter(|n| *n >= 0),
                "out_time_us" => stats.processed_ms = value.parse::<i64>().ok().map(|n| n / 1000),
                // Despite its historic name, FFmpeg reports microseconds here.
                "out_time_ms" if stats.processed_ms.is_none() => {
                    stats.processed_ms = value.parse::<i64>().ok().map(|n| n / 1000)
                }
                "fps" => stats.fps = number(),
                "speed" => stats.speed = number(),
                _ => {}
            }
        }
        stats
    }

    fn encoded_time(&self, frame_rate: &str) -> i64 {
        // With -copyts FFmpeg can report out_time=0 for a complete short
        // encode. All segments are CFR, so output frame count also measures
        // completed media time and avoids a stalled zero-percent display.
        let frame_time = self
            .frames
            .zip(frame_rate.split_once('/'))
            .and_then(|(frames, (num, den))| {
                let num = num.parse::<f64>().ok()?;
                let den = den.parse::<f64>().ok()?;
                (num > 0. && den > 0.).then_some((frames as f64 * 1000. * den / num).round() as i64)
            })
            .unwrap_or(0);
        self.processed_ms.unwrap_or(0).max(frame_time)
    }

    fn with_elapsed(mut self, frame_rate: &str, elapsed: Duration) -> Self {
        let seconds = elapsed.as_secs_f64();
        if seconds > 0. {
            if self.fps.is_none_or(|fps| fps == 0.) {
                self.fps = self.frames.map(|frames| frames as f64 / seconds);
            }
            if self.speed.is_none_or(|speed| speed == 0.) {
                self.speed = Some(self.encoded_time(frame_rate).max(0) as f64 / 1000. / seconds);
            }
        }
        self
    }
}

fn engine_progress(
    selected: Option<&SelectedEncoder>,
    phase: String,
    ratio: Option<f64>,
    processed_ms: i64,
    total_ms: i64,
    stats: FfmpegProgress,
) -> EngineProgress {
    EngineProgress {
        phase,
        ratio,
        encoder: selected.map(|s| s.encoder.name().to_owned()),
        encoder_device: selected.and_then(|s| s.device.clone()),
        fallback_reason: selected.and_then(|s| s.fallback_reason.clone()),
        fps: stats.fps,
        speed: stats.speed,
        processed_ms: Some(processed_ms.clamp(0, total_ms)),
        total_ms: Some(total_ms),
    }
}

fn record_filter_fallback(selected: &mut SelectedEncoder, error: &str) {
    let detail: String = error
        .trim()
        .lines()
        .last()
        .unwrap_or(error)
        .chars()
        .take(400)
        .collect();
    let reason = format!("QSV 处理图失败，已回退软件滤镜：{detail}");
    selected.fallback_reason = Some(match selected.fallback_reason.take() {
        Some(previous) => format!("{previous}；{reason}"),
        None => reason,
    });
}

#[derive(Debug, Clone)]
struct MediaInfo {
    width: u32,
    height: u32,
    has_audio: bool,
    duration_ms: Option<i64>,
    video_origin_seconds: f64,
    frame_rate: String,
}

/// Child stdout progress and stderr are drained concurrently and bounded. The
/// watchdog accepts advancing media time or output frames, so an hours-long
/// encode is fine while a hung child is stopped. Cancellation kills and reaps.
pub(super) async fn run_process(
    mut command: Command,
    cancel: &CancellationToken,
    stall: Duration,
    mut report: impl FnMut(i64, &str),
) -> Result<String, String> {
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("启动处理程序失败：{e}"))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let errors = tokio::spawn(async move {
        let mut stderr = stderr;
        let mut tail = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buffer).await {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buffer[..n]);
            if tail.len() > MAX_DIAGNOSTICS {
                tail.drain(..tail.len() - MAX_DIAGNOSTICS);
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let mut lines = BufReader::new(stdout).lines();
    let mut latest = -1i64;
    let mut latest_frame = -1i64;
    let mut stdout_open = true;
    let mut block = String::new();
    let mut exit_status = None;
    let watch = tokio::time::sleep(stall);
    tokio::pin!(watch);
    let outcome = loop {
        // Drain the final progress=end block even if wait() becomes ready
        // before the pipe. Otherwise fast encodes can lose all telemetry.
        if !stdout_open {
            if let Some(status) = exit_status.take() {
                break status;
            }
        }
        tokio::select! {
            _ = cancel.cancelled() => { break Err("合成已取消".to_string()); }
            _ = &mut watch => { break Err(format!("处理程序连续 {} 秒没有推进，任务已停止；可重试", stall.as_secs())); }
            line = lines.next_line(), if stdout_open => {
                match line {
                    Ok(Some(line)) => {
                        let at = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms="))
                            .and_then(|v| v.parse::<i64>().ok()).map(|v| v / 1000);
                        if let Some(at) = at {
                            if at > latest {
                                latest = at;
                                watch.as_mut().reset(tokio::time::Instant::now() + stall);
                            }
                        }
                        let frame = line.strip_prefix("frame=").and_then(|v| v.trim().parse::<i64>().ok());
                        if let Some(frame) = frame.filter(|frame| *frame > latest_frame) {
                            latest_frame = frame;
                            watch.as_mut().reset(tokio::time::Instant::now() + stall);
                        }
                        // Only store the small, documented FFmpeg progress
                        // fields; other commands may print arbitrary stdout.
                        if line.starts_with("frame=") || line.starts_with("out_time_us=") || line.starts_with("out_time_ms=")
                            || line.starts_with("fps=") || line.starts_with("speed=") {
                            if block.len() + line.len() > MAX_DIAGNOSTICS { block.clear(); }
                            if line.len() > MAX_DIAGNOSTICS { continue; }
                            block.push_str(&line);
                            block.push('\n');
                        }
                        if line.starts_with("progress=") {
                            let stats = FfmpegProgress::parse(&block);
                            if stats.processed_ms.is_some() || stats.frames.is_some() { report(stats.processed_ms.unwrap_or(0), &block); }
                            block.clear();
                        }
                    }
                    _ => stdout_open = false,
                }
            }
            status = child.wait(), if exit_status.is_none() => {
                exit_status = Some(status.map_err(|e| format!("等待处理程序失败：{e}")).and_then(|status| {
                    if status.success() { Ok(()) } else { Err(format!("处理程序失败（{status}）")) }
                }));
            }
        }
    };
    if outcome.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    let stderr = errors.await.unwrap_or_default();
    outcome.map_err(|error| {
        if stderr.contains("No space left on device") {
            "磁盘空间不足，合成失败".into()
        } else if cancel.is_cancelled() {
            "合成已取消".into()
        } else {
            format!("{error}：{}", stderr.trim())
        }
    })?;
    Ok(stderr)
}

async fn probe(path: &Path, cancel: &CancellationToken) -> Result<MediaInfo, String> {
    let mut command = crate::tools::ffmpeg_command();
    command
        .args(["-nostdin", "-hide_banner", "-copyts", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-vf",
            "showinfo",
            "-frames:v",
            "1",
            "-fps_mode",
            "passthrough",
            "-an",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null());
    let text = run_process(command, cancel, Duration::from_secs(30), |_, _| {}).await?;
    let dimensions = regex::Regex::new(r"Video:.*?,\s*(\d+)x(\d+)(?:\s|,|\[|$)").unwrap();
    let captures = dimensions.captures(&text).ok_or("无法读取录像分辨率")?;
    let width = captures[1].parse::<u32>().map_err(|_| "录像宽度无效")?;
    let height = captures[2].parse::<u32>().map_err(|_| "录像高度无效")?;
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err("录像尺寸超出处理范围".into());
    }
    // Match source streams only (not the null output descriptions).
    let input = text.split("Output #").next().unwrap_or(&text);
    let duration = regex::Regex::new(r"Duration: (\d+):(\d+):(\d+(?:\.\d+)?)").unwrap();
    let duration_ms = duration.captures(input).and_then(|captures| {
        let h = captures[1].parse::<f64>().ok()?;
        let m = captures[2].parse::<f64>().ok()?;
        let s = captures[3].parse::<f64>().ok()?;
        Some(((h * 3600. + m * 60. + s) * 1000.).round() as i64)
    });
    let video_pts =
        regex::Regex::new(r"showinfo[^\n]*\bn:\s*0\s+pts:\s*-?\d+\s+pts_time:([-+0-9.eE]+)")
            .unwrap();
    let video_origin_seconds = video_pts
        .captures(&text)
        .and_then(|c| c[1].parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .ok_or("无法读取录像首帧时间戳")?;
    let frame_rate_pattern = regex::Regex::new(r"showinfo[^\n]*frame_rate:\s*(\d+)/(\d+)").unwrap();
    let frame_rate = frame_rate_pattern
        .captures(&text)
        .filter(|c| {
            c[1].parse::<f64>()
                .ok()
                .zip(c[2].parse::<f64>().ok())
                .is_some_and(|(num, den)| den > 0. && (1.0..=240.0).contains(&(num / den)))
        })
        .map(|c| format!("{}/{}", &c[1], &c[2]))
        .unwrap_or_else(|| "30/1".into());
    Ok(MediaInfo {
        width,
        height,
        has_audio: input.contains("Audio:"),
        duration_ms,
        video_origin_seconds,
        frame_rate,
    })
}

/// Tool support is probed on demand before queueing an export. The danmaku
/// converter is checked by converting a one-line XML in a real worker
/// process, which also proves this host can start the worker.
pub async fn capabilities() -> RenderCapabilities {
    let cancel = CancellationToken::new();
    let converter = super::danmaku::check_converter(&cancel).await;
    let encoders = encoding::capabilities(&cancel).await;
    let mut status = RenderCapabilities {
        available: false,
        ffmpeg: encoders.iter().any(|encoder| encoder.available),
        libass: false,
        libx264: encoders
            .iter()
            .any(|encoder| encoder.name == "libx264" && encoder.available),
        danmaku_factory: converter.is_ok(),
        font: crate::tools::render_font().is_some(),
        encoders,
        error: None,
    };
    if let Err(error) = converter {
        tracing::warn!(%error, "built-in danmaku converter check failed");
        status.error = Some(format!("内置弹幕转换不可用：{error}"));
    }
    if !status.ffmpeg {
        status
            .error
            .get_or_insert_with(|| "没有可用的 H.264 编码器，请检查 FFmpeg 与显卡驱动".into());
        return status;
    }
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(e) => {
            status.error.get_or_insert(e.to_string());
            return status;
        }
    };
    let ass = temp.path().join("check.ass");
    if let Err(e) = tokio::fs::write(&ass, super::danmaku::empty_ass(16, 16)).await {
        status.error.get_or_insert(e.to_string());
        return status;
    }
    let mut command = crate::tools::ffmpeg_command();
    command
        .current_dir(temp.path())
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=s=16x16:d=0.04",
        ])
        .args(["-vf", "ass=check.ass", "-frames:v", "1", "-f", "null", "-"]);
    match run_process(command, &cancel, Duration::from_secs(15), |_, _| {}).await {
        Ok(_) => status.libass = true,
        Err(error) => {
            status.error.get_or_insert(error);
        }
    }
    status.available = status.ffmpeg && status.libass && status.danmaku_factory && status.font;
    if status.error.is_none() && !status.available {
        status.error = Some("未找到弹幕字体，请安装随包字体或设置 BILIUP_RENDER_TOOLS_DIR".into());
    }
    status
}

pub async fn preview(
    source: &SourceSpan,
    settings: &RenderRecipe,
    cancel: &CancellationToken,
) -> Result<AssPreview, String> {
    preview_on_canvas(source, settings, cancel, None).await
}

pub async fn preview_on_canvas(
    source: &SourceSpan,
    settings: &RenderRecipe,
    cancel: &CancellationToken,
    canvas_path: Option<&Path>,
) -> Result<AssPreview, String> {
    settings.validate()?;
    let media = probe(canvas_path.unwrap_or(&source.path), cancel).await?;
    let width = media.width.div_ceil(2) * 2;
    let height = media.height.div_ceil(2) * 2;
    let temp = tempfile::tempdir().map_err(|e| format!("无法创建预览目录：{e}"))?;
    super::danmaku::generate(
        source,
        &settings.danmaku,
        width,
        height,
        temp.path(),
        cancel,
    )
    .await
}

fn secs(ms: i64) -> String {
    format!("{:.6}", ms as f64 / 1000.)
}

fn enable(region: &MaskRegion, source: &SourceSpan, origin: i64) -> Option<String> {
    if region.intervals.is_empty() {
        return Some("1".into());
    }
    let intervals: Vec<String> = region
        .intervals
        .iter()
        .filter_map(|interval| {
            let from = interval.from_ms.max(source.start_ms) - source.start_ms - origin;
            let to = interval.to_ms.min(source.end_ms) - source.start_ms - origin;
            (from < to).then(|| format!("gte(t,{})*lt(t,{})", secs(from), secs(to)))
        })
        .collect();
    (!intervals.is_empty()).then(|| intervals.join("+"))
}

fn region_rect(region: &MaskRegion, media: &MediaInfo) -> (u32, u32, u32, u32) {
    let x = (region.x * media.width as f64).floor() as u32;
    let y = (region.y * media.height as f64).floor() as u32;
    let w = (region.width * media.width as f64).ceil().max(1.) as u32;
    let h = (region.height * media.height as f64).ceil().max(1.) as u32;
    (x, y, w.min(media.width - x), h.min(media.height - y))
}

fn qsv_danmaku_band_height(settings: &super::model::DanmakuSettings, height: u32) -> u32 {
    let display_band = (height as f64 * settings.display_area).ceil() as u32;
    // DanmakuFactory keeps lanes inside display_area. For an unusually large
    // font that cannot fit there, retain room for its glyph and outline.
    let glyph_extent = settings.font_size.ceil() as u32 + (settings.outline.ceil() as u32 * 2);
    let band = if glyph_extent > display_band {
        display_band.saturating_add(glyph_extent)
    } else {
        display_band
    };
    band.min(height).div_ceil(2).saturating_mul(2).min(height)
}

fn qsv_danmaku_ass(ass: &str, full_height: u32, band_height: u32) -> Option<String> {
    let full = format!("PlayResY: {full_height}");
    let band = format!("PlayResY: {band_height}");
    ass.contains(&full).then(|| ass.replacen(&full, &band, 1))
}

/// A QSV decoder/encoder path for persistent mosaic and smooth-blur masks.
/// Mosaic keeps the existing nearest-neighbour block pattern; smooth blur uses
/// QSV scaling. ASS stays on libass and only its occupied display band crosses
/// between QSV surfaces and system memory.
fn qsv_filter_graph(
    source: &SourceSpan,
    settings: &RenderRecipe,
    media: &MediaInfo,
    target: (u32, u32),
    origin: i64,
    ass: bool,
    audio_input: usize,
    include_audio: bool,
    frame_rate: &str,
    encoder: &SelectedEncoder,
) -> Option<String> {
    if encoder.encoder != encoding::VideoEncoder::H264Qsv
        || target.0 % 2 != 0
        || target.1 % 2 != 0
        || (media.width, media.height) != target
        || settings.regions.iter().any(|region| {
            !matches!(
                region.effect_type,
                EffectType::Mosaic | EffectType::SmoothBlur
            ) || !region.intervals.is_empty()
        })
        || settings.regions.len() > 4
    {
        return None;
    }

    let mut parts = vec![format!(
        "[0:v:0]setpts=PTS-({:.9})/TB-({})/TB[v0]",
        media.video_origin_seconds,
        secs(origin)
    )];
    let mut index = 0;
    for region in &settings.regions {
        if region.opacity == 0. {
            continue;
        }
        let (x, y, w, h) = region_rect(region, media);
        let left = x & !1;
        let top = y & !1;
        let right = (x + w).div_ceil(2) * 2;
        let bottom = (y + h).div_ceil(2) * 2;
        let right = right.min(media.width);
        let bottom = bottom.min(media.height);
        let w = right.checked_sub(left)?;
        let h = bottom.checked_sub(top)?;
        let block = region.strength.round().max(1.) as u32;
        let low_w = (w / block).max(1);
        let low_h = (h / block).max(1);
        // QSV/NV12 surfaces need even dimensions. Rare geometry that cannot
        // retain the effect grid goes through the proven CPU path.
        if w < 2 || h < 2 || low_w < 2 || low_h < 2 || low_w % 2 != 0 || low_h % 2 != 0 {
            return None;
        }
        let next = index + 1;
        let alpha = (region.opacity * 255.).round().clamp(0., 255.) as u8;
        parts.push(format!("[v{index}]split=2[keep{index}][crop{index}]"));
        match region.effect_type {
            EffectType::Mosaic => {
                // Download one QSV sample per block; nearest CPU upscale retains hard block edges.
                parts.push(format!(
                    "[crop{index}]vpp_qsv=cw={w}:ch={h}:cx={left}:cy={top}:w={low_w}:h={low_h}:async_depth=8,hwdownload,format=nv12,scale={w}:{h}:flags=neighbor,format=nv12,hwupload=extra_hw_frames=64[effect{index}]"
                ));
            }
            EffectType::SmoothBlur => parts.push(format!(
                "[crop{index}]vpp_qsv=cw={w}:ch={h}:cx={left}:cy={top}:w={low_w}:h={low_h}:async_depth=8,scale_qsv=w={w}:h={h}:format=nv12[effect{index}]"
            )),
            _ => unreachable!("QSV eligibility checked above"),
        }
        parts.push(format!(
            "[keep{index}][effect{index}]overlay_qsv=x={left}:y={top}:w={w}:h={h}:alpha={alpha}:eof_action=repeat[v{next}]"
        ));
        index = next;
    }

    let start = source.trim_start_ms - origin;
    let end = source.trim_end_ms - origin;
    if ass {
        let band_height = qsv_danmaku_band_height(&settings.danmaku, target.1);
        parts.push(format!("[v{index}]split=2[main-source][danmaku-source]"));
        parts.push(format!(
            "[main-source]setsar=1,trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate}[main]",
            secs(start),
            secs(end),
            secs(start)
        ));
        parts.push(format!(
            "[danmaku-source]vpp_qsv=cw={}:ch={band_height}:cx=0:cy=0:w={}:h={band_height}:async_depth=8,hwdownload,format=nv12,setsar=1,ass=filename=danmaku-qsv.ass:fontsdir=fonts,trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate},format=nv12,hwupload=extra_hw_frames=64[danmaku]",
            target.0,
            target.0,
            secs(start),
            secs(end),
            secs(start)
        ));
        parts.push(format!(
            "[main][danmaku]overlay_qsv=x=0:y=0:w={}:h={band_height}:alpha=255:eof_action=repeat[video]",
            target.0
        ));
    } else {
        parts.push(format!(
            "[v{index}]setsar=1,trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate}[video]",
            secs(start),
            secs(end),
            secs(start)
        ));
    }

    if include_audio {
        let span = source.trim_end_ms - source.trim_start_ms;
        if media.has_audio {
            parts.push(format!("[0:a:0]asetpts=PTS-({:.9})/TB,atrim=start={}:end={},asetpts=PTS-({})/TB,aresample=48000:async=1:first_pts=0,aformat=sample_rates=48000:channel_layouts=stereo,apad,atrim=duration={}[audio]", media.video_origin_seconds, secs(source.trim_start_ms), secs(source.trim_end_ms), secs(source.trim_start_ms), secs(span)));
        } else {
            parts.push(format!(
                "[{audio_input}:a:0]atrim=duration={},asetpts=PTS-STARTPTS[audio]",
                secs(span)
            ));
        }
    }
    Some(parts.join(";"))
}

fn segment_command(
    selected: &SelectedEncoder,
    segment_dir: &Path,
    source: &SourceSpan,
    media: &MediaInfo,
    seek_ms: i64,
    image_count: usize,
    filter_file_option: &str,
    filter_file: &str,
    qsv_decode: bool,
    filter_complex_threads: &str,
    probe_frames: Option<u32>,
) -> Result<Command, String> {
    let mut command = if qsv_decode {
        // QSV can keep the full filter graph in the hardware path. Give this
        // short-lived encode the normal scheduler priority so it can reach
        // real-time multiples instead of yielding like background CPU work.
        crate::tools::ffmpeg_command()
    } else {
        crate::tools::low_priority_ffmpeg_command()
    };
    command
        .current_dir(segment_dir)
        .args(["-nostdin", "-hide_banner", "-loglevel", "error"]);
    if probe_frames.is_none() {
        command.args(["-nostats", "-progress", "pipe:1"]);
    }
    command.args(["-copyts"]);
    command.args(selected.input_args());
    if qsv_decode {
        command.args([
            "-async_depth",
            "8",
            "-hwaccel",
            "qsv",
            "-hwaccel_device",
            "encoder",
            "-hwaccel_output_format",
            "qsv",
        ]);
    } else {
        command.args(["-threads", "2"]);
    }
    if seek_ms > 0 {
        command.args([
            "-seek_timestamp",
            "1",
            "-ss",
            &format!("{:.9}", media.video_origin_seconds + seek_ms as f64 / 1000.),
        ]);
    }
    command
        .arg("-i")
        .arg(std::fs::canonicalize(&source.path).map_err(|e| format!("打开原录像失败：{e}"))?);
    for input in 1..=image_count {
        command
            .arg("-i")
            .arg(segment_dir.join(format!("image-{input}.png")));
    }
    if !media.has_audio && probe_frames.is_none() {
        command.args(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"]);
    }
    command.args([
        "-filter_complex_threads",
        filter_complex_threads,
        filter_file_option,
        filter_file,
    ]);
    if probe_frames.is_some() {
        command
            .args(["-map", "[video]", "-an"])
            .args(["-frames:v", &probe_frames.unwrap().to_string()])
            .args(selected.output_args());
        if qsv_decode {
            command.args(["-async_depth:v", "8"]);
        }
        command.args(["-f", "null", "-"]);
    } else {
        command.args(["-map", "[video]", "-map", "[audio]"]);
        command.args(selected.output_args());
        if qsv_decode {
            command.args(["-async_depth:v", "8"]);
        }
        command
            .args(["-c:a", "aac", "-ar", "48000", "-ac", "2", "-b:a", "192k"])
            .args([
                "-t",
                &secs(source.trim_end_ms - source.trim_start_ms),
                "-video_track_timescale",
                "90000",
                "-f",
                "mp4",
                "-y",
                "encoded.mp4",
            ]);
    }
    command.stdin(Stdio::null());
    Ok(command)
}

/// Coordinates refer to the actual source frame; scaling/padding is performed
/// after masks so a changing aspect ratio cannot expose a strip underneath.
fn filter_graph(
    source: &SourceSpan,
    settings: &RenderRecipe,
    media: &MediaInfo,
    target: (u32, u32),
    origin: i64,
    ass: bool,
    image_inputs: &HashMap<i64, usize>,
    audio_input: usize,
    frame_rate: &str,
    encoder: &SelectedEncoder,
) -> String {
    let mut parts = vec![format!(
        "[0:v:0]setpts=PTS-({:.9})/TB-({})/TB,format=rgb24[v0]",
        media.video_origin_seconds,
        secs(origin)
    )];
    let mut index = 0;
    for region in &settings.regions {
        let Some(enabled) = enable(region, source, origin) else {
            continue;
        };
        let x = (region.x * media.width as f64).floor() as u32;
        let y = (region.y * media.height as f64).floor() as u32;
        let w = (region.width * media.width as f64).ceil().max(1.) as u32;
        let h = (region.height * media.height as f64).ceil().max(1.) as u32;
        let w = w.min(media.width - x);
        let h = h.min(media.height - y);
        let next = index + 1;
        match region.effect_type {
            EffectType::Solid => parts.push(format!("[v{index}]drawbox=x={x}:y={y}:w={w}:h={h}:color={}@{}:t=fill:enable='{enabled}'[v{next}]", region.color, region.opacity)),
            EffectType::Image => {
                let input = image_inputs[&region.asset_id.expect("validated image")];
                parts.push(format!("[{input}:v:0]scale={w}:{h},format=rgba,colorchannelmixer=aa={}[picture{index}]", region.opacity));
                parts.push(format!("[v{index}][picture{index}]overlay=x={x}:y={y}:format=rgb:eof_action=repeat:enable='{enabled}'[v{next}]"));
            }
            effect => {
                parts.push(format!("[v{index}]split=2[keep{index}][crop{index}]"));
                let transform = match effect {
                    EffectType::Mosaic => {
                        let block = region.strength.round().max(1.) as u32;
                        format!("scale={}:{}:flags=neighbor,scale={w}:{h}:flags=neighbor", (w / block).max(1), (h / block).max(1))
                    }
                    EffectType::SmoothBlur => {
                        let block = region.strength.round().max(1.) as u32;
                        format!("scale={}:{}:flags=bilinear,scale={w}:{h}:flags=bilinear", (w / block).max(1), (h / block).max(1))
                    }
                    EffectType::Blur => format!("gblur=sigma={}", region.strength),
                    EffectType::Solid | EffectType::Image => unreachable!("handled above"),
                };
                parts.push(format!("[crop{index}]crop={w}:{h}:{x}:{y}:exact=1,{transform},format=rgba,colorchannelmixer=aa={}[effect{index}]", region.opacity));
                parts.push(format!("[keep{index}][effect{index}]overlay=x={x}:y={y}:format=rgb:eof_action=repeat:enable='{enabled}'[v{next}]"));
            }
        }
        index = next;
    }
    let (width, height) = target;
    parts.push(format!("[v{index}]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1[canvas]"));
    let ass_filter = if ass {
        "ass=filename=danmaku.ass:fontsdir=fonts,"
    } else {
        ""
    };
    // Keep the complete segment ASS clock until the *last* trim. libass renders
    // in-flight events at their existing position without video pre-roll.
    let start = source.trim_start_ms - origin;
    let end = source.trim_end_ms - origin;
    parts.push(format!(
        "[canvas]{ass_filter}trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate},{}[video]",
        secs(start),
        secs(end),
        secs(start),
        encoder.video_filter(),
    ));
    let span = source.trim_end_ms - source.trim_start_ms;
    if media.has_audio {
        parts.push(format!("[0:a:0]asetpts=PTS-({:.9})/TB,atrim=start={}:end={},asetpts=PTS-({})/TB,aresample=48000:async=1:first_pts=0,aformat=sample_rates=48000:channel_layouts=stereo,apad,atrim=duration={}[audio]", media.video_origin_seconds, secs(source.trim_start_ms), secs(source.trim_end_ms), secs(source.trim_start_ms), secs(span)));
    } else {
        parts.push(format!(
            "[{audio_input}:a:0]atrim=duration={},asetpts=PTS-STARTPTS[audio]",
            secs(span)
        ));
    }
    parts.join(";")
}

pub async fn render(
    request: EngineRequest,
    cancel: CancellationToken,
    mut report: impl FnMut(EngineProgress) + Send,
) -> Result<EngineOutput, String> {
    request.settings.validate()?;
    if request.sources.is_empty() {
        return Err("所选范围没有录像".into());
    }
    let mut previous_end = None;
    for source in &request.sources {
        if source.start_ms < 0
            || source.end_ms <= source.start_ms
            || source.trim_start_ms < 0
            || source.trim_end_ms <= source.trim_start_ms
            || source.trim_end_ms > source.end_ms - source.start_ms
            || previous_end.is_some_and(|end| source.start_ms < end)
        {
            return Err("录像分段时间范围无效或重叠".into());
        }
        previous_end = Some(source.end_ms);
        if source.path == request.output_path {
            return Err("合成输出不能覆盖原片".into());
        }
    }
    let parent = request.output_path.parent().unwrap_or(Path::new("."));
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("创建合成目录失败：{e}"))?;
    if tokio::fs::try_exists(&request.output_path)
        .await
        .unwrap_or(true)
    {
        return Err("输出文件已经存在，拒绝覆盖".into());
    }
    let work = tempfile::Builder::new()
        .prefix(".render-")
        .tempdir_in(parent)
        .map_err(|e| format!("创建合成临时目录失败：{e}"))?;
    let total = request
        .sources
        .iter()
        .map(|s| s.trim_end_ms - s.trim_start_ms)
        .sum::<i64>()
        .max(1);
    report(engine_progress(
        None,
        "检测编码器".into(),
        Some(0.),
        0,
        total,
        FfmpegProgress::default(),
    ));
    let mut completed = 0i64;
    // FFmpeg 7 introduced /option file loading; older builds use the now-removed
    // filter_complex_script spelling. Probe once to support both without a
    // second encoding attempt after an argument parsing error.
    let filter_file_option = {
        let mut probe = crate::tools::ffmpeg_command();
        probe.args([
            "-hide_banner",
            "-nostdin",
            "-/filter_complex",
            "__biliup_missing_filter_probe__",
            "-f",
            "null",
            "-",
        ]);
        match run_process(probe, &cancel, Duration::from_secs(10), |_, _| {}).await {
            Err(error)
                if error.contains("Unrecognized option") || error.contains("Option not found") =>
            {
                "-filter_complex_script"
            }
            _ if cancel.is_cancelled() => return Err("合成已取消".into()),
            _ => "-/filter_complex",
        }
    };
    let first = probe(&request.sources[0].path, &cancel).await?;
    // All segments use the first segment's canvas/FPS and a fixed audio format
    // so concat can copy even when input codecs, resolution or audio differ.
    let target = (first.width.div_ceil(2) * 2, first.height.div_ceil(2) * 2);
    // Freeze the selection before encoding any source. A later hardware
    // failure stops the job rather than joining incompatible segment streams.
    let selected = encoding::select(
        &request.settings.encoding,
        target,
        &first.frame_rate,
        &cancel,
    )
    .await?;
    let mut active_selected = selected.clone();
    let mut manifest = String::from("ffconcat version 1.0\n");
    for (number, source) in request.sources.iter().enumerate() {
        report(engine_progress(
            Some(&active_selected),
            format!("准备分段 {}/{}", number + 1, request.sources.len()),
            Some(completed as f64 / total as f64 * 0.95),
            completed,
            total,
            FfmpegProgress::default(),
        ));
        let media = if number == 0 {
            first.clone()
        } else {
            probe(&source.path, &cancel).await?
        };
        let segment_dir = work.path().join(format!("segment-{number}"));
        tokio::fs::create_dir(&segment_dir)
            .await
            .map_err(|e| e.to_string())?;
        let ass = super::danmaku::generate(
            source,
            &request.settings.danmaku,
            target.0,
            target.1,
            &segment_dir,
            &cancel,
        )
        .await?;
        let has_ass = request.settings.danmaku.enabled
            && ass.comment_count > 0
            && request.settings.danmaku.opacity > 0.;
        if has_ass {
            tokio::fs::write(segment_dir.join("danmaku.ass"), &ass.ass)
                .await
                .map_err(|e| e.to_string())?;
            let fonts = segment_dir.join("fonts");
            tokio::fs::create_dir(&fonts)
                .await
                .map_err(|e| e.to_string())?;
            let font = request
                .font_path
                .clone()
                .map(Ok)
                .unwrap_or_else(crate::tools::render_font_path)?;
            // Opaque safe filename avoids filter escaping and prevents names
            // from leaking arbitrary paths into the generated filter graph.
            let extension = font
                .extension()
                .and_then(|e| e.to_str())
                .filter(|e| matches!(*e, "ttf" | "otf" | "ttc"))
                .unwrap_or("otf")
                .to_owned();
            tokio::fs::copy(font, fonts.join(format!("render-font.{extension}")))
                .await
                .map_err(|e| format!("复制弹幕字体失败：{e}"))?;
            tokio::fs::write(
                fonts.join("NotoEmoji.ttf"),
                crate::tools::render_emoji_font_bytes(),
            )
            .await
            .map_err(|e| format!("写入弹幕 Emoji 字体失败：{e}"))?;
        }
        // Input-side seek prevents decoding hours of material for a short tail
        // clip. Complete-segment ASS has already been laid out, and original
        // timestamps are retained so seeking cannot restart a scrolling cue.
        let pre_roll_ms = if has_ass {
            (request.settings.danmaku.scroll_seconds * 1000.).ceil() as i64 + 1000
        } else {
            2000
        };
        let seek_ms = (source.trim_start_ms - pre_roll_ms).max(0);
        let mut images = HashMap::new();
        for region in request.settings.regions.iter().filter(|r| {
            r.effect_type == EffectType::Image && enable(r, source, ass.time_origin_ms).is_some()
        }) {
            let id = region.asset_id.expect("validated image");
            if images.contains_key(&id) {
                continue;
            }
            let path = request
                .assets
                .get(&id.to_string())
                .ok_or("图片素材不存在")?;
            let input = images.len() + 1;
            let local = segment_dir.join(format!("image-{input}.png"));
            tokio::fs::copy(path, &local)
                .await
                .map_err(|e| format!("读取图片素材失败：{e}"))?;
            images.insert(id, input);
        }
        let audio_input = images.len() + 1;
        let qsv_graph = qsv_filter_graph(
            source,
            &request.settings,
            &media,
            target,
            ass.time_origin_ms,
            has_ass,
            audio_input,
            true,
            &first.frame_rate,
            &selected,
        );
        let mut use_qsv_graph = qsv_graph.is_some();
        if use_qsv_graph && has_ass {
            let band_height = qsv_danmaku_band_height(&request.settings.danmaku, target.1);
            if let Some(script) = qsv_danmaku_ass(&ass.ass, target.1, band_height) {
                tokio::fs::write(segment_dir.join("danmaku-qsv.ass"), script)
                    .await
                    .map_err(|e| format!("保存 QSV 弹幕图层失败：{e}"))?;
            } else {
                tracing::warn!(
                    segment_id = source.segment_id,
                    "ASS 画布尺寸不匹配，改用软件滤镜"
                );
                record_filter_fallback(
                    &mut active_selected,
                    "ASS 画布尺寸不匹配，不能裁剪弹幕处理带",
                );
                use_qsv_graph = false;
            }
        }
        if use_qsv_graph {
            let probe_graph = qsv_filter_graph(
                source,
                &request.settings,
                &media,
                target,
                ass.time_origin_ms,
                has_ass,
                audio_input,
                false,
                &first.frame_rate,
                &selected,
            )
            .expect("video-only QSV graph has the same eligibility as the full graph");
            tokio::fs::write(segment_dir.join("filters-qsv-probe.txt"), probe_graph)
                .await
                .map_err(|e| format!("保存 QSV 探测滤镜失败：{e}"))?;
            let command = segment_command(
                &selected,
                &segment_dir,
                source,
                &media,
                seek_ms,
                0,
                filter_file_option,
                "filters-qsv-probe.txt",
                true,
                "2",
                Some(8),
            )?;
            match run_process(command, &cancel, Duration::from_secs(20), |_, _| {}).await {
                Ok(_) => {}
                Err(_) if cancel.is_cancelled() => return Err("合成已取消".into()),
                Err(error) => {
                    tracing::warn!(segment_id = source.segment_id, %error, "QSV 解码与滤镜探测失败，改用软件滤镜");
                    record_filter_fallback(&mut active_selected, &error);
                    use_qsv_graph = false;
                }
            }
        }

        let filter_file = if use_qsv_graph {
            "filters-qsv.txt"
        } else {
            "filters.txt"
        };
        let filter = if use_qsv_graph {
            qsv_graph.expect("successful QSV probe has a render graph")
        } else {
            filter_graph(
                source,
                &request.settings,
                &media,
                target,
                ass.time_origin_ms,
                has_ass,
                &images,
                audio_input,
                &first.frame_rate,
                &selected,
            )
        };
        // A region can contain many intervals; loading a task-private script
        // avoids the OS argument-length limit on Windows and Linux.
        tokio::fs::write(segment_dir.join(filter_file), filter)
            .await
            .map_err(|e| format!("保存合成滤镜失败：{e}"))?;
        let command = segment_command(
            &selected,
            &segment_dir,
            source,
            &media,
            seek_ms,
            images.len(),
            filter_file_option,
            filter_file,
            use_qsv_graph,
            if use_qsv_graph { "2" } else { "1" },
            None,
        )?;
        let phase = format!("合成分段 {}/{}", number + 1, request.sources.len());
        let duration = source.trim_end_ms - source.trim_start_ms;
        let encode_started = std::time::Instant::now();
        let encoded = run_process(command, &cancel, PROCESS_STALL, |_, block| {
            let stats = FfmpegProgress::parse(block)
                .with_elapsed(&first.frame_rate, encode_started.elapsed());
            let at = stats.encoded_time(&first.frame_rate);
            report(engine_progress(
                Some(&active_selected),
                phase.clone(),
                Some(
                    ((completed + at.clamp(0, duration)) as f64 / total as f64 * 0.95)
                        .clamp(0., 0.95),
                ),
                completed + at.clamp(0, duration),
                total,
                stats,
            ));
        })
        .await;
        if let Err(error) = encoded {
            if use_qsv_graph && !cancel.is_cancelled() && !error.contains("磁盘空间不足") {
                tracing::warn!(segment_id = source.segment_id, %error, "QSV 渲染失败，重试软件滤镜");
                record_filter_fallback(&mut active_selected, &error);
                let _ = tokio::fs::remove_file(segment_dir.join("encoded.mp4")).await;
                let fallback = filter_graph(
                    source,
                    &request.settings,
                    &media,
                    target,
                    ass.time_origin_ms,
                    has_ass,
                    &images,
                    audio_input,
                    &first.frame_rate,
                    &selected,
                );
                tokio::fs::write(segment_dir.join("filters.txt"), fallback)
                    .await
                    .map_err(|e| format!("保存回退滤镜失败：{e}"))?;
                let command = segment_command(
                    &selected,
                    &segment_dir,
                    source,
                    &media,
                    seek_ms,
                    images.len(),
                    filter_file_option,
                    "filters.txt",
                    false,
                    "1",
                    None,
                )?;
                run_process(command, &cancel, PROCESS_STALL, |_, block| {
                    let stats = FfmpegProgress::parse(block)
                        .with_elapsed(&first.frame_rate, encode_started.elapsed());
                    let at = stats.encoded_time(&first.frame_rate);
                    report(engine_progress(
                        Some(&active_selected),
                        phase.clone(),
                        Some(
                            ((completed + at.clamp(0, duration)) as f64 / total as f64 * 0.95)
                                .clamp(0., 0.95),
                        ),
                        completed + at.clamp(0, duration),
                        total,
                        stats,
                    ));
                })
                .await?;
            } else {
                return Err(error);
            }
        }
        completed += duration;
        manifest.push_str(&format!("file 'segment-{number}/encoded.mp4'\n"));
    }
    tokio::fs::write(work.path().join("join.txt"), manifest)
        .await
        .map_err(|e| e.to_string())?;
    let mut command = crate::tools::low_priority_ffmpeg_command();
    command
        .current_dir(work.path())
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostats",
            "-progress",
            "pipe:1",
        ])
        .args([
            "-f", "concat", "-safe", "1", "-i", "join.txt", "-map", "0:v:0", "-map", "0:a:0", "-c",
            "copy",
        ])
        .args([
            "-movflags",
            "+faststart",
            "-video_track_timescale",
            "90000",
            "-f",
            "mp4",
            "-y",
            "result.mp4",
        ])
        .stdin(Stdio::null());
    report(engine_progress(
        Some(&active_selected),
        "拼接并保存".into(),
        Some(0.95),
        total,
        total,
        FfmpegProgress::default(),
    ));
    run_process(command, &cancel, PROCESS_STALL, |at, _| {
        // Copy-only joining speed is not encoding speed; keep it unavailable.
        report(engine_progress(
            Some(&active_selected),
            "拼接并保存".into(),
            Some(0.95 + 0.04 * (at as f64 / total as f64).clamp(0., 1.)),
            total,
            total,
            FfmpegProgress::default(),
        ));
    })
    .await?;
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    let result = work.path().join("result.mp4");
    let duration_ms = probe(&result, &cancel).await?.duration_ms.unwrap_or(total);
    let bytes = tokio::fs::metadata(&result)
        .await
        .map_err(|e| e.to_string())?
        .len();
    if bytes == 0 {
        return Err("FFmpeg 没有生成视频".into());
    }
    tokio::fs::File::open(&result)
        .await
        .map_err(|e| e.to_string())?
        .sync_all()
        .await
        .map_err(|e| format!("保存合成视频失败：{e}"))?;
    // A hard link is an atomic no-clobber publish on the same filesystem. Drop
    // of the temporary directory removes only intermediate links and files.
    tokio::fs::hard_link(&result, &request.output_path)
        .await
        .map_err(|e| format!("发布合成产物失败：{e}"))?;
    report(engine_progress(
        Some(&active_selected),
        "已完成".into(),
        Some(1.),
        total,
        total,
        FfmpegProgress::default(),
    ));
    Ok(EngineOutput {
        output_path: request.output_path,
        output_bytes: bytes as i64,
        duration_ms,
        encoder: Some(active_selected.encoder.name().into()),
        encoder_device: active_selected.device,
        fallback_reason: active_selected.fallback_reason,
    })
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
