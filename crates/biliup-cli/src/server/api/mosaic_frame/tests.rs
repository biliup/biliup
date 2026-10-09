use super::*;
use crate::server::config::Config;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use async_trait::async_trait;
use axum::Router;
use axum::body::to_bytes;
use axum::http::Request;
use axum::routing::get;
use biliup::client::StatelessClient;
use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult};
use biliup::downloader::preview::{ChunkKind, PreviewSink};
use image::GenericImageView;
use std::sync::RwLock;
use tower::ServiceExt;

fn fixture_stream(raw_url: &str) -> LiveStream {
    serde_json::from_value(serde_json::json!({
        "name": "fixture", "url": "https://fixture.example/1", "title": "fixture",
        "date": "2026-01-01T00:00:00Z", "live_cover_url": "https://fixture.example/cover.jpg",
        "raw_stream_url": raw_url, "platform": "fixture", "stream_headers": {},
        "suffix": "flv", "danmaku": null, "downloader_hint": "StreamGears", "runtime_options": null
    }))
    .unwrap()
}

async fn synthetic_flv(size: &str, filter: Option<&str>) -> Vec<u8> {
    synthetic_media(size, filter, "flv").await
}

async fn synthetic_media(size: &str, filter: Option<&str>, format: &str) -> Vec<u8> {
    synthetic_video(size, filter, format, "libx264").await
}

async fn synthetic_video(size: &str, filter: Option<&str>, format: &str, encoder: &str) -> Vec<u8> {
    let mut command = crate::tools::ffmpeg_command();
    command
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(format!("color=c=red:s={size}:r=10"));
    if let Some(filter) = filter {
        command.args(["-vf", filter]);
    }
    command.args([
        "-t",
        "0.6",
        "-an",
        "-c:v",
        encoder,
        "-preset",
        "ultrafast",
        "-tune",
        "zerolatency",
        "-pix_fmt",
        "yuv420p",
        "-g",
        "3",
        "-threads",
        "1",
    ]);
    if encoder == "libx265" {
        command.args(["-x265-params", "pools=1:frame-threads=1:log-level=error"]);
    }
    if format == "mp4" {
        command.args(["-movflags", "empty_moov+frag_keyframe"]);
    }
    let output = command
        .args(["-f", format, "pipe:1"])
        .kill_on_drop(true)
        .output()
        .await
        .expect("FFmpeg required for live-frame tests");
    assert!(
        output.status.success(),
        "fixture encoding failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if format == "flv" {
        assert!(output.stdout.starts_with(b"FLV"));
    }
    output.stdout
}

/// Feed the same container header/sequence/header and per-tag GOP markers as the
/// real downloaders, so this exercises PreviewHub's latest-GOP subscription.
fn feed_flv(sink: &mut PreviewSink, flv: &[u8]) {
    let offset = u32::from_be_bytes(flv[5..9].try_into().unwrap()) as usize + 4;
    sink.push(ChunkKind::Header, Bytes::copy_from_slice(&flv[..offset]));
    let mut at = offset;
    while at + 11 <= flv.len() {
        let tag_type = flv[at];
        let size =
            ((flv[at + 1] as usize) << 16) | ((flv[at + 2] as usize) << 8) | flv[at + 3] as usize;
        let end = at + 11 + size + 4;
        assert!(end <= flv.len());
        let data = &flv[at + 11..at + 11 + size];
        let enhanced = data.first().is_some_and(|first| first & 0x80 != 0);
        let sequence_header = if enhanced {
            data[0] & 0x0f == 0
        } else {
            data.len() >= 2 && data[1] == 0
        };
        let keyframe = if enhanced {
            (data[0] >> 4) & 0x07 == 1 && matches!(data[0] & 0x0f, 1 | 3)
        } else {
            data.len() >= 2 && data[0] >> 4 == 1 && data[1] == 1
        };
        let kind = if tag_type == 9 && sequence_header {
            ChunkKind::SequenceHeader(9)
        } else if tag_type == 9 && keyframe {
            ChunkKind::Keyframe
        } else {
            ChunkKind::Media
        };
        sink.push(kind, Bytes::copy_from_slice(&flv[at..end]));
        at = end;
    }
}

