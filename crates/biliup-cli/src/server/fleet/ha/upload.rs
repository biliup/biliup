//! 配对房间的投稿流程：`common/upload.rs` 里 `process_with_upload` 开头一行转到这里。
//!
//! 主机：先等备机上报（最多 10 秒）；该主机投的照常边录边传，开始时发 `UploadStarted`，之后每隔
//! `progress_interval` 发一次累计字节（§6 C），提交前再确认备机没有接手，投成发 `Uploaded{bvid}`，
//! 失败发 `UploadFailed`。交给备机的场次不传、不跑后处理，录像留在本地。
//!
//! 备机：录的时候只把分段收下（[`Plan::Collect`]），决定要投时由 [`super::agent`] 另起任务调
//! [`run_standby`]，提交前再确认主机没有投成。
//!
//! 与 B 站打交道的几步（登录、传分段、提交）集中在 [`Session`]。测试构建里可以装上 [`double`]
//! 代替 B 站；发布构建里没有这个模块，只有真实实现。

use super::agent::Standby;
use super::primary::{Begin, Primary};
use crate::server::common::upload::{
    UploadContext, build_studio, execute_postprocessor, initialize_upload_context,
    pipeline_upload_videos, submit_to_bilibili, upload_single_file_with_progress,
};
use crate::server::core::downloader::SegmentInfo;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::events::scrub;
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::hook_step::HookStep;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use biliup::bilibili::{ResponseData, Video};
use futures::{Stream, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::pin;
use tokio::task::JoinHandle;
use tracing::info;

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
}

#[cfg(test)]
pub(crate) mod double {
    //! 测试里代替 B 站的上传端（只在测试构建里存在）：不发任何网络请求，登录、分段上传、提交都只记进
    //! JSONL 日志。控制目录里放 `stall`（进度停住）、`fail-upload`、`fail-submit`、`fail-login` 文件可以
    //! 让对应的一步停住或失败；`rate` 是每秒推进多少字节（0 为瞬间传完）。

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
        pub(crate) fn new(log: PathBuf, control: PathBuf, rate: u64, prefix: &str) -> Self {
            Double {
                log,
                control,
                rate,
                prefix: prefix.to_string(),
                submitted: AtomicU64::new(0),
                lock: Mutex::new(()),
            }
        }

        fn flag(&self, name: &str) -> bool {
            self.control.join(name).exists()
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
                let step = match self.rate {
                    0 => size - sent,
                    rate => (rate / 5).max(1).min(size - sent),
                };
                sent += step;
                bytes.fetch_add(step, Ordering::Relaxed);
                if self.rate > 0 {
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
