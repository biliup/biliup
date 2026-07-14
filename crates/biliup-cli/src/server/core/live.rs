use crate::server::common::util::danmaku_filename_template;
use crate::server::config::Config;
use crate::server::core::downloader::streamlink::{Platform, Streamlink, StreamlinkDownloader};
use crate::server::core::downloader::ytdlp::{
    Backend as RuntimeYtDlpBackend, DownloadConfig as YtDlpConfig,
    YouTubeDownloader as YtDlpDownloader,
};
use crate::server::core::downloader::{DownloaderRuntime, DownloaderType, RustDanmakuClient};
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::models::StreamerInfo;
use biliup::downloader::live::{
    BatchCheckRequest, BilibiliOptions, CcOptions, DanmakuSource, DouyinOptions, DouyuOptions,
    DownloaderHint, HuyaOptions, KilakilaOptions, KuaishouOptions, LiveCredentials, LiveOptions,
    LiveRequest, LiveStream, RuntimeOptions, StreamlinkOptions, StreamlinkPlatform,
    TwitcastingOptions, TwitchOptions, YoutubeOptions, YtDlpBackend, YtDlpOptions,
};
use danmaku_client::{PlatformContext, RecorderConfig};
use std::path::PathBuf;
use std::sync::Arc;

pub fn live_request(worker: &Worker) -> LiveRequest {
    let config = worker.get_config();
    LiveRequest {
        client: worker.client.client.clone(),
        url: worker.get_streamer().url.clone(),
        name: worker.get_streamer().remark.clone(),
        options: live_options(&config),
        credentials: live_credentials(&config),
    }
}

/// 以某个 worker 的客户端与配置为基础，构造同平台的批量检测请求。
pub fn batch_check_request(worker: &Worker, urls: Vec<String>) -> BatchCheckRequest {
    let config = worker.get_config();
    BatchCheckRequest {
        client: worker.client.client.clone(),
        urls,
        options: live_options(&config),
        credentials: live_credentials(&config),
    }
}

fn live_options(config: &Config) -> LiveOptions {
    LiveOptions {
        bilibili: BilibiliOptions {
            qn: config.bili_qn.unwrap_or(25000),
            protocol: config
                .bili_protocol
                .clone()
                .unwrap_or_else(|| "stream".to_string()),
            cdn: config.bili_cdn.clone().unwrap_or_default(),
            cdn_fallback: config.bili_cdn_fallback.unwrap_or(false),
            hls_transcode_timeout: config.bili_hls_transcode_timeout.unwrap_or(60),
            anonymous_origin: config.bili_anonymous_origin.unwrap_or(false),
            live_api: config.bili_liveapi.clone(),
            fallback_api: config.bili_fallback_api.clone(),
            danmaku: config.bilibili_danmaku.unwrap_or(false),
            danmaku_raw: config.bilibili_danmaku_raw.unwrap_or(false),
            danmaku_detail: config.bilibili_danmaku_detail.unwrap_or(false),
        },
        cc: CcOptions {
            protocol: config
                .cc_protocol
                .clone()
                .unwrap_or_else(|| "hls".to_string()),
        },
        douyin: DouyinOptions {
            quality: config
                .douyin_quality
                .clone()
                .unwrap_or_else(|| "origin".to_string()),
            protocol: config
                .douyin_protocol
                .clone()
                .unwrap_or_else(|| "flv".to_string()),
            double_screen: config.douyin_double_screen.unwrap_or(false),
            true_origin: config.douyin_true_origin.unwrap_or(false),
            danmaku: config.douyin_danmaku.unwrap_or(false),
        },
        douyu: DouyuOptions {
            cdn: config
                .douyu_cdn
                .clone()
                .unwrap_or_else(|| "hw-h5".to_string()),
            force_hs: config.douyu_force_hs.unwrap_or(false),
            rate: config.douyu_rate.unwrap_or(0),
            disable_interactive_game: config.douyu_disable_interactive_game.unwrap_or(false),
            danmaku: config.douyu_danmaku.unwrap_or(false),
        },
        huya: HuyaOptions {
            cdn: config.huya_cdn.clone().unwrap_or_default(),
            cdn_fallback: config.huya_cdn_fallback.unwrap_or(false),
            max_ratio: config.huya_max_ratio.unwrap_or(0),
            protocol: config
                .huya_protocol
                .clone()
                .unwrap_or_else(|| "Flv".to_string()),
            imgplus: config.huya_imgplus.unwrap_or(true),
            mobile_api: config.huya_mobile_api.unwrap_or(false),
            codec: config
                .huya_codec
                .clone()
                .unwrap_or_else(|| "264".to_string()),
            danmaku: config.huya_danmaku.unwrap_or(false),
        },
        kilakila: KilakilaOptions {
            protocol: config
                .kila_protocol
                .clone()
                .unwrap_or_else(|| "hls".to_string()),
        },
        kuaishou: KuaishouOptions {
            cookie: config.kuaishou_cookie.clone(),
        },
        twitcasting: TwitcastingOptions {
            password: config.twitcasting_password.clone(),
            quality: config.twitcasting_quality.clone(),
            danmaku: config.twitcasting_danmaku.unwrap_or(false),
        },
        twitch: TwitchOptions {
            danmaku: config.twitch_danmaku.unwrap_or(false),
            disable_ads: config.twitch_disable_ads.unwrap_or(true),
        },
        youtube: YoutubeOptions {
            enable_download_live: config.youtube_enable_download_live.unwrap_or(true),
            enable_download_playback: config.youtube_enable_download_playback.unwrap_or(true),
            after_date: config.youtube_after_date.clone(),
            before_date: config.youtube_before_date.clone(),
            prefer_vcodec: config.youtube_prefer_vcodec.clone(),
            prefer_acodec: config.youtube_prefer_acodec.clone(),
            max_resolution: config.youtube_max_resolution,
            max_videosize: config.youtube_max_videosize.clone(),
            danmaku: config
                .youtube_danmaku
                .or(config.ytb_danmaku)
                .unwrap_or(false),
        },
    }
}

