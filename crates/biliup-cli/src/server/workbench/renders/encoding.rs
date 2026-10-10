//! H.264 encoder selection. Every advertised encoder has encoded real frames;
//! FFmpeg's compiled-in encoder list alone does not prove a working driver.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const SUCCESS_TTL: Duration = Duration::from_secs(300);
const FAILURE_TTL: Duration = Duration::from_secs(30);
const DEVICE_QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const DEVICE_QUERY_MAX_BYTES: usize = 4096;
const DEVICE_QUERY_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoEncoder {
    #[default]
    Auto,
    Libx264,
    H264Nvenc,
    H264Qsv,
    H264Vaapi,
    H264Amf,
}

impl VideoEncoder {
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Libx264 => "libx264",
            Self::H264Nvenc => "h264_nvenc",
            Self::H264Qsv => "h264_qsv",
            Self::H264Vaapi => "h264_vaapi",
            Self::H264Amf => "h264_amf",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Auto => "自动选择",
            Self::Libx264 => "CPU · H.264 (libx264)",
            Self::H264Nvenc => "NVIDIA · H.264 (NVENC)",
            Self::H264Qsv => "Intel · H.264 (QSV)",
            Self::H264Vaapi => "Intel / AMD · H.264 (VAAPI)",
            Self::H264Amf => "AMD · H.264 (AMF)",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EncoderConfig {
    pub encoder: VideoEncoder,
    /// DRM render node for VAAPI/Linux QSV; adapter index for NVENC/Windows QSV.
    pub device: Option<String>,
}

impl EncoderConfig {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(device) = &self.device {
            // Hardware device specifications also accept comma-separated
            // options. Restrict this field to a single device, never options.
            if device.is_empty()
                || device.len() > 256
                || device.trim() != device
                || device
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ',' | '=' | ':' | ';'))
            {
                return Err("编码设备无效，请选择 GPU 序号或 DRM render 节点".into());
            }
            if self.encoder == VideoEncoder::Libx264 {
                return Err("CPU 编码器不接受 GPU 设备配置".into());
            }
            if self.encoder == VideoEncoder::Auto
                && !(is_adapter_index(device) || is_render_node(device))
            {
                return Err("编码设备必须是 GPU 序号或 /dev/dri/ 下的 render 节点".into());
            }
            if self.encoder == VideoEncoder::H264Nvenc && !is_adapter_index(device) {
                return Err("NVENC 设备必须是非负 GPU 序号，例如 0".into());
            }
            if self.encoder == VideoEncoder::H264Vaapi && !is_render_node(device) {
                return Err("VAAPI 设备必须是 /dev/dri/ 下的 render 节点".into());
            }
            if matches!(self.encoder, VideoEncoder::H264Qsv | VideoEncoder::H264Amf)
                && !(is_adapter_index(device) || is_render_node(device))
            {
                return Err("编码设备必须是 GPU 序号或 /dev/dri/ 下的 render 节点".into());
            }
            if self.encoder == VideoEncoder::H264Qsv
                && ((cfg!(target_os = "linux") && !is_render_node(device))
                    || (cfg!(windows) && !is_adapter_index(device)))
            {
                return Err(
                    "QSV 设备在 Linux 上应为 DRM render 节点，Windows 上应为 GPU 序号".into(),
                );
            }
            if self.encoder == VideoEncoder::H264Amf && !cfg!(windows) {
                return Err("此平台的 AMF 使用驱动默认 GPU；指定 DRM 节点请使用 VAAPI".into());
            }
            if self.encoder == VideoEncoder::H264Amf && !is_adapter_index(device) {
                return Err("Windows AMF 设备必须是非负 GPU 序号".into());
            }
        }
        Ok(())
    }
}

fn is_adapter_index(device: &str) -> bool {
    !device.is_empty()
        && device.bytes().all(|b| b.is_ascii_digit())
        && device
            .parse::<u32>()
            .is_ok_and(|index| index <= i32::MAX as u32)
}

fn is_render_node(device: &str) -> bool {
    device
        .strip_prefix("/dev/dri/renderD")
        .is_some_and(is_adapter_index)
}