#[tokio::test]
async fn actual_preview_gop_decodes_a_live_frame_with_non_widescreen_shape() {
    let flv = synthetic_flv("320x240", None).await;
    let hub = PreviewHub::new(1);
    let mut sink = hub.attach(PreviewFormat::Flv);
    feed_flv(&mut sink, &flv);
    let query_hub = hub.clone();
    let query = tokio::spawn(async move { frame_from_hub(&query_hub).await });
    // Snapshot requests are serviced on the write path, next arriving tag.
    tokio::time::sleep(Duration::from_millis(20)).await;
    sink.push(ChunkKind::Media, Bytes::new());
    let jpeg = query.await.unwrap().unwrap();
    assert_eq!(
        hub.subscribers(),
        0,
        "frame decoding must release its viewer slot"
    );
    let decoded = image::load_from_memory(&jpeg).unwrap();
    assert_eq!(decoded.dimensions(), (320, 240));
    let pixel = decoded.to_rgb8().get_pixel(160, 120).0;
    assert!(
        pixel[0] > 220 && pixel[1] < 30 && pixel[2] < 30,
        "actual video pixels, not cover/placeholder: {pixel:?}"
    );
}

#[tokio::test]
async fn hevc_live_frame_decodes_while_browser_preview_stays_unavailable() {
    let flv = synthetic_video("320x240", None, "flv", "libx265").await;
    let hub = PreviewHub::new(1);
    let mut sink = hub.attach(PreviewFormat::Flv);
    feed_flv(&mut sink, &flv);
    sink.mark_browser_unavailable("HEVC is unsupported by the browser player");
    assert!(!hub.status().available);
    assert!(matches!(
        hub.subscribe(Duration::from_millis(20)).await,
        Err(SubscribeError::Unavailable(_))
    ));
    let query_hub = hub.clone();
    let query = tokio::spawn(async move { frame_from_hub(&query_hub).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    sink.push(ChunkKind::Media, Bytes::new());
    let jpeg = query.await.unwrap().unwrap();
    assert_eq!(hub.subscribers(), 0);
    assert!(
        !hub.status().available,
        "JPEG extraction must not enable HEVC browser streaming"
    );
    let decoded = image::load_from_memory(&jpeg).unwrap();
    assert_eq!(decoded.dimensions(), (320, 240));
    let pixel = decoded.to_rgb8().get_pixel(160, 120).0;
    assert!(pixel[0] > 220 && pixel[1] < 30 && pixel[2] < 30);
}

#[tokio::test]
async fn anamorphic_frame_is_expanded_to_its_display_aspect_ratio() {
    let flv = synthetic_flv("320x240", Some("setsar=2")).await;
    let jpeg = frame_from_snapshot("flv", vec![Bytes::from(flv)])
        .await
        .unwrap();
    assert_eq!(
        image::load_from_memory(&jpeg).unwrap().dimensions(),
        (640, 240)
    );
}

#[tokio::test]
async fn segmented_recording_bytes_decode_from_mpegts_and_fragmented_mp4() {
    for format in ["mpegts", "mp4"] {
        let media = synthetic_media("256x144", None, format).await;
        let jpeg = frame_from_snapshot(format, vec![Bytes::from(media)])
            .await
            .unwrap();
        assert_eq!(
            image::load_from_memory(&jpeg).unwrap().dimensions(),
            (256, 144),
            "{format}"
        );
    }
}

#[tokio::test]
async fn uninitialized_or_unsupported_recording_never_opens_a_cdn_connection() {
    assert_eq!(
        frame_from_hub(&PreviewHub::new(1)).await,
        Err(FrameError::Pending)
    );
    assert_eq!(
        frame_from_hub(&PreviewHub::unavailable("private source path")).await,
        Err(FrameError::Unsupported)
    );
    let error = FrameError::Unsupported.response();
    let body = to_bytes(error.into_body(), 2048).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("private source path"));
}

#[tokio::test]
async fn oversized_snapshot_is_rejected_before_spawning_a_decoder() {
    let huge = Bytes::from(vec![0u8; MAX_INPUT_BYTES + 1]);
    assert_eq!(
        frame_from_snapshot("flv", vec![huge]).await,
        Err(FrameError::TooLarge)
    );
}

#[tokio::test]
async fn decoder_timeout_cancels_feed_and_kills_child() {
    // Tokio's timeout is checked between async polls; the pending feeder and
    // empty pipe make this a real stuck decoder rather than a malformed file.
    let run = decode_frame(Some("flv"), |stdin| async move {
        let _keep_input_open = stdin;
        std::future::pending::<Result<(), FrameError>>().await
    });
    let result = tokio::time::timeout(Duration::from_millis(100), run).await;
    assert!(result.is_err());
}

#[test]
fn media_input_rejects_paths_credentials_and_non_http_protocols() {
    for invalid in [
        "/etc/passwd",
        "file:///etc/passwd",
        "concat:a|b",
        "ftp://fixture.example/frame.flv",
        "http://user:password@fixture.example/frame.flv",
        "https://user@fixture.example/frame.flv",
    ] {
        assert_eq!(
            media_url(invalid),
            Err(FrameError::Unsupported),
            "{invalid}"
        );
    }
    assert!(media_url("https://cdn.fixture.example/live.flv?signature=private").is_ok());
}

#[tokio::test]
async fn hls_playlist_is_not_passed_to_ffmpeg_for_unsafe_subrequests() {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let result = frame_from_stream(
        &client,
        &fixture_stream("https://fixture.invalid/live.m3u8?token=private"),
    )
    .await;
    assert_eq!(result, Err(FrameError::Unsupported));
}

#[tokio::test]
async fn live_http_frame_has_bounded_input_and_no_remote_url_in_decoder() {
    let flv = synthetic_flv("240x320", None).await;
    let app = Router::new().route(
        "/live.flv",
        get(move || {
            let flv = flv.clone();
            async move { ([(header::CONTENT_TYPE, "video/x-flv")], flv) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let stream = fixture_stream(&format!("http://{address}/live.flv?token=private"));
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let jpeg = frame_from_stream(&client, &stream).await.unwrap();
    assert_eq!(
        image::load_from_memory(&jpeg).unwrap().dimensions(),
        (240, 320)
    );
    server.abort();
}

struct Offline;

#[async_trait]
impl LivePlugin for Offline {
    fn name(&self) -> &'static str {
        "fixture"
    }
    fn matches(&self, url: &str) -> bool {
        url.starts_with("https://fixture.example/")
    }
    async fn check_stream(&self, _: LiveRequest) -> LiveResult<LiveStatus> {
        Ok(LiveStatus::Offline)
    }
}

#[tokio::test]
async fn endpoint_missing_room_and_offline_room_return_explicit_no_store_errors() {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("db.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let managers = Arc::new(DownloadManager::new(1, 1, pool));
    managers.add_plugin(Arc::new(Offline)).await;
    let streamer = LiveStreamer {
        id: 19361,
        url: "https://fixture.example/1".into(),
        remark: "fixture".into(),
        filename_prefix: None,
        time_range: None,
        upload_streamers_id: None,
        format: None,
        override_cfg: None,
        preprocessor: None,
        segment_processor: None,
        downloaded_processor: None,
        postprocessor: None,
        opt_args: None,
        excluded_keywords: None,
    };
    let worker = Worker::new(
        streamer,
        None,
        Arc::new(RwLock::new(Config::default())),
        StatelessClient::default(),
    );
    managers.add_room(worker).await.unwrap();
    let app = Router::new()
        .route("/v1/streamers/{id}/mosaic-frame", get(get_mosaic_frame))
        .with_state(managers);
    for (id, status, expected) in [
        (99999, StatusCode::NOT_FOUND, "直播间不存在"),
        (19361, StatusCode::CONFLICT, "尚未开播"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/v1/streamers/{id}/mosaic-frame"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 2048).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains(expected));
        assert!(!text.contains("cover.jpg"));
        assert!(!text.contains("fixture.example"));
    }
}
