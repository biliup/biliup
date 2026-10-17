//! Shared video effects: source inputs -> one filter graph -> one final encode.
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
    /// The built-in DanmakuFactory-derived converter, checked by converting a
    /// one-comment XML in this process. The name is kept for API
    /// compatibility; no external program is involved.
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

#[derive(Debug, Clone)]
struct MediaInfo {
    width: u32,
    height: u32,
    has_audio: bool,
    duration_ms: Option<i64>,
    format_start_seconds: f64,
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
    let format_start_pattern = regex::Regex::new(r"start:\s+([-+0-9.eE]+)").unwrap();
    let format_start_seconds = format_start_pattern
        .captures(input)
        .and_then(|c| c[1].parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(0.);
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
        format_start_seconds,
        video_origin_seconds,
        frame_rate,
    })
}

/// Tool support is probed on demand before queueing an export. The danmaku
/// converter is checked with an in-process, one-line XML conversion.
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
    // A real libass decode only needs one small ASS input, not a work directory.
    let mut ass = match tempfile::Builder::new()
        .prefix(".biliup-ass-check-")
        .suffix(".ass")
        .tempfile()
    {
        Ok(file) => file,
        Err(error) => {
            status.error.get_or_insert(error.to_string());
            return status;
        }
    };
    use std::io::Write;
    if let Err(error) = ass.write_all(super::danmaku::empty_ass(16, 16).as_bytes()) {
        status.error.get_or_insert(error.to_string());
        return status;
    }
    let mut command = crate::tools::ffmpeg_command();
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=s=16x16:d=0.04",
            "-vf",
        ])
        .arg(format!("ass=filename={}", filter_path(ass.path())))
        .args(["-frames:v", "1", "-f", "null", "-"]);
    match run_process(command, &cancel, Duration::from_secs(15), |_, _| {}).await {
        Ok(_) => status.libass = true,
        Err(error) => {
            status.error.get_or_insert(error);
        }
    }
    status.available = status.ffmpeg && status.libass && status.danmaku_factory && status.font;
    if status.error.is_none() && !status.available {
        status.error = Some("未找到弹幕字体：请在配置里填写 render_fonts_dir，或安装随包字体".into());
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
    super::danmaku::generate(source, &settings.danmaku, width, height, cancel).await
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

/// Coordinates refer to the actual source frame; scaling/padding is performed
/// after masks so a changing aspect ratio cannot expose a strip underneath.
fn filter_graph(
    source: &SourceSpan,
    settings: &RenderRecipe,
    media: &MediaInfo,
    target: (u32, u32),
    origin: i64,
    ass: Option<&Path>,
    font_dir: Option<&Path>,
    repair_audio: bool,
    image_inputs: &HashMap<i64, usize>,
    audio_input: usize,
    frame_rate: &str,
    input: usize,
    prefix: &str,
) -> String {
    let mut parts = vec![format!(
        "[SOURCE:v:0]setpts=PTS-({:.9})/TB-({})/TB,format=rgb24[v0]",
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
        let right = ((region.x + region.width) * media.width as f64).ceil() as u32;
        let bottom = ((region.y + region.height) * media.height as f64).ceil() as u32;
        let w = right.min(media.width).saturating_sub(x).max(1);
        let h = bottom.min(media.height).saturating_sub(y).max(1);
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
    let ass_filter = ass
        .map(|path| {
            let fonts = font_dir
                .map(|path| format!(":fontsdir={}", filter_path(path)))
                .unwrap_or_default();
            format!("ass=filename={}{fonts},", filter_path(path))
        })
        .unwrap_or_default();
    // Keep the complete segment ASS clock until the *last* trim. libass renders
    // in-flight events at their existing position without video pre-roll.
    let start = source.trim_start_ms - origin;
    let end = source.trim_end_ms - origin;
    parts.push(format!(
        "[canvas]{ass_filter}trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate},{}[video]",
        secs(start),
        secs(end),
        secs(start),
        "format=yuv420p",
    ));
    let span = source.trim_end_ms - source.trim_start_ms;
    if media.has_audio {
        // Clamp conflicting AAC packet clocks before resampling. This retains
        // decoded voice samples instead of dropping them to satisfy bad PTS.
        let repair = if repair_audio {
            "asetpts=if(isnan(PREV_OUTPTS)\\,PTS\\,max(PTS\\,PREV_OUTPTS+NB_SAMPLES/SR/TB)),"
        } else {
            ""
        };
        parts.push(format!("[SOURCE:a:0]asetpts=PTS-({:.9})/TB,{repair}atrim=start={}:end={},asetpts=PTS-({})/TB,aresample=48000:async=1:first_pts=0,aformat=sample_rates=48000:channel_layouts=stereo,apad,atrim=duration={}[audio]", media.video_origin_seconds, secs(source.trim_start_ms), secs(source.trim_end_ms), secs(source.trim_start_ms), secs(span)));
    } else {
        parts.push(format!(
            "[{audio_input}:a:0]atrim=duration={},asetpts=PTS-STARTPTS[audio]",
            secs(span)
        ));
    }
    // Prefix local link labels so all sources coexist in a single graph.
    let graph = parts.join(";").replace("[SOURCE:", &format!("[{input}:"));
    regex::Regex::new(r"\[([A-Za-z][A-Za-z0-9]*)\]")
        .unwrap()
        .replace_all(&graph, |c: &regex::Captures<'_>| {
            format!("[{prefix}{}]", &c[1])
        })
        .into_owned()
}

/// Escape a filename through the filter option parser and the graph parser.
fn filter_path(path: &Path) -> String {
    let text = path
        .to_string_lossy()
        .replace("\\\\?\\", "")
        .replace('\\', "/");
    let mut option = String::new();
    for c in text.chars() {
        if matches!(c, '\\' | '\'' | ':') {
            option.push('\\');
        }
        option.push(c);
    }
    let mut graph = String::new();
    for c in option.chars() {
        if matches!(c, '\\' | '\'' | '[' | ']' | ';' | ',') {
            graph.push('\\');
        }
        graph.push(c);
    }
    graph
}

pub async fn render(
    request: EngineRequest,
    cancel: CancellationToken,
    report: impl FnMut(EngineProgress) + Send,
) -> Result<EngineOutput, String> {
    let permit = crate::server::common::processing::transcode::acquire(&cancel).await?;
    render_with_permit(request, cancel, report, permit).await
}

pub(crate) async fn render_with_permit(
    request: EngineRequest,
    cancel: CancellationToken,
    mut report: impl FnMut(EngineProgress) + Send,
    _permit: tokio::sync::OwnedSemaphorePermit,
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
    let parent = request
        .output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| e.to_string())?;
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if request
        .output_path
        .try_exists()
        .map_err(|e| e.to_string())?
    {
        return Err("输出文件已经存在，拒绝覆盖".into());
    }
    let total = request
        .sources
        .iter()
        .map(|s| s.trim_end_ms - s.trim_start_ms)
        .sum::<i64>();
    report(engine_progress(
        None,
        "检测编码器".into(),
        Some(0.),
        0,
        total,
        FfmpegProgress::default(),
    ));
    let first = probe(&request.sources[0].path, &cancel).await?;
    let target = (first.width.div_ceil(2) * 2, first.height.div_ceil(2) * 2);
    let selected = encoding::select(
        &request.settings.encoding,
        target,
        &first.frame_rate,
        &cancel,
    )
    .await?;
    let mut command = crate::tools::low_priority_ffmpeg_command();
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostats",
            "-progress",
            "pipe:1",
            "-copyts",
        ])
        .args(selected.input_args());
    let mut media = vec![first.clone()];
    let mut audio_repair = Vec::with_capacity(request.sources.len());
    for (n, source) in request.sources.iter().enumerate() {
        if n > 0 {
            media.push(probe(&source.path, &cancel).await?);
        }
        audio_repair.push(matches!(
            super::audio::inspect_file(&source.path)
                .await
                .map_err(|e| e.to_string())?,
            super::audio::AudioEncoding::RepairAac { .. }
        ));
        let info = &media[n];
        // Decode from a nearby keyframe while retaining the original PTS clock.
        let seek = source.trim_start_ms.saturating_sub(2000).max(0);
        if seek > 0 {
            command.args([
                "-ss",
                &format!(
                    "{:.9}",
                    (info.video_origin_seconds + seek as f64 / 1000. - info.format_start_seconds)
                        .max(0.)
                ),
            ]);
        }
        command.args(["-threads", "2", "-i"]).arg(
            source
                .path
                .canonicalize()
                .map_err(|e| format!("打开原录像失败：{e}"))?,
        );
    }
    let mut inputs = request.sources.len();
    let mut images = HashMap::new();
    for region in &request.settings.regions {
        if region.effect_type != EffectType::Image {
            continue;
        }
        let id = region.asset_id.ok_or("图片遮挡缺少素材")?;
        if images.contains_key(&id) {
            continue;
        }
        let path = request
            .assets
            .get(&id.to_string())
            .ok_or("找不到遮挡图片")?;
        command.arg("-i").arg(
            path.canonicalize()
                .map_err(|e| format!("读取遮挡图片失败：{e}"))?,
        );
        images.insert(id, inputs);
        inputs += 1;
    }
    let silent = inputs;
    if media.iter().any(|m| !m.has_audio) {
        command.args(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"]);
    }
    let font_dir = request
        .font_path
        .as_ref()
        .and_then(|path| path.parent().map(Path::to_owned))
        .or_else(crate::tools::render_fonts_dir);
    let mut ass_files = Vec::new();
    let mut graph = Vec::new();
    let mut links = String::new();
    for (n, source) in request.sources.iter().enumerate() {
        let mut origin = 0;
        let ass = if request.settings.danmaku.enabled {
            let (file, metadata) = super::danmaku::generate_for_render(
                source,
                &request.settings.danmaku,
                target.0,
                target.1,
                &parent,
                &cancel,
            )
            .await?;
            origin = metadata.time_origin_ms;
            ass_files.push(file);
            Some(ass_files.last().unwrap().path())
        } else {
            None
        };
        graph.push(filter_graph(
            source,
            &request.settings,
            &media[n],
            target,
            origin,
            ass,
            font_dir.as_deref(),
            audio_repair[n],
            &images,
            silent,
            &first.frame_rate,
            n,
            &format!("s{n}"),
        ));
        links.push_str(&format!("[s{n}video][s{n}audio]"));
    }
    graph.push(format!(
        "{links}concat=n={}:v=1:a=1[joined][audio];[joined]{}[video]",
        request.sources.len(),
        selected.video_filter()
    ));
    // A file avoids Windows' command-line size limit for long multi-source graphs.
    // It is an owned sibling file, not a second set of intermediate video outputs.
    use std::io::Write;
    let mut script = tempfile::Builder::new()
        .prefix(".biliup-filter-")
        .suffix(".ffgraph")
        .tempfile_in(&parent)
        .map_err(|e| e.to_string())?;
    script
        .write_all(graph.join(";").as_bytes())
        .map_err(|e| e.to_string())?;
    let mut check = crate::tools::ffmpeg_command();
    check.args([
        "-hide_banner",
        "-nostdin",
        "-/filter_complex",
        "__biliup_missing_filter__",
        "-f",
        "null",
        "-",
    ]);
    let file_option = match run_process(check, &cancel, Duration::from_secs(10), |_, _| {}).await {
        Err(error)
            if error.contains("Unrecognized option") || error.contains("Option not found") =>
        {
            "-filter_complex_script"
        }
        _ => "-/filter_complex",
    };
    let part = tempfile::Builder::new()
        .prefix(".biliup-video-")
        .suffix(".part")
        .tempfile_in(&parent)
        .map_err(|e| e.to_string())?;
    let format = match request
        .output_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase()
        .as_str()
    {
        "flv" => "flv",
        "ts" => "mpegts",
        "mkv" => "matroska",
        "mov" => "mov",
        _ => "mp4",
    };
    command
        .args(["-filter_complex_threads", "1", file_option])
        .arg(script.path())
        .args(["-map", "[video]", "-map", "[audio]"])
        .args(selected.output_args_with_quality(request.settings.encoding.quality))
        .args([
            "-c:a",
            "aac",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-b:a",
            "192k",
            "-t",
            &secs(total),
            "-f",
            format,
            "-y",
        ])
        .arg(part.path());
    let started = std::time::Instant::now();
    run_process(command, &cancel, PROCESS_STALL, |_, block| {
        let stats = FfmpegProgress::parse(block).with_elapsed(&first.frame_rate, started.elapsed());
        let done = stats.encoded_time(&first.frame_rate).clamp(0, total);
        report(engine_progress(
            Some(&selected),
            "合成分段".into(),
            Some(done as f64 / total as f64 * 0.99),
            done,
            total,
            stats,
        ));
    })
    .await?;
    let bytes = part.as_file().metadata().map_err(|e| e.to_string())?.len();
    if bytes == 0 {
        return Err("FFmpeg 生成了空文件".into());
    }
    let output = probe(part.path(), &cancel).await?;
    let duration_ms = output.duration_ms.ok_or("无法读取输出时长")?;
    if (duration_ms - total).abs() > 1000 {
        return Err("输出时长与所选范围不一致".into());
    }
    super::audio::validate_output(part.path(), format, true)
        .await
        .map_err(|e| e.to_string())?;
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    part.as_file().sync_all().map_err(|e| e.to_string())?;
    part.persist_noclobber(&request.output_path)
        .map_err(|e| format!("发布合成产物失败：{}", e.error))?;
    report(engine_progress(
        Some(&selected),
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
        encoder: Some(selected.encoder.name().into()),
        encoder_device: selected.device,
        fallback_reason: selected.fallback_reason,
    })
}

