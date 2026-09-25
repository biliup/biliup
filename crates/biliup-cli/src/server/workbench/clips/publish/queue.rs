//! 切片发布队列：一次只发一个稿件，不占录播自动上传的 UActor。
//!
//! 一个任务 = 一个稿件（每个切片一个稿件，或几个切片合成一个多 P 稿件），依次：
//! 1. 需要时先导出（没导出过、导出失败、产物不见了的切片按 `mode` 导出，已经在导出的等它）；
//! 2. 登录 B 站、选线路（[`initialize_upload_context`]），逐 P 上传（[`upload_single_file_with_progress`]）；
//! 3. 上传封面、投稿（[`submit_to_bilibili`]），成功后记下稿件号，撤销切片对源录像的引用。
//!
//! B 站返回 601（上传太频繁）时整个队列暂停，等用户点「继续」，不自动重试。失败的任务留着原因，
//! 重试时已经传好的 P 不再重传。任务只在内存里：服务重启后队列清空，已发布的切片不受影响。

use super::{Archive, ClipVars, StudioOverride, cover_file, session_info, template_for};
use crate::server::common::upload::{
    UploadContext, initialize_upload_context, submit_to_bilibili, upload_single_file_with_progress,
};
use crate::server::config::Config;
use crate::server::errors::AppError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::workbench::clips::export::ClipExports;
use crate::server::workbench::clips::{self, Clip, Mode, State as ClipState};
use crate::server::workbench::recorder::now_ms;
use crate::server::workbench::store;
use async_trait::async_trait;
use biliup::bilibili::{Studio, Video};
use biliup::client::StatelessClient;
use biliup::error::Kind;
use error_stack::Report;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::AbortHandle;
use tracing::{info, warn};

/// 队列里最多留多少个已结束的任务（成功的先丢）。
const KEEP_FINISHED: usize = 200;
/// 等导出时多久看一次。
const EXPORT_POLL: Duration = Duration::from_millis(500);
pub const RATE_LIMITED: &str = "B 站提示上传太频繁，已暂停，稍后点继续";

/// 上传、投稿出错。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// 601：暂停队列。
    RateLimited(String),
    Other(String),
}

impl Failure {
    fn message(&self) -> &str {
        match self {
            Failure::RateLimited(m) | Failure::Other(m) => m,
        }
    }
}

/// 投稿成功。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submitted {
    pub bvid: String,
}

/// B 站那一侧：登录、上传、封面、投稿。真实实现是 [`BiliBackend`]，测试里换成假的。
#[async_trait]
pub trait Bilibili: Send + Sync {
    async fn connect(&self, template: &UploadStreamer) -> Result<Box<dyn Connection>, Failure>;
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn upload(
        &self,
        path: &Path,
        progress: &(dyn Fn(usize) + Send + Sync),
    ) -> Result<Video, Failure>;
    /// 上传封面，返回 B 站的图片地址。
    async fn cover(&self, path: &Path) -> Result<String, Failure>;
    async fn submit(&self, studio: &Studio) -> Result<Submitted, Failure>;
}

/// 从错误链里挑给人看的话：自己的说明（`AppError::Custom`）加上 B 站客户端的原因。
fn describe(report: &Report<AppError>) -> Failure {
    let mut parts: Vec<String> = Vec::new();
    for frame in report.frames() {
        if let Some(kind) = frame.downcast_ref::<Kind>() {
            if let Kind::RateLimit { message, .. } = kind {
                return Failure::RateLimited(format!("{RATE_LIMITED}（{message}）"));
            }
            parts.push(bilibili_message(&kind.to_string(), "投稿"));
        } else if let Some(AppError::Custom(message)) = frame.downcast_ref::<AppError>() {
            parts.push(message.clone());
        }
    }
    parts.dedup();
    if parts.is_empty() {
        parts.push(report.to_string());
    }
    Failure::Other(parts.join("："))
}

