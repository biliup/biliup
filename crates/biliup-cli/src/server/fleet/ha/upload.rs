//! 配对房间的投稿流程：`common/upload.rs` 里 `process_with_upload` 开头一行转到这里。
//!
//! 主机：先等备机上报（有时限，见 `primary::REPORT_WAIT_MS`）；该主机投的照常边录边传，开始时发 `UploadStarted`，之后每隔
//! `progress_interval` 发一次累计字节（§6 C），提交前再确认备机没有接手，投成发 `Uploaded{bvid}`，
//! 失败发 `UploadFailed`。交给备机的场次不传、不跑后处理，录像留在本地。
//!
//! 备机：录的时候只把分段收下（[`Plan::Collect`]），决定要投时由 [`super::agent`] 另起任务调
//! [`run_standby`]（提交前再确认主机没有投成），模式 2 追加分 P 时调 [`run_append`]。
//!
//! 模式 2 主机回来补投上次中断的那半时调 [`run_resume`]：分段从切片工作台找回，之后同主机的投稿流程。
//!
//! 与 B 站打交道的几步（登录、传分段、提交）集中在 [`Session`]。测试构建里可以装上 [`double`]
//! 代替 B 站；发布构建里没有这个模块，只有真实实现。

use super::agent::Standby;
use super::primary::{Begin, Primary, Resume};
use super::wire::SkipReason;
use crate::server::common::upload::{
    UploadContext, build_studio, edit_to_bilibili, execute_postprocessor,
    initialize_upload_context, pipeline_upload_videos, submit_to_bilibili,
    upload_single_file_with_progress,
};
use crate::server::common::util::FileValidator;
use crate::server::core::downloader::SegmentInfo;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::events::scrub;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::infrastructure::models::hook_step::HookStep;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::workbench::store::{SegmentRow, SegmentState};
use crate::server::workbench::{self, index};
use biliup::bilibili::{ResponseData, Vid, Video};
use biliup::downloader::live::LiveStream;
use error_stack::ResultExt;
use futures::{Stream, StreamExt};
use ormlite::Model;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::pin;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// 配对房间这一段怎么投
pub enum Plan {
    /// 主机：照常投，向备机报告生命周期
    Primary { primary: Arc<Primary>, key: String },
    /// 备机：只把分段收下，投不投录完再定
    Collect { standby: Arc<Standby>, id: String },
    /// 备机：镜像房间里不归场次管的一段，只把录像留在本地
    Keep { reason: &'static str },
}

impl Plan {
    pub async fn run<S>(self, rx: S, ctx: &Context, upload_config: &UploadStreamer) -> AppResult<()>
    where
        S: Stream<Item = SegmentInfo>,
    {
        match self {
            Plan::Primary { primary, key } => {
                run_primary(&primary, &key, rx, ctx, upload_config).await
            }
            Plan::Collect { standby, id } => {
                collect(&standby, &id, rx).await;
                Ok(())
            }
            Plan::Keep { reason } => {
                let kept = drain(rx).await;
                info!(
                    url = ctx.live_streamer().url,
                    kept, reason, "HA：镜像房间的这一段不投，录像留在本地"
                );
                Ok(())
            }
        }
    }
}

/// 把管道里的分段取完，不传：录像留在本地
async fn drain<S: Stream<Item = SegmentInfo>>(rx: S) -> usize {
    pin!(rx);
    let mut kept = 0;
    while rx.next().await.is_some() {
        kept += 1;
    }
    kept
}

/// 备机：把分段记进场次，录像留在本地
async fn collect<S: Stream<Item = SegmentInfo>>(standby: &Arc<Standby>, id: &str, rx: S) {
    pin!(rx);
    let mut kept = 0;
    while let Some(event) = rx.next().await {
        standby.segment(id, &event);
        kept += 1;
    }
    info!(id, kept, "HA：备机收下这一场的分段，先不投");
    standby.collected(id);
}

pub(crate) enum Submitted {
    Done {
        bvid: String,
        paths: Vec<PathBuf>,
    },
    /// 提交前发现备机已经接手
    Fenced,
}

async fn run_primary<S>(
    primary: &Arc<Primary>,
    key: &str,
    rx: S,
    ctx: &Context,
    upload_config: &UploadStreamer,
) -> AppResult<()>
where
    S: Stream<Item = SegmentInfo>,
{
    primary.wait_gate().await;
    if primary.upload_begin(key) == Begin::Skip {
        let kept = drain(rx).await;
        info!(key, kept, "HA：这一场由备机负责，主机不投，录像留在本地");
        return Ok(());
    }
    info!(key, "HA：主机开始投这一场");
    let reporter = Reporter::spawn(primary.clone(), key.to_string());
    let result = upload_and_submit(primary, key, rx, ctx, upload_config, &reporter.bytes).await;
    drop(reporter);
    match result {
        Ok(Submitted::Done { bvid, paths }) => {
            primary.uploaded(key, &bvid);
            execute_postprocessor(paths, ctx).await
        }
        Ok(Submitted::Fenced) => {
            primary.upload_fenced(key);
            Ok(())
        }
        Err(e) => {
            primary.upload_failed(key, &scrub(&format!("{e:#}")));
            Err(e)
        }
    }
}

async fn upload_and_submit<S>(
    primary: &Primary,
    key: &str,
    rx: S,
    ctx: &Context,
    upload_config: &UploadStreamer,
    bytes: &AtomicU64,
) -> AppResult<Submitted>
where
    S: Stream<Item = SegmentInfo>,
{
    let session = Session::login(ctx, upload_config).await?;
    let processors = segment_processors(ctx);
    let uploaded =
        pipeline_upload_videos(rx, &processors, |path| session.upload(path, bytes)).await?;
    if uploaded.videos.is_empty() {
        return Err(AppError::Custom("没有一个分段上传成功".into()).into());
    }
    if !primary.may_submit(key) {
        return Ok(Submitted::Fenced);
    }
    let bvid = session.submit(ctx, upload_config, uploaded.videos).await?;
    Ok(Submitted::Done {
        bvid,
        paths: uploaded.paths,
    })
}

/// 备机把收下的分段作为完整稿件投：分段一样先过 segment_processor，提交前确认主机没有投成
pub(crate) async fn run_standby(
    standby: &Arc<Standby>,
    id: &str,
    ctx: &Context,
    segments: Vec<SegmentInfo>,
) -> AppResult<Submitted> {
    let upload_config = ctx
        .upload_config()
        .clone()
        .ok_or_else(|| AppError::Custom("这个房间没有投稿模板".into()))?;
    let session = Session::login(ctx, &upload_config).await?;
    let processors = segment_processors(ctx);
    let bytes = AtomicU64::new(0);
    let uploaded = pipeline_upload_videos(futures::stream::iter(segments), &processors, |path| {
        session.upload(path, &bytes)
    })
    .await?;
    if uploaded.videos.is_empty() {
        return Err(AppError::Custom("没有一个分段上传成功".into()).into());
    }
    if !standby.begin_submit(id) {
        return Ok(Submitted::Fenced);
    }
    let bvid = session.submit(ctx, &upload_config, uploaded.videos).await?;
    Ok(Submitted::Done {
        bvid,
        paths: uploaded.paths,
    })
}

/// 模式 2：主机投成了它那半，备机把收下的分段追加为主机稿件（`bvid`）的后续分 P（§4）。
/// 分 P 标题与普通投稿同一规则（按模板的文件名），接在主机那半后面，B 站的分 P 序号就续上了
pub(crate) async fn run_append(
    standby: &Arc<Standby>,
    id: &str,
    ctx: &Context,
    segments: Vec<SegmentInfo>,
    bvid: &str,
) -> AppResult<Submitted> {
    let upload_config = ctx
        .upload_config()
        .clone()
        .ok_or_else(|| AppError::Custom("这个房间没有投稿模板".into()))?;
    let session = Session::login(ctx, &upload_config).await?;
    let processors = segment_processors(ctx);
    let bytes = AtomicU64::new(0);
    let uploaded = pipeline_upload_videos(futures::stream::iter(segments), &processors, |path| {
        session.upload(path, &bytes)
    })
    .await?;
    if uploaded.videos.is_empty() {
        return Err(AppError::Custom("没有一个分段上传成功".into()).into());
    }
    if !standby.begin_submit(id) {
        return Ok(Submitted::Fenced);
    }
    session.append(ctx, bvid, uploaded.videos).await?;
    Ok(Submitted::Done {
        bvid: bvid.to_string(),
        paths: uploaded.paths,
    })
}

/// 模式 2：主机进程中断、备机接手了这一段，主机回来后先投盘上它那半（§4「主机先投、副机追加分 P」），
/// 投成后备机据 `Uploaded` 追加它那半。`started` 是这个主机的启动时刻。占上传池的一个槽位
pub(crate) async fn run_resume(
    primary: Arc<Primary>,
    services: ServiceRegister,
    job: Resume,
    started: i64,
) {
    let slots = services.managers.upload_slots();
    let _slot = slots.acquire().await;
    match resume_input(&primary, &services, &job, started).await {
        Ok(found) => {
            primary.resume_ended(&job.key, found.ended_at);
            let segments = futures::stream::iter(found.segments);
            let result = run_primary(&primary, &job.key, segments, &found.ctx, &found.config).await;
            if let Err(e) = result {
                warn!(key = job.key, error = ?e, "HA：主机补投它那半没有做完");
            }
        }
        Err(Missing::Skip(reason, detail)) => primary.resume_skipped(&job.key, reason, detail),
        Err(Missing::Fail(reason)) => primary.upload_failed(&job.key, &reason),
    }
}

struct Found {
    ctx: Context,
    config: UploadStreamer,
    segments: Vec<SegmentInfo>,
    /// 最后一个分段写盘的时刻
    ended_at: i64,
}

enum Missing {
    Skip(SkipReason, &'static str),
    Fail(String),
}

/// 主机那半：切片工作台这一场里、这一段检测到开播之后到本主机启动之前写完的分段，
/// 按 `filtering_threshold` 过滤（太小的留在盘上，不删）
async fn resume_input(
    primary: &Primary,
    services: &ServiceRegister,
    job: &Resume,
    started: i64,
) -> Result<Found, Missing> {
    const GONE: &str = "主机那半的录像已不在盘上";
    let session = job
        .session
        .ok_or(Missing::Skip(SkipReason::NoFiles, GONE))?;
    let info = match StreamerInfo::fetch_one(session, &services.pool).await {
        Ok(info) => info,
        Err(ormlite::Error::SqlxError(sqlx::Error::RowNotFound)) => {
            return Err(Missing::Skip(SkipReason::NoFiles, GONE));
        }
        Err(e) => return Err(Missing::Fail(format!("读不到这一场的开播信息：{e}"))),
    };
    let url = primary
        .room_url(job.room)
        .ok_or_else(|| Missing::Fail("这个房间已经不在配对里".into()))?;
    let worker = services
        .managers
        .get_rooms()
        .await
        .into_iter()
        .find(|worker| worker.live_streamer.url == url)
        .ok_or_else(|| Missing::Fail("这个房间已经不在本机".into()))?;
    let ctx = Context::new(
        session,
        worker,
        services.pool.clone(),
        live_stream(&info, job.unit_started_at).map_err(Missing::Fail)?,
    );
    let config = ctx
        .upload_config()
        .clone()
        .filter(|config| !config.is_noop_uploader())
        .ok_or_else(|| Missing::Fail("这个房间没有投稿模板".into()))?;
    let rows = workbench::store::session_segments(&services.pool, session)
        .await
        .map_err(|e| Missing::Fail(format!("读不到这一场的分段：{e}")))?;
    let validator = FileValidator::new(ctx.config().filtering_threshold * 1000 * 1000, true);
    let (mut segments, mut filtered, mut ended_at) = (Vec::new(), false, job.unit_started_at);
    for row in rows
        .iter()
        .filter(|row| row.state == SegmentState::Finished)
    {
        let written = modified_ms(Path::new(&row.path));
        let Some(written) = written.filter(|at| (job.unit_started_at..started).contains(at)) else {
            continue;
        };
        if validator.will_delete(Path::new(&row.path)) {
            filtered = true;
            continue;
        }
        let path = finish_part(&services.pool, row).await;
        let danmaku = row
            .danmaku_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| path.exists());
        segments.push(SegmentInfo::new(path, danmaku, None, segments.len()));
        ended_at = ended_at.max(written);
    }
    match (segments.is_empty(), filtered) {
        (false, _) => Ok(Found {
            ctx,
            config,
            segments,
            ended_at,
        }),
        (true, true) => Err(Missing::Skip(
            SkipReason::Filtered,
            "主机那半小于 filtering_threshold",
        )),
        (true, false) => Err(Missing::Skip(SkipReason::NoFiles, GONE)),
    }
}

/// 按场次记录重建开播信息。主播名是房间备注（场次里只记了它），`{streamer}` 按它填
fn live_stream(info: &StreamerInfo, unit_started_at: i64) -> Result<LiveStream, String> {
    let date = chrono::DateTime::from_timestamp_millis(unit_started_at).unwrap_or(info.date);
    serde_json::from_value(serde_json::json!({
        "name": info.name,
        "url": info.url,
        "title": info.title,
        "date": date,
        "live_cover_url": info.live_cover_path,
        "raw_stream_url": "",
        "platform": "",
        "stream_headers": {},
        "suffix": "",
        "danmaku": null,
        "downloader_hint": "StreamGears",
        "runtime_options": null,
    }))
    .map_err(|e| format!("重建这一场的开播信息失败：{e}"))
}

/// 崩溃时正在写的分段还带着 `.part`：改回正式文件名再投，索引缓存与切片工作台的记录一起改
async fn finish_part(pool: &ConnectionPool, row: &SegmentRow) -> PathBuf {
    let path = PathBuf::from(&row.path);
    let Some(done) = row.path.strip_suffix(".part").map(PathBuf::from) else {
        return path;
    };
    if done.exists() {
        return path;
    }
    if let Err(e) = std::fs::rename(&path, &done) {
        warn!(path = row.path, error = %e, "HA：没能给中断留下的分段去掉 .part");
        return path;
    }
    let index_path = row.index_path.as_ref().map(|old| {
        let new = index::index_path(&done);
        let _ = std::fs::rename(old, &new);
        new.to_string_lossy().into_owned()
    });
    let moved = workbench::store::move_segment(
        pool,
        row.id,
        &done.to_string_lossy(),
        index_path.as_deref(),
    )
    .await;
    if let Err(e) = moved {
        warn!(path = row.path, error = %e, "HA：分段改名后没能更新切片工作台的记录");
    }
    done
}

fn segment_processors(ctx: &Context) -> Vec<HookStep> {
    ctx.live_streamer()
        .segment_processor
        .clone()
        .unwrap_or_default()
}

/// 上传中每隔 `progress_interval` 把累计字节报给备机
struct Reporter {
    bytes: Arc<AtomicU64>,
    task: JoinHandle<()>,
}

impl Reporter {
    fn spawn(primary: Arc<Primary>, key: String) -> Self {
        let bytes = Arc::new(AtomicU64::new(0));
        let every = Duration::from_secs(primary.params().progress_interval.max(1));
        let counted = bytes.clone();
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(every);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                primary.progress(&key, counted.load(Ordering::Relaxed));
            }
        });
        Reporter { bytes, task }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// 投成后的稿件号：B 站返回的 bvid，没有就用 aid