/// Path-mutating native hook used by the ordinary segment processing chain.
pub async fn process_video(
    input: &Path,
    xml: Option<&Path>,
    settings: &RenderRecipe,
    source: Option<&SourceSpan>,
) -> Result<PathBuf, String> {
    if super::masking::is_unmasked(input) && settings.regions.is_empty() {
        return Err("隔离原片需要有效遮挡配置，禁止发布为普通录像".into());
    }
    let cancel = CancellationToken::new();
    let mut span = match source {
        Some(source) => source.clone(),
        None => {
            let duration = probe(input, &cancel)
                .await?
                .duration_ms
                .ok_or("无法读取录像时长")?;
            SourceSpan {
                segment_id: 0,
                source_session_id: 0,
                path: input.to_owned(),
                danmaku_path: xml.map(Path::to_owned),
                start_ms: 0,
                end_ms: duration,
                source_origin_ms: None,
                trim_start_ms: 0,
                trim_end_ms: duration,
            }
        }
    };
    span.path = input.to_owned();
    span.danmaku_path = xml.map(Path::to_owned).or(span.danmaku_path);
    let output = automatic_output_path(input, span.source_session_id);
    render(
        EngineRequest {
            sources: vec![span],
            settings: settings.clone(),
            assets: HashMap::new(),
            output_path: output.clone(),
            font_path: None,
        },
        cancel,
        |_| {},
    )
    .await?;
    Ok(output)
}

fn automatic_output_path(input: &Path, session_id: i64) -> PathBuf {
    let root = PathBuf::from(super::OUTPUT_DIR);
    let root = if session_id > 0 {
        root.join(session_id.to_string())
    } else {
        root
    };
    let name = if super::masking::is_unmasked(input) {
        super::masking::masked_path(input)
            .file_name()
            .unwrap_or_default()
            .to_owned()
    } else {
        format!(
            "{}.processed.mp4",
            input.file_stem().unwrap_or_default().to_string_lossy()
        )
        .into()
    };
    root.join(name)
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