/// 投稿 / 封面接口出错时库里只给 `ResponseData { code: .., message: ".." .. }` 的 Debug 串，
/// 挑出 code 和 message；`what` 是被拒的东西（投稿、封面）。
fn bilibili_message(text: &str, what: &str) -> String {
    let code = text
        .split("code: ")
        .nth(1)
        .and_then(|rest| rest.split([',', ' ']).next())
        .and_then(|c| c.parse::<i64>().ok());
    let message = text
        .split("message: \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next());
    match (code, message) {
        (Some(code), Some(message)) if text.starts_with("ResponseData") => {
            format!("B 站拒绝了{what}：{message}（code {code}）")
        }
        _ => text.to_string(),
    }
}

/// 测试里不连 B 站：一连就失败。
#[cfg(test)]
pub(crate) struct Offline;

#[cfg(test)]
#[async_trait]
impl Bilibili for Offline {
    async fn connect(&self, _: &UploadStreamer) -> Result<Box<dyn Connection>, Failure> {
        Err(Failure::Other("测试里不连 B 站".into()))
    }
}

/// 真实的 B 站：用上传模板里的账号登录，按全局配置选线路、并发和投稿接口。
pub struct BiliBackend {
    config: Arc<RwLock<Config>>,
    client: StatelessClient,
}

impl BiliBackend {
    pub fn new(config: Arc<RwLock<Config>>, client: StatelessClient) -> Self {
        Self { config, client }
    }
}

struct BiliConnection {
    context: UploadContext,
    submit_api: Option<String>,
}

#[async_trait]
impl Bilibili for BiliBackend {
    async fn connect(&self, template: &UploadStreamer) -> Result<Box<dyn Connection>, Failure> {
        let config = self
            .config
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let context = initialize_upload_context(&config, &self.client, template)
            .await
            .map_err(|e| describe(&e))?;
        Ok(Box::new(BiliConnection {
            context,
            submit_api: config.submit_api.clone(),
        }))
    }
}

#[async_trait]
impl Connection for BiliConnection {
    async fn upload(
        &self,
        path: &Path,
        progress: &(dyn Fn(usize) + Send + Sync),
    ) -> Result<Video, Failure> {
        upload_single_file_with_progress(path, &self.context, progress)
            .await
            .map_err(|e| describe(&e))
    }

    async fn cover(&self, path: &Path) -> Result<String, Failure> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| Failure::Other(format!("读不到封面文件 {}：{e}", path.display())))?;
        self.context
            .bilibili
            .cover_up(&bytes)
            .await
            .map_err(|e| Failure::Other(bilibili_message(&e.to_string(), "封面")))
    }

    async fn submit(&self, studio: &Studio) -> Result<Submitted, Failure> {
        let ret = submit_to_bilibili(&self.context.bilibili, studio, self.submit_api.as_deref())
            .await
            .map_err(|e| describe(&e))?;
        let bvid = ret
            .data
            .as_ref()
            .and_then(|d| d.get("bvid"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();
        Ok(Submitted { bvid })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    /// 601 后暂停，等「继续」。
    Paused,
    Failed,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    Export,
    Upload,
    Submit,
}

/// 发布设置（入队时的快照）。
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub template_id: Option<i64>,
    pub over: StudioOverride,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: u64,
    pub session_id: i64,
    /// 各 P 的切片（合成多 P 时有多个）。
    pub clip_ids: Vec<i64>,
    pub combine: bool,
    pub state: JobState,
    pub step: Option<Step>,
    /// 现在在做什么（给人看）。
    pub detail: String,
    pub ratio: Option<f64>,
    /// 已经传好的 P 数。
    pub uploaded: usize,
    pub error: Option<String>,
    pub bvid: Option<String>,
    /// 稿件标题（开始上传后才有）。
    pub title: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

struct Job {
    view: JobView,
    settings: Settings,
    mode: Mode,
    /// 各 P 传好之后的结果，重试时不重传。
    videos: Vec<Option<Video>>,
    abort: Option<AbortHandle>,
}

#[derive(Default)]
struct Queue {
    jobs: Vec<Job>,
    paused: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueView {
    /// 暂停的原因（601）；`None` = 没暂停。
    pub paused: Option<String>,
    pub jobs: Vec<JobView>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnqueueError {
    Invalid(String),
    Conflict(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActionError {
    NotFound,
    Conflict(String),
}

pub struct ClipPublisher {
    pool: ConnectionPool,
    exports: Arc<ClipExports>,
    backend: Arc<dyn Bilibili>,
    queue: Mutex<Queue>,
    wake: Notify,
    next_id: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 任务出错后怎么收场。
enum Outcome {
    Done { bvid: String, title: String },
    Paused(String),
    Failed(String),
}

impl ClipPublisher {
    pub fn new(
        pool: ConnectionPool,
        exports: Arc<ClipExports>,
        backend: Arc<dyn Bilibili>,
    ) -> Self {
        Self {
            pool,
            exports,
            backend,
            queue: Mutex::new(Queue::default()),
            wake: Notify::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// 启动后台的发布循环（整个进程一个）。
    pub fn spawn(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move { this.run_loop().await });
    }

    pub fn view(&self, session_id: Option<i64>) -> QueueView {
        let queue = lock(&self.queue);
        QueueView {
            paused: queue.paused.clone(),
            jobs: queue
                .jobs
                .iter()
                .filter(|j| session_id.is_none_or(|s| j.view.session_id == s))
                .map(|j| j.view.clone())
                .collect(),
        }
    }

    pub fn job(&self, id: u64) -> Option<JobView> {
        lock(&self.queue)
            .jobs
            .iter()
            .find(|j| j.view.id == id)
            .map(|j| j.view.clone())
    }

    /// 排进队列。`clips` 已按时间排好、都属于 `session_id`；调用方已检查过状态。
    pub fn enqueue(
        &self,
        session_id: i64,
        clips: &[Clip],
        settings: Settings,
        mode: Mode,
        created_by: Option<i64>,
    ) -> Result<JobView, EnqueueError> {
        if clips.is_empty() {
            return Err(EnqueueError::Invalid("没有选切片".into()));
        }
        if let Some(clip) = clips.iter().find(|c| c.state == ClipState::Published) {
            return Err(EnqueueError::Conflict(format!(
                "切片 #{} 已经发布过了（{}）",
                clip.id,
                clip.archive_bvid.as_deref().unwrap_or("稿件号未知")
            )));
        }
        let mut queue = lock(&self.queue);
        for job in &queue.jobs {
            if job.view.state == JobState::Done {
                continue;
            }
            if let Some(id) = clips
                .iter()
                .map(|c| c.id)
                .find(|id| job.view.clip_ids.contains(id))
            {
                let why = if job.view.state == JobState::Failed {
                    "上次发布失败的任务还在：点「重试」，或先把它移出队列"
                } else {
                    "已经在发布队列里"
                };
                return Err(EnqueueError::Conflict(format!("切片 #{id} {why}")));
            }
        }
        let now = now_ms();
        let view = JobView {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            session_id,
            clip_ids: clips.iter().map(|c| c.id).collect(),
            combine: clips.len() > 1,
            state: JobState::Queued,
            step: None,
            detail: if queue.paused.is_some() {
                "排队中（队列已暂停）".into()
            } else {
                "排队中".into()
            },
            ratio: None,
            uploaded: 0,
            error: None,
            bvid: None,
            title: None,
            created_by,
            created_at: now,
            updated_at: now,
        };
        queue.jobs.push(Job {
            view: view.clone(),
            settings,
            mode,
            videos: vec![None; clips.len()],
            abort: None,
        });
        prune(&mut queue.jobs);
        drop(queue);
        self.wake.notify_one();
        Ok(view)
    }

    /// 601 暂停后继续。
    pub fn resume(&self) -> bool {
        let mut queue = lock(&self.queue);
        let was = queue.paused.take().is_some();
        for job in &mut queue.jobs {
            if job.view.state == JobState::Paused {
                job.view.state = JobState::Queued;
                job.view.detail = "排队中".into();
                job.view.error = None;
                job.view.updated_at = now_ms();
            } else if job.view.state == JobState::Queued {
                job.view.detail = "排队中".into();
            }
        }
        drop(queue);
        self.wake.notify_one();
        was
    }

    /// 失败的任务重新排队；已经传好的 P 不重传。
    pub fn retry(&self, id: u64) -> Result<JobView, ActionError> {
        let mut queue = lock(&self.queue);
        let paused = queue.paused.is_some();
        let job = queue
            .jobs
            .iter_mut()
            .find(|j| j.view.id == id)
            .ok_or(ActionError::NotFound)?;
        if job.view.state != JobState::Failed {
            return Err(ActionError::Conflict("只有失败的任务可以重试".into()));
        }
        job.view.state = JobState::Queued;
        job.view.error = None;
        job.view.detail = if paused {
            "排队中（队列已暂停）".into()
        } else {
            "排队中".into()
        };
        job.view.updated_at = now_ms();
        let view = job.view.clone();
        // 重试的排到队尾，与新任务一样按先来后到
        let index = queue.jobs.iter().position(|j| j.view.id == id).unwrap();
        let job = queue.jobs.remove(index);
        queue.jobs.push(job);
        drop(queue);
        self.wake.notify_one();
        Ok(view)
    }

    /// 移出队列：排队中、暂停、失败、已完成的直接移除；正在导出或上传的停下；正在投稿时不能取消
    /// （请求可能已经到了 B 站，停下来会不知道稿件建没建）。
    pub fn remove(&self, id: u64) -> Result<(), ActionError> {
        let mut queue = lock(&self.queue);
        let index = queue
            .jobs
            .iter()
            .position(|j| j.view.id == id)
            .ok_or(ActionError::NotFound)?;
        let job = &queue.jobs[index];
        if job.view.state == JobState::Running {
            if job.view.step == Some(Step::Submit) {
                return Err(ActionError::Conflict(
                    "正在投稿，不能取消；等它结束（成功或失败）".into(),
                ));
            }
            if let Some(abort) = &job.abort {
                abort.abort();
            }
        }
        queue.jobs.remove(index);
        drop(queue);
        self.wake.notify_one();
        Ok(())
    }

    /// 切片所在的未完成任务的状态（排队、进行中、暂停、失败）；不在队列里时为 `None`。
    pub fn state_of(&self, clip_id: i64) -> Option<JobState> {
        lock(&self.queue)
            .jobs
            .iter()
            .find(|j| j.view.state != JobState::Done && j.view.clip_ids.contains(&clip_id))
            .map(|j| j.view.state)
    }

    /// 切片的范围改了：失败任务里已经传好的这一 P 作废，重试时重新导出、上传。
    pub fn forget_upload(&self, clip_id: i64) {
        let mut queue = lock(&self.queue);
        for job in &mut queue.jobs {
            if job.view.state != JobState::Failed {
                continue;
            }
            if let Some(index) = job.view.clip_ids.iter().position(|&c| c == clip_id) {
                job.videos[index] = None;
                job.view.uploaded = job.videos.iter().filter(|v| v.is_some()).count();
            }
        }
    }

    fn update(&self, id: u64, f: impl FnOnce(&mut Job)) {
        if let Some(job) = lock(&self.queue).jobs.iter_mut().find(|j| j.view.id == id) {
            f(job);
            job.view.updated_at = now_ms();
        }
    }

    fn progress(&self, id: u64, step: Step, detail: impl Into<String>, ratio: Option<f64>) {
        let detail = detail.into();
        self.update(id, |job| {
            job.view.step = Some(step);
            job.view.detail = detail;
            job.view.ratio = ratio;
        });
    }

    /// 取下一个排队的任务，标成进行中；暂停时不取。
    fn take_next(&self) -> Option<u64> {
        let mut queue = lock(&self.queue);
        if queue.paused.is_some() {
            return None;
        }
        let job = queue
            .jobs
            .iter_mut()
            .find(|j| j.view.state == JobState::Queued)?;
        job.view.state = JobState::Running;
        job.view.error = None;
        job.view.detail = "准备中".into();
        job.view.updated_at = now_ms();
        Some(job.view.id)
    }

    async fn run_loop(self: Arc<Self>) {
        loop {
            let notified = self.wake.notified();
            let Some(id) = self.take_next() else {
                notified.await;
                continue;
            };
            let this = self.clone();
            let handle = tokio::spawn(async move { this.run(id).await });
            self.update(id, |job| job.abort = Some(handle.abort_handle()));
            let outcome = match handle.await {
                Ok(outcome) => outcome,
                // 被移出队列时停下，任务已经不在列表里
                Err(e) if e.is_cancelled() => continue,
                Err(e) => Outcome::Failed(format!("发布任务异常退出：{e}")),
            };
            self.finish(id, outcome);
        }
    }

    fn finish(&self, id: u64, outcome: Outcome) {
        let mut queue = lock(&self.queue);
        let Some(job) = queue.jobs.iter_mut().find(|j| j.view.id == id) else {
            return;
        };
        job.abort = None;
        job.view.updated_at = now_ms();
        job.view.ratio = None;
        match outcome {
            Outcome::Done { bvid, title } => {
                info!(job = id, %bvid, %title, "切片发布成功");
                job.view.state = JobState::Done;
                job.view.step = None;
                job.view.detail = "已发布".into();
                job.view.bvid = Some(bvid);
                job.view.title = Some(title);
                job.videos.clear();
            }
            Outcome::Paused(message) => {
                warn!(job = id, %message, "B 站限流，切片发布队列暂停");
                job.view.state = JobState::Paused;
                job.view.detail = "已暂停".into();
                job.view.error = Some(message.clone());
                queue.paused = Some(message);
            }
            Outcome::Failed(message) => {
                warn!(job = id, %message, "切片发布失败");
                job.view.state = JobState::Failed;
                job.view.detail = "失败".into();
                job.view.error = Some(message);
            }
        }
        prune(&mut queue.jobs);
    }

    fn snapshot(&self, id: u64) -> Option<(JobView, Settings, Mode, Vec<Option<Video>>)> {
        lock(&self.queue)
            .jobs
            .iter()
            .find(|j| j.view.id == id)
            .map(|j| (j.view.clone(), j.settings.clone(), j.mode, j.videos.clone()))
    }

    async fn run(&self, id: u64) -> Outcome {
        let Some((view, settings, mode, mut videos)) = self.snapshot(id) else {
            return Outcome::Failed("任务不见了".into());
        };
        // 1. 导出
        let mut parts: Vec<Clip> = Vec::with_capacity(view.clip_ids.len());
        let total = view.clip_ids.len();
        for (index, &clip_id) in view.clip_ids.iter().enumerate() {
            let label = if total > 1 {
                format!("P{}/{total} ", index + 1)
            } else {
                String::new()
            };
            match self.ensure_exported(id, clip_id, mode, &label).await {
                Ok(clip) => parts.push(clip),
                Err(message) => return Outcome::Failed(message),
            }
        }
        // 2. 稿件内容
        let archive = match archive(
            &self.pool,
            &self.exports,
            view.session_id,
            &parts,
            &settings,
        )
        .await
        {
            Ok(archive) => archive,
            Err(message) => return Outcome::Failed(message),
        };
        if let Some(problem) = archive.problem() {
            return Outcome::Failed(problem);
        }
        let title = archive.render().title;
        self.update(id, |job| job.view.title = Some(title.clone()));
        // 3. 登录、上传
        self.progress(id, Step::Upload, "登录 B 站、选上传线路", None);
        let connection = match self.backend.connect(&archive.template).await {
            Ok(c) => c,
            Err(Failure::RateLimited(m)) => return Outcome::Paused(m),
            Err(Failure::Other(m)) => return Outcome::Failed(format!("登录 B 站失败：{m}")),
        };
        let sizes: Vec<u64> = parts
            .iter()
            .map(|c| c.output_bytes.unwrap_or(0).max(0) as u64)
            .collect();
        let all: u64 = sizes.iter().sum::<u64>().max(1);
        for (index, clip) in parts.iter().enumerate() {
            if videos[index].is_some() {
                continue;
            }
            let done_before: u64 = sizes[..index].iter().sum();
            let label = if total > 1 {
                format!("上传 P{}/{total}", index + 1)
            } else {
                "上传中".into()
            };
            self.progress(
                id,
                Step::Upload,
                &label,
                Some(done_before as f64 / all as f64),
            );
            let path = PathBuf::from(clip.output_path.as_deref().unwrap_or_default());
            let read = AtomicU64::new(0);
            let report = |len: usize| {
                let n = read.fetch_add(len as u64, Ordering::Relaxed) + len as u64;
                let ratio = ((done_before + n.min(sizes[index])) as f64 / all as f64).min(1.0);
                self.update(id, |job| job.view.ratio = Some(ratio));
            };
            match connection.upload(&path, &report).await {
                Ok(video) => {
                    videos[index] = Some(video.clone());
                    self.update(id, |job| {
                        job.videos[index] = Some(video);
                        job.view.uploaded = job.videos.iter().filter(|v| v.is_some()).count();
                    });
                }
                Err(Failure::RateLimited(m)) => return Outcome::Paused(m),
                Err(Failure::Other(m)) => {
                    let which = if total > 1 {
                        format!("上传 P{} 失败", index + 1)
                    } else {
                        "上传失败".into()
                    };
                    return Outcome::Failed(format!("{which}：{m}"));
                }
            }
        }
        // 4. 封面、投稿
        self.progress(id, Step::Submit, "投稿中", None);
        let videos = videos
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(index, mut video)| {
                video.title = Some(archive.part_title(index));
                video
            })
            .collect();
        let mut studio = archive.studio(videos);
        if !studio.cover.is_empty() {
            let from_template = archive.cover.is_none();
            match connection.cover(Path::new(&studio.cover)).await {
                Ok(url) => studio.cover = url,
                Err(failure) => {
                    let hint = if from_template {
                        "在发布设置里换一个封面，或修正上传模板的封面"
                    } else {
                        "重试，或在发布设置里改用模板封面"
                    };
                    return Outcome::Failed(format!("封面上传失败：{}；{hint}", failure.message()));
                }
            }
        }
        let submitted = match connection.submit(&studio).await {
            Ok(s) => s,
            Err(Failure::RateLimited(m)) => return Outcome::Paused(m),
            Err(Failure::Other(m)) => return Outcome::Failed(format!("投稿失败：{m}")),
        };
        let ids: Vec<i64> = parts.iter().map(|c| c.id).collect();
        if let Err(e) = clips::mark_published(&self.pool, &ids, &submitted.bvid, now_ms()).await {
            warn!(job = id, error = %e, bvid = %submitted.bvid, "已投稿，但记录发布结果失败");
        }
        Outcome::Done {
            bvid: submitted.bvid,
            title: studio.title,
        }
    }

    /// 切片导出好了就返回；没导出、失败或产物不见了按 `mode` 导出；正在导出的等它。
    async fn ensure_exported(
        &self,
        id: u64,
        clip_id: i64,
        mode: Mode,
        label: &str,
    ) -> Result<Clip, String> {
        let mut started = false;
        loop {
            let clip = clips::get(&self.pool, clip_id)
                .await
                .map_err(|e| format!("读切片出错：{e}"))?
                .ok_or_else(|| format!("切片 #{clip_id} 已经被删掉了"))?;
            match clip.state {
                ClipState::Ready | ClipState::Published => {
                    let exists = match clip.output_path.as_deref() {
                        Some(path) => tokio::fs::try_exists(path).await.unwrap_or(false),
                        None => false,
                    };
                    if exists {
                        return Ok(clip);
                    }
                    if clip.state == ClipState::Published || started {
                        return Err(format!("切片 #{clip_id} 的文件不见了"));
                    }
                }
                ClipState::Exporting => {
                    let progress = self.exports.progress(clip_id);
                    let (phase, ratio) = progress
                        .map(|p| (p.phase, p.ratio))
                        .unwrap_or(("准备中", None));
                    self.progress(id, Step::Export, format!("{label}导出 · {phase}"), ratio);
                    tokio::time::sleep(EXPORT_POLL).await;
                    continue;
                }
                ClipState::Failed if started => {
                    return Err(format!(
                        "{label}导出失败：{}",
                        clip.error.as_deref().unwrap_or("原因未知")
                    ));
                }
                ClipState::Discarded => {
                    return Err(format!("切片 #{clip_id} 已经放弃了"));
                }
                ClipState::Draft | ClipState::Failed => {}
            }
            let mode = if clip.state == ClipState::Draft {
                mode
            } else {
                clip.mode.unwrap_or(mode)
            };
            self.progress(id, Step::Export, format!("{label}开始导出"), None);
            match clips::begin_export(&self.pool, clip_id, mode, now_ms()).await {
                Ok(Some(exporting)) => self.exports.start(exporting),
                Ok(None) => {}
                Err(e) => return Err(format!("开始导出时读写数据库出错：{e}")),
            }
            started = true;
        }
    }
}

/// 按发布设置拼出稿件内容。`parts` 按时间排好、都属于 `session_id`；导出前后都能调（入队时先检查一遍）。
pub async fn archive(
    pool: &ConnectionPool,
    exports: &ClipExports,
    session_id: i64,
    parts: &[Clip],
    settings: &Settings,
) -> Result<Archive, String> {
    let template = template_for(pool, session_id, settings.template_id)
        .await
        .map_err(|e| e.to_string())?;
    let info = session_info(pool, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let started_at = store::session(pool, session_id)
        .await
        .map_err(|e| format!("读场次出错：{e}"))?
        .and_then(|s| s.started_at)
        .unwrap_or_else(|| info.date.timestamp_millis());
    let cover = match (&settings.over.cover, parts.first()) {
        (Some(_), Some(first)) => {
            let path = cover_file(&exports.dir(session_id), first.id);
            if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
                return Err("选的封面文件不见了，在发布设置里重新选一次封面".into());
            }
            Some(path)
        }
        _ => None,
    };
    Ok(Archive {
        template,
        over: settings.over.clone(),
        info,
        parts: parts
            .iter()
            .map(|c| ClipVars {
                title: c.title.clone(),
                at_ms: started_at + c.in_ms,
            })
            .collect(),
        cover,
    })
}

/// 已结束的任务最多留 [`KEEP_FINISHED`] 个，先丢成功的旧任务，失败的留给用户处理。
fn prune(jobs: &mut Vec<Job>) {
    while jobs
        .iter()
        .filter(|j| j.view.state == JobState::Done)
        .count()
        > KEEP_FINISHED
    {
        if let Some(index) = jobs.iter().position(|j| j.view.state == JobState::Done) {
            jobs.remove(index);
        }
    }
}

#[cfg(test)]
mod tests;
