//! Inspect and repair incompatible AAC packet timelines in completed FLV files.

use crate::server::errors::{AppError, AppResult};
use crate::tools;
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use std::time::SystemTime;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::warn;

static REPAIR_SLOT: Semaphore = Semaphore::const_new(1);
const REPAIR_TIMEOUT: Duration = Duration::from_secs(3600);
const STDERR_TAIL: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioEncoding {
    Copy,
    RepairAac { non_increasing_packets: usize },
}

/// Some FLV sources emit ancillary-only AAC raw packets at the same timestamp as
/// the next audible frame. Copying those packets also copies their conflicting
/// timestamps, which causes stuttering in players that expect one audio frame per
/// packet. Scan only tag headers and the first two audio bytes; never load video
/// payloads or identify a broadcaster using a hard-coded packet fingerprint.
pub(crate) fn flv_audio_encoding(path: &Path) -> std::io::Result<AudioEncoding> {
    flv_audio_packet_timeline(path).map(|(encoding, _)| encoding)
}

fn flv_audio_packet_timeline(path: &Path) -> std::io::Result<(AudioEncoding, usize)> {
    let file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "录像的 FLV 标签不完整，无法安全检查音频时间戳",
        )
    };
    let mut header = [0; 9];
    reader.read_exact(&mut header)?;
    if &header[..3] != b"FLV" || header[3] != 1 {
        return Err(invalid());
    }
    let data_offset = u32::from_be_bytes(header[5..9].try_into().unwrap()) as u64;
    if data_offset < 9 || data_offset.checked_add(4).is_none_or(|end| end > length) {
        return Err(invalid());
    }
    reader.seek(SeekFrom::Start(data_offset))?;
    let mut previous_size = [0; 4];
    reader.read_exact(&mut previous_size)?;
    if u32::from_be_bytes(previous_size) != 0 {
        return Err(invalid());
    }
    let mut position = data_offset + 4;
    let mut last_aac_timestamp = None;
    let mut non_increasing_packets = 0;
    let mut audio_packets = 0;
    while position < length {
        if length - position < 15 {
            return Err(invalid());
        }
        let mut tag = [0; 11];
        reader.read_exact(&mut tag)?;
        let size = u32::from_be_bytes([0, tag[1], tag[2], tag[3]]) as u64;
        let end = position.checked_add(15 + size).ok_or_else(invalid)?;
        if end > length {
            return Err(invalid());
        }
        if tag[0] & 0x1f == 8 && size > 0 {
            let mut audio = [0; 2];
            reader.read_exact(&mut audio[..size.min(2) as usize])?;
            if audio[0] >> 4 == 10 {
                if size < 2 {
                    return Err(invalid());
                }
                // AAC sequence headers are repeated at segment/encoder boundaries
                // and are not audio samples. Only compare AAC raw-data packets.
                if audio[1] == 1 {
                    audio_packets += 1;
                    let timestamp = u32::from_be_bytes([tag[7], tag[4], tag[5], tag[6]]);
                    if last_aac_timestamp.is_some_and(|previous| {
                        let delta = timestamp.wrapping_sub(previous);
                        // Serial arithmetic permits a legitimate u32 rollover.
                        delta == 0 || delta >= 1 << 31
                    }) {
                        non_increasing_packets += 1;
                    }
                    last_aac_timestamp = Some(timestamp);
                }
            }
        }
        reader.seek(SeekFrom::Start(end - 4))?;
        reader.read_exact(&mut previous_size)?;
        if u32::from_be_bytes(previous_size) as u64 != size + 11 {
            return Err(invalid());
        }
        position = end;
    }
    Ok((
        if non_increasing_packets == 0 {
            AudioEncoding::Copy
        } else {
            AudioEncoding::RepairAac {
                non_increasing_packets,
            }
        },
        audio_packets,
    ))
}