#[derive(Debug, Clone, Serialize)]
pub struct EncoderCapability {
    pub name: String,
    pub label: String,
    pub available: bool,
    pub device: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SelectedEncoder {
    pub encoder: VideoEncoder,
    pub device: Option<String>,
    pub fallback_reason: Option<String>,
}

impl SelectedEncoder {
    /// Device creation precedes all inputs; software filters run first and
    /// only their final result is uploaded to the encoder's hardware context.
    pub fn input_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![];
        let mut add = |key: &str, value: String| args.extend([key.to_owned(), value]);
        match self.encoder {
            VideoEncoder::H264Vaapi => {
                add(
                    "-init_hw_device",
                    self.device.as_ref().map_or_else(
                        || "vaapi=render".into(),
                        |device| format!("vaapi=render:{device}"),
                    ),
                );
                add("-filter_hw_device", "render".into());
            }
            VideoEncoder::H264Qsv => {
                if let Some(device) = self.device.as_ref().filter(|d| is_render_node(d)) {
                    add("-init_hw_device", format!("vaapi=render:{device}"));
                    add("-init_hw_device", "qsv=encoder@render".into());
                } else {
                    let suffix = self.device.as_ref().map_or_else(String::new, |device| {
                        format!(",child_device={device},child_device_type=d3d11va")
                    });
                    add("-init_hw_device", format!("qsv=encoder:hw{suffix}"));
                }
                add("-filter_hw_device", "encoder".into());
            }
            VideoEncoder::H264Amf if self.device.is_some() => {
                add(
                    "-init_hw_device",
                    format!("d3d11va=render:{}", self.device.as_deref().unwrap()),
                );
                add("-filter_hw_device", "render".into());
            }
            _ => {}
        }
        args
    }

    pub fn video_filter(&self) -> &'static str {
        match self.encoder {
            VideoEncoder::H264Vaapi => "format=nv12,hwupload",
            VideoEncoder::H264Qsv => "format=nv12,hwupload=extra_hw_frames=64",
            VideoEncoder::H264Amf if self.device.is_some() => "format=nv12,hwupload",
            VideoEncoder::H264Nvenc | VideoEncoder::H264Amf => "format=nv12",
            _ => "format=yuv420p",
        }
    }

    pub fn output_args(&self) -> Vec<String> {
        let mut args = vec!["-c:v".into(), self.encoder.name().into()];
        let options: &[&str] = match self.encoder {
            VideoEncoder::Libx264 | VideoEncoder::Auto => &[
                "-preset", "veryfast", "-crf", "20", "-threads", "2", "-pix_fmt", "yuv420p",
            ],
            VideoEncoder::H264Nvenc => &[
                "-preset",
                "p4",
                "-rc",
                "vbr",
                "-cq",
                "20",
                "-b:v",
                "0",
                "-profile:v",
                "high",
            ],
            VideoEncoder::H264Qsv => &[
                "-preset",
                "veryfast",
                "-global_quality",
                "20",
                "-profile:v",
                "high",
            ],
            VideoEncoder::H264Vaapi => &["-rc_mode", "CQP", "-qp", "20", "-profile:v", "high"],
            VideoEncoder::H264Amf => &[
                "-quality",
                "speed",
                "-rc",
                "cqp",
                "-qp_i",
                "20",
                "-qp_p",
                "20",
                "-profile:v",
                "high",
            ],
        };
        args.extend(options.iter().map(|option| (*option).to_owned()));
        if self.encoder == VideoEncoder::H264Nvenc {
            if let Some(device) = &self.device {
                args.extend(["-gpu".into(), device.clone()]);
            }
        }
        args
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProbeKey {
    program: PathBuf,
    program_version: Option<(u64, std::time::SystemTime)>,
    encoder: VideoEncoder,
    device: Option<String>,
    width: u32,
    height: u32,
    frame_rate: String,
}

type ProbeResults = HashMap<ProbeKey, (Instant, Result<(), String>)>;
static PROBE_CACHE: OnceLock<Mutex<ProbeResults>> = OnceLock::new();
static PROBE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn cached(key: &ProbeKey) -> Option<Result<(), String>> {
    PROBE_CACHE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key)
        .filter(|(at, result)| {
            at.elapsed()
                < if result.is_ok() {
                    SUCCESS_TTL
                } else {
                    FAILURE_TTL
                }
        })
        .map(|(_, result)| result.clone())
}