fn archive_id(ret: &ResponseData) -> String {
    let data = ret.data.as_ref();
    if let Some(bvid) = data
        .and_then(|data| data.get("bvid"))
        .and_then(|v| v.as_str())
    {
        return bvid.to_string();
    }
    match data
        .and_then(|data| data.get("aid"))
        .and_then(|v| v.as_u64())
    {
        Some(aid) => format!("av{aid}"),
        // 提交已经成功：记成投成，避免备机再投一份
        None => "unknown".into(),
    }
}

/// 一次投稿里与 B 站打交道的几步
enum Session {
    Real(Box<UploadContext>),
    #[cfg(test)]
    Double(Arc<double::Double>),
}

impl Session {
    async fn login(ctx: &Context, upload_config: &UploadStreamer) -> AppResult<Self> {
        #[cfg(test)]
        if let Some(double) = double::installed() {
            double.login(upload_config)?;
            return Ok(Session::Double(double));
        }
        initialize_upload_context(&ctx.config(), ctx.stateless_client(), upload_config)
            .await
            .map(|context| Session::Real(Box::new(context)))
    }

    async fn upload(&self, path: PathBuf, bytes: &AtomicU64) -> AppResult<Video> {
        match self {
            Session::Real(context) => {
                upload_single_file_with_progress(&path, context, |len| {
                    bytes.fetch_add(len as u64, Ordering::Relaxed);
                    true
                })
                .await
            }
            #[cfg(test)]
            Session::Double(double) => double.upload(&path, bytes).await,
        }
    }