/// Run the bounded disk scan off the async runtime's worker threads.
pub(crate) async fn inspect_file(path: &Path) -> AppResult<AudioEncoding> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        // Download markers may follow the extension, so detect the real input
        // container rather than trusting its filename or selected output format.
        let mut file = std::fs::File::open(&path)?;
        let mut magic = [0; 3];
        let length = file.read(&mut magic)?;
        if length < 3 || &magic != b"FLV" {
            return Ok(AudioEncoding::Copy);
        }
        flv_audio_encoding(&path)
    })
    .await
    .map_err(|e| AppError::Custom(format!("检查录像音频失败: {e}")))?
    .map_err(|e| AppError::Custom(format!("检查录像音频失败: {e}")).into())
}

/// Apply the same AAC repair to masking and audio-only remuxing. The initial A/V
/// origin is preserved by the caller's -copyts, while conflicts are clamped to
/// keep all decoded voice samples. Forward gaps are deliberately left intact.
pub(crate) fn append_encoding_args(cmd: &mut Command, audio: AudioEncoding) {
    match audio {
        AudioEncoding::Copy => {
            cmd.args(["-c:a", "copy"]);
        }
        AudioEncoding::RepairAac { .. } => {
            cmd.args([
                "-af",
                "asetpts=if(isnan(PREV_OUTPTS)\\,PTS\\,max(PTS\\,PREV_OUTPTS+NB_SAMPLES/SR/TB))",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
            ]);
        }
    }
}

pub(crate) async fn validate_output(path: &Path, format: &str, require_aac: bool) -> AppResult<()> {
    if format != "flv" {
        return validate_packet_timestamps(path).await;
    }
    let path = path.to_path_buf();
    let (encoding, packets) = tokio::task::spawn_blocking(move || flv_audio_packet_timeline(&path))
        .await
        .map_err(|error| AppError::Custom(format!("检查修复后音频失败: {error}")))?
        .map_err(|error| AppError::Custom(format!("检查修复后音频失败: {error}")))?;
    if matches!(encoding, AudioEncoding::RepairAac { .. }) || (require_aac && packets == 0) {
        return Err(
            AppError::Custom("音频修复后仍存在重复或回退的 AAC 时间戳，已保留原片".into()).into(),
        );
    }
    Ok(())
}

/// FFmpeg's stream-copy framecrc muxer exposes original packet PTS for containers
/// other than FLV. Use the existing FFmpeg installation, without requiring
/// ffprobe or decoding/reencoding a second time. Consume output incrementally.
async fn validate_packet_timestamps(path: &Path) -> AppResult<()> {
    let mut cmd = tools::ffmpeg_command();
    cmd.args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-copyts",
        "-i",
    ])
    .arg(path)
    .args(["-map", "0:a", "-c:a", "copy", "-f", "framecrc", "-"])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Custom(format!("检查修复后音频失败: {e}")))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let scan = async move {
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        let mut aac_streams = HashSet::new();
        let mut last_pts = HashMap::new();
        let mut invalid = false;
        let mut aac_packets = 0;
        while let Some(line) = lines.next_line().await? {
            if let Some(codec) = line.strip_prefix("#codec_id ") {
                if let Some((stream, codec)) = codec.split_once(':') {
                    if codec.trim() == "aac" {
                        aac_streams.insert(stream.trim().parse::<usize>().unwrap_or(usize::MAX));
                    }
                }
            } else if !line.starts_with('#') {
                let mut fields = line.split(',').map(str::trim);
                let stream = fields.next().and_then(|value| value.parse::<usize>().ok());
                let _dts = fields.next();
                let pts = fields.next().and_then(|value| value.parse::<i64>().ok());
                if let Some(stream) = stream.filter(|stream| aac_streams.contains(stream)) {
                    match pts {
                        Some(pts) => {
                            aac_packets += 1;
                            if last_pts
                                .insert(stream, pts)
                                .is_some_and(|previous| pts <= previous)
                            {
                                invalid = true;
                            }
                        }
                        None => invalid = true,
                    }
                }
            }
        }
        Ok::<_, std::io::Error>(!invalid && aac_packets > 0)
    };
    let result = tokio::time::timeout(Duration::from_secs(300), async {
        tokio::join!(child.wait(), scan)
    })
    .await;
    match result {
        Ok((Ok(status), Ok(true))) if status.success() => Ok(()),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(AppError::Custom("检查修复后音频超时，已保留原片".into()).into())
        }
        _ => Err(AppError::Custom("修复后的 AAC 音频时间戳校验失败，已保留原片".into()).into()),
    }
}

