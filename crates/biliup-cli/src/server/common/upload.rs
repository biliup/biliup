use crate::UploadLine;
use crate::server::common::util::Recorder;
use crate::server::config::{Config, TemplateCredit};
use crate::server::core::downloader::SegmentInfo;
use crate::server::core::slots::Slots;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::InsertFileItem;
use crate::server::infrastructure::models::hook_step::{
    HookStep, process_video, process_video_paths,
};
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::workbench::retention::Retention;
use async_channel::Receiver;
use biliup::bilibili::{BiliBili, Credit, ResponseData, Studio, Video};
use biliup::client::StatelessClient;
use biliup::credential::login_by_cookies;
use biliup::error::Kind;
use biliup::uploader::line::{Line, Probe, StreamParcel, UploadedStream};
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, line};
use bytes::Bytes;
use error_stack::ResultExt;
use futures::Stream;
use futures::StreamExt;
use futures::stream::Inspect;
use ormlite::Insert;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::pin;
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info, warn};

// 辅助结构体
#[derive(Clone)]
pub(crate) struct UploadContext {
    pub(crate) bilibili: BiliBili,
    pub(crate) line: Line,
    pub(crate) threads: usize,
    pub(crate) client: StatelessClient,
}

#[derive(Default)]
pub(crate) struct UploadedVideos {
    pub(crate) videos: Vec<Video>,
    pub(crate) paths: Vec<PathBuf>,
}

pub async fn process_with_upload<F>(
    rx: Inspect<Receiver<SegmentInfo>, F>,
    ctx: &Context,
    upload_config: &UploadStreamer,
) -> AppResult<()>
where
    F: FnMut(&SegmentInfo),
{
    info!(upload_config=?upload_config, "Starting process with upload");
    if let Some(plan) = crate::server::fleet::ha::upload_plan(ctx).await {
        return plan.run(rx, ctx, upload_config).await;
    }
    // 1. 初始化上传环境
    let upload_context =
        initialize_upload_context(&ctx.config(), &ctx.stateless_client(), upload_config).await?;

    // 2. 流水线处理视频上传（segment_processor 在每段上传前执行；用于 Remux 等
    // 在原地改写路径的预处理）
    let segment_processors: Vec<HookStep> = ctx
        .live_streamer()
        .segment_processor
        .clone()
        .unwrap_or_default();
    let uploaded_videos = pipeline_upload_videos(rx, &segment_processors, |path| {
        upload_owned_file(path, &upload_context)
    })
    .await?;

    // 3. 提交到B站
    if !uploaded_videos.videos.is_empty() {
        let mut recorder = ctx.recorder(ctx.streamer_info().clone()).clone();
        recorder.filename_prefix = upload_config.title.clone();

        let studio = build_studio(
            &upload_config,
            &upload_context.bilibili,
            uploaded_videos.videos,
            &recorder,
        )
        .await?;
        let submit_api = ctx.config().submit_api.clone();
        submit_to_bilibili(&upload_context.bilibili, &studio, submit_api.as_deref()).await?;
    }

    // 4. 执行后处理
    if !uploaded_videos.paths.is_empty() {
        execute_postprocessor(uploaded_videos.paths, ctx).await?;
    }

    Ok(())
}

async fn process_without_upload<F>(
    rx: Inspect<Receiver<SegmentInfo>, F>,
    ctx: &Context,
) -> AppResult<()>
where
    F: FnMut(&SegmentInfo),
{
    let mut paths = Vec::new();
    pin!(rx);
    while let Some(event) = rx.next().await {
        paths.extend(segment_paths(&event));
    }
    execute_postprocessor(paths, ctx).await
}

pub(crate) async fn initialize_upload_context(
    config: &Config,
    client: &StatelessClient,
    upload_config: &UploadStreamer,
) -> AppResult<UploadContext> {
    // 登录处理
    let cookie_file = upload_config
        .user_cookie
        .clone()
        .unwrap_or("cookies.json".to_string());
    let bilibili = login_by_cookies(&cookie_file, None).await;
    let bilibili = match bilibili {
        Err(Kind::IO(_)) => bilibili.change_context_lazy(|| {
            AppError::Custom(format!("open cookies file: {cookie_file}"))
        })?,
        _ => bilibili.change_context_lazy(|| {
            AppError::Custom(format!("login by cookies file failed: {cookie_file}"))
        })?,
    };

    // 获取上传线路
    let line = get_upload_line(&client.client, &config.lines).await?;

    Ok(UploadContext {
        bilibili,
        line,
        threads: config.threads as usize,
        client: client.clone(),
    })
}

async fn get_upload_line(client: &reqwest::Client, line: &str) -> AppResult<Line> {
    let line = match line {
        "bda2" => line::bda2(),
        "tx" => line::tx(),
        "txa" => line::txa(),
        "bldsa" => line::bldsa(),
        "alia" => line::alia(),
        "estx" => line::estx(),
        "akbd" => line::akbd(),
        _ => match Probe::probe(client).await {
            Ok(line) => line,
            Err(e) => {
                let fallback = Line::default();
                warn!(error = %e, ?fallback, "AUTO 线路测速失败，回退到默认线路");
                fallback
            }
        },
    };
    Ok(line)
}

pub(crate) fn segment_paths(event: &SegmentInfo) -> Vec<PathBuf> {
    let mut paths = vec![event.prev_file_path.clone()];
    if let Some(danmaku_file_path) = &event.danmaku_file_path {
        paths.push(danmaku_file_path.clone());
    }
    paths
}

/// 逐段跑 segment_processor 再交给 `upload` 上传
pub(crate) async fn pipeline_upload_videos<S, U, Fut>(
    rx: S,
    segment_processors: &[HookStep],
    upload: U,
) -> AppResult<UploadedVideos>
where
    S: Stream<Item = SegmentInfo>,
    U: Fn(PathBuf) -> Fut,
    Fut: Future<Output = AppResult<Video>>,
{
    let mut uploaded = UploadedVideos::default();
    pin!(rx);
    // 流式处理后续事件
    while let Some(event) = rx.next().await {
        // segment_processor 在上传前对路径列表做就地转换（如 Remux .ts→.mp4）。
        // 单段失败（典型场景：磁盘满让 ffmpeg remux 写头失败）不应拖死整批——
        // 否则已成功上传的段也无法到达 submit + postprocessor，本地 `rm` 不触发，
        // 文件越堆越多，磁盘进一步紧张，形成正反馈。
        let mut paths = segment_paths(&event);
        if !segment_processors.is_empty()
            && let Err(e) = process_video_paths(&mut paths, segment_processors).await
        {
            error!(
                file = ?event.prev_file_path,
                "segment_processor failed, skipping segment: {:?}", e
            );
            continue;
        }
        let upload_path = paths
            .first()
            .cloned()
            .unwrap_or_else(|| event.prev_file_path.clone());
        match upload(upload_path.clone()).await {
            Ok(video) => {
                uploaded.videos.push(video);
                // 1.0.7 的 FileInfo(video, danmaku) 语义：上传完成后的 postprocessor
                // 继续接收本段视频路径和对应弹幕路径。segment_processor 可能已把
                // 首个视频路径原地替换（例如 Remux .ts→.mp4），因此这里保留转换后的路径集。
                uploaded.paths.extend(paths);
            }
            Err(e) => {
                error!(
                    file = ?upload_path,
                    "upload_single_file failed, skipping segment: {:?}", e
                );
            }
        }
    }

    Ok(uploaded)
}

