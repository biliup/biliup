use crate::server::core::download_manager::UActor;
use crate::server::core::downloader::SegmentEvent;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::models::UploadStreamer;
use crate::server::infrastructure::models::hook_step::process_video;
use async_channel::Receiver;
use biliup::bilibili::{BiliBili, ResponseData, Studio, Video};
use biliup::client::StatelessClient;
use biliup::credential::login_by_cookies;
use biliup::uploader::line::{Line, Probe};
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, line};
use error_stack::ResultExt;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;
use tracing::{error, info};

// 辅助结构体
struct UploadContext {
    bilibili: BiliBili,
    client: StatelessClient,
    line: Line,
    threads: usize,
    upload_config: UploadStreamer,
}

#[derive(Default)]
struct UploadedVideos {
    videos: Vec<Video>,
    paths: Vec<PathBuf>,
}

pub async fn process_with_upload(
    first_event: SegmentEvent,
    rx: Receiver<SegmentEvent>,
    worker: &Worker,
    upload_config: UploadStreamer,
) -> AppResult<()> {
    // 1. 初始化上传环境
    let upload_context = initialize_upload_context(&worker, upload_config).await?;

    // 2. 流水线处理视频上传
    let uploaded_videos = pipeline_upload_videos(first_event, rx, &upload_context).await?;

    // 3. 提交到B站
    if !uploaded_videos.videos.is_empty() {
        submit_to_bilibili(&upload_context, uploaded_videos.videos, &worker).await?;
    }

    // 4. 执行后处理
    if !uploaded_videos.paths.is_empty() {
        execute_postprocessor(uploaded_videos.paths, &worker).await?;
    }

    Ok(())
}

async fn initialize_upload_context(
    worker: &Worker,
    upload_config: UploadStreamer,
) -> AppResult<UploadContext> {
    // 登录处理
    let cookie_file = upload_config
        .user_cookie
        .clone()
        .unwrap_or("cookies.json".to_string());
    let bilibili = login_by_cookies(&cookie_file, None)
        .await
        .change_context(AppError::Unknown)?;
    let config = worker.get_config();

    // 获取上传线路
    let line = get_upload_line(&worker.client, &config.lines).await?;

    Ok(UploadContext {
        bilibili,
        client: worker.client.clone(),
        line,
        threads: worker.get_config().threads as usize,
        upload_config,
    })
}

async fn get_upload_line(client: &StatelessClient, line: &str) -> AppResult<Line> {
    let line = match line {
        "bda2" => line::bda2(),
        "bda" => line::bda(),
        "tx" => line::tx(),
        "txa" => line::txa(),
        "bldsa" => line::bldsa(),
        "alia" => line::alia(),
        _ => Probe::probe(&client.client).await.unwrap_or_default(),
    };
    Ok(line)
}

async fn pipeline_upload_videos(
    first_event: SegmentEvent,
    rx: Receiver<SegmentEvent>,
    context: &UploadContext,
) -> AppResult<UploadedVideos> {
    // let mut desc_v2 = Vec::new();
    // for credit in desc_v2_credit {
    //     desc_v2.push(Credit {
    //         type_id: credit.type_id,
    //         raw_text: credit.raw_text,
    //         biz_id: credit.biz_id,
    //     });
    // }

    let mut uploaded = UploadedVideos::default();

    // 处理第一个事件
    let video = upload_single_file(&first_event.file_path, context).await?;
    uploaded.videos.push(video);
    uploaded.paths.push(first_event.file_path);

    // 流式处理后续事件
    while let Ok(event) = rx.recv().await {
        let video = upload_single_file(&event.file_path, context).await?;
        uploaded.videos.push(video);
        uploaded.paths.push(event.file_path);
        // 失败的文件不加入路径列表，避免后处理出错
    }

    Ok(uploaded)
}

