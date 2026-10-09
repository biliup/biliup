//! Immutable source recordings -> one encode per segment -> a copy-only MP4 join.
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
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineProgress {
    pub phase: String,
    pub ratio: Option<f64>,
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
    pub danmaku_factory: bool,
    pub font: bool,
    pub error: Option<String>,
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
/// watchdog is based on *advancing media time*, so an hours-long encode is fine
/// while a hung child is stopped. Cancellation kills and reaps the child.
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
    let mut stdout_open = true;
    let watch = tokio::time::sleep(stall);
    tokio::pin!(watch);
    let outcome = loop {
        tokio::select! {
            _ = cancel.cancelled() => { break Err("合成已取消".to_string()); }
            _ = &mut watch => { break Err(format!("处理程序连续 {} 秒没有推进，任务已停止；可重试", stall.as_secs())); }
            line = lines.next_line(), if stdout_open => {
                match line {
                    Ok(Some(line)) => {
                        let at = line.strip_prefix("out_time_us=").and_then(|v| v.parse::<i64>().ok()).map(|v| v / 1000);
                        if let Some(at) = at {
                            if at > latest {
                                latest = at;
                                watch.as_mut().reset(tokio::time::Instant::now() + stall);
                            }
                            report(at, &line);
                        }
                    }
                    _ => stdout_open = false,
                }
            }
            status = child.wait() => {
                break status.map_err(|e| format!("等待处理程序失败：{e}")).and_then(|status| {
                    if status.success() { Ok(()) } else { Err(format!("处理程序失败（{status}）")) }
                });
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

/// Tool support is probed on demand before queueing an export. Empty XML still
/// counts as recorded danmaku; it does not require invoking Factory.
pub async fn capabilities() -> RenderCapabilities {
    let mut status = RenderCapabilities {
        available: false,
        ffmpeg: false,
        libass: false,
        libx264: false,
        danmaku_factory: crate::tools::danmaku_factory().is_ok(),
        font: crate::tools::render_font().is_some(),
        error: None,
    };
    let cancel = CancellationToken::new();
    let mut command = crate::tools::ffmpeg_command();
    // ffmpeg prints filter/encoder lists to stdout; run_process intentionally
    // ignores those lines, so the actual one-frame graph below tests both.
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
        ])
        .args(["-frames:v", "1", "-c:v", "libx264", "-f", "null", "-"]);
    match run_process(command, &cancel, Duration::from_secs(15), |_, _| {}).await {
        Ok(_) => {
            status.ffmpeg = true;
            status.libx264 = true;
        }
        Err(error) => {
            status.error = Some(error);
            return status;
        }
    }
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(e) => {
            status.error = Some(e.to_string());
            return status;
        }
    };
    let ass = temp.path().join("check.ass");
    if let Err(e) = tokio::fs::write(&ass, super::danmaku::empty_ass(16, 16)).await {
        status.error = Some(e.to_string());
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
        Err(error) => status.error = Some(error),
    }
    status.available =
        status.ffmpeg && status.libx264 && status.libass && status.danmaku_factory && status.font;
    if status.error.is_none() && !status.available {
        status.error = Some("未找到 DanmakuFactory 或弹幕字体，请安装所需工具".into());
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
                let transform = if effect == EffectType::Mosaic {
                    let block = region.strength.round().max(1.) as u32;
                    format!("scale={}:{}:flags=neighbor,scale={w}:{h}:flags=neighbor", (w / block).max(1), (h / block).max(1))
                } else { format!("gblur=sigma={}", region.strength) };
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
        "[canvas]{ass_filter}trim=start={}:end={},setpts=PTS-({})/TB,fps={frame_rate},format=yuv420p[video]",
        secs(start),
        secs(end),
        secs(start)
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
    let mut manifest = String::from("ffconcat version 1.0\n");
    for (number, source) in request.sources.iter().enumerate() {
        report(EngineProgress {
            phase: format!("准备分段 {}/{}", number + 1, request.sources.len()),
            ratio: Some(completed as f64 / total as f64 * 0.95),
        });
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
        }
        let mut command = crate::tools::low_priority_ffmpeg_command();
        command
            .current_dir(&segment_dir)
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostats",
                "-progress",
                "pipe:1",
            ])
            .args(["-copyts", "-threads", "2"]);
        // Input-side seek prevents decoding hours of material for a short tail
        // clip. Complete-segment ASS has already been laid out, and original
        // timestamps are retained so seeking cannot restart a scrolling cue.
        let pre_roll_ms = if has_ass {
            (request.settings.danmaku.scroll_seconds * 1000.).ceil() as i64 + 1000
        } else {
            2000
        };
        let seek_ms = (source.trim_start_ms - pre_roll_ms).max(0);
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
            command.args(["-i", &format!("image-{input}.png")]);
            images.insert(id, input);
        }
        let audio_input = images.len() + 1;
        if !media.has_audio {
            command.args(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"]);
        }
        let filter = filter_graph(
            source,
            &request.settings,
            &media,
            target,
            ass.time_origin_ms,
            has_ass,
            &images,
            audio_input,
            &first.frame_rate,
        );
        // A region can contain many intervals; loading a task-private script
        // avoids the OS argument-length limit on Windows and Linux.
        tokio::fs::write(segment_dir.join("filters.txt"), filter)
            .await
            .map_err(|e| format!("保存合成滤镜失败：{e}"))?;
        command
            .args([
                "-filter_complex_threads",
                "1",
                filter_file_option,
                "filters.txt",
            ])
            .args([
                "-map", "[video]", "-map", "[audio]", "-c:v", "libx264", "-preset", "veryfast",
                "-crf", "20", "-threads", "2",
            ])
            .args([
                "-pix_fmt", "yuv420p", "-c:a", "aac", "-ar", "48000", "-ac", "2", "-b:a", "192k",
            ])
            .args([
                "-t",
                &secs(source.trim_end_ms - source.trim_start_ms),
                "-video_track_timescale",
                "90000",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
                "-y",
                "encoded.mp4",
            ])
            .stdin(Stdio::null());
        let phase = format!("合成分段 {}/{}", number + 1, request.sources.len());
        let duration = source.trim_end_ms - source.trim_start_ms;
        run_process(command, &cancel, PROCESS_STALL, |at, _| {
            report(EngineProgress {
                phase: phase.clone(),
                ratio: Some(
                    ((completed + at.clamp(0, duration)) as f64 / total as f64 * 0.95)
                        .clamp(0., 0.95),
                ),
            });
        })
        .await?;
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
    report(EngineProgress {
        phase: "拼接并保存".into(),
        ratio: Some(0.95),
    });
    run_process(command, &cancel, PROCESS_STALL, |at, _| {
        report(EngineProgress {
            phase: "拼接并保存".into(),
            ratio: Some(0.95 + 0.04 * (at as f64 / total as f64).clamp(0., 1.)),
        });
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
    report(EngineProgress {
        phase: "已完成".into(),
        ratio: Some(1.),
    });
    Ok(EngineOutput {
        output_path: request.output_path,
        output_bytes: bytes as i64,
        duration_ms,
    })
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