    /// 按模板建稿件并提交，返回稿件号
    async fn submit(
        &self,
        ctx: &Context,
        upload_config: &UploadStreamer,
        videos: Vec<Video>,
    ) -> AppResult<String> {
        let mut recorder = ctx.recorder(ctx.streamer_info().clone());
        recorder.filename_prefix = upload_config.title.clone();
        match self {
            Session::Real(context) => {
                let studio =
                    build_studio(upload_config, &context.bilibili, videos, &recorder).await?;
                let submit_api = ctx.config().submit_api.clone();
                let ret =
                    submit_to_bilibili(&context.bilibili, &studio, submit_api.as_deref()).await?;
                Ok(archive_id(&ret))
            }
            #[cfg(test)]
            Session::Double(double) => {
                let studio = crate::server::common::upload::studio_from_template(
                    upload_config,
                    videos,
                    &recorder,
                );
                double.submit(&studio)
            }
        }
    }

    /// 取回已有稿件、把新的分 P 接在后面再编辑提交（与 `biliup append` 同一做法）
    async fn append(&self, ctx: &Context, bvid: &str, mut videos: Vec<Video>) -> AppResult<()> {
        match self {
            Session::Real(context) => {
                let mut studio = context
                    .bilibili
                    .studio_data(&Vid::Bvid(bvid.to_string()), None)
                    .await
                    .change_context(AppError::Unknown)?;
                studio.videos.append(&mut videos);
                let submit_api = ctx.config().submit_api.clone();
                edit_to_bilibili(&context.bilibili, &studio, submit_api.as_deref()).await?;
                Ok(())
            }
            #[cfg(test)]
            Session::Double(double) => double.append(bvid, &videos),
        }
    }
}

