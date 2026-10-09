//! One real live video frame for the mosaic region editor. Active recordings are
//! decoded from their latest preview GOP, without opening a second CDN connection.
//! The request accepts a saved room id only: no input URL or local file path.

use crate::server::core::download_manager::DownloadManager;
use crate::server::core::live::live_request;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use biliup::downloader::live::{LiveStatus, LiveStream, strip_ws_expire_override};
use biliup::downloader::preview::{PreviewFormat, PreviewHub, SubscribeError};
use bytes::Bytes;
use std::collections::HashSet;
use std::future::Future;
use std::process::Stdio;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use url::Url;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(15);
const DECODE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_INPUT_BYTES: usize = 20 * 1024 * 1024;
const MAX_JPEG_BYTES: usize = 5 * 1024 * 1024;

static SLOTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(2)));
static ROOMS: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(Mutex::default);

fn media_client() -> Result<reqwest::Client, FrameError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 3
                || media_url(attempt.url().as_str()).is_err()
                || (attempt.url().scheme() == "http"
                    && attempt
                        .previous()
                        .last()
                        .is_some_and(|url| url.scheme() == "https"))
            {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| FrameError::Failed)
}

/// Do not queue frame requests or let multiple requests for the same room open
/// multiple short-lived connections when that room is not recording.
struct FramePermit {
    room: i64,
    _slot: OwnedSemaphorePermit,
}

impl FramePermit {
    fn acquire(room: i64) -> Result<Self, FrameError> {
        let slot = SLOTS
            .clone()
            .try_acquire_owned()
            .map_err(|_| FrameError::Busy)?;
        if !ROOMS.lock().unwrap().insert(room) {
            return Err(FrameError::Busy);
        }
        Ok(Self { room, _slot: slot })
    }
}

impl Drop for FramePermit {
    fn drop(&mut self) {
        ROOMS.lock().unwrap().remove(&self.room);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FrameError {
    Missing,
    Offline,
    Busy,
    Unsupported,
    Pending,
    Timeout,
    NoFfmpeg,
    TooLarge,
    Failed,
}

impl FrameError {
    // Keep platform errors, request URLs, process diagnostics and credentials out
    // of both HTTP error responses and tracing. The input here is always pipe:0.
    fn response(self) -> Response {
        let (status, message) = match self {
            Self::Missing => (StatusCode::NOT_FOUND, "直播间不存在，请先保存直播间"),
            Self::Offline => (StatusCode::CONFLICT, "直播间尚未开播，无法获取当前画面"),
            Self::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                "正在获取直播画面，请稍后重试",
            ),
            Self::Unsupported => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "当前流不支持取画面；录制中请使用 mesio 或 stream-gears 下载器，未录制的 HLS 流请先开始录制",
            ),
            Self::Pending => (
                StatusCode::SERVICE_UNAVAILABLE,
                "录制尚未提供关键帧或正在重连，请稍后刷新画面",
            ),
            Self::Timeout => (StatusCode::GATEWAY_TIMEOUT, "获取直播画面超时，请稍后重试"),
            Self::NoFfmpeg => (
                StatusCode::SERVICE_UNAVAILABLE,
                "获取直播画面需要 FFmpeg，请检查 FFmpeg 路径配置",
            ),
            Self::TooLarge => (
                StatusCode::SERVICE_UNAVAILABLE,
                "直播画面数据过大，无法取帧",
            ),
            Self::Failed => (
                StatusCode::SERVICE_UNAVAILABLE,
                "获取直播画面失败，请检查直播是否正常及 FFmpeg 是否支持该流的编码",
            ),
        };
        let mut response = (status, message).into_response();
        if matches!(
            status,
            StatusCode::SERVICE_UNAVAILABLE | StatusCode::TOO_MANY_REQUESTS
        ) {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("3"));
        }
        no_store(response)
    }
}