fn live_credentials(config: &Config) -> LiveCredentials {
    let Some(user) = &config.user else {
        return LiveCredentials::default();
    };
    LiveCredentials {
        bilibili_cookie: user.bili_cookie.clone(),
        bilibili_cookie_file: user.bili_cookie_file.clone(),
        douyin_cookie: user.douyin_cookie.clone(),
        twitcasting_cookie: user.twitcasting_cookie.clone(),
        twitch_cookie: user.twitch_cookie.clone(),
        youtube_cookie: user.youtube_cookie.clone(),
        afreecatv_username: user.afreecatv_username.clone(),
        afreecatv_password: user.afreecatv_password.clone(),
        niconico_email: user.niconico_email.clone(),
        niconico_password: user.niconico_password.clone(),
        niconico_user_session: user.niconico_user_session.clone(),
        niconico_purge_credentials: user.niconico_purge_credentials.clone(),
    }
}

pub fn streamer_info(stream: &LiveStream) -> StreamerInfo {
    StreamerInfo::new(
        &stream.name,
        &stream.url,
        &stream.title,
        stream.date,
        &stream.live_cover_url,
    )
}

pub fn downloader_runtime(
    config_type: Option<DownloaderType>,
    stream: &LiveStream,
) -> DownloaderRuntime {
    let downloader_type = config_type.unwrap_or_else(|| match stream.downloader_hint {
        DownloaderHint::StreamGears => DownloaderType::StreamGears,
        DownloaderHint::Ffmpeg => DownloaderType::Ffmpeg,
        DownloaderHint::Streamlink => DownloaderType::Streamlink,
        DownloaderHint::YtDlp => DownloaderType::YtDlp,
    });

    match downloader_type {
        DownloaderType::Streamlink => streamlink_runtime(stream),
        // Twitch 指定 ffmpeg 时也走 streamlink：Python 版 ffmpeg 消费的是
        // streamlink --player-external-http 的代理输出（twitch.py:114-144），
        // 去广告与 OAuth 鉴权都由 streamlink 完成，直连 usher 直链会丢掉这两项
        DownloaderType::Ffmpeg
            if matches!(
                stream.runtime_options.as_ref(),
                Some(RuntimeOptions::Streamlink(StreamlinkOptions {
                    platform: StreamlinkPlatform::Twitch { .. },
                    ..
                }))
            ) =>
        {
            streamlink_runtime(stream)
        }
        DownloaderType::YtDlp | DownloaderType::Ytarchive => ytdlp_runtime(stream, downloader_type),
        _ => DownloaderRuntime::from_type(downloader_type),
    }
}