async fn test_encoder(
    selected: &SelectedEncoder,
    target: (u32, u32),
    frame_rate: &str,
    cancel: &CancellationToken,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    let program = crate::tools::ffmpeg();
    let metadata = std::fs::metadata(&program).ok();
    let key = ProbeKey {
        program,
        program_version: metadata
            .and_then(|m| m.modified().ok().map(|modified| (m.len(), modified))),
        encoder: selected.encoder,
        device: selected.device.clone(),
        width: target.0,
        height: target.1,
        frame_rate: frame_rate.to_owned(),
    };
    if let Some(result) = cached(&key) {
        return result;
    }
    // Serialize the very short probes, including concurrent capability/job
    // requests, so probing cannot exhaust hardware encoding session limits.
    let _guard = tokio::select! {
        _ = cancel.cancelled() => return Err("合成已取消".into()),
        guard = PROBE_LOCK.lock() => guard,
    };
    if let Some(result) = cached(&key) {
        return result;
    }
    let temp = tempfile::tempdir().map_err(|error| format!("创建编码探测目录失败：{error}"))?;
    let mut command = crate::tools::low_priority_ffmpeg_command();
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-filter_threads",
            "1",
        ])
        .args(selected.input_args())
        .args([
            "-f",
            "lavfi",
            "-i",
            &format!("color=s={}x{}:r={frame_rate}", target.0, target.1),
        ])
        .args(["-vf", selected.video_filter(), "-frames:v", "8", "-an"])
        .args(selected.output_args())
        .args(["-f", "mp4", "-y"])
        .arg(temp.path().join("probe.mp4"));
    // The probe emits no FFmpeg progress, making run_process's watchdog an
    // absolute timeout here. It also kills and reaps on timeout/cancellation.
    let mut outcome = super::engine::run_process(command, cancel, PROBE_TIMEOUT, |_, _| {})
        .await
        .map(|_| ());
    if outcome.is_ok() {
        let mut decode = crate::tools::low_priority_ffmpeg_command();
        decode
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-xerror",
                "-c:v",
                "h264",
                "-i",
            ])
            .arg(temp.path().join("probe.mp4"))
            .args(["-map", "0:v:0", "-frames:v", "8", "-an", "-f", "null", "-"]);
        outcome = super::engine::run_process(decode, cancel, PROBE_TIMEOUT, |_, _| {})
            .await
            .map(|_| ());
    }
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    let outcome = outcome
        .and_then(|()| {
            std::fs::metadata(temp.path().join("probe.mp4"))
                .map_err(|error| format!("未生成编码探测视频：{error}"))
                .and_then(|metadata| {
                    if metadata.len() > 0 {
                        Ok(())
                    } else {
                        Err("编码探测输出为空".into())
                    }
                })
        })
        .map_err(|error| compact_error(&error));
    let mut cache = PROBE_CACHE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if cache.len() >= 128 {
        cache.retain(|_, (at, _)| at.elapsed() < FAILURE_TTL);
        if cache.len() >= 128 {
            cache.clear();
        }
    }
    cache.insert(key, (Instant::now(), outcome.clone()));
    outcome
}

fn compact_error(error: &str) -> String {
    let error = error.trim();
    let chars: Vec<_> = error.chars().collect();
    if chars.len() > 1000 {
        format!(
            "…{}",
            chars[chars.len() - 1000..].iter().collect::<String>()
        )
    } else {
        error.into()
    }
}

/// DRM nodes are shared by Intel QSV/VAAPI and AMD VAAPI. Unknown-vendor
/// nodes are still tested rather than assuming a driver is unsupported.
fn render_nodes(intel_only: bool) -> Vec<String> {
    let mut devices: Vec<_> = std::fs::read_dir("/dev/dri")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path().to_string_lossy().into_owned();
            if !is_render_node(&path) {
                return None;
            }
            let vendor = std::fs::read_to_string(format!("/sys/class/drm/{name}/device/vendor"))
                .unwrap_or_default();
            if intel_only && !vendor.trim().is_empty() && vendor.trim() != "0x8086" {
                return None;
            }
            Some(path)
        })
        .collect();
    devices.sort();
    devices.truncate(16);
    devices
}

/// NVIDIA adapter indices are obtained from the driver, then each adapter is
/// genuinely probed with -gpu. A default-only capability would incorrectly
/// reject a saved configuration targeting another working adapter.
fn parse_nvidia_indexes(text: &str) -> Vec<String> {
    let mut indexes: Vec<u32> = text
        .lines()
        .filter_map(|line| {
            let index = line.trim();
            is_adapter_index(index)
                .then(|| index.parse::<u32>().ok())
                .flatten()
        })
        .collect();
    indexes.sort_unstable();
    indexes.dedup();
    indexes.truncate(16);
    indexes.into_iter().map(|index| index.to_string()).collect()
}

