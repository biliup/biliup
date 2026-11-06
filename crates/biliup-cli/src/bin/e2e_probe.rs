use biliup::client::StatelessClient;
use biliup::downloader::live::{LiveStatus, builtin_plugins};
use biliup_cli::server::config::{Config, StreamerConfig};
use biliup_cli::server::core::live::{batch_check_request, live_request};
use biliup_cli::server::errors::{AppError, AppResult};
use biliup_cli::server::infrastructure::context::Worker;
use biliup_cli::server::infrastructure::models::live_streamer::LiveStreamer;
use clap::Parser;
use error_stack::{Report, bail};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Probe Rust download plugins with biliup 1.0.7 config files"
)]
struct Args {
    /// biliup 1.0.7 style config.toml/config.yaml
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    if config.streamers.is_empty() {
        bail!(AppError::Custom(format!(
            "no streamers in {}",
            args.config.display()
        )));
    }

    let shared_config = Arc::new(RwLock::new(config.clone()));
    let plugins = builtin_plugins();

    let mut total = 0usize;
    let mut matched = 0usize;
    let mut live = 0usize;
    let mut offline = 0usize;
    let mut failed = 0usize;

    for (remark, streamer) in &config.streamers {
        for url in &streamer.url {
            total += 1;
            let Some(plugin) = plugins.iter().find(|plugin| plugin.matches(url)) else {
                failed += 1;
                println!("[NO_MATCH] {remark} {url}");
                continue;
            };
            matched += 1;

            let worker = Worker::new(
                to_live_streamer(total as i64, remark, url, streamer),
                None,
                shared_config.clone(),
                StatelessClient::default(),
            );
            let request = live_request(&worker);

            match plugin.check_stream(request).await {
                Ok(LiveStatus::Live { stream }) => {
                    live += 1;
                    println!(
                        "[LIVE] plugin={} remark={} url={} title={} raw_url={} suffix={} headers={}",
                        plugin.name(),
                        remark,
                        url,
                        stream.title,
                        stream.raw_stream_url,
                        stream.suffix,
                        stream.stream_headers.len()
                    );
                }
                Ok(LiveStatus::Offline) => {
                    offline += 1;
                    println!(
                        "[OFFLINE] plugin={} remark={} url={}",
                        plugin.name(),
                        remark,
                        url
                    );
                }
                Err(err) => {
                    failed += 1;
                    println!(
                        "[ERROR] plugin={} remark={} url={} error={err:?}",
                        plugin.name(),
                        remark,
                        url
                    );
                }
            }
        }
    }

    println!(
        "summary: total={total} matched={matched} live={live} offline={offline} failed={failed}"
    );

    // 批量检测探针：对支持 batch_check 的平台（Twitch/Picarto）各发起一次批量请求，
    // 验证 monitor 侧走的批量通路可用。批量失败不计入 failed（可能因地域/风控），仅告警。
    let mut by_platform: HashMap<&'static str, Vec<String>> = HashMap::new();
    for (remark, streamer) in &config.streamers {
        for url in &streamer.url {
            if let Some(plugin) = plugins.iter().find(|plugin| plugin.matches(url))
                && plugin.supports_batch_check()
            {
                by_platform
                    .entry(plugin.name())
                    .or_default()
                    .push(url.to_string());
            }
            let _ = remark;
        }
    }
    for (platform_name, urls) in by_platform {
        let Some(plugin) = plugins.iter().find(|plugin| plugin.name() == platform_name) else {
            continue;
        };
        let worker = Worker::new(
            to_live_streamer(0, platform_name, &urls[0], &StreamerConfig::default()),
            None,
            shared_config.clone(),
            StatelessClient::default(),
        );
        let request = batch_check_request(&worker, urls.clone());
        match plugin.batch_check(request).await {
            Ok(live_urls) => println!(
                "[BATCH] plugin={platform_name} checked={} live={} live_urls={live_urls:?}",
                urls.len(),
                live_urls.len()
            ),
            Err(err) => println!("[BATCH_WARN] plugin={platform_name} error={err:?}"),
        }
    }

    if failed > 0 {
        return Err(Report::new(AppError::Custom(format!(
            "{failed} e2e probes failed"
        ))));
    }

    Ok(())
}

fn to_live_streamer(id: i64, remark: &str, url: &str, streamer: &StreamerConfig) -> LiveStreamer {
    LiveStreamer {
        id,
        url: url.to_string(),
        remark: remark.to_string(),
        filename_prefix: streamer
            .filename_prefix
            .clone()
            .or_else(|| streamer.title.clone()),
        time_range: streamer.time_range.clone(),
        upload_streamers_id: None,
        format: streamer.format.clone(),
        override_cfg: streamer
            .override_cfg
            .clone()
            .map(|cfg| serde_json::from_value(serde_json::Value::Object(cfg.into_iter().collect())))
            .transpose()
            .unwrap_or_default(),
        preprocessor: streamer.preprocessor.clone(),
        segment_processor: streamer.segment_processor.clone(),
        downloaded_processor: streamer.downloaded_processor.clone(),
        postprocessor: streamer.postprocessor.clone(),
        opt_args: streamer
            .opt_args
            .as_ref()
            .map(|args| serde_json::Value::Array(args.iter().cloned().map(Into::into).collect())),
        excluded_keywords: streamer.excluded_keywords.as_ref().map(|keywords| {
            serde_json::Value::Array(keywords.iter().cloned().map(Into::into).collect())
        }),
    }
}