fn streamlink_runtime(stream: &LiveStream) -> DownloaderRuntime {
    let (url, platform) = match stream.runtime_options.as_ref() {
        Some(RuntimeOptions::Streamlink(StreamlinkOptions { url, platform })) => (
            url.clone().unwrap_or_else(|| stream.raw_stream_url.clone()),
            streamlink_platform(platform),
        ),
        // yt-dlp 型来源（如 YouTube）交给 streamlink 时，传入网页地址让其自行提取，
        // 而非已解析的 manifest 直链（对齐 youtube.py:96-101）
        Some(RuntimeOptions::YtDlp(options)) if !options.webpage_url.is_empty() => {
            (options.webpage_url.clone(), Platform::Generic)
        }
        _ => (stream.raw_stream_url.clone(), Platform::Generic),
    };
    let downloader =
        StreamlinkDownloader::new(url, platform).with_headers(stream.stream_headers.clone());
    DownloaderRuntime::StreamLink(Streamlink::new(downloader))
}

fn streamlink_platform(platform: &StreamlinkPlatform) -> Platform {
    match platform {
        StreamlinkPlatform::Bilibili => Platform::Bilibili,
        StreamlinkPlatform::Twitch {
            disable_ads,
            auth_token,
        } => Platform::Twitch {
            disable_ads: *disable_ads,
            auth_token: auth_token.clone(),
        },
        StreamlinkPlatform::Niconico {
            email,
            password,
            user_session,
            purge_credentials,
        } => Platform::Niconico {
            email: email.clone(),
            password: password.clone(),
            user_session: user_session.clone(),
            purge_credentials: purge_credentials.clone(),
        },
        StreamlinkPlatform::Generic => Platform::Generic,
    }
}

fn ytdlp_runtime(stream: &LiveStream, downloader_type: DownloaderType) -> DownloaderRuntime {
    let cfg = match stream.runtime_options.as_ref() {
        Some(RuntimeOptions::YtDlp(options)) => ytdlp_config(options, downloader_type),
        _ => YtDlpConfig {
            webpage_url: stream.url.clone(),
            download_url: Some(stream.raw_stream_url.clone()),
            backend: ytdlp_backend(None, downloader_type),
            ..Default::default()
        },
    };
    DownloaderRuntime::YtDlp(YtDlpDownloader::new(cfg))
}

fn ytdlp_config(options: &YtDlpOptions, downloader_type: DownloaderType) -> YtDlpConfig {
    YtDlpConfig {
        webpage_url: options.webpage_url.clone(),
        download_url: options.download_url.clone(),
        backend: ytdlp_backend(Some(options.backend), downloader_type),
        is_live: options.is_live,
        use_live_cover: options.use_live_cover,
        cover_url: options.cover_url.clone(),
        cookies_file: options.cookies_file.clone(),
        prefer_vcodec: options.prefer_vcodec.clone(),
        prefer_acodec: options.prefer_acodec.clone(),
        max_filesize: options.max_filesize.clone(),
        max_height: options.max_height,
        download_archive: options.download_archive.clone(),
        extra_ytdlp_args: options.extra_ytdlp_args.clone(),
        ..Default::default()
    }
}

fn ytdlp_backend(
    option: Option<YtDlpBackend>,
    downloader_type: DownloaderType,
) -> RuntimeYtDlpBackend {
    match (downloader_type, option) {
        (DownloaderType::Ytarchive, _) => RuntimeYtDlpBackend::YtArchive,
        (_, Some(YtDlpBackend::YtArchive)) => RuntimeYtDlpBackend::YtArchive,
        _ => RuntimeYtDlpBackend::YtDlp,
    }
}

pub fn danmaku_client(
    source: Option<&DanmakuSource>,
    filename_prefix: Option<&str>,
    name: &str,
) -> Option<Arc<dyn crate::server::core::downloader::DanmakuClient + Send + Sync>> {
    let source = source?;
    let mut context = PlatformContext::new();
    if let Some(room_id) = &source.room_id {
        context = context.with_room_id(room_id.clone());
    }
    if let Some(cookie) = &source.cookie {
        context = context.with_cookie(cookie.clone());
    }
    context.extra = source.extra.clone();
    context.movie_id = source.movie_id.clone();
    context.password = source.password.clone();

    let config = RecorderConfig::new(
        source.url.clone(),
        PathBuf::from(danmaku_filename_template(filename_prefix, name)),
    )
    .with_context(context)
    .with_raw(source.raw)
    .with_detail(source.detail);

    Some(Arc::new(RustDanmakuClient::new(config)))
}