async fn upload_single_file(file_path: &Path, context: &UploadContext) -> AppResult<Video> {
    let bilibili = &context.bilibili;
    let limit = context.threads;
    let client = &context.client;
    let line = &context.line;
    let video_path = file_path;

    println!(
        "{:?}",
        video_path
            .canonicalize()
            .change_context(AppError::Unknown)?
            .to_str()
    );
    info!("{line:?}");
    let video_file = VideoFile::new(video_path).change_context(AppError::Unknown)?;
    let total_size = video_file.total_size;
    let file_name = video_file.file_name.clone();
    let uploader = line
        .pre_upload(bilibili, video_file)
        .await
        .change_context(AppError::Unknown)?;

    let instant = Instant::now();

    let video = uploader
        .upload(client.clone(), limit, |vs| {
            vs.map(|vs| {
                let chunk = vs?;
                let len = chunk.len();
                Ok((chunk, len))
            })
        })
        .await
        .change_context(AppError::Unknown)?;
    let t = instant.elapsed().as_millis();
    info!(
        "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
        t as f64 / 1000.,
        total_size as f64 / 1000. / t as f64
    );
    Ok(video)
}

async fn submit_to_bilibili(
    context: &UploadContext,
    videos: Vec<Video>,
    worker: &Worker,
) -> AppResult<ResponseData> {
    let studio = build_studio(context, videos).await?;
    // let submit = match worker.config.read().unwrap().submit_api {
    //     Some(submit) => SubmitOption::from_str(&submit).unwrap_or(SubmitOption::App),
    //     _ => SubmitOption::App,
    // };

    // let submit_result = match submit {
    //     SubmitOption::BCutAndroid => {
    //         bilibili.submit_by_bcut_android(&studio, None).await
    //     }
    //     _ => bilibili.submit_by_app(&studio, None).await,
    // };

    let submit_option = worker
        .config
        .read()
        .unwrap()
        .submit_api
        .as_deref()
        .and_then(|s| SubmitOption::from_str(s).ok())
        .unwrap_or(SubmitOption::App);

    let result = match submit_option {
        SubmitOption::BCutAndroid => context
            .bilibili
            .submit_by_bcut_android(&studio, None)
            .await
            .change_context(AppError::Unknown)?,
        _ => context
            .bilibili
            .submit_by_app(&studio, None)
            .await
            .change_context(AppError::Unknown)?,
    };
    info!("Submit successful");
    Ok(result)
}

async fn build_studio(context: &UploadContext, videos: Vec<Video>) -> AppResult<Studio> {
    let upload_config = &context.upload_config;
    // 使用 Builder 模式简化构建
    let mut studio: Studio = Studio::builder()
        .desc(upload_config.description.clone().unwrap_or_default())
        .dtime(upload_config.dtime)
        .copyright(upload_config.copyright.unwrap_or(2))
        .cover(upload_config.cover_path.clone().unwrap_or_default())
        .dynamic(upload_config.dynamic.clone().unwrap_or_default())
        .source(upload_config.copyright_source.clone().unwrap_or_default())
        .tag(upload_config.tags.join(","))
        .tid(upload_config.tid.unwrap_or(171))
        .title(upload_config.title.clone().unwrap_or_default())
        .videos(videos)
        .dolby(upload_config.dolby.unwrap_or_default())
        // .lossless_music(upload_config.)
        .no_reprint(upload_config.no_reprint.unwrap_or_default())
        .charging_pay(upload_config.charging_pay.unwrap_or_default())
        .up_close_reply(upload_config.up_close_reply.unwrap_or_default())
        .up_selection_reply(upload_config.up_selection_reply.unwrap_or_default())
        .up_close_danmu(upload_config.up_close_danmu.unwrap_or_default())
        .desc_v2(None)
        .extra_fields(
            serde_json::from_str(&upload_config.extra_fields.clone().unwrap_or_default())
                .unwrap_or_default(), // 处理额外字段
        )
        .build();
    // 处理封面上传
    if !studio.cover.is_empty()
        && let Ok(c) = &std::fs::read(&studio.cover).map_err(|e| error!(e=?e))
        && let Ok(url) = context.bilibili.cover_up(c).await.map_err(|e| error!(e=?e))
    {
        studio.cover = url;
    };

    Ok(studio)
}

pub async fn execute_postprocessor(video_paths: Vec<PathBuf>, worker: &Worker) -> AppResult<()> {
    if let Some(processor) = worker.get_streamer().postprocessor {
        let paths: Vec<&Path> = video_paths.iter().map(|p| p.as_path()).collect();
        process_video(&paths, &processor).await?;
    }
    Ok(())
}