async fn query_nvidia_indexes(
    mut command: Command,
    cancel: &CancellationToken,
) -> Result<Vec<String>, String> {
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let Ok(mut child) = command.spawn() else {
        return Ok(vec![]);
    };
    let stdout = child.stdout.take().expect("piped NVIDIA query stdout");
    let outcome = tokio::select! {
        _ = cancel.cancelled() => Err("合成已取消".to_owned()),
        result = tokio::time::timeout(DEVICE_QUERY_TIMEOUT, async {
            let read = async {
                let mut bytes = vec![];
                stdout.take((DEVICE_QUERY_MAX_BYTES + 1) as u64).read_to_end(&mut bytes).await?;
                Ok::<_, std::io::Error>(bytes)
            };
            tokio::try_join!(read, child.wait())
        }) => match result {
            Ok(Ok((bytes, status))) if status.success() && bytes.len() <= DEVICE_QUERY_MAX_BYTES =>
                Ok(parse_nvidia_indexes(&String::from_utf8_lossy(&bytes))),
            _ => Ok(vec![]),
        },
    };
    // On query timeout, oversized output, or cancellation, do not leave the
    // driver utility running after returning the fallback candidates.
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    outcome
}

type DeviceQueryCache = Option<(Instant, Vec<String>)>;
static NVIDIA_DEVICES: OnceLock<Mutex<DeviceQueryCache>> = OnceLock::new();
static DEVICE_QUERY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn cached_nvidia_indexes() -> Option<Vec<String>> {
    NVIDIA_DEVICES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .filter(|(at, _)| at.elapsed() < DEVICE_QUERY_TTL)
        .map(|(_, indexes)| indexes.clone())
}

async fn nvidia_indexes(cancel: &CancellationToken) -> Result<Vec<String>, String> {
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    if let Some(indexes) = cached_nvidia_indexes() {
        return Ok(indexes);
    }
    let _guard = tokio::select! {
        _ = cancel.cancelled() => return Err("合成已取消".into()),
        guard = DEVICE_QUERY_LOCK.lock() => guard,
    };
    if let Some(indexes) = cached_nvidia_indexes() {
        return Ok(indexes);
    }
    let mut command = crate::tools::command("nvidia-smi");
    command.args(["--query-gpu=index", "--format=csv,noheader,nounits"]);
    let indexes = query_nvidia_indexes(command, cancel).await?;
    *NVIDIA_DEVICES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), indexes.clone()));
    Ok(indexes)
}

