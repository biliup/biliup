//! Audio passes finish before range video encoding, preserving effect history without mux queues.
use super::{Progress, browser::Browser};
use crate::server::common::processing::engine;
use serde::Deserialize;
use std::{path::Path, time::{Duration, Instant}};
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Command {
    inputs: Vec<String>,
    graph: String,
    output_args: Vec<String>,
    total_ms: i64,
}

/// Shared by loudness measurement and the selected float-PCM mix.
pub(super) async fn run(
    pass: Command,
    directory: &Path,
    cancel: &CancellationToken,
    mut report: impl FnMut(i64, i64),
) -> Result<String, String> {
    let total = pass.total_ms.max(1);
    tokio::fs::write(directory.join("audio.txt"), pass.graph).await.map_err(|e| e.to_string())?;
    let mut command = crate::tools::ffmpeg_command();
    command.current_dir(directory)
        .args(["-hide_banner", "-nostdin", "-y", "-nostats", "-loglevel", "info", "-progress", "pipe:1", "-filter_complex_threads", "1"])
        .args(pass.inputs)
        .args(["-filter_complex_script", "audio.txt"])
        .args(pass.output_args);
    report(0, total);
    let mut latest = 0;
    let mut reported = 0;
    let mut last_report = Instant::now();
    let stderr = engine::run_process(command, cancel, Duration::from_secs(180), |at, _| {
        latest = latest.max(at.clamp(0, total));
        if latest > reported && last_report.elapsed() >= Duration::from_millis(250) {
            report(latest, total);
            reported = latest;
            last_report = Instant::now();
        }
    }).await?;
    report(total, total);
    Ok(stderr)
}

pub(super) async fn prepare_range(
    browser: &mut Browser,
    directory: &Path,
    cancel: &CancellationToken,
    mut report: impl FnMut(Progress),
) -> Result<bool, String> {
    let value = browser.eval("BiliupExport.prepareRangeAudio()").await?;
    if value.is_null() { return Ok(false); }
    let pass: Command = serde_json::from_value(value).map_err(|e| format!("音频处理计划无效：{e}"))?;
    run(pass, directory, cancel, |processed_ms, total_ms| {
        report(Progress::Audio { processed_ms, total_ms });
    }).await?;
    Ok(true)
}
