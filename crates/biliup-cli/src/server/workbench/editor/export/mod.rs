//! Native engineering export: saved project -> shared frame planner -> Chromium rasters -> FFmpeg.
mod audio;
#[cfg(test)]
mod audio_tests;
pub(super) mod browser;
mod estimate;
pub(super) mod frames;
pub mod generation;
mod graphics;
mod inputs;
pub mod jobs;
mod recording_effects;
mod shaders;
pub struct RenderProject {
    pub project: Value,
    pub paths: BTreeMap<String, String>,
    pub graphics: BTreeMap<String, String>,
    pub assets: super::fonts::RenderAssets,
}

use crate::server::common::processing::{encoding, engine};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub quality: String,
    #[serde(default)]
    pub video: Value,
    #[serde(default)]
    pub range: Value,
    #[serde(default)]
    pub burn_captions: Option<bool>,
    #[serde(default)]
    pub motion_blur: bool,
    #[serde(default)]
    pub target_lufs: Option<f64>,
    #[serde(default)]
    pub subtitles: Option<SubtitleOptions>,
}
pub(super) fn filename(value: &str) -> String {
    let clean = value
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect::<String>();
    let clean = clean.trim().trim_matches('.');
    let stem = if clean.to_ascii_lowercase().ends_with(".mp4") {
        &clean[..clean.len() - 4]
    } else {
        clean
    };
    let mut name = String::new();
    for c in stem.trim().chars() {
        if name.len() + c.len_utf8() > 180 {
            break;
        }
        name.push(c)
    }
    if name.is_empty() {
        name = "成片".into()
    }
    format!("{name}.mp4")
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleOptions {
    pub file: Option<String>,
    #[serde(default)]
    pub track: bool,
    pub language: Option<String>,
}
#[derive(Deserialize, Default)]
struct SubtitlePlan {
    file: Option<SubtitleFile>,
    soft: Option<String>,
    language: Option<String>,
}
#[derive(Deserialize)]
struct SubtitleFile {
    format: String,
    text: String,
}

#[derive(Deserialize)]
struct Plan {
    #[serde(default)]
    subtitles: SubtitlePlan,
    width: u32,
    height: u32,
    canvas: Canvas,
    fps: f64,
    frames: u64,
    background: String,
    rasters: Vec<Raster>,
    graphics: Vec<graphics::Graphic>,
    shaders: Vec<shaders::Shader>,
    luts: Vec<Lut>,
    warnings: Vec<String>,
}
#[derive(Deserialize)]
struct Canvas {
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
struct Raster {
    index: usize,
    states: Vec<String>,
    frames: Vec<usize>,
}
#[derive(Deserialize)]
struct Lut {
    key: String,
    text: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Encode {
    inputs: Vec<String>,
    graph: String,
    video_args: Vec<String>,
    output_args: Vec<String>,
}

impl Encode {
    /// Composite in software, then upload only the finished picture to the selected device.
    fn use_device(&mut self, selected: &encoding::SelectedEncoder) -> Result<(), String> {
        self.graph
            .push_str(&format!(";\n[vout]{}[vencoded]", selected.video_filter()));
        let mut index = 0;
        while index < self.output_args.len() {
            if self.output_args[index] == "-pix_fmt" {
                let end = (index + 2).min(self.output_args.len());
                self.output_args.drain(index..end);
                continue;
            }
            if self.output_args[index] == "[vout]" {
                self.output_args[index] = "[vencoded]".into();
            }
            index += 1;
        }
        // Per-output device options must precede the output filename.
        let output = self.output_args.pop().ok_or("导出计划缺少输出路径")?;
        self.output_args.extend(selected.device_output_args());
        self.output_args.push(output);
        Ok(())
    }
}

pub struct Output {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub seconds: f64,
    pub bytes: u64,
    pub warnings: Vec<String>,
    pub encoder: String,
    pub subtitles_file: Option<String>,
    pub subtitle_track: Option<String>,
}
pub enum Progress {
    Loudness {
        pass: usize,
        processed_ms: i64,
        total_ms: i64,
    },
    Audio {
        processed_ms: i64,
        total_ms: i64,
    },
    Sound {
        done: usize,
        total: usize,
    },
    Planned {
        width: u32,
        height: u32,
        fps: f64,
        seconds: f64,
    },
    Preparing {
        done: usize,
        total: usize,
    },
    Encoder {
        name: String,
    },
    Encoding {
        processed_ms: i64,
        total_ms: i64,
    },
}

pub fn available() -> Result<(), String> {
    browser::executable().map(|_| ())
}

/// The planner emits opaque media IDs. Resolve only images registered in this saved project.
async fn embed_images(
    html: &str,
    project: &Value,
    paths: &BTreeMap<String, String>,
    cache: &mut BTreeMap<String, String>,
) -> Result<String, String> {
    let pattern = regex::Regex::new(r"biliup-media:([A-Za-z0-9._-]+)").unwrap();
    for found in pattern.captures_iter(html) {
        let id = &found[1];
        if cache.contains_key(id) {
            continue;
        }
        if !project["media"]
            .as_array()
            .is_some_and(|media| media.iter().any(|m| m["id"] == id && m["kind"] == "image"))
        {
            return Err("图形引用的图片不存在".into());
        }
        let path = paths.get(id).ok_or("图片文件不存在")?;
        let path = Path::new(path);
        if tokio::fs::metadata(path)
            .await
            .map_err(|e| e.to_string())?
            .len()
            > 32 * 1024 * 1024
        {
            return Err("图形嵌入图片不能超过 32 MiB".into());
        }
        let mime = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "tif" | "tiff" => "image/tiff",
            _ => return Err("图形嵌入图片格式无效".into()),
        };
        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        cache.insert(
            id.to_string(),
            format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
        );
    }
    Ok(pattern
        .replace_all(html, |m: &regex::Captures<'_>| cache[&m[1]].clone())
        .into_owned())
}

fn fonts() -> Value {
    let data = |bytes: &[u8]| format!("data:font/woff2;base64,{}", STANDARD.encode(bytes));
    json!({
        "DMSans-Variable.woff2":data(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../public/editor-fonts/DMSans-Variable.woff2"))),
        "DMMono-Regular.woff2":data(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../public/editor-fonts/DMMono-Regular.woff2"))),
        "DMMono-Medium.woff2":data(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../public/editor-fonts/DMMono-Medium.woff2")))
    })
}

/// Paths are resolved exclusively from registered sources. No graph or filesystem path is accepted from HTTP clients.
#[cfg(test)]
pub async fn render(
    project: Value,
    paths: BTreeMap<String, String>,
    options: Options,
    cache_root: &Path,
    output: &Path,
    cancel: &CancellationToken,
    report: impl FnMut(Progress),
) -> Result<Output, String> {
    render_graphics(
        RenderProject {
            project,
            paths,
            graphics: BTreeMap::new(),
            assets: super::fonts::RenderAssets {
                fonts: super::fonts::bundled(),
                logos: BTreeMap::new(),
                warnings: vec![],
            },
        },
        options,
        cache_root,
        output,
        cancel,
        None,
        report,
    )
    .await
}
pub async fn render_graphics(
    RenderProject {
        project,
        paths,
        graphics,
        assets,
    }: RenderProject,
    options: Options,
    cache_root: &Path,
    output: &Path,
    cancel: &CancellationToken,
    prepared_browser: Option<browser::Browser>,
    mut report: impl FnMut(Progress),
) -> Result<Output, String> {
    let _permit = crate::server::common::processing::transcode::acquire(cancel).await?;
    if !matches!(options.quality.as_str(), "draft" | "standard" | "high") {
        return Err("导出质量无效".into());
    }
    let target_lufs = options.target_lufs;
    if target_lufs.is_some_and(|target| !target.is_finite() || !(-70. ..=-5.).contains(&target)) {
        return Err("响度目标无效".into());
    }
    let options = json!({"quality":options.quality,"video":options.video,"range":options.range,
        "burnCaptions":options.burn_captions.unwrap_or(true),"motionBlur":options.motion_blur,"subtitles":options.subtitles});
    let mut paths = paths
        .into_iter()
        .map(|(id, path)| {
            std::path::absolute(path)
                .map(|path| (id, path.to_string_lossy().into_owned()))
                .map_err(|e| e.to_string())
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let parent = output.parent().ok_or("导出目录无效")?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| e.to_string())?;
    let temp = tempfile::Builder::new()
        .prefix("editor-export-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let mut browser = match prepared_browser {
        Some(browser) => browser,
        None => browser::Browser::launch(cancel).await?,
    };
    let result = async {
        recording_effects::prepare_mask_images(&mut browser, &project, &options["range"], &mut paths, temp.path()).await?;
        let sources = browser
            .eval(&format!(
                "BiliupExport.envelopeSources({project},{})",
                options["range"]
            ))
            .await?;
        let sources = sources.as_array().ok_or("音频分析计划无效")?;
        let mut envelopes = BTreeMap::new();
        for (index, source) in sources.iter().enumerate() {
            report(Progress::Sound {
                done: index,
                total: sources.len(),
            });
            let id = source["id"].as_str().ok_or("音频素材标识无效")?;
            let value = if source["hasAudio"] == false {
                json!({"hasAudio":false})
            } else {
                let path = paths.get(id).ok_or("音频素材不可读取")?;
                let seconds = source["seconds"]
                    .as_f64()
                    .filter(|v| v.is_finite() && *v > 0.)
                    .ok_or("音频素材时长无效")?;
                super::analysis::measure(cache_root, Path::new(path), seconds, cancel, |_| {})
                    .await?
            };
            envelopes.insert(id, value);
            report(Progress::Sound {
                done: index + 1,
                total: sources.len(),
            });
        }
        let raw = browser
            .eval(&format!(
                "BiliupExport.prepare({project},{},{options},{},{},{})",
                serde_json::to_string(&paths).unwrap(),
                serde_json::to_string(&envelopes).unwrap(),serde_json::to_string(&graphics).unwrap(),serde_json::to_string(&assets).unwrap()
            ))
            .await?;
        let plan: Plan = serde_json::from_value(raw).map_err(|e| format!("导出计划无效：{e}"))?;
        if plan.frames == 0
            || !plan.fps.is_finite()
            || plan.fps <= 0.
            || plan.canvas.width > 8192
            || plan.canvas.height > 8192
        {
            return Err("导出范围或画布尺寸无效".into());
        }
        report(Progress::Planned {
            width: plan.width,
            height: plan.height,
            fps: options["video"]["fps"].as_f64().unwrap_or(plan.fps),
            seconds: plan.frames as f64 / plan.fps,
        });
        browser
            .viewport(plan.canvas.width, plan.canvas.height)
            .await?;
        browser
            .eval(&format!("globalThis.biliupFonts={}", fonts()))
            .await?;
        let total = 1
            + plan.graphics.iter().map(graphics::Graphic::len).sum::<usize>()
            + plan.rasters.iter().map(|r| r.states.len()).sum::<usize>()
            + plan.shaders.iter().map(|s| s.frames).sum::<usize>();
        let mut done = 0;
        let mut rasters = BTreeMap::new();
        let mut embedded_images=BTreeMap::new();
        // Every distinct look is drawn once; sequences link those images without re-encoding identical frames.
        for (index, states, frames) in
            std::iter::once((usize::MAX, vec![plan.background.clone()], vec![0])).chain(
                plan.rasters
                    .iter()
                    .map(|r| (r.index, r.states.clone(), r.frames.clone())),
            )
        {
            let name = if index == usize::MAX {
                "background".into()
            } else {
                format!("r{index}")
            };
            for (state, html) in states.iter().enumerate() {
                let html=embed_images(html,&project,&paths,&mut embedded_images).await?;
                browser
                    .eval(&format!(
                        "BiliupExport.draw({},globalThis.biliupFonts)",
                        serde_json::to_string(&html).unwrap()
                    ))
                    .await?;
                let png = browser.png().await?;
                tokio::fs::write(temp.path().join(format!("{name}-s{state}.png")), png)
                    .await
                    .map_err(|e| e.to_string())?;
                done += 1;
                report(Progress::Preparing { done, total });
            }
            if index == usize::MAX {
                continue;
            }
            if states.len() == 1 {
                rasters.insert(
                    index,
                    json!({"kind":"still","png":format!("{name}-s0.png")}),
                );
            } else {
                for (frame, state) in frames.iter().enumerate() {
                    if cancel.is_cancelled() {
                        return Err("导出已取消".into());
                    }
                    if *state >= states.len() {
                        return Err("导出图层帧索引无效".into());
                    }
                    let from = temp.path().join(format!("{name}-s{state}.png"));
                    let to = temp.path().join(format!("{name}-{frame:08}.png"));
                    if tokio::fs::hard_link(&from, &to).await.is_err() {
                        tokio::fs::copy(from, to).await.map_err(|e| e.to_string())?;
                    }
                }
                rasters.insert(
                    index,
                    json!({"kind":"sequence","pattern":format!("{name}-%08d.png")}),
                );
            }
        }
        rasters.extend(graphics::render(&mut browser, graphics::Scene { graphics: &plan.graphics, width: plan.canvas.width, height: plan.canvas.height, project: &project, paths: &paths }, temp.path(), cancel, ||{done+=1;report(Progress::Preparing{done,total})}).await?);
        let shaded = shaders::render(
            &mut browser,
            &plan.shaders,
            plan.fps,
            temp.path(),
            cancel,
            || {
                done += 1;
                report(Progress::Preparing { done, total });
            },
        )
        .await?;
        let mut luts = BTreeMap::new();
        for (index, lut) in plan.luts.iter().enumerate() {
            let name = format!("lut-{index}.cube");
            tokio::fs::write(temp.path().join(&name), &lut.text)
                .await
                .map_err(|e| e.to_string())?;
            luts.insert(&lut.key, name);
        }
        if let Some(target) = target_lufs {
            estimate::measure(&mut browser,target,temp.path(),cancel,false,&mut report).await?;
        }
        let prepared_audio = audio::prepare_range(&mut browser, temp.path(), cancel, &mut report).await?;
        let hardware = options["video"]["encoder"]
            .as_str()
            .map(|value| value == "hardware")
            .unwrap_or(options["quality"] != "high");
        let mut warnings = plan.warnings.clone();
        let codec = if options["video"]["codec"] == "hevc" { encoding::VideoCodec::Hevc } else { encoding::VideoCodec::H264 };
        let mut choices = vec![];
        if hardware {
            for backend in encoding::HARDWARE_ORDER {
                match encoding::candidates(backend, None, cancel).await {
                    Ok(devices) => choices.extend(devices),
                    Err(_) if cancel.is_cancelled() => return Err("导出已取消".into()),
                    Err(error) => warnings.push(format!("无法查询 {} 设备：{error}", backend.name())),
                }
            }
        }
        choices.push(encoding::SelectedEncoder { encoder: encoding::VideoEncoder::Libx264, device: None, fallback_reason: None });
        if let Some(subtitles) = &plan.subtitles.soft {
            tokio::fs::write(temp.path().join("subtitles.srt"), subtitles)
                .await
                .map_err(|e| e.to_string())?;
        }
        let total_ms = (plan.frames as f64 / plan.fps * 1000.).round() as i64;
        let output_fps = options["video"]["fps"].as_f64().unwrap_or(plan.fps).to_string();
        let mut completed_encoder = None;
        for selected in choices {
            if cancel.is_cancelled() { return Err("导出已取消".into()); }
            let name = selected.codec_name(codec);
            let is_hardware = selected.encoder != encoding::VideoEncoder::Libx264;
            report(Progress::Encoder { name: name.into() });
            let raw = browser
                .eval(&format!(
                    "BiliupExport.command('background-s0.png',{}, {}, 'output.mp4',{}, {},{prepared_audio})",
                    serde_json::to_string(&rasters).unwrap(),
                    serde_json::to_string(&luts).unwrap(),
                    serde_json::to_string(&[name]).unwrap(),
                    serde_json::to_string(&shaded).unwrap()
                ))
                .await?;
            let mut encode: Encode = serde_json::from_value(raw).map_err(|e| e.to_string())?;
            let planned_encoder = encode
                .video_args
                .windows(2)
                .find(|pair| pair[0] == "-c:v")
                .map(|pair| pair[1].as_str())
                .ok_or("未选择视频编码器")?;
            if planned_encoder != name { return Err("导出计划与所选编码器不一致".into()); }
            if is_hardware {
                let mut probe_args = encode.video_args.clone();
                probe_args.extend(selected.device_output_args());
                if let Err(error) = encoding::test_encoder_with_args(&selected, codec, (plan.width, plan.height), &output_fps, &probe_args, cancel).await {
                    if cancel.is_cancelled() { return Err("导出已取消".into()); }
                    warnings.push(format!("编码器 {name}{} 不可用，尝试其他编码器：{error}", selected.device.as_ref().map_or_else(String::new, |device| format!(" ({device})"))));
                    continue;
                }
                encode.use_device(&selected)?;
            }
            tokio::fs::write(temp.path().join("graph.txt"), encode.graph)
                .await
                .map_err(|e| e.to_string())?;
            let mut command = crate::tools::ffmpeg_command();
            command
                .current_dir(temp.path())
                .args([
                    "-hide_banner",
                    "-nostdin",
                    "-y",
                    "-nostats",
                    "-progress",
                    "pipe:1",
                    "-filter_complex_threads",
                    "1",
                ])
                .args(selected.input_args())
                .args(encode.inputs)
                .args(["-filter_complex_script", "graph.txt"])
                .args(encode.output_args);
            report(Progress::Encoding {
                processed_ms: 0,
                total_ms,
            });
            match engine::run_process(
                command,
                cancel,
                Duration::from_secs(180),
                |processed_ms, _| {
                    report(Progress::Encoding {
                        processed_ms,
                        total_ms,
                    })
                },
            )
            .await
            {
                Ok(stderr) => {
                    if stderr.contains("Failed to inject frame")
                        || stderr.contains("Error while filtering")
                    {
                        return Err(stderr);
                    }
                    completed_encoder = Some(name.to_owned());
                    break;
                }
                Err(error) if !cancel.is_cancelled() && is_hardware => {
                    warnings.push(format!(
                        "硬件编码器 {name}{} 未完成，尝试其他编码器：{error}",
                        selected.device.as_ref().map_or_else(String::new, |device| format!(" ({device})"))
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        let actual_encoder = completed_encoder.ok_or("没有可用的视频编码器")?;
        if hardware && actual_encoder.starts_with("lib") {
            warnings.push("没有可用的硬件编码器，本次使用软件编码".into());
        }
        let encoded = temp.path().join("output.mp4");
        let bytes = tokio::fs::metadata(&encoded)
            .await
            .map_err(|e| e.to_string())?
            .len();
        if bytes == 0 {
            return Err("导出文件为空".into());
        }
        if cancel.is_cancelled() {
            return Err("导出已取消".into());
        }
        // Sidecars belong to the frozen export, not the current revision of its project.
        let sidecar = if let Some(subtitles) = &plan.subtitles.file {
            if !matches!(subtitles.format.as_str(), "srt" | "vtt") {
                return Err("字幕格式无效".into());
            }
            let staged = temp.path().join(format!("sidecar.{}", subtitles.format));
            tokio::fs::write(&staged, &subtitles.text)
                .await
                .map_err(|e| e.to_string())?;
            let destination = output.with_extension(&subtitles.format);
            tokio::fs::hard_link(staged, &destination)
                .await
                .map_err(|e| e.to_string())?;
            Some(destination)
        } else {
            None
        };
        // Never replace a recording or a previous result. Every job owns a unique destination.
        if let Err(error) = tokio::fs::hard_link(&encoded, output).await {
            if let Some(path) = sidecar {
                let _ = tokio::fs::remove_file(path).await;
            }
            return Err(format!("保存导出文件失败：{error}"));
        }
        Ok(Output {
            width: plan.width,
            height: plan.height,
            fps: options["video"]["fps"].as_f64().unwrap_or(plan.fps),
            seconds: plan.frames as f64 / plan.fps,
            bytes,
            warnings,
            encoder: actual_encoder,
            subtitles_file: plan.subtitles.file.map(|file| file.format),
            subtitle_track: plan.subtitles.language,
        })
    }
    .await;
    browser.close().await;
    result
}

#[cfg(test)]
mod tests;