/// `GET /v1/streamers/{id}/mosaic-frame` -> JPEG; no cover fallback.
pub async fn get_mosaic_frame(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Response {
    let result = tokio::time::timeout(REQUEST_TIMEOUT, async {
        let worker = managers
            .get_room_by_id(id)
            .await
            .ok_or(FrameError::Missing)?;
        let _permit = FramePermit::acquire(id)?;
        if let Some(hub) = recording_hub(&worker) {
            return frame_from_hub(&hub).await;
        }
        let plugin = managers
            .plugin_for(&worker.get_streamer().url)
            .await
            .ok_or(FrameError::Unsupported)?;
        let stream = match plugin.check_stream(live_request(&worker)).await {
            Ok(LiveStatus::Live { stream }) => stream,
            Ok(LiveStatus::Offline) => return Err(FrameError::Offline),
            Err(_) => return Err(FrameError::Failed),
        };
        // A monitor may have begun recording while we queried the platform.
        if let Some(hub) = recording_hub(&worker) {
            return frame_from_hub(&hub).await;
        }
        // Credentials and quality selection were already resolved by the same
        // live_request path as recordings. The media request uses only headers
        // deliberately supplied by that plugin and has guarded redirects.
        frame_from_stream(&media_client()?, &stream).await
    })
    .await
    .unwrap_or(Err(FrameError::Timeout));
    match result {
        Ok(jpeg) => {
            let mut response = jpeg.into_response();
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
            response.headers_mut().insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            no_store(response)
        }
        Err(error) => error.response(),
    }
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn recording_hub(worker: &Worker) -> Option<PreviewHub> {
    match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => Some(task.preview().clone()),
        _ => None,
    }
}

async fn frame_from_hub(hub: &PreviewHub) -> Result<Vec<u8>, FrameError> {
    let subscription = hub
        .subscribe_for_decoder(SNAPSHOT_TIMEOUT)
        .await
        .map_err(|error| match error {
            SubscribeError::Unavailable(_) => FrameError::Unsupported,
            SubscribeError::NotAttached => FrameError::Pending,
            SubscribeError::Timeout => FrameError::Timeout,
            SubscribeError::TooManySubscribers(_) => FrameError::Busy,
        })?;
    let format = match subscription.format {
        PreviewFormat::Flv => "flv",
        PreviewFormat::MpegTs => "mpegts",
        PreviewFormat::Fmp4 => "mp4",
    };
    let snapshot = subscription.snapshot.clone();
    // A single snapshot is enough to decode its keyframe; drop the subscription
    // immediately so the decoder does not hold a viewer slot or live broadcasts.
    drop(subscription);
    frame_from_snapshot(format, snapshot).await
}

async fn frame_from_snapshot(format: &str, chunks: Vec<Bytes>) -> Result<Vec<u8>, FrameError> {
    let total = chunks
        .iter()
        .try_fold(0usize, |total, chunk| total.checked_add(chunk.len()))
        .ok_or(FrameError::TooLarge)?;
    if total > MAX_INPUT_BYTES {
        return Err(FrameError::TooLarge);
    }
    decode_frame(Some(format), move |mut stdin| async move {
        for chunk in chunks {
            // The decoder exits after its first frame. BrokenPipe is expected.
            if stdin.write_all(&chunk).await.is_err() {
                break;
            }
        }
        Ok(())
    })
    .await
}

fn media_url(raw: &str) -> Result<Url, FrameError> {
    let url = Url::parse(raw).map_err(|_| FrameError::Unsupported)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(FrameError::Unsupported);
    }
    Ok(url)
}

