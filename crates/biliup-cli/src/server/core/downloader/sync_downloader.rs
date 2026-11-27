use crate::server::common::util::redact_process_debug;
use crate::server::core::downloader::DownloadConfig;
use crate::server::errors::{AppError, AppResult};
use crate::tools;
use bytes::Bytes;
use error_stack::ResultExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use url::Url;

/// 边录边传下载器。
///
/// 对齐 Python `biliup/engine/sync_downloader.py`：ffmpeg（HLS 时可选 streamlink）
/// 把直播流 remux 成 Matroska 写到 stdout，再按 UPOS 分片切给上传端，默认不落盘。
pub struct SyncDownloader {
    ffmpeg: Arc<RwLock<Option<Child>>>,
    streamlink: Arc<RwLock<Option<Child>>>,
}

impl Default for SyncDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncDownloader {
    pub fn new() -> Self {
        Self {
            ffmpeg: Arc::new(RwLock::new(None)),
            streamlink: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) async fn stop(&self) -> AppResult<()> {
        self.kill_children().await;
        Ok(())
    }

    async fn kill_children(&self) {
        if let Some(mut child) = self.ffmpeg.write().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if let Some(mut child) = self.streamlink.write().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    /// 启动一段录制并读出第一块数据。没有数据时返回 `None`（对应 Python 空 stdout 重试）。
    pub async fn start_segment(
        &self,
        download_config: &DownloadConfig,
        max_file_size: u64,
        token: &CancellationToken,
    ) -> AppResult<Option<SegmentStdout>> {
        self.kill_children().await;

        let url = &download_config.url;
        let hls = is_hls_url(url);
        let use_streamlink = hls && command_exists("streamlink", "--version");
        if hls && !use_streamlink {
            warn!("HLS 流未找到 streamlink，回退为 ffmpeg 直拉");
        }

        let mut ffmpeg = if use_streamlink {
            self.spawn_streamlink_ffmpeg(download_config, max_file_size)
                .await?
        } else {
            self.spawn_ffmpeg(download_config, max_file_size, false)?
        };

        drain_stderr(&mut ffmpeg, "ffmpeg");
        let mut stdout = ffmpeg
            .stdout
            .take()
            .ok_or_else(|| AppError::Custom("failed to capture ffmpeg stdout".to_string()))?;
        *self.ffmpeg.write().await = Some(ffmpeg);

        let mut buf = vec![0u8; 4096];
        let first = tokio::select! {
            _ = token.cancelled() => {
                self.kill_children().await;
                return Ok(None);
            }
            read = timeout(Duration::from_secs(20), stdout.read(&mut buf)) => read,
        };

        match first {
            Ok(Ok(0)) | Err(_) => {
                self.kill_children().await;
                Ok(None)
            }
            Ok(Ok(n)) => {
                buf.truncate(n);
                Ok(Some(SegmentStdout {
                    stdout,
                    peeked: buf,
                }))
            }
            Ok(Err(e)) => {
                self.kill_children().await;
                Err(AppError::Custom(format!("读取 ffmpeg stdout 失败: {e}")).into())
            }
        }
    }

    fn spawn_ffmpeg(
        &self,
        download_config: &DownloadConfig,
        max_file_size: u64,
        pipe_input: bool,
    ) -> AppResult<Child> {
        let args = build_ffmpeg_args(download_config, max_file_size, pipe_input);
        let mut cmd = tools::ffmpeg_command();
        cmd.args(&args)
            .stdin(if pipe_input {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        info!(cmd = %redact_process_debug(&cmd), "Starting sync-downloader ffmpeg");
        cmd.spawn().change_context(AppError::Custom(
            "找不到 FFmpeg，边录边传无法启动：请安装 FFmpeg 并加入 PATH，或在配置里填写 ffmpeg_path"
                .into(),
        ))
    }

    async fn spawn_streamlink_ffmpeg(
        &self,
        download_config: &DownloadConfig,
        max_file_size: u64,
    ) -> AppResult<Child> {
        let sl_args = build_streamlink_args(download_config);
        let mut sl_cmd = tools::command("streamlink");
        sl_cmd
            .args(&sl_args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        info!(cmd = %redact_process_debug(&sl_cmd), "Starting sync-downloader streamlink");
        let mut streamlink = sl_cmd
            .spawn()
            .change_context(AppError::Custom("启动 streamlink 失败".into()))?;
        drain_stderr(&mut streamlink, "streamlink");
        let mut sl_stdout = streamlink
            .stdout
            .take()
            .ok_or_else(|| AppError::Custom("failed to capture streamlink stdout".to_string()))?;
        *self.streamlink.write().await = Some(streamlink);

        let mut ffmpeg = self.spawn_ffmpeg(download_config, max_file_size, true)?;
        let mut ff_stdin = ffmpeg
            .stdin
            .take()
            .ok_or_else(|| AppError::Custom("failed to capture ffmpeg stdin".to_string()))?;
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut sl_stdout, &mut ff_stdin).await;
        });
        Ok(ffmpeg)
    }
}

pub struct SegmentStdout {
    pub stdout: ChildStdout,
    pub peeked: Vec<u8>,
}

impl SegmentStdout {
    pub(crate) async fn pump(
        self,
        chunk_size: usize,
        total_size: u64,
        save_path: Option<&PathBuf>,
        tx: async_channel::Sender<Bytes>,
        token: &CancellationToken,
    ) -> AppResult<PumpResult> {
        match save_path {
            None => pump_live(self.stdout, self.peeked, chunk_size, total_size, tx, token).await,
            Some(path) => {
                pump_chunks(
                    self.stdout,
                    self.peeked,
                    chunk_size,
                    total_size,
                    path.clone(),
                    tx,
                    token.clone(),
                )
                .await
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PumpResult {
    pub actual_size: u64,
    pub streamed_size: u64,
    pub stream_complete: bool,
}

/// 无本地副本时只保留一个正在拼装的块；通道满时等待上传，绝不转存到磁盘。
/// 与旧版相同，短 P 只补齐最后一个已产生的 UPOS 分块，不填充到配置的整 P 大小。
pub async fn pump_live<R: tokio::io::AsyncRead + Unpin>(
    mut stdout: R,
    peeked: Vec<u8>,
    chunk_size: usize,
    total_size: u64,
    tx: async_channel::Sender<Bytes>,
    token: &CancellationToken,
) -> AppResult<PumpResult> {
    let mut buffer = Vec::new();
    let mut remaining = total_size;
    let mut streamed_size = 0;
    let mut incoming = peeked;
    loop {
        for chunk in take_full_chunks(&mut buffer, &incoming, chunk_size, &mut remaining) {
            let len = chunk.len();
            tx.send(chunk)
                .await
                .map_err(|_| AppError::Custom("流式上传通道已关闭，无本地副本".into()))?;
            streamed_size += len as u64;
        }
        if remaining == 0 {
            break;
        }
        incoming.resize(64 * 1024, 0);
        let n = tokio::select! {
            _ = token.cancelled() => break,
            n = stdout.read(&mut incoming) => n.change_context(AppError::Unknown)?,
        };
        if n == 0 {
            break;
        }
        incoming.truncate(n);
    }
    let actual_size = total_size - remaining;
    if !buffer.is_empty() {
        let len = (total_size - streamed_size).min(chunk_size as u64) as usize;
        buffer.resize(len, 0);
        tx.send(Bytes::from(buffer))
            .await
            .map_err(|_| AppError::Custom("尾块上传通道已关闭，无本地副本".into()))?;
        streamed_size += len as u64;
    }
    Ok(PumpResult {
        actual_size,
        streamed_size,
        stream_complete: true,
    })
}

/// 把 ffmpeg stdout 同时写入指定的本地副本并按 UPOS chunk 切给上传通道。
///
/// 通道跟不上时停止预传但继续完整落盘，调用方随后可按实际长度回退为文件上传。
///
/// 取消（停止/暂停/退出）只是结束本段：已写入的字节保留在文件里并按实际长度返回，
/// 由调用方回退为文件上传并投稿，而不是把录好的内容当作错误丢弃。
pub async fn pump_chunks<R: tokio::io::AsyncRead + Unpin>(
    mut stdout: R,
    peeked: Vec<u8>,
    chunk_size: usize,
    total_size: u64,
    save_path: PathBuf,
    tx: async_channel::Sender<Bytes>,
    token: CancellationToken,
) -> AppResult<PumpResult> {
    if let Some(dir) = save_path.parent() {
        tokio::fs::create_dir_all(dir)
            .await
            .change_context(AppError::Unknown)?;
    }
    let mut save = tokio::fs::File::create(&save_path)
        .await
        .change_context(AppError::Unknown)?;

    let mut buffer = Vec::new();
    let mut remaining = total_size;
    let mut written = 0u64;
    let mut streamed = 0u64;
    let mut incoming = peeked;
    let mut tmp = vec![0u8; 64 * 1024];
    let mut sender = Some(tx);

    loop {
        if remaining == 0 {
            break;
        }
        if incoming.is_empty() {
            let read = tokio::select! {
                _ = token.cancelled() => break,
                read = stdout.read(&mut tmp) => read,
            };
            match read {
                Ok(0) => break,
                Ok(n) => incoming = tmp[..n].to_vec(),
                Err(e) => {
                    return Err(AppError::Custom(format!("读取 ffmpeg stdout 失败: {e}")).into());
                }
            }
        }
        let take = remaining.min(incoming.len() as u64) as usize;
        // 本地写盘不受网络影响，不参与取消竞争，保证 `written` 与文件内容一致
        save.write_all(&incoming[..take])
            .await
            .change_context(AppError::Unknown)?;
        written += take as u64;
        let chunks = take_full_chunks(&mut buffer, &incoming[..take], chunk_size, &mut remaining);
        if let Some(tx) = sender.as_ref() {
            for chunk in chunks {
                let len = chunk.len() as u64;
                if tx.try_send(chunk).is_ok() {
                    streamed += len;
                } else {
                    warn!("流式上传跟不上录制速度，停止预传并使用本地副本回退");
                    sender.take();
                    buffer.clear();
                    break;
                }
            }
        }
        incoming.drain(..take);
    }

    if written == total_size
        && let Some(chunk) = finish_chunks(buffer, chunk_size)
        && let Some(tx) = sender.as_ref()
    {
        let len = chunk.len() as u64;
        let sent = tokio::select! {
            _ = token.cancelled() => false,
            result = tx.send(chunk) => result.is_ok(),
        };
        if sent {
            streamed += len;
        } else {
            sender.take();
        }
    }
    drop(sender);
    save.flush().await.change_context(AppError::Unknown)?;
    Ok(PumpResult {
        actual_size: written,
        streamed_size: streamed,
        stream_complete: written == total_size && streamed == total_size,
    })
}

/// 把 `file_size` 向上对齐到 10MiB，缺省 2GiB。对齐 Python sync_download。
pub fn align_file_size(file_size: Option<u64>) -> u64 {
    const MIN: u64 = 10 * 1024 * 1024;
    const DEFAULT: u64 = 2 * 1024 * 1024 * 1024;
    let size = match file_size {
        Some(s) if s > 0 => s,
        _ => DEFAULT,
    };
    size.div_ceil(MIN) * MIN
}

pub fn is_hls_url(url: &str) -> bool {
    Url::parse(url)
        .map(|parsed| parsed.path().contains(".m3u8"))
        .unwrap_or_else(|_| url.contains(".m3u8"))
}

/// 把新到的数据写入缓冲，满 `chunk_size` 就切出去；同时不超过 `remaining`。
pub fn take_full_chunks(
    buffer: &mut Vec<u8>,
    incoming: &[u8],
    chunk_size: usize,
    remaining: &mut u64,
) -> Vec<Bytes> {
    if *remaining == 0 || incoming.is_empty() || chunk_size == 0 {
        return Vec::new();
    }
    let take = (*remaining as usize).min(incoming.len());
    buffer.extend_from_slice(&incoming[..take]);
    *remaining -= take as u64;
    let mut chunks = Vec::new();
    while buffer.len() >= chunk_size {
        let chunk: Vec<u8> = buffer.drain(..chunk_size).collect();
        chunks.push(Bytes::from(chunk));
    }
    chunks
}

/// 流结束时返回实际尾块，不向媒体尾部补零。
pub fn finish_chunks(buffer: Vec<u8>, chunk_size: usize) -> Option<Bytes> {
    if buffer.is_empty() || chunk_size == 0 {
        return None;
    }
    Some(Bytes::from(buffer))
}

pub fn build_ffmpeg_args(
    download_config: &DownloadConfig,
    max_file_size: u64,
    pipe_input: bool,
) -> Vec<String> {
    let mut args = vec![
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
    ];
    if !pipe_input {
        if !download_config.headers.is_empty() {
            let headers_str = format_ffmpeg_headers(&download_config.headers);
            args.extend(["-headers".to_string(), headers_str]);
        }
        args.extend(["-rw_timeout".to_string(), "20000000".to_string()]);
        if is_hls_url(&download_config.url) {
            args.extend(["-max_reload".to_string(), "1000".to_string()]);
        }
        args.extend(["-fflags".to_string(), "+genpts".to_string()]);
        args.extend(["-i".to_string(), download_config.url.clone()]);
    } else {
        args.extend(["-fflags".to_string(), "+genpts".to_string()]);
        args.extend(["-i".to_string(), "pipe:0".to_string()]);
    }

    args.extend(["-fs".to_string(), max_file_size.to_string()]);
    if let Some(remaining) = download_config.time_range_remaining() {
        args.extend(["-t".to_string(), remaining]);
    }
    args.extend([
        "-c:v".to_string(),
        "copy".to_string(),
        "-c:a".to_string(),
        "copy".to_string(),
        "-reset_timestamps".to_string(),
        "1".to_string(),
        "-avoid_negative_ts".to_string(),
        "make_zero".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        "-".to_string(),
    ]);
    args
}

pub fn build_streamlink_args(download_config: &DownloadConfig) -> Vec<String> {
    let mut args = vec![
        "--stream-segment-threads".to_string(),
        "3".to_string(),
        "--hls-playlist-reload-attempts".to_string(),
        "1".to_string(),
    ];
    for (key, value) in &download_config.headers {
        args.push("--http-header".to_string());
        args.push(format!("{key}={value}"));
    }
    args.push(download_config.url.clone());
    args.push("best".to_string());
    args.push("-O".to_string());
    args
}

fn format_ffmpeg_headers(headers: &HashMap<String, String>) -> String {
    headers
        .iter()
        .map(|(k, v)| format!("{k}: {v}\r\n"))
        .collect()
}

fn command_exists(name: &str, arg: &str) -> bool {
    tools::std_command(name)
        .arg(arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn drain_stderr(child: &mut Child, prefix: &'static str) {
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("[{prefix}] {line}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::core::downloader::DownloadConfig;
    use biliup::downloader::live::{
        Douyin, LiveCredentials, LiveOptions, LivePlugin, LiveRequest, LiveStatus,
    };

    #[test]
    fn align_file_size_rounds_up_to_10mib() {
        assert_eq!(align_file_size(Some(1)), 10 * 1024 * 1024);
        assert_eq!(align_file_size(Some(10 * 1024 * 1024)), 10 * 1024 * 1024);
        assert_eq!(
            align_file_size(Some(2_621_440_000)),
            2_621_440_000,
            "默认 2.5GiB 已经是 10MiB 对齐"
        );
        assert_eq!(align_file_size(None), 205 * 10 * 1024 * 1024);
        assert_eq!(align_file_size(Some(0)), 205 * 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn diskless_stream_waits_for_the_reader_and_only_pads_the_last_chunk() {
        // Capacity one forces backpressure. No file or directory is passed to this path.
        let (tx, rx) = async_channel::bounded(1);
        let token = CancellationToken::new();
        let (pump, chunks) = tokio::join!(
            pump_live(&b"cdefghi"[..], b"ab".to_vec(), 4, 64, tx, &token),
            async {
                let mut chunks = Vec::new();
                while let Ok(chunk) = rx.recv().await {
                    chunks.push(chunk.to_vec());
                }
                chunks
            }
        );
        let pump = pump.unwrap();
        assert!(pump.stream_complete);
        assert_eq!((pump.actual_size, pump.streamed_size), (9, 12));
        assert_eq!(chunks, [b"abcd", b"efgh", b"i\0\0\0"]);
    }

    #[tokio::test]
    async fn diskless_cancel_keeps_peeked_bytes_and_receiver_loss_is_an_error() {
        let token = CancellationToken::new();
        token.cancel();
        let (tx, rx) = async_channel::bounded(1);
        let r = pump_live(tokio::io::empty(), b"abc".to_vec(), 4, 16, tx, &token)
            .await
            .unwrap();
        assert_eq!(r.actual_size, 3);
        assert_eq!(&rx.recv().await.unwrap()[..], b"abc\0");
        let (tx, rx) = async_channel::bounded(1);
        drop(rx);
        assert!(
            pump_live(tokio::io::empty(), b"abcd".to_vec(), 4, 16, tx, &token)
                .await
                .is_err()
        );
    }

    #[test]
    fn hls_is_detected_from_path_not_query() {
        assert!(is_hls_url("https://example.com/live/index.m3u8"));
        assert!(is_hls_url("https://example.com/live/index.m3u8?token=abc"));
        assert!(!is_hls_url("https://example.com/live.flv"));
        assert!(!is_hls_url("https://example.com/room/m3u8-not-ext"));
    }

    #[test]
    fn take_full_chunks_splits_and_caps_at_remaining() {
        let mut buffer = Vec::new();
        let mut remaining = 10u64;
        let chunks = take_full_chunks(&mut buffer, b"abcdefghijklmnop", 4, &mut remaining);
        assert_eq!(chunks.len(), 2);
        assert_eq!(&chunks[0][..], b"abcd");
        assert_eq!(&chunks[1][..], b"efgh");
        assert_eq!(&buffer[..], b"ij");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn finish_chunks_keeps_the_last_part_exact() {
        let chunk = finish_chunks(b"hi".to_vec(), 4).unwrap();
        assert_eq!(&chunk[..], b"hi");
        assert!(finish_chunks(Vec::new(), 4).is_none());
    }

    #[tokio::test]
    async fn ffmpeg_without_a_save_path_streams_a_short_recording_without_a_video_file() {
        if !tools::ffmpeg_status().await.available {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("input.mkv");
        let root = dir.path().join("recordings");
        std::fs::create_dir(&root).unwrap();
        assert!(
            tools::ffmpeg_command()
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "color=size=32x32:duration=0.5",
                    "-c:v",
                    "mpeg4"
                ])
                .arg(&src)
                .status()
                .await
                .unwrap()
                .success()
        );
        let cfg = DownloadConfig {
            url: src.display().to_string(),
            output_dir: root.clone(),
            ..Default::default()
        };
        let d = SyncDownloader::new();
        let token = CancellationToken::new();
        let size = 65536;
        let segment = d.start_segment(&cfg, size, &token).await.unwrap().unwrap();
        let (tx, rx) = async_channel::bounded(1);
        let (pump, data) = tokio::join!(segment.pump(256, size, None, tx, &token), async {
            let mut data = Vec::new();
            while let Ok(chunk) = rx.recv().await {
                data.extend_from_slice(&chunk);
            }
            data
        });
        d.stop().await.unwrap();
        let pump = pump.unwrap();
        assert!((100..size).contains(&pump.actual_size));
        assert_eq!(&data[..4], &[0x1a, 0x45, 0xdf, 0xa3]); // Matroska header
        assert_eq!(data.len() as u64, pump.actual_size.div_ceil(256) * 256);
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }

    #[test]
    fn ffmpeg_args_use_matroska_stdout_and_size_limit() {
        let config = DownloadConfig {
            url: "https://example.com/live.flv".into(),
            headers: HashMap::from([("Referer".into(), "https://example.com/".into())]),
            ..Default::default()
        };
        let args = build_ffmpeg_args(&config, 10 * 1024 * 1024, false);
        assert!(args.contains(&"-f".to_string()));
        assert!(args.contains(&"matroska".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("-"));
        assert!(args.windows(2).any(|w| w[0] == "-fs" && w[1] == "10485760"));
        assert!(
            args.windows(2)
                .any(|w| w[0] == "-i" && w[1] == "https://example.com/live.flv")
        );
        assert!(args.contains(&"-headers".to_string()));
    }

    #[test]
    fn ffmpeg_pipe_input_does_not_repeat_http_headers() {
        let config = DownloadConfig {
            url: "https://example.com/live/index.m3u8".into(),
            headers: HashMap::from([("User-Agent".into(), "test".into())]),
            ..Default::default()
        };
        let args = build_ffmpeg_args(&config, 1024, true);
        assert!(!args.contains(&"-headers".to_string()));
        assert!(args.windows(2).any(|w| w[0] == "-i" && w[1] == "pipe:0"));
    }

    #[tokio::test]
    async fn pump_short_stream_spools_fully_but_marks_incomplete() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("short.bin");
        let data: Vec<u8> = (0u8..10).collect();
        let (tx, rx) = async_channel::bounded(6);
        let result = pump_chunks(
            &data[..],
            Vec::new(),
            4,
            16,
            path.clone(),
            tx,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.actual_size, 10);
        assert_eq!(result.streamed_size, 8, "只应流出完整块，尾部留给文件回退");
        assert!(!result.stream_complete);
        let mut chunks = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(), 2);
        assert_eq!(std::fs::read(&path).unwrap(), data, "临时文件必须完整落盘");
    }

    #[tokio::test]
    async fn pump_exact_total_streams_all_chunks_with_short_tail() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("exact.bin");
        // ffmpeg -fs 会略微超出限制，超出部分应被丢弃
        let data: Vec<u8> = (0u8..13).collect();
        let (tx, rx) = async_channel::bounded(6);
        let result = pump_chunks(
            &data[..],
            Vec::new(),
            4,
            10,
            path.clone(),
            tx,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.actual_size, 10);
        assert_eq!(result.streamed_size, 10);
        assert!(result.stream_complete);
        let mut chunks = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            chunks.push(chunk);
        }
        assert_eq!(
            chunks.iter().map(|c| c.len()).collect::<Vec<_>>(),
            vec![4, 4, 2],
            "尾块按实际长度发送，不补零"
        );
        assert_eq!(std::fs::read(&path).unwrap(), &data[..10]);
    }

    #[tokio::test]
    async fn pump_full_queue_degrades_to_spool_only() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("slow.bin");
        let data: Vec<u8> = (0u8..12).collect();
        // 容量 1 且无人消费：第二块 try_send 失败后应放弃预传、继续落盘
        let (tx, rx) = async_channel::bounded(1);
        let result = pump_chunks(
            &data[..],
            Vec::new(),
            4,
            12,
            path.clone(),
            tx,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.actual_size, 12);
        assert_eq!(result.streamed_size, 4);
        assert!(!result.stream_complete);
        assert_eq!(std::fs::read(&path).unwrap(), data);
        drop(rx);
    }

    #[tokio::test]
    async fn pump_cancellation_ends_segment_and_keeps_written_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cancel.bin");
        // simplex 读端在写端不写入时保持 pending，确保 select 只能命中取消分支
        let (reader, _writer) = tokio::io::simplex(8);
        let (tx, rx) = async_channel::bounded::<Bytes>(6);
        let token = CancellationToken::new();
        token.cancel();
        // 取消前已经读到的数据必须落盘并计入实际长度，供文件回退上传使用
        let peeked = b"abcdef".to_vec();
        let pump = pump_chunks(reader, peeked.clone(), 4, 16, path.clone(), tx, token)
            .await
            .expect("取消只应结束本段，不应报错丢弃已录内容");
        assert_eq!(pump.actual_size, 6);
        assert_eq!(pump.streamed_size, 4, "已成块的数据照常预传");
        assert!(!pump.stream_complete);
        assert_eq!(std::fs::read(&path).unwrap(), peeked);
        assert!(rx.try_recv().is_ok());
        assert!(rx.is_closed(), "取消后必须关闭上传通道让预传任务收尾");
    }

    #[tokio::test]
    #[ignore = "需要外网、正在开播的抖音直播间和 PATH 中的 ffmpeg"]
    async fn real_douyin_stream_remuxes_to_matroska_chunks() {
        let test_url = std::env::var("DOUYIN_TEST_URL")
            .unwrap_or_else(|_| "https://live.douyin.com/451984634216".to_string());
        let status = Douyin::new()
            .check_stream(LiveRequest {
                client: reqwest::Client::new(),
                url: test_url.clone(),
                name: "sync-real-test".to_string(),
                options: LiveOptions::default(),
                credentials: LiveCredentials::default(),
            })
            .await
            .expect("抖音开播检测不应返回硬错误");
        let stream = match status {
            LiveStatus::Live { stream } => stream,
            LiveStatus::Offline => panic!("测试直播间当前未开播：{test_url}"),
        };

        let total_size = 2 * 1024 * 1024;
        let chunk_size = 256 * 1024;
        let config = DownloadConfig {
            url: stream.raw_stream_url,
            file_size: Some(total_size),
            headers: stream.stream_headers,
            ..Default::default()
        };
        let downloader = SyncDownloader::new();
        let segment = downloader
            .start_segment(&config, total_size, &CancellationToken::new())
            .await
            .expect("启动真实 ffmpeg 管道失败")
            .expect("ffmpeg 未产生输出");
        let (tx, rx) = async_channel::bounded(6);
        let temp = tempfile::tempdir().expect("create temp dir");
        let save_path = temp.path().join("real-sync-test.mkv");
        let pump_result = tokio::time::timeout(
            Duration::from_secs(60),
            pump_chunks(
                segment.stdout,
                segment.peeked,
                chunk_size,
                total_size,
                save_path.clone(),
                tx,
                CancellationToken::new(),
            ),
        )
        .await;
        downloader.stop().await.expect("停止 ffmpeg 失败");
        let pump = pump_result
            .expect("读取真实 ffmpeg 输出超时")
            .expect("切分真实 ffmpeg 输出失败");

        let mut chunks = Vec::new();
        while let Ok(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        assert!(
            pump.actual_size > 100,
            "真实直播流输出过小：{}",
            pump.actual_size
        );
        assert!(!chunks.is_empty(), "真实直播流没有生成上传分块");
        assert_eq!(&chunks[0][..4], &[0x1a, 0x45, 0xdf, 0xa3]);
        assert!(chunks.iter().all(|chunk| chunk.len() == chunk_size));
        let uploaded_bytes: usize = chunks.iter().map(Bytes::len).sum();
        assert_eq!(uploaded_bytes, pump.actual_size as usize);
        assert_eq!(
            std::fs::metadata(save_path).unwrap().len(),
            pump.actual_size
        );
        eprintln!(
            "LIVE title={} written={} chunks={} chunk_size={}",
            stream.title,
            pump.actual_size,
            chunks.len(),
            chunk_size
        );
    }
}