/// Repair a completed FLV recording even when masking is disabled. Ordinary
/// audio and other containers are untouched. On failure the original file is
/// unchanged; only a fully checked, same-directory temporary file replaces it.
/// The caller must invalidate any byte-offset indexes when this returns true.
pub(crate) async fn repair_file_if_needed(path: &Path) -> AppResult<bool> {
    if path
        .extension()
        .is_some_and(|extension| extension == "part")
        || path.to_string_lossy().ends_with(".part")
    {
        return Err(AppError::Custom("录制中的临时文件不能执行音频修复".into()).into());
    }
    let _slot = REPAIR_SLOT
        .acquire()
        .await
        .map_err(|_| AppError::Custom("音频修复队列已关闭".into()))?;
    let before = FileIdentity::read(path)?;
    let audio = inspect_file(path).await?;
    let AudioEncoding::RepairAac {
        non_increasing_packets,
    } = audio
    else {
        return Ok(false);
    };
    warn!(file = %path.display(), non_increasing_packets,
        "检测到 AAC 音频包时间戳重复或回退，将修复音频以避免播放卡顿");
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::Builder::new()
        .prefix(".biliup-audio-")
        .suffix(".flv")
        .tempfile_in(parent)
        .map_err(|e| AppError::Custom(format!("创建音频修复临时文件失败: {e}")))?;
    let mut cmd = tools::low_priority_ffmpeg_command();
    cmd.args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-copyts",
        "-i",
    ])
    .arg(path)
    .args(["-map", "0:v?", "-map", "0:a?", "-c:v", "copy"]);
    append_encoding_args(&mut cmd, audio);
    cmd.args(["-f", "flv", "-y"])
        .arg(temporary.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Custom(format!("启动音频修复 FFmpeg 失败: {e}")))?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let capture = async move {
        let mut tail = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(length) = stderr.read(&mut buffer).await {
            if length == 0 {
                break;
            }
            tail.extend_from_slice(&buffer[..length]);
            if tail.len() > STDERR_TAIL {
                tail.drain(..tail.len() - STDERR_TAIL);
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    };
    let (status, stderr) = match tokio::time::timeout(REPAIR_TIMEOUT, async {
        tokio::join!(child.wait(), capture)
    })
    .await
    {
        Ok((status, stderr)) => (
            status.map_err(|e| AppError::Custom(format!("音频修复 FFmpeg 进程错误: {e}")))?,
            stderr,
        ),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(AppError::Custom("FFmpeg 音频修复超时，已保留原片".into()).into());
        }
    };
    if !status.success() {
        return Err(AppError::Custom(format!(
            "FFmpeg 音频修复失败 ({:?}): {stderr}",
            status.code()
        ))
        .into());
    }
    if temporary
        .as_file()
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0)
        <= 13
    {
        return Err(
            AppError::Custom("FFmpeg 没有生成有效的音频修复文件，已保留原片".into()).into(),
        );
    }
    validate_output(temporary.path(), "flv", true).await?;
    if !before.still_matches(path)? {
        return Err(AppError::Custom("音频修复期间录像文件发生变化，已保留原片".into()).into());
    }
    temporary
        .persist(path)
        .map_err(|e| AppError::Custom(format!("保存音频修复录像失败: {e}")))?;
    Ok(true)
}

struct FileIdentity {
    length: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileIdentity {
    fn read(path: &Path) -> AppResult<Self> {
        let metadata = std::fs::metadata(path)
            .map_err(|error| AppError::Custom(format!("读取待修复录像状态失败: {error}")))?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            length: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }

    fn still_matches(&self, path: &Path) -> AppResult<bool> {
        let metadata = std::fs::metadata(path)
            .map_err(|error| AppError::Custom(format!("确认录像状态失败: {error}")))?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(
            metadata.len() == self.length && metadata.modified().ok() == self.modified && {
                #[cfg(unix)]
                {
                    metadata.dev() == self.device && metadata.ino() == self.inode
                }
                #[cfg(not(unix))]
                {
                    true
                }
            },
        )
    }
}
