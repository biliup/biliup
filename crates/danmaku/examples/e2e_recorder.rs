use std::path::PathBuf;
use std::time::Duration;

use danmaku::{DanmakuRecorder, PlatformContext, RecorderConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DANMAKU_TEST_URL")?;
    let seconds = std::env::var("DANMAKU_TEST_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60);
    let output = std::env::var("DANMAKU_TEST_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/tmp/danmaku_e2e_%Y%m%d_%H%M%S"));

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut config = RecorderConfig::new(&url, &output);
    if let Ok(room_id) = std::env::var("DANMAKU_TEST_ROOM_ID") {
        config = config.with_context(PlatformContext::new().with_room_id(room_id));
    }
    let recorder = DanmakuRecorder::new(config)?;
    let handle = recorder.start();
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    handle.stop().await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem_prefix = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("danmaku_e2e")
        .split('%')
        .next()
        .unwrap_or("danmaku_e2e");

    let mut files = std::fs::read_dir(parent)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("xml")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with(stem_prefix))
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();

    let xml_path = files
        .last()
        .ok_or_else(|| format!("no XML file produced under {}", parent.display()))?;
    let content = std::fs::read_to_string(xml_path)?;
    let has_xml_message =
        content.contains("<d ") || content.contains("<s ") || content.contains("<o ");

    println!("url={url}");
    println!("xml={}", xml_path.display());
    println!("has_xml_message={has_xml_message}");
    println!("bytes={}", content.len());

    if !has_xml_message {
        return Err("XML file does not contain danmaku message nodes".into());
    }

    Ok(())
}