async fn frame_from_stream(
    client: &reqwest::Client,
    stream: &LiveStream,
) -> Result<Vec<u8>, FrameError> {
    let raw_url =
        strip_ws_expire_override(&stream.raw_stream_url).unwrap_or(&stream.raw_stream_url);
    let url = media_url(raw_url)?;
    // Decoding HLS through FFmpeg would let its playlist open other URLs or
    // files. Active HLS recordings already provide safe TS/fMP4 bytes via hub.
    if url.path().to_ascii_lowercase().ends_with(".m3u8") {
        return Err(FrameError::Unsupported);
    }
    let mut request = client.get(url).timeout(DECODE_TIMEOUT);
    for (name, value) in &stream.stream_headers {
        request = request.header(name, value);
    }
    let mut response = request.send().await.map_err(|_| FrameError::Failed)?;
    if !response.status().is_success() {
        return Err(FrameError::Failed);
    }
    media_url(response.url().as_str())?;
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.contains("mpegurl") {
        return Err(FrameError::Unsupported);
    }
    // Automatic demuxer probing is safe here: only pipe is whitelisted, so an
    // unexpected playlist can never cause FFmpeg to fetch network/local paths.
    decode_frame(None, move |mut stdin| async move {
        let mut total = 0usize;
        while let Some(chunk) = response.chunk().await.map_err(|_| FrameError::Failed)? {
            total = total.checked_add(chunk.len()).ok_or(FrameError::TooLarge)?;
            if total > MAX_INPUT_BYTES {
                return Err(FrameError::TooLarge);
            }
            if stdin.write_all(&chunk).await.is_err() {
                break;
            }
        }
        Ok(())
    })
    .await
}

async fn decode_frame<F, Fut>(format: Option<&str>, feed: F) -> Result<Vec<u8>, FrameError>
where
    F: FnOnce(ChildStdin) -> Fut,
    Fut: Future<Output = Result<(), FrameError>>,
{
    let mut command = crate::tools::ffmpeg_command();
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostats",
        "-threads",
        "1",
        "-protocol_whitelist",
        "pipe",
        "-probesize",
        "8388608",
        "-analyzeduration",
        "3000000",
    ]);
    if let Some(format) = format {
        command.args(["-f", format]);
    }
    let mut child = command
        .args(["-i", "pipe:0", "-map", "0:v:0", "-frames:v", "1"])
        // Expand anamorphic pixels before writing JPEG (which has square pixels)
        // and keep the actual frame shape. This is never a fixed 16:9 canvas.
        .args([
            "-vf",
            "scale=w='min(1920,iw*sar)':h='max(2,trunc(ow/dar/2)*2)',setsar=1",
            "-c:v",
            "mjpeg",
            "-q:v",
            "3",
            "-threads",
            "1",
            "-f",
            "image2pipe",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FrameError::NoFfmpeg
            } else {
                FrameError::Failed
            }
        })?;
    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let run = async {
        let read = async {
            let mut jpeg = Vec::new();
            stdout
                .take(MAX_JPEG_BYTES as u64 + 1)
                .read_to_end(&mut jpeg)
                .await
                .map_err(|_| FrameError::Failed)?;
            if jpeg.len() > MAX_JPEG_BYTES {
                return Err(FrameError::TooLarge);
            }
            Ok(jpeg)
        };
        // Cancel upstream reads as soon as one frame has been decoded. Joining
        // feed+read would keep reading a quiet live response until its timeout
        // even after FFmpeg produced a complete JPEG and exited.
        let feed = feed(stdin);
        tokio::pin!(feed, read);
        let jpeg = tokio::select! {
            result = &mut read => result?,
            result = &mut feed => {
                result?;
                read.await?
            }
        };
        let status = child.wait().await.map_err(|_| FrameError::Failed)?;
        if status.success() && jpeg.starts_with(&[0xff, 0xd8]) && jpeg.ends_with(&[0xff, 0xd9]) {
            Ok(jpeg)
        } else {
            Err(FrameError::Failed)
        }
    };
    tokio::time::timeout(DECODE_TIMEOUT, run)
        .await
        .unwrap_or(Err(FrameError::Timeout))
}

#[cfg(test)]
mod tests;