/// 文件最后修改时间（毫秒）；读不到时为 `None`
fn modified_ms(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

#[cfg(test)]
pub(crate) mod double {
    //! 测试里代替 B 站的上传端（只在测试构建里存在）：不发任何网络请求，登录、分段上传、提交都只记进
    //! JSONL 日志。控制目录里放 `stall`（进度停住）、`fail-upload`、`fail-submit`、`fail-append`、
    //! `fail-login` 文件可以让对应的一步停住或失败；`rate` 是每秒推进多少字节（0 为瞬间传完），
    //! 控制目录里的 `rate` 文件（内容是一个数）在运行中覆盖它。

    use crate::server::errors::{AppError, AppResult};
    use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
    use biliup::bilibili::{Studio, Video};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::Duration;

    static INSTALLED: RwLock<Option<Arc<Double>>> = RwLock::new(None);

    const STEP: Duration = Duration::from_millis(200);

    pub(crate) fn install(double: Option<Arc<Double>>) {
        *INSTALLED.write().unwrap() = double;
    }

    pub(crate) fn installed() -> Option<Arc<Double>> {
        INSTALLED.read().unwrap().clone()
    }

    pub(crate) struct Double {
        log: PathBuf,
        control: PathBuf,
        rate: u64,
        /// 稿件号前缀，区分哪台机器投的
        prefix: String,
        submitted: AtomicU64,
        lock: Mutex<()>,
    }

    impl Double {
        /// 日志里已有的成功提交接着编号：进程重启后稿件号不重复
        pub(crate) fn new(log: PathBuf, control: PathBuf, rate: u64, prefix: &str) -> Self {
            let double = Double {
                log,
                control,
                rate,
                prefix: prefix.to_string(),
                submitted: AtomicU64::new(0),
                lock: Mutex::new(()),
            };
            let submitted = double
                .entries()
                .iter()
                .filter(|entry| entry["op"] == "submit" && entry["ok"] == true)
                .count();
            double.submitted.store(submitted as u64, Ordering::Relaxed);
            double
        }

        fn flag(&self, name: &str) -> bool {
            self.control.join(name).exists()
        }

        fn rate(&self) -> u64 {
            std::fs::read_to_string(self.control.join("rate"))
                .ok()
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or(self.rate)
        }

        pub(crate) fn record(&self, mut entry: serde_json::Value) {
            entry["at"] = crate::server::fleet::now_ms().into();
            let _guard = self.lock.lock().unwrap();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log)
            {
                let _ = writeln!(file, "{entry}");
            }
        }

        pub(crate) fn login(&self, upload_config: &UploadStreamer) -> AppResult<()> {
            if self.flag("fail-login") {
                self.record(serde_json::json!({ "op": "login", "ok": false }));
                return Err(AppError::Custom("upload double: login refused".into()).into());
            }
            self.record(serde_json::json!({
                "op": "login",
                "ok": true,
                "template": upload_config.template_name,
            }));
            Ok(())
        }

        pub(crate) async fn upload(&self, path: &Path, bytes: &AtomicU64) -> AppResult<Video> {
            let size = std::fs::metadata(path)
                .map_err(|e| AppError::Custom(format!("upload double: {e}")))?
                .len();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.record(serde_json::json!({ "op": "upload_start", "file": name, "size": size }));
            let mut sent = 0;
            while sent < size {
                if self.flag("fail-upload") {
                    self.record(serde_json::json!({ "op": "upload_failed", "file": name }));
                    return Err(AppError::Custom("upload double: upload failed".into()).into());
                }
                if self.flag("stall") {
                    tokio::time::sleep(STEP).await;
                    continue;
                }
                let rate = self.rate();
                let step = match rate {
                    0 => size - sent,
                    rate => (rate / 5).max(1).min(size - sent),
                };
                sent += step;
                bytes.fetch_add(step, Ordering::Relaxed);
                if rate > 0 {
                    tokio::time::sleep(STEP).await;
                }
            }
            self.record(serde_json::json!({ "op": "upload", "file": name, "size": size }));
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned());
            Ok(Video {
                title: stem,
                ..Video::new(&format!("double/{name}"))
            })
        }

        fn parts(videos: &[Video]) -> Vec<String> {
            videos
                .iter()
                .map(|video| video.title.clone().unwrap_or_default())
                .collect()
        }

        pub(crate) fn submit(&self, studio: &Studio) -> AppResult<String> {
            if self.flag("fail-submit") {
                self.record(
                    serde_json::json!({ "op": "submit", "ok": false, "title": studio.title }),
                );
                return Err(AppError::Custom("upload double: submit rejected".into()).into());
            }
            let n = self.submitted.fetch_add(1, Ordering::Relaxed) + 1;
            let bvid = format!("BV{}{n:04}", self.prefix);
            self.record(serde_json::json!({
                "op": "submit",
                "ok": true,
                "bvid": bvid,
                "title": studio.title,
                "parts": Self::parts(&studio.videos),
            }));
            Ok(bvid)
        }

        /// 给已有稿件追加分 P；控制目录里有 `fail-append` 时失败
        pub(crate) fn append(&self, bvid: &str, videos: &[Video]) -> AppResult<()> {
            if self.flag("fail-append") {
                self.record(serde_json::json!({ "op": "append", "ok": false, "bvid": bvid }));
                return Err(AppError::Custom("upload double: append rejected".into()).into());
            }
            self.record(serde_json::json!({
                "op": "append",
                "ok": true,
                "bvid": bvid,
                "parts": Self::parts(videos),
            }));
            Ok(())
        }

        /// 日志里的每一条
        pub(crate) fn entries(&self) -> Vec<serde_json::Value> {
            std::fs::read_to_string(&self.log)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        }
    }
}