pub(crate) async fn upload_single_file(
    file_path: &Path,
    context: &UploadContext,
) -> AppResult<Video> {
    upload_single_file_with_progress(file_path, context, |_| true).await
}

async fn upload_owned_file(file_path: PathBuf, context: &UploadContext) -> AppResult<Video> {
    upload_single_file(&file_path, context).await
}

/// 同 [`upload_single_file`]，每读出一块交给上传前用这块的字节数回调 `progress`；
/// 回调返回 `false` 时不再传后面的分块，上传以错误结束。
pub(crate) async fn upload_single_file_with_progress(
    file_path: &Path,
    context: &UploadContext,
    progress: impl Fn(usize) -> bool + Send + Sync,
) -> AppResult<Video> {
    let video_path = file_path;
    let UploadContext {
        bilibili,
        line,
        threads: limit,
        client,
    } = context;

    info!(
        "开始上传文件：{:?}",
        video_path
            .canonicalize()
            .change_context(AppError::Unknown)?
            .to_str()
    );
    info!("线路选择：{line:?}");
    let video_file = VideoFile::new(video_path).change_context(AppError::Unknown)?;
    let total_size = video_file.total_size;
    let file_name = video_file.file_name.clone();
    let uploader = line
        .pre_upload(bilibili, video_file)
        .await
        .change_context(AppError::Unknown)?;

    let instant = Instant::now();

    let video = uploader
        .upload(client.clone(), *limit, |vs| {
            vs.map(|vs| {
                let chunk = vs?;
                let len = chunk.len();
                if !progress(len) {
                    return Err(Kind::Custom("上传已取消".into()));
                }
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

pub async fn submit_to_bilibili(
    bilibili: &BiliBili,
    studio: &Studio,
    submit_api: Option<&str>,
) -> AppResult<ResponseData> {
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

    let submit_option = match submit_api {
        Some(submit) => SubmitOption::from_str(submit).unwrap_or(SubmitOption::App),
        _ => SubmitOption::App,
    };

    let result = submit_with_web_fallback(submit_option, |option| async move {
        match option {
            SubmitOption::BCutAndroid => bilibili.submit_by_bcut_android(studio, None).await,
            SubmitOption::Web => bilibili.submit_by_web(studio, None).await,
            SubmitOption::App => bilibili.submit_by_app(studio, None).await,
        }
    })
    .await?;
    info!("Submit successful");
    Ok(result)
}

/// 「转载类型稿件不支持活动参加哦~」。B 站 app 端投稿在稿件没带活动 ID 时，会拿第一个标签去匹配
/// 进行中的活动，匹配上就当作参加了这个活动，转载稿因此被拒——用户并没有选活动。Web 接口没有这一步。
const REPRINT_JOINED_MISSION: i32 = 21071;

/// app / 必剪接口返回这些 code 时改用 Web 接口再投一次：`(code, 重投前的 warn, Web 投成功后的 info)`。
/// 都是投稿前的校验，稿件还没建，重投不会重复。
static WEB_FALLBACKS: [(i32, &str, &str); 1] = [(
    REPRINT_JOINED_MISSION,
    "B 站把这条转载稿当成参加了活动（第一个标签和进行中的活动同名时会这样），改用 Web 接口重投",
    "改用 Web 接口投稿成功；B 站会从稿件里去掉活动标签",
)];

/// 按 `option` 投稿；app / 必剪接口报 [`WEB_FALLBACKS`] 里的 code 时改用 Web 接口再投一次。
async fn submit_with_web_fallback<F, Fut>(
    option: SubmitOption,
    submit: F,
) -> AppResult<ResponseData>
where
    F: Fn(SubmitOption) -> Fut,
    Fut: Future<Output = biliup::error::Result<ResponseData>>,
{
    let ret = match submit(option.clone()).await {
        Err(Kind::SubmitRejected(ret)) if !matches!(option, SubmitOption::Web) => ret,
        result => return result.change_context(AppError::Unknown),
    };
    let Some((_, reason, recovered)) = WEB_FALLBACKS.iter().find(|(code, ..)| *code == ret.code)
    else {
        return Err(Kind::SubmitRejected(ret)).change_context(AppError::Unknown);
    };
    warn!(api = ?option, code = ret.code, message = ret.message(), "{reason}");
    let retried = submit(SubmitOption::Web).await;
    if retried.is_ok() {
        info!("{recovered}");
    }
    let fallback_failed = format!(
        "{option:?} 接口投稿被拒：{}（code {}），改用 Web 接口重投也失败了",
        ret.message(),
        ret.code
    );
    retried
        .change_context(AppError::Unknown)
        .change_context_lazy(|| AppError::Custom(fallback_failed))
}

pub async fn edit_to_bilibili(
    bilibili: &BiliBili,
    studio: &Studio,
    submit_api: Option<&str>,
) -> AppResult<serde_json::Value> {
    let submit_option = match submit_api {
        Some(submit) => SubmitOption::from_str(submit).unwrap_or(SubmitOption::App),
        _ => SubmitOption::App,
    };

    let result = match submit_option {
        SubmitOption::Web => bilibili
            .edit_by_web(studio)
            .await
            .change_context(AppError::Unknown)?,
        _ => bilibili
            .edit_by_app(studio, None)
            .await
            .change_context(AppError::Unknown)?,
    };
    info!("Edit successful");
    Ok(result)
}

pub(crate) fn aid_from_submit(ret: &ResponseData) -> AppResult<u64> {
    ret.data
        .as_ref()
        .and_then(|v| v.get("aid"))
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        .ok_or_else(|| AppError::Custom("投稿成功但未返回 aid".into()).into())
}

/// 边录边传：把内存分片流上传到 UPOS。上传并发固定为 3，对齐原 sync-downloader。
pub(crate) async fn upload_byte_stream_parts<S>(
    context: &UploadContext,
    parcel: StreamParcel,
    stream: S,
) -> AppResult<UploadedStream>
where
    S: Stream<Item = biliup::error::Result<(Bytes, usize)>>,
{
    let file_name = parcel.file_name().to_string();
    let total_size = parcel.total_size();
    info!("开始流式上传：{file_name} ({total_size} bytes)");
    info!("线路选择：{:?}", context.line);
    let instant = Instant::now();
    let uploaded = parcel
        .upload_parts(context.client.clone(), 3, stream)
        .await
        .change_context(AppError::Unknown)?;
    let t = instant.elapsed().as_millis().max(1);
    info!(
        "Stream parts uploaded: {file_name} => cost {:.2}s, {:.2} MB/s.",
        t as f64 / 1000.,
        uploaded.uploaded_size() as f64 / 1000. / t as f64
    );
    Ok(uploaded)
}

pub(crate) async fn complete_byte_stream(uploaded: UploadedStream) -> AppResult<Video> {
    uploaded.complete().await.change_context(AppError::Unknown)
}

// 解析投稿的「转载来源」(source) 字段。
// 前端表单留空时会把 copyright_source 提交为空字符串 `Some("")`，
// 若直接透传则 B 站接口收到空 source，且不会回退到直播间地址。
// 这里把 None 以及空白字符串都视作「未填写」，统一回退到直播间地址，
pub(crate) fn resolve_source(copyright_source: Option<&str>, fallback_url: &str) -> String {
    match copyright_source.map(str::trim) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => fallback_url.to_string(),
    }
}

/// 把配置里的 `dtime` 转成 B 站要求的 10 位 Unix 时间戳。
///
/// Web UI / Python 版存的是**延迟秒数**（提交后再等这么久公开），B 站接口要的是绝对时间。
/// 已经是 Unix 时间戳（≥ 1_000_000_000）的值原样透传，避免 CLI `--dtime` 被加两次。
pub(crate) fn scheduled_publish_ts(dtime: Option<u32>, now_unix: u64) -> Option<u32> {
    let value = dtime?;
    const UNIX_TS_FLOOR: u32 = 1_000_000_000; // 2001-09-09
    let ts = if value >= UNIX_TS_FLOOR {
        value as u64
    } else {
        now_unix.saturating_add(value as u64)
    };
    u32::try_from(ts).ok()
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

const CREDIT_PLACEHOLDER: &str = "@credit";

/// 读出模板里能用的 credits：用户名去掉首尾空白和误填的 `@`，uid 必须是纯数字。
/// 不合格的项跳过并告警，不占用 `@credit` 占位符，免得一项填错让整次投稿被 B 站拒掉。
fn template_credits(credits: Option<&serde_json::Value>) -> Vec<TemplateCredit> {
    let Some(serde_json::Value::Array(items)) = credits else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let credit = serde_json::from_value::<TemplateCredit>(item.clone())
                .inspect_err(|e| warn!(credit = %item, error = %e, "忽略无法解析的简介 @ 配置"))
                .ok()?;
            let username = credit.username.trim().trim_start_matches('@').trim();
            let uid = credit.uid.trim();
            if username.is_empty() || uid.is_empty() || !uid.bytes().all(|b| b.is_ascii_digit()) {
                warn!(credit = %item, "忽略用户名为空或 uid 不是数字的简介 @ 配置");
                return None;
            }
            Some(TemplateCredit {
                username: username.to_string(),
                uid: uid.to_string(),
            })
        })
        .collect()
}

/// 把简介里的 `@credit` 依次换成 `credits`，返回纯文本简介和 B 站的 `desc_v2`。
///
/// 形状与旧 Python 版 `creditsToDesc_v2` 一致（B 站已长期接受）：纯文本里写成
/// `@用户名` 加两个空格；`desc_v2` 里被 @ 的用户是 `type: 2` 节点，其后的文本节点前补一个空格。
/// 与旧版不同的是不产出空文本节点——`@credit` 在开头时，B 站对开头的空 `type: 1`
/// 节点报 21010。没有 credits 或简介里没有占位符时返回 `None`，简介原样提交。
fn credits_to_desc_v2(desc: &str, credits: &[TemplateCredit]) -> Option<(String, Vec<Credit>)> {
    if credits.is_empty() || !desc.contains(CREDIT_PLACEHOLDER) {
        return None;
    }
    fn push_text(nodes: &mut Vec<Credit>, text: &str, after_mention: bool) {
        if text.is_empty() {
            return;
        }
        let raw_text = if after_mention {
            format!(" {text}")
        } else {
            text.to_string()
        };
        nodes.push(Credit {
            type_id: 1,
            raw_text,
            biz_id: Some(String::new()),
        });
    }

    let mut plain = String::with_capacity(desc.len());
    let mut nodes = Vec::new();
    let mut rest = desc;
    let mut used = 0;
    for credit in credits {
        let Some(pos) = rest.find(CREDIT_PLACEHOLDER) else {
            break;
        };
        let before = &rest[..pos];
        push_text(&mut nodes, before, used > 0);
        plain.push_str(before);
        plain.push('@');
        plain.push_str(&credit.username);
        plain.push_str("  ");
        nodes.push(Credit {
            type_id: 2,
            raw_text: credit.username.clone(),
            biz_id: Some(credit.uid.clone()),
        });
        rest = &rest[pos + CREDIT_PLACEHOLDER.len()..];
        used += 1;
    }
    push_text(&mut nodes, rest, true);
    plain.push_str(rest);

    if used < credits.len() {
        warn!(
            credits = credits.len(),
            placeholders = used,
            "简介里的 @credit 少于 credits，多出的 credits 未使用"
        );
    } else if rest.contains(CREDIT_PLACEHOLDER) {
        warn!(
            credits = credits.len(),
            "简介里的 @credit 多于 credits，多出的占位符按原文提交"
        );
    }
    Some((plain, nodes))
}

/// 按模板的 credits 展开简介里的 `@credit`，返回提交用的简介和 `desc_v2`
/// （没有可用的 credits 或占位符时为 `None`，简介原样返回）。
pub(crate) fn desc_with_credits(
    desc: String,
    credits: Option<&serde_json::Value>,
) -> (String, Option<Vec<Credit>>) {
    match credits_to_desc_v2(&desc, &template_credits(credits)) {
        Some((plain, nodes)) => (plain, Some(nodes)),
        None => (desc, None),
    }
}

pub(crate) async fn build_studio(
    upload_config: &UploadStreamer,
    bilibili: &BiliBili,
    videos: Vec<Video>,
    recorder: &Recorder,
) -> AppResult<Studio> {
    let mut studio = studio_from_template(upload_config, videos, recorder);
    // 处理封面上传
    if !studio.cover.is_empty()
        && let Ok(c) = &std::fs::read(&studio.cover).inspect_err(|e| error!(e=?e))
        && let Ok(url) = bilibili.cover_up(c).await.inspect_err(|e| error!(e=?e))
    {
        studio.cover = url;
    };

    Ok(studio)
}

/// 按上传模板拼出稿件；`cover` 还是本地路径，由调用方上传。
pub(crate) fn studio_from_template(
    upload_config: &UploadStreamer,
    videos: Vec<Video>,
    recorder: &Recorder,
) -> Studio {
    let (desc, desc_v2) = desc_with_credits(
        recorder.format(&upload_config.description.clone().unwrap_or_default()),
        upload_config.credits.as_ref(),
    );
    Studio::builder()
        .desc(desc)
        .maybe_dtime(scheduled_publish_ts(upload_config.dtime, now_unix()))
        .maybe_copyright(upload_config.copyright)
        .cover(upload_config.cover_path.clone().unwrap_or_default())
        .dynamic(upload_config.dynamic.clone().unwrap_or_default())
        .source(resolve_source(
            upload_config.copyright_source.as_deref(),
            &recorder.streamer_info.url,
        ))
        .tag(upload_config.tags.join(","))
        .maybe_tid(upload_config.tid)
        .maybe_tid_v2(upload_config.tid_v2)
        .title(recorder.format_title())
        .videos(videos)
        .dolby(upload_config.dolby.unwrap_or_default())
        // .lossless_music(upload_config.)
        .no_reprint(upload_config.no_reprint.unwrap_or_default())
        .charging_pay(upload_config.charging_pay.unwrap_or_default())
        .up_close_reply(upload_config.up_close_reply.unwrap_or_default())
        .up_selection_reply(upload_config.up_selection_reply.unwrap_or_default())
        .up_close_danmu(upload_config.up_close_danmu.unwrap_or_default())
        .maybe_is_only_self(upload_config.is_only_self)
        .maybe_desc_v2(desc_v2)
        .extra_fields(
            serde_json::from_str(&upload_config.extra_fields.clone().unwrap_or_default())
                .unwrap_or_default(), // 处理额外字段
        )
        .build()
}

pub async fn execute_postprocessor(video_paths: Vec<PathBuf>, ctx: &Context) -> AppResult<()> {
    if let Some(processor) = &ctx.live_streamer().postprocessor {
        let paths: Vec<&Path> = video_paths.iter().map(|p| p.as_path()).collect();
        let retention = Retention::after_upload(ctx.pool().clone(), &ctx.config());
        process_video(&paths, processor, Some(&retention)).await?;
    }
    Ok(())
}

pub async fn upload(
    cookie_file: impl AsRef<Path>,
    proxy: Option<&str>,
    line: Option<UploadLine>,
    video_paths: &[PathBuf],
    limit: usize,
) -> AppResult<(BiliBili, Vec<Video>)> {
    let bilibili = login_by_cookies(&cookie_file, proxy).await;
    let bilibili = match bilibili {
        Err(Kind::IO(_)) => bilibili.change_context_lazy(|| {
            AppError::Custom(format!(
                "open cookies file: {}",
                &cookie_file.as_ref().to_string_lossy()
            ))
        })?,
        _ => bilibili.change_context_lazy(|| AppError::Unknown)?,
    };

    let client = StatelessClient::default();
    let mut videos = Vec::new();
    let line = match line {
        Some(UploadLine::Bldsa) => line::bldsa(),
        Some(UploadLine::Cnbldsa) => line::cnbldsa(),
        Some(UploadLine::Andsa) => line::andsa(),
        Some(UploadLine::Atdsa) => line::atdsa(),
        Some(UploadLine::Bda2) => line::bda2(),
        Some(UploadLine::Cnbd) => line::cnbd(),
        Some(UploadLine::Anbd) => line::anbd(),
        Some(UploadLine::Atbd) => line::atbd(),
        Some(UploadLine::Tx) => line::tx(),
        Some(UploadLine::Cntx) => line::cntx(),
        Some(UploadLine::Antx) => line::antx(),
        Some(UploadLine::Attx) => line::attx(),
        Some(UploadLine::Txa) => line::txa(),
        Some(UploadLine::Alia) => line::alia(),
        Some(UploadLine::Estx) => line::estx(),
        Some(UploadLine::Akbd) => line::akbd(),
        _ => match Probe::probe(&client.client).await {
            Ok(line) => line,
            Err(e) => {
                let fallback = Line::default();
                warn!(error = %e, ?fallback, "AUTO 线路测速失败，回退到默认线路");
                fallback
            }
        },
    };
    for video_path in video_paths {
        println!(
            "{:?}",
            video_path
                .canonicalize()
                .change_context_lazy(|| AppError::Unknown)?
                .to_str()
        );
        info!("{line:?}");
        let video_file = VideoFile::new(video_path).change_context_lazy(|| AppError::Unknown)?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let uploader = line
            .pre_upload(&bilibili, video_file)
            .await
            .change_context_lazy(|| AppError::Unknown)?;

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
            .change_context_lazy(|| AppError::Unknown)?;
        let t = instant.elapsed().as_millis();
        info!(
            "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
            t as f64 / 1000.,
            total_size as f64 / 1000. / t as f64
        );
        videos.push(video);
    }

    Ok((bilibili, videos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_paths_keeps_video_only_without_danmaku() {
        let video = PathBuf::from("segment.ts");
        let event = SegmentInfo::new(video.clone(), None, None, 0);

        assert_eq!(segment_paths(&event), vec![video]);
    }

    #[test]
    fn segment_paths_keeps_video_then_danmaku_when_present() {
        let video = PathBuf::from("segment.ts");
        let danmaku = PathBuf::from("segment.xml");
        let event = SegmentInfo::new(video.clone(), Some(danmaku.clone()), None, 0);

        assert_eq!(segment_paths(&event), vec![video, danmaku]);
    }

    const LIVE_URL: &str = "https://live.douyin.com/123456";

    #[test]
    fn resolve_source_falls_back_when_none() {
        // 配置文件未提供 copyright_source
        assert_eq!(resolve_source(None, LIVE_URL), LIVE_URL);
    }

    #[test]
    fn resolve_source_falls_back_when_empty_string() {
        // 前端表单留空 -> Some("")，应回退到直播间地址（核心 bug 场景）
        assert_eq!(resolve_source(Some(""), LIVE_URL), LIVE_URL);
    }

    #[test]
    fn resolve_source_falls_back_when_whitespace_only() {
        // 仅空白同样视作未填写
        assert_eq!(resolve_source(Some("   "), LIVE_URL), LIVE_URL);
    }

    #[test]
    fn resolve_source_keeps_user_value_and_trims() {
        // 用户填写了真实来源则保留（并去除首尾空白）
        assert_eq!(
            resolve_source(Some("  https://b23.tv/abc  "), LIVE_URL),
            "https://b23.tv/abc"
        );
    }

    #[test]
    fn scheduled_publish_ts_adds_delay_seconds() {
        // UI 选 4 小时后公开：存 14400，投稿时应写成 now+14400
        assert_eq!(
            scheduled_publish_ts(Some(4 * 3600), 1_700_000_000),
            Some(1_700_000_000 + 4 * 3600)
        );
    }

    #[test]
    fn scheduled_publish_ts_passes_through_unix_timestamp() {
        assert_eq!(
            scheduled_publish_ts(Some(1_700_014_400), 1_700_000_000),
            Some(1_700_014_400)
        );
    }

    #[test]
    fn scheduled_publish_ts_none_stays_none() {
        assert_eq!(scheduled_publish_ts(None, 1_700_000_000), None);
    }

    #[test]
    fn studio_submit_payload_includes_tid_v2_when_set() {
        // submit_by_app / submit_by_web both POST `.json(studio)`; verify body shape.
        let studio: Studio = serde_json::from_value(serde_json::json!({
            "tid": 95,
            "tid_v2": 2102,
            "title": "payload",
            "copyright": 1,
            "up_selection_reply": false,
            "up_close_reply": false,
            "up_close_danmu": false
        }))
        .unwrap();
        let body = serde_json::to_value(&studio).unwrap();
        assert_eq!(body["tid"], 95);
        assert_eq!(body["tid_v2"], 2102);
    }

    #[test]
    fn studio_submit_payload_omits_tid_v2_for_tid_only() {
        let studio: Studio = serde_json::from_value(serde_json::json!({
            "tid": 171,
            "title": "payload",
            "copyright": 1,
            "up_selection_reply": false,
            "up_close_reply": false,
            "up_close_danmu": false
        }))
        .unwrap();
        let body = serde_json::to_value(&studio).unwrap();
        assert_eq!(body["tid"], 171);
        assert!(body.get("tid_v2").is_none());
    }

    #[test]
    fn aid_from_submit_reads_numeric_aid() {
        let ret: ResponseData = serde_json::from_value(serde_json::json!({
            "code": 0,
            "data": {"aid": 12345, "bvid": "BV1xx"},
            "message": "0",
            "ttl": 1
        }))
        .unwrap();
        assert_eq!(aid_from_submit(&ret).unwrap(), 12345);
    }

    #[test]
    fn aid_from_submit_rejects_missing_data() {
        let ret: ResponseData = serde_json::from_value(serde_json::json!({
            "code": 0,
            "data": {},
            "message": "0",
            "ttl": 1
        }))
        .unwrap();
        assert!(aid_from_submit(&ret).is_err());
    }
}

/// 上传Actor
/// 负责处理上传相关的消息和任务
pub struct UActor {
    /// 上传消息接收器
    receiver: Receiver<UploaderMessage>,
    /// 上传池槽位（pool2_size）：每条消息的上传流程占用一个，处理完归还
    slots: Arc<Slots>,
}

impl UActor {
    /// 创建新的上传Actor实例
    pub fn new(receiver: Receiver<UploaderMessage>, slots: Arc<Slots>) -> Self {
        Self { receiver, slots }
    }

    /// 运行Actor主循环，处理接收到的消息
    ///
    /// 同时处理的消息数不超过上传池容量，容量调整后立即生效。
    pub(crate) async fn run(self) {
        run_in_slots(self.receiver, self.slots, handle_message).await
    }
}

/// 按到达顺序取出消息，占到一个槽位后交给 `handle` 在独立任务里处理，处理完归还槽位。
///
/// 先取消息再占槽位：只有真有消息要处理时才占用，调小容量后不会有闲置却占着的槽位。
/// 处理任务都在本函数的 `JoinSet` 里，本函数所在任务被 abort 时一并取消。
async fn run_in_slots<M, F, Fut>(receiver: Receiver<M>, slots: Arc<Slots>, handle: F)
where
    F: Fn(M) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut tasks = JoinSet::new();
    while let Ok(msg) = receiver.recv().await {
        let slot = slots.acquire().await;
        // 回收已经结束的任务
        while let Some(result) = tasks.try_join_next() {
            report_task_exit(result);
        }
        let task = handle(msg);
        tasks.spawn(async move {
            let _slot = slot;
            task.await
        });
    }
    while let Some(result) = tasks.join_next().await {
        report_task_exit(result);
    }
}

fn report_task_exit(result: Result<(), JoinError>) {
    if let Err(e) = result {
        error!(error = %e, "上传任务异常退出");
    }
}

/// 处理上传消息
///
/// # 参数
/// * `msg` - 要处理的上传消息
async fn handle_message(msg: UploaderMessage) {
    match msg {
        UploaderMessage::SegmentEvent(rx, ctx) => {
            ctx.change_status(Stage::Upload, WorkerStatus::Pending)
                .await;
            let inspect = rx.inspect(|f| {
                let pool = ctx.pool().clone();
                let session_id = ctx.id();
                let file = f.prev_file_path.display().to_string();
                tokio::spawn(async move {
                    let result = InsertFileItem { file, session_id }.insert(&pool).await;
                    info!(result=?result, "Insert file");
                });
            });
            let result = match ctx.upload_config() {
                Some(config) if config.is_noop_uploader() => {
                    info!(
                        uploader = ?config.uploader,
                        "Skipping upload because uploader is Noop"
                    );
                    process_without_upload(inspect, &ctx).await
                }
                Some(config) => process_with_upload(inspect, &ctx, config).await,
                None => {
                    let mut paths = Vec::new();
                    pin!(inspect);
                    while let Some(event) = inspect.next().await {
                        paths.extend(segment_paths(&event));
                    }
                    // 无上传配置时，直接执行后处理
                    execute_postprocessor(paths, &ctx).await
                }
            };

            if let Err(e) = &result {
                error!("Process segment event failed: {}", e);
                crate::server::fleet::events::upload_failed(&ctx, e);
            }
            info!(url=ctx.live_streamer().url, result=?result, "后处理执行完毕：Finished processing segment event");
            ctx.change_status(Stage::Upload, WorkerStatus::Idle).await;
        }
    }
}

/// 上传消息枚举
/// 定义上传Actor可以处理的消息类型
#[derive(Debug)]
pub enum UploaderMessage {
    /// 分段事件消息，包含事件、接收器和工作器
    SegmentEvent(Receiver<SegmentInfo>, Context),
}

#[cfg(test)]
mod upload_pool_tests {
    use super::run_in_slots;
    use crate::server::core::slots::Slots;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    /// 等下一个开始处理的消息；超时说明本该开始的没有开始
    async fn next_start<T>(started: &mut mpsc::UnboundedReceiver<T>) -> T {
        tokio::time::timeout(Duration::from_secs(5), started.recv())
            .await
            .expect("应有上传任务开始")
            .unwrap()
    }

    async fn assert_nothing_starts<T: std::fmt::Debug>(started: &mut mpsc::UnboundedReceiver<T>) {
        let next = tokio::time::timeout(Duration::from_millis(100), started.recv()).await;
        assert!(next.is_err(), "不应有新的上传任务开始：{next:?}");
    }

    /// 同时处理的消息数不超过上传池容量；扩容 / 缩容都立即生效，缩容不打断在跑的任务
    #[tokio::test]
    async fn uploads_stay_within_the_pool_and_resizing_applies_immediately() {
        let (tx, rx) = async_channel::bounded(16);
        let slots = Arc::new(Slots::new(1));
        let (started_tx, mut started) = mpsc::unbounded_channel();
        let dispatcher = tokio::spawn(run_in_slots(
            rx,
            slots.clone(),
            move |(id, release): (usize, oneshot::Receiver<()>)| {
                let started_tx = started_tx.clone();
                async move {
                    started_tx.send(id).unwrap();
                    let _ = release.await;
                }
            },
        ));

        let mut releases = Vec::new();
        for id in 0..3 {
            let (release, wait) = oneshot::channel();
            tx.send((id, wait)).await.unwrap();
            releases.push(release);
        }
        let mut releases = releases.into_iter();

        // 容量 1：只有第一条在处理
        assert_eq!(next_start(&mut started).await, 0);
        assert_nothing_starts(&mut started).await;

        // 扩容后下一条立即开始，不必等在跑的结束
        slots.resize(2);
        assert_eq!(next_start(&mut started).await, 1);
        assert_nothing_starts(&mut started).await;

        // 缩回 1：在跑的两条照常跑完；结束一条后还占着 1 个，第三条要等占用数低于新容量
        slots.resize(1);
        releases.next().unwrap().send(()).unwrap();
        assert_nothing_starts(&mut started).await;
        releases.next().unwrap().send(()).unwrap();
        assert_eq!(next_start(&mut started).await, 2);

        releases.next().unwrap().send(()).unwrap();
        drop(tx);
        dispatcher.await.unwrap();
    }

    /// 某条消息的处理 panic 只结束它自己，槽位照常归还，后面的消息继续处理
    #[tokio::test]
    async fn a_panicking_upload_returns_its_slot() {
        let (tx, rx) = async_channel::bounded(16);
        let (started_tx, mut started) = mpsc::unbounded_channel();
        let dispatcher = tokio::spawn(run_in_slots(
            rx,
            Arc::new(Slots::new(1)),
            move |id: usize| {
                let started_tx = started_tx.clone();
                async move {
                    started_tx.send(id).unwrap();
                    assert_ne!(id, 0, "第一条消息的处理故意 panic");
                }
            },
        ));

        tx.send(0).await.unwrap();
        tx.send(1).await.unwrap();
        assert_eq!(next_start(&mut started).await, 0);
        assert_eq!(next_start(&mut started).await, 1);

        drop(tx);
        dispatcher.await.unwrap();
    }

    /// DownloadManager 销毁时 abort 上传Actor，在跑的上传任务要一起取消并归还槽位
    #[tokio::test]
    async fn aborting_the_actor_cancels_running_uploads() {
        let (tx, rx) = async_channel::bounded(16);
        let slots = Arc::new(Slots::new(1));
        let (started_tx, mut started) = mpsc::unbounded_channel();
        let dispatcher = tokio::spawn(run_in_slots(
            rx,
            slots.clone(),
            move |alive: oneshot::Sender<()>| {
                let started_tx = started_tx.clone();
                async move {
                    let _alive = alive;
                    started_tx.send(()).unwrap();
                    std::future::pending::<()>().await;
                }
            },
        ));

        let (alive, cancelled) = oneshot::channel();
        tx.send(alive).await.unwrap();
        next_start(&mut started).await;
        assert!(slots.try_acquire().is_none());

        dispatcher.abort();
        tokio::time::timeout(Duration::from_secs(5), cancelled)
            .await
            .expect("在跑的上传任务应随上传Actor一起取消")
            .unwrap_err();
        tokio::time::timeout(Duration::from_secs(5), slots.acquire())
            .await
            .expect("取消的上传任务应归还槽位");
    }
}

#[cfg(test)]
mod credit_tests {
    use super::*;

    fn credits(pairs: &[(&str, &str)]) -> Vec<TemplateCredit> {
        pairs
            .iter()
            .map(|(username, uid)| TemplateCredit {
                username: (*username).into(),
                uid: (*uid).into(),
            })
            .collect()
    }

    fn desc_v2_json(desc: &str, pairs: &[(&str, &str)]) -> (String, serde_json::Value) {
        let (plain, nodes) = credits_to_desc_v2(desc, &credits(pairs)).expect("应生成 desc_v2");
        (plain, serde_json::to_value(nodes).unwrap())
    }

    fn text(raw: &str) -> serde_json::Value {
        serde_json::json!({"type": 1, "raw_text": raw, "biz_id": ""})
    }

    fn mention(name: &str, uid: &str) -> serde_json::Value {
        serde_json::json!({"type": 2, "raw_text": name, "biz_id": uid})
    }

    #[test]
    fn desc_v2_leading_credit_has_no_empty_text_node() {
        let (plain, v2) = desc_v2_json(
            "@credit 2026年09月24日直播回放-游戏日",
            &[("羊腿umer", "22158819")],
        );
        assert_eq!(plain, "@羊腿umer   2026年09月24日直播回放-游戏日");
        assert_eq!(
            v2,
            serde_json::json!([
                mention("羊腿umer", "22158819"),
                text("  2026年09月24日直播回放-游戏日"),
            ])
        );
    }

    #[test]
    fn desc_v2_leading_credit_keeps_following_lines() {
        let desc =
            "@credit2026年09月23日直播录屏\nhttps://live.douyin.com/1\nhttps://live.douyin.com/2";
        let (plain, v2) = desc_v2_json(desc, &[("允崽来啦", "2063092494")]);
        assert_eq!(
            plain,
            "@允崽来啦  2026年09月23日直播录屏\nhttps://live.douyin.com/1\nhttps://live.douyin.com/2"
        );
        assert_eq!(
            v2,
            serde_json::json!([
                mention("允崽来啦", "2063092494"),
                text(
                    " 2026年09月23日直播录屏\nhttps://live.douyin.com/1\nhttps://live.douyin.com/2"
                ),
            ])
        );
    }

    #[test]
    fn desc_v2_credit_in_middle_of_sentence() {
        let (plain, v2) = desc_v2_json("感谢@credit的投喂", &[("花花", "1")]);
        assert_eq!(plain, "感谢@花花  的投喂");
        assert_eq!(
            v2,
            serde_json::json!([text("感谢"), mention("花花", "1"), text(" 的投喂")])
        );
    }

    #[test]
    fn desc_v2_credit_at_end_has_no_trailing_node() {
        let (plain, v2) = desc_v2_json("剪辑：@credit", &[("剪刀手", "2")]);
        assert_eq!(plain, "剪辑：@剪刀手  ");
        assert_eq!(
            v2,
            serde_json::json!([text("剪辑："), mention("剪刀手", "2")])
        );
    }

    #[test]
    fn desc_v2_multiple_credits_replace_in_order() {
        let (plain, v2) = desc_v2_json(
            "主播@credit 剪辑@credit\n【@credit】",
            &[
                ("羊腿umer", "22158819"),
                ("允崽来啦", "2063092494"),
                ("Nya Rime", "3"),
            ],
        );
        assert_eq!(plain, "主播@羊腿umer   剪辑@允崽来啦  \n【@Nya Rime  】");
        assert_eq!(
            v2,
            serde_json::json!([
                text("主播"),
                mention("羊腿umer", "22158819"),
                text("  剪辑"),
                mention("允崽来啦", "2063092494"),
                text(" \n【"),
                mention("Nya Rime", "3"),
                text(" 】"),
            ])
        );
    }

    #[test]
    fn desc_v2_adjacent_credits() {
        let (plain, v2) = desc_v2_json("@credit@credit", &[("a", "1"), ("b", "2")]);
        assert_eq!(plain, "@a  @b  ");
        assert_eq!(
            v2,
            serde_json::json!([mention("a", "1"), mention("b", "2")])
        );
    }

    #[test]
    fn desc_v2_extra_credits_are_ignored() {
        let (plain, v2) = desc_v2_json("by @credit", &[("a", "1"), ("b", "2")]);
        assert_eq!(plain, "by @a  ");
        assert_eq!(v2, serde_json::json!([text("by "), mention("a", "1")]));
    }

    #[test]
    fn desc_v2_extra_placeholders_stay_literal() {
        let (plain, v2) = desc_v2_json("@credit 和 @credit", &[("a", "1")]);
        assert_eq!(plain, "@a   和 @credit");
        assert_eq!(
            v2,
            serde_json::json!([mention("a", "1"), text("  和 @credit")])
        );
    }

    #[test]
    fn desc_v2_absent_without_credits_or_placeholder() {
        assert!(credits_to_desc_v2("@credit 简介", &[]).is_none());
        assert!(credits_to_desc_v2("没有占位符", &credits(&[("a", "1")])).is_none());
    }

    #[test]
    fn template_credits_accepts_web_form_and_config_shapes() {
        let value = serde_json::json!([
            {"uid": "2063092494", "username": "允崽来啦"},
            {"uid": 22158819, "username": " @羊腿umer "},
            {"uid": " 3 ", "username": "Nya Rime"},
        ]);
        assert_eq!(
            template_credits(Some(&value)),
            credits(&[
                ("允崽来啦", "2063092494"),
                ("羊腿umer", "22158819"),
                ("Nya Rime", "3")
            ])
        );
    }

    #[test]
    fn template_credits_skips_unusable_entries() {
        let value = serde_json::json!([
            {"uid": "", "username": "空uid"},
            {"uid": "abc", "username": "非数字"},
            {"uid": "1", "username": "  "},
            {"username": "缺uid"},
            null,
            {"uid": "7", "username": "ok"},
        ]);
        assert_eq!(template_credits(Some(&value)), credits(&[("ok", "7")]));
        assert!(template_credits(None).is_empty());
        assert!(template_credits(Some(&serde_json::Value::Null)).is_empty());
    }

    fn fake_bilibili() -> BiliBili {
        BiliBili {
            client: reqwest::Client::new(),
            login_info: serde_json::from_value(serde_json::json!({
                "cookie_info": {"cookies": []},
                "sso": [],
                "token_info": {"access_token": "", "expires_in": 0, "mid": 0, "refresh_token": ""},
                "platform": null
            }))
            .unwrap(),
        }
    }

    fn template(description: &str, credits: serde_json::Value) -> UploadStreamer {
        serde_json::from_value(serde_json::json!({
            "id": 3,
            "template_name": "羊",
            "title": "羊腿umer%Y年%m月%d日直播回放",
            "tid": 21,
            "copyright": 1,
            "description": description,
            "tags": ["直播回放"],
            "credits": credits,
        }))
        .unwrap()
    }

    fn recorder() -> Recorder {
        use crate::server::infrastructure::models::StreamerInfo;
        let date = chrono::DateTime::parse_from_rfc3339("2026-06-15T12:00:00Z")
            .unwrap()
            .to_utc();
        Recorder::new(
            None,
            StreamerInfo::new(
                "羊",
                "https://live.bilibili.com/1",
                "游戏日！来博弈了",
                date,
                "",
            ),
        )
    }

    /// `submit_by_app` / `submit_by_web` / `edit_by_*` 都是 `.json(studio)`，
    /// 这里断言的就是发给 B 站的请求体。
    #[tokio::test]
    async fn build_studio_request_body_carries_credit_mentions() {
        let upload_config = template(
            "@credit %Y年%m月%d日直播回放-{title}",
            serde_json::json!([{"uid": "22158819", "username": "羊腿umer"}]),
        );
        let studio = build_studio(&upload_config, &fake_bilibili(), Vec::new(), &recorder())
            .await
            .unwrap();
        let body = serde_json::to_value(&studio).unwrap();
        assert_eq!(
            body["desc"],
            "@羊腿umer   2026年06月15日直播回放-游戏日！来博弈了"
        );
        assert_eq!(body["desc_format_id"], 0);
        assert_eq!(
            body["desc_v2"],
            serde_json::json!([
                mention("羊腿umer", "22158819"),
                text("  2026年06月15日直播回放-游戏日！来博弈了"),
            ])
        );
    }

    /// issue #1762 的模板：转载、来源留空、`extra_fields` 为空串、只有一个标签。
    fn issue_1762_template(copyright: u8, tags: &[&str]) -> UploadStreamer {
        serde_json::from_value(serde_json::json!({
            "id": 3,
            "template_name": "test",
            "title": "{title}%Y-%m-%d",
            "tid": 21,
            "copyright": copyright,
            "copyright_source": "",
            "cover_path": "",
            "description": "",
            "dynamic": "",
            "dolby": 0,
            "hires": 0,
            "charging_pay": 0,
            "no_reprint": 0,
            "is_only_self": 1,
            "uploader": "biliup-rs",
            "tags": tags,
            "credits": null,
            "up_selection_reply": false,
            "up_close_reply": false,
            "up_close_danmu": false,
            "extra_fields": "",
        }))
        .unwrap()
    }

    fn request_body(copyright: u8, tags: &[&str]) -> serde_json::Value {
        let studio = studio_from_template(
            &issue_1762_template(copyright, tags),
            Vec::new(),
            &recorder(),
        );
        serde_json::to_value(&studio).unwrap()
    }

    #[test]
    fn issue_1762_request_body_carries_no_mission() {
        for copyright in [1, 2] {
            let body = request_body(copyright, &["测试"]);
            let mut keys: Vec<&str> = body
                .as_object()
                .unwrap()
                .keys()
                .map(|k| k.as_str())
                .collect();
            keys.sort_unstable();
            // 空串 `extra_fields` 不注入任何键；没有 topic_id / act_reserve_create 之类的活动字段。
            assert_eq!(
                keys,
                [
                    "aid",
                    "charging_pay",
                    "copyright",
                    "cover",
                    "desc",
                    "desc_format_id",
                    "desc_v2",
                    "dolby",
                    "dtime",
                    "dynamic",
                    "interactive",
                    "is_only_self",
                    "lossless_music",
                    "mission_id",
                    "no_reprint",
                    "open_subtitle",
                    "source",
                    "subtitle",
                    "tag",
                    "tid",
                    "title",
                    "up_close_danmu",
                    "up_close_reply",
                    "up_selection_reply",
                    "videos",
                ]
            );
            assert!(body["mission_id"].is_null());
            assert_eq!(body["copyright"], copyright);
            assert_eq!(body["source"], "https://live.bilibili.com/1");
            assert_eq!(body["tid"], 21);
            assert_eq!(body["tag"], "测试");
            assert_eq!(body["is_only_self"], 1);
        }
    }

    fn response(code: i32, message: &str) -> ResponseData {
        serde_json::from_value(serde_json::json!({
            "code": code,
            "data": if code == 0 { serde_json::json!({"aid": 1, "bvid": "BV1"}) } else { serde_json::Value::Null },
            "message": message,
            "ttl": 1
        }))
        .unwrap()
    }

    /// 按 B 站投稿服务的校验顺序模拟两个接口（go-common `videoup`：`AppAdd` 的
    /// `freshAppMissionByFirstTag`、`preAdd` 的 `checkMission` / `checkMissionTag`）。
    /// `missions` 是进行中活动的第一个标签。
    fn fake_add(
        api: &SubmitOption,
        body: &serde_json::Value,
        missions: &[&str],
    ) -> biliup::error::Result<ResponseData> {
        let tags: Vec<&str> = body["tag"].as_str().unwrap().split(',').collect();
        let mut mission = body["mission_id"].as_u64().unwrap_or(0);
        if mission == 0 && !matches!(api, SubmitOption::Web) && missions.contains(&tags[0]) {
            mission = 1;
        }
        if mission > 0 && body["copyright"] == 2 {
            return Err(Kind::SubmitRejected(response(
                21071,
                "转载类型稿件不支持活动参加哦~",
            )));
        }
        if mission == 0 && tags.iter().all(|t| missions.contains(t)) {
            return Err(Kind::SubmitRejected(response(
                21067,
                "自定义标签包含不可选的活动tag，请修改后重新提交",
            )));
        }
        Ok(response(0, "0"))
    }

    fn custom_messages(report: &error_stack::Report<AppError>) -> Vec<String> {
        report
            .frames()
            .filter_map(|f| match f.downcast_ref::<AppError>() {
                Some(AppError::Custom(m)) => Some(m.clone()),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn reprint_rejected_by_app_as_mission_falls_back_to_web() {
        use std::sync::Mutex;
        struct Case {
            api: SubmitOption,
            copyright: u8,
            tags: &'static [&'static str],
            calls: &'static [&'static str],
            code: Option<i32>,
        }
        let cases = [
            // 转载 × app：首个标签是活动标签 → 21071，改走 Web，B 站剔掉活动标签后投稿成功。
            Case {
                api: SubmitOption::App,
                copyright: 2,
                tags: &["测试", "直播回放"],
                calls: &["App", "Web"],
                code: None,
            },
            Case {
                api: SubmitOption::BCutAndroid,
                copyright: 2,
                tags: &["测试", "直播回放"],
                calls: &["BCutAndroid", "Web"],
                code: None,
            },
            // 只有这一个标签：Web 剔完没有标签 → 21067。
            Case {
                api: SubmitOption::App,
                copyright: 2,
                tags: &["测试"],
                calls: &["App", "Web"],
                code: Some(21067),
            },
            Case {
                api: SubmitOption::App,
                copyright: 2,
                tags: &["直播回放", "测试"],
                calls: &["App"],
                code: None,
            },
            // 自制 × app：B 站默默让稿件参加活动，不报错。
            Case {
                api: SubmitOption::App,
                copyright: 1,
                tags: &["测试", "直播回放"],
                calls: &["App"],
                code: None,
            },
            // Web 接口不会自动参加活动，也就不会 21071；21071 之外的错误不重投。
            Case {
                api: SubmitOption::Web,
                copyright: 2,
                tags: &["测试", "直播回放"],
                calls: &["Web"],
                code: None,
            },
            Case {
                api: SubmitOption::Web,
                copyright: 2,
                tags: &["测试"],
                calls: &["Web"],
                code: Some(21067),
            },
            Case {
                api: SubmitOption::Web,
                copyright: 1,
                tags: &["测试"],
                calls: &["Web"],
                code: Some(21067),
            },
            Case {
                api: SubmitOption::App,
                copyright: 1,
                tags: &["测试"],
                calls: &["App"],
                code: None,
            },
        ];
        for case in cases {
            let body = request_body(case.copyright, case.tags);
            let calls = Mutex::new(Vec::new());
            let result = submit_with_web_fallback(case.api.clone(), |api| {
                calls.lock().unwrap().push(format!("{api:?}"));
                let result = fake_add(&api, &body, &["测试"]);
                async move { result }
            })
            .await;
            let label = format!(
                "{:?} copyright={} tags={:?}",
                case.api, case.copyright, case.tags
            );
            assert_eq!(*calls.lock().unwrap(), case.calls, "{label}");
            match case.code {
                None => assert!(result.is_ok(), "{label}: {result:?}"),
                Some(code) => {
                    let report = result.expect_err(&label);
                    let rejected = report
                        .frames()
                        .find_map(|f| match f.downcast_ref::<Kind>() {
                            Some(Kind::SubmitRejected(ret)) => Some(ret.code),
                            _ => None,
                        });
                    assert_eq!(rejected, Some(code), "{label}");
                    let messages = custom_messages(&report);
                    let fallback_failed = messages.iter().any(|m| {
                        m.contains("code 21071") && m.contains("改用 Web 接口重投也失败了")
                    });
                    assert_eq!(
                        fallback_failed,
                        case.calls.len() == 2,
                        "{label}: {messages:?}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn web_fallback_failure_keeps_both_reasons() {
        let result = submit_with_web_fallback(SubmitOption::App, |api| async move {
            match api {
                SubmitOption::Web => Err(Kind::Custom("cookie 里没有 bili_jct".into())),
                _ => Err(Kind::SubmitRejected(response(
                    21071,
                    "转载类型稿件不支持活动参加哦~",
                ))),
            }
        })
        .await;
        let report = result.unwrap_err();
        assert_eq!(
            custom_messages(&report),
            [
                "App 接口投稿被拒：转载类型稿件不支持活动参加哦~（code 21071），改用 Web 接口重投也失败了"
            ]
        );
        assert!(format!("{report:?}").contains("cookie 里没有 bili_jct"));
    }

    #[tokio::test]
    async fn other_rejections_are_not_retried() {
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let result = submit_with_web_fallback(SubmitOption::App, |_| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { Err(Kind::SubmitRejected(response(21012, "标题不合法"))) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(custom_messages(&result.unwrap_err()).is_empty());
    }

    #[tokio::test]
    async fn build_studio_without_credits_keeps_desc_and_null_desc_v2() {
        let upload_config = template("%Y年%m月%d日直播回放-{title}", serde_json::Value::Null);
        let studio = build_studio(&upload_config, &fake_bilibili(), Vec::new(), &recorder())
            .await
            .unwrap();
        let body = serde_json::to_value(&studio).unwrap();
        assert_eq!(body["desc"], "2026年06月15日直播回放-游戏日！来博弈了");
        assert!(body["desc_v2"].is_null());
    }
}