async fn candidates(
    encoder: VideoEncoder,
    requested_device: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Vec<SelectedEncoder>, String> {
    if let Some(device) = requested_device {
        let config = EncoderConfig {
            encoder,
            device: Some(device.to_owned()),
        };
        return Ok(if config.validate().is_ok() {
            vec![SelectedEncoder {
                encoder,
                device: config.device,
                fallback_reason: None,
            }]
        } else {
            vec![]
        });
    }
    let nodes = match encoder {
        VideoEncoder::H264Qsv => render_nodes(true),
        VideoEncoder::H264Vaapi => render_nodes(false),
        VideoEncoder::H264Nvenc => nvidia_indexes(cancel).await?,
        _ => vec![],
    };
    if nodes.is_empty() {
        Ok(vec![SelectedEncoder {
            encoder,
            device: None,
            fallback_reason: None,
        }])
    } else {
        Ok(nodes
            .into_iter()
            .map(|device| SelectedEncoder {
                encoder,
                device: Some(device),
                fallback_reason: None,
            })
            .collect())
    }
}

const HARDWARE_ORDER: [VideoEncoder; 4] = [
    VideoEncoder::H264Nvenc,
    VideoEncoder::H264Qsv,
    VideoEncoder::H264Vaapi,
    VideoEncoder::H264Amf,
];

pub(super) async fn select(
    config: &EncoderConfig,
    target: (u32, u32),
    frame_rate: &str,
    cancel: &CancellationToken,
) -> Result<SelectedEncoder, String> {
    config.validate()?;
    let order = if config.encoder == VideoEncoder::Auto {
        HARDWARE_ORDER.to_vec()
    } else {
        vec![config.encoder]
    };
    let mut errors = vec![];
    for encoder in order {
        for selected in candidates(encoder, config.device.as_deref(), cancel).await? {
            match test_encoder(&selected, target, frame_rate, cancel).await {
                Ok(()) => return Ok(selected),
                Err(_) if cancel.is_cancelled() => return Err("合成已取消".into()),
                Err(error) => errors.push(format!(
                    "{}{}：{error}",
                    encoder.name(),
                    selected
                        .device
                        .as_ref()
                        .map_or_else(String::new, |d| format!(" ({d})"))
                )),
            }
        }
    }
    if config.encoder != VideoEncoder::Auto {
        return Err(format!(
            "指定编码器 {} 不可用（{}×{} @ {frame_rate}）：{}",
            config.encoder.name(),
            target.0,
            target.1,
            errors.join("；")
        ));
    }
    let mut cpu = SelectedEncoder {
        encoder: VideoEncoder::Libx264,
        device: None,
        fallback_reason: Some(format!(
            "GPU 编码不可用，已回退 CPU：{}",
            compact_error(&errors.join("；"))
        )),
    };
    if errors.is_empty() {
        cpu.fallback_reason = Some("所选设备没有可用 GPU 编码器，已回退 CPU".into());
    }
    test_encoder(&cpu, target, frame_rate, cancel)
        .await
        .map_err(|error| format!("GPU 编码不可用，CPU 编码也失败：{error}"))?;
    Ok(cpu)
}

pub async fn capabilities(cancel: &CancellationToken) -> Vec<EncoderCapability> {
    let mut status = vec![];
    for encoder in std::iter::once(VideoEncoder::Libx264).chain(HARDWARE_ORDER) {
        let Ok(candidates) = candidates(encoder, None, cancel).await else {
            return status;
        };
        for selected in candidates {
            let result = test_encoder(&selected, (640, 360), "30/1", cancel).await;
            status.push(EncoderCapability {
                name: encoder.name().into(),
                label: encoder.label().into(),
                available: result.is_ok(),
                device: selected.device,
                error: result.err(),
            });
            if cancel.is_cancelled() {
                return status;
            }
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_adapter_indexes_are_validated_sorted_deduplicated_and_bounded() {
        assert_eq!(
            parse_nvidia_indexes("1\n 0 \n1\n001\nN/A\n-1\n2147483648\n2, name\n"),
            vec!["0", "1"]
        );
        let many = (0..24)
            .rev()
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let indexes = parse_nvidia_indexes(&many);
        assert_eq!(indexes.len(), 16);
        assert_eq!(indexes[0], "0");
        assert_eq!(indexes[15], "15");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nvidia_adapter_query_handles_real_output_missing_tools_and_cancellation() {
        let cancel = CancellationToken::new();
        let mut command = crate::tools::command("sh");
        command.args(["-c", "printf '2\\n0\\n2\\n'"]);
        assert_eq!(
            query_nvidia_indexes(command, &cancel).await.unwrap(),
            vec!["0", "2"]
        );
        let missing = crate::tools::command("__biliup_missing_nvidia_query__");
        assert!(
            query_nvidia_indexes(missing, &cancel)
                .await
                .unwrap()
                .is_empty()
        );
        cancel.cancel();
        assert!(
            query_nvidia_indexes(crate::tools::command("sh"), &cancel)
                .await
                .unwrap_err()
                .contains("取消")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nvidia_adapter_query_bounds_output_and_reaps_timed_out_process() {
        let mut oversized = crate::tools::command("sh");
        oversized.args(["-c", "printf '%5000s' ''"]);
        assert!(
            query_nvidia_indexes(oversized, &CancellationToken::new())
                .await
                .unwrap()
                .is_empty()
        );
        let temp = tempfile::tempdir().unwrap();
        let pid_file = temp.path().join("query.pid");
        let mut stalled = crate::tools::command("sh");
        stalled
            .args(["-c", "echo $$ > \"$1\"; exec sleep 30", "sh"])
            .arg(&pid_file);
        let started = Instant::now();
        assert!(
            query_nvidia_indexes(stalled, &CancellationToken::new())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        let pid = std::fs::read_to_string(pid_file)
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap();
        // SAFETY: signal 0 only checks whether the child remains alive; the
        // query helper must have killed and reaped its own timed-out process.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[tokio::test]
    async fn explicitly_selected_nvenc_adapter_does_not_depend_on_discovery() {
        let selected = candidates(
            VideoEncoder::H264Nvenc,
            Some("1"),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].device.as_deref(), Some("1"));
        assert!(
            selected[0]
                .output_args()
                .windows(2)
                .any(|pair| pair == ["-gpu", "1"])
        );
    }

    #[test]
    fn public_config_has_stable_names_and_rejects_device_option_injection() {
        assert_eq!(
            serde_json::to_value(EncoderConfig::default()).unwrap(),
            serde_json::json!({"encoder":"auto", "device": null})
        );
        for encoder in [
            VideoEncoder::Libx264,
            VideoEncoder::H264Nvenc,
            VideoEncoder::H264Qsv,
            VideoEncoder::H264Vaapi,
            VideoEncoder::H264Amf,
        ] {
            let value = serde_json::to_value(encoder).unwrap();
            assert_eq!(value, serde_json::json!(encoder.name()));
            assert_eq!(
                serde_json::from_value::<VideoEncoder>(value).unwrap(),
                encoder
            );
        }
        for device in [
            "",
            "0,gpu=1",
            "/dev/dri/renderD128:foo",
            " 0",
            "0\n",
            "2147483648",
        ] {
            let config = EncoderConfig {
                encoder: VideoEncoder::H264Nvenc,
                device: Some(device.into()),
            };
            assert!(config.validate().is_err(), "{device:?}");
        }
        assert!(
            EncoderConfig {
                encoder: VideoEncoder::H264Vaapi,
                device: Some("/etc/passwd".into())
            }
            .validate()
            .is_err()
        );
        assert!(
            EncoderConfig {
                encoder: VideoEncoder::Libx264,
                device: Some("0".into())
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn hardware_upload_follows_software_filters_and_nvenc_uses_its_adapter() {
        let selected = SelectedEncoder {
            encoder: VideoEncoder::H264Vaapi,
            device: Some("/dev/dri/renderD128".into()),
            fallback_reason: None,
        };
        assert_eq!(selected.video_filter(), "format=nv12,hwupload");
        assert_eq!(
            selected.input_args(),
            [
                "-init_hw_device",
                "vaapi=render:/dev/dri/renderD128",
                "-filter_hw_device",
                "render"
            ]
        );
        assert!(
            selected
                .output_args()
                .windows(2)
                .any(|pair| pair == ["-qp", "20"])
        );
        let nvenc = SelectedEncoder {
            encoder: VideoEncoder::H264Nvenc,
            device: Some("1".into()),
            fallback_reason: None,
        };
        assert_eq!(nvenc.video_filter(), "format=nv12");
        assert!(
            nvenc
                .output_args()
                .windows(2)
                .any(|pair| pair == ["-gpu", "1"])
        );
        let qsv = SelectedEncoder {
            encoder: VideoEncoder::H264Qsv,
            device: Some("/dev/dri/renderD128".into()),
            fallback_reason: None,
        };
        assert!(qsv.input_args().contains(&"qsv=encoder@render".into()));
        assert!(qsv.video_filter().ends_with("hwupload=extra_hw_frames=64"));
    }

    #[tokio::test]
    async fn unavailable_explicit_device_is_an_error_and_auto_falls_back_to_cpu() {
        let cancel = CancellationToken::new();
        let mut config = EncoderConfig {
            encoder: VideoEncoder::H264Vaapi,
            device: Some("/dev/dri/renderD2147483647".into()),
        };
        let error = select(&config, (128, 128), "30/1", &cancel)
            .await
            .unwrap_err();
        assert!(error.contains("指定编码器 h264_vaapi 不可用"), "{error}");
        config.encoder = VideoEncoder::Auto;
        let selected = select(&config, (128, 128), "30/1", &cancel).await.unwrap();
        assert_eq!(selected.encoder, VideoEncoder::Libx264);
        assert_eq!(selected.device, None);
        assert!(
            selected
                .fallback_reason
                .as_ref()
                .unwrap()
                .contains("回退 CPU")
        );
        // A cached result may not swallow cancellation.
        cancel.cancel();
        assert!(
            select(&config, (128, 128), "30/1", &cancel)
                .await
                .unwrap_err()
                .contains("取消")
        );
    }
}
