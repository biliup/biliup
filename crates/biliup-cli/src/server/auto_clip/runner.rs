//! 按场次的自动切片任务：排队、调度、抽音频、转写。
//!
//! - 下播后：全局 `auto_clip.enabled` 打开、且这个主播在覆写里开了 `auto_clip_after_live`，
//!   就排一条自动任务，过了断流合并窗口（`live_merge_minutes` + 1 分钟）才跑；到点时这一场又接着录了
//!   （`ended_at` 被清空）就删掉这条，等下次下播再排。
//! - 手动：`POST /v1/sessions/{id}/auto-clip`，先给预估，确认后入队。
//! - 全局一个调度任务、同时只跑一个任务；ffmpeg 以低优先级运行。
//! - 取消：库里改成 `canceled`，运行中的任务每秒查一次，发现后停下（ffmpeg 随之被杀，在途的请求丢弃）。
//! - 重启：运行中的回到排队，按 `stage` 和场次目录下的缓存续跑，已转写的块不重传。
//! - 每场送转写的分钟数有上限（`max_asr_minutes`，默认 300），超了不调用转写，任务失败并说明。
//!
//! 没配置 `auto_clip`（或没打开）时不起调度任务、不建任务、不跑 ffmpeg。

use super::audio::{self, AudioError, Chunk, SegmentAudio, Span};
use super::files::{Line, SessionFiles};
use super::jobs::{self, Job, JobState, Stage, Trigger};
use super::model::{
    AudioFile, ClientOptions, Endpoint, ErrorKind, ModelClient, TranscribeOptions, Transcript,
};
use super::probe;
use super::settings::{API_KEY_ENV, AutoClipConfig, display_host};
use crate::server::config::Config;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::workbench::live;
use crate::server::workbench::recorder::now_ms;
use crate::server::workbench::store::{self, SegmentRow, SegmentState};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
use tracing::{info, warn};

/// 分析数据的目录（相对工作目录），与切片产物的 `clips/` 并列。
pub const AUTO_CLIP_DIR: &str = "auto_clip";

/// 上传太大时最多对半切几次
const MAX_SPLITS: u32 = 4;

#[derive(bon::Builder, Debug, Clone)]
pub struct RunnerOptions {
    #[builder(default = PathBuf::from(AUTO_CLIP_DIR), into)]
    pub root: PathBuf,
    /// 覆盖模型客户端的重试间隔（测试用）；空 = 5 s / 20 s / 60 s
    pub backoff: Option<Vec<Duration>>,
    /// 运行中的任务多久查一次有没有被取消
    #[builder(default = Duration::from_secs(1))]
    pub cancel_poll: Duration,
    /// 没有到点的任务时多久醒一次（排任务时会立即叫醒，这只是兜底）
    #[builder(default = Duration::from_secs(600))]
    pub idle_poll: Duration,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        RunnerOptions::builder().build()
    }
}

/// 一个任务怎么结束的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Failed(String),
    Canceled,
    /// 自动任务到点时这一场又接着录了（或场次没了），任务已删除
    Dropped,
}

pub struct Runner {
    pool: ConnectionPool,
    config: Arc<RwLock<Config>>,
    options: RunnerOptions,
    wake: Notify,
    started: AtomicBool,
}

static RUNNER: OnceLock<Arc<Runner>> = OnceLock::new();

/// 服务启动时调用：登记调度器。配置了 `auto_clip` 才把上次没跑完的任务放回排队，打开了才起调度任务。
pub async fn install(pool: ConnectionPool, config: Arc<RwLock<Config>>) {
    let runner = RUNNER.get_or_init(|| Runner::new(pool, config, RunnerOptions::default()));
    runner.recover().await;
    runner.poke();
}

/// 有新任务或配置变了：打开了就确保调度任务在跑，并叫醒它。没登记调度器（测试）时什么都不做。
pub fn kick() {
    if let Some(runner) = RUNNER.get() {
        runner.poke();
    }
}

fn root() -> PathBuf {
    RUNNER
        .get()
        .map_or_else(|| PathBuf::from(AUTO_CLIP_DIR), |r| r.options.root.clone())
}

pub fn enabled(config: &Config) -> bool {
    config.auto_clip.as_ref().is_some_and(|c| c.enabled)
}

/// 这个主播在覆写里开了「下播后自动生成候选」。全局配置里的同名字段不算。
pub fn streamer_opted_in(streamer: &LiveStreamer) -> bool {
    streamer
        .override_cfg
        .as_ref()
        .and_then(|patch| patch.auto_clip_after_live)
        .flatten()
        == Some(true)
}

/// 下载任务收尾、写完 `ended_at` 之后调用。没打开或主播没开时立即返回，不碰数据库。
pub fn session_finished(
    pool: &ConnectionPool,
    config: &Config,
    streamer: &LiveStreamer,
    session_id: i64,
) {
    if !enabled(config) || !streamer_opted_in(streamer) {
        return;
    }
    let pool = pool.clone();
    let merge_minutes = config.live_merge_minutes;
    tokio::spawn(async move {
        match schedule_after_live(&pool, merge_minutes, session_id, now_ms()).await {
            Ok(Some(job)) => info!(
                session = session_id,
                job = job.id,
                not_before = job.not_before,
                "自动切片：下播后排上任务，过了断流合并窗口再跑"
            ),
            Ok(None) => {}
            Err(error) => warn!(%error, session = session_id, "自动切片：排任务失败"),
        }
        kick();
    });
}

/// 排自动任务，`not_before` = 现在 + 断流合并窗口 + 1 分钟。
pub async fn schedule_after_live(
    pool: &ConnectionPool,
    live_merge_minutes: u64,
    session_id: i64,
    now: i64,
) -> sqlx::Result<Option<Job>> {
    let wait_ms = (live_merge_minutes as i64 + 1).saturating_mul(60_000);
    jobs::schedule_auto(pool, session_id, now.saturating_add(wait_ms), now).await
}

/// 转写用的端点：没填地址或模型为 `None`。
pub fn asr_endpoint(config: &AutoClipConfig) -> Option<Endpoint> {
    let url = config.asr_base_url()?;
    let model = config.asr_model.as_ref()?;
    Some(
        Endpoint::new(url, model.clone())
            .with_key(config.asr_key_with(std::env::var(API_KEY_ENV).ok())),
    )
}

/// 能拿去转写的分段：已录完、文件还在（含等着被删的）、容器认得。
fn transcribable(segment: &SegmentRow) -> bool {
    matches!(
        segment.state,
        SegmentState::Finished | SegmentState::PendingDelete
    ) && segment.end_ms.is_some()
        && store::container_of(Path::new(&segment.path)).is_some()
}

fn segment_name(segment: &SegmentRow) -> String {
    Path::new(&segment.path).file_name().map_or_else(
        || segment.path.clone(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn ceil_secs(ms: i64) -> i64 {
    (ms.max(0) + 999) / 1000
}

fn minutes(ms: i64) -> String {
    format!("{:.1}", ms as f64 / 60_000.0)
}

/// 预估这一场要送转写多少。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Estimate {
    /// 能拿去转写的录像总时长
    pub recorded_seconds: i64,
    /// 这次要送转写的秒数（已跳过静音、扣掉已转写的）
    pub asr_seconds: i64,
    /// 沿用已有转写时，已经转写过的秒数
    pub transcribed_seconds: i64,
    pub basis: Basis,
    pub max_asr_minutes: u64,
    pub over_limit: bool,
    /// 超上限时给用户看的一句话
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Basis {
    /// 按之前抽音频时的静音表算的
    Silence,
    /// 还没抽过音频，按录像时长算（静音跳过后会更少）
    Duration,
}

pub async fn estimate(
    pool: &ConnectionPool,
    config: &AutoClipConfig,
    session_id: i64,
    reuse_transcript: bool,
) -> sqlx::Result<Estimate> {
    estimate_in(pool, &root(), config, session_id, reuse_transcript).await
}

pub async fn estimate_in(
    pool: &ConnectionPool,
    root: &Path,
    config: &AutoClipConfig,
    session_id: i64,
    reuse_transcript: bool,
) -> sqlx::Result<Estimate> {
    let files = SessionFiles::new(root, session_id);
    let silence: HashMap<i64, SegmentAudio> = files
        .load_silence()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|audio| (audio.segment_id, audio))
        .collect();
    let (mut recorded_ms, mut speech_ms, mut all_known) = (0, 0, true);
    for segment in store::session_segments(pool, session_id)
        .await?
        .iter()
        .filter(|s| transcribable(s))
    {
        let length = segment.end_ms.unwrap_or(segment.start_ms) - segment.start_ms;
        recorded_ms += length.max(0);
        match silence
            .get(&segment.id)
            .filter(|audio| audio.start_ms == segment.start_ms)
        {
            Some(audio) => speech_ms += audio.speech_ms(),
            None => {
                speech_ms += length.max(0);
                all_known = false;
            }
        }
    }
    let transcribed_ms = if reuse_transcript {
        let done = files.done_chunks().await;
        files
            .load_plan()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|chunk| done.contains(&chunk.key))
            .map(Chunk::speech_ms)
            .sum()
    } else {
        0
    };
    let to_send = (speech_ms - transcribed_ms).max(0);
    let limit = config.max_asr_minutes();
    let over_limit = to_send > (limit as i64).saturating_mul(60_000);
    let basis = if all_known {
        Basis::Silence
    } else {
        Basis::Duration
    };
    let message = over_limit.then(|| match basis {
        Basis::Silence => limit_message(to_send, limit),
        Basis::Duration => format!(
            "按录像时长估，要转写约 {} 分钟，超过每场上限 {limit} 分钟；抽完音频、跳过静音后如果仍然超过，任务会停下，不调用转写",
            minutes(to_send)
        ),
    });
    Ok(Estimate {
        recorded_seconds: ceil_secs(recorded_ms),
        asr_seconds: ceil_secs(to_send),
        transcribed_seconds: ceil_secs(transcribed_ms),
        basis,
        max_asr_minutes: limit,
        over_limit,
        message,
    })
}

fn limit_message(to_send_ms: i64, limit: u64) -> String {
    format!(
        "跳过静音后这一场还要转写 {} 分钟，超过每场上限 {limit} 分钟，没有调用转写：要处理这一场，请在设置「自动切片（实验）」里调高每场转写上限（max_asr_minutes）后重试",
        minutes(to_send_ms)
    )
}

type Boxed<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl Runner {
    pub fn new(
        pool: ConnectionPool,
        config: Arc<RwLock<Config>>,
        options: RunnerOptions,
    ) -> Arc<Self> {
        Arc::new(Runner {
            pool,
            config,
            options,
            wake: Notify::new(),
            started: AtomicBool::new(false),
        })
    }

    fn enabled(&self) -> bool {
        enabled(&self.config.read().unwrap())
    }

    /// 调度任务是否已经起了。
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Relaxed)
    }

    /// 配置了 `auto_clip` 才把上次没跑完的任务放回排队。
    async fn recover(&self) {
        if self.config.read().unwrap().auto_clip.is_none() {
            return;
        }
        match jobs::recover(&self.pool).await {
            Ok(0) => {}
            Ok(n) => info!(jobs = n, "自动切片：上次没跑完的任务放回排队，续跑"),
            Err(error) => warn!(%error, "自动切片：收尾上次的任务失败"),
        }
    }

    fn poke(self: &Arc<Self>) {
        if self.enabled() {
            self.start();
        }
        self.wake.notify_one();
    }

    fn start(self: &Arc<Self>) {
        if !self.started.swap(true, Ordering::Relaxed) {
            info!("自动切片：调度任务启动");
            tokio::spawn(self.clone().run_loop());
        }
    }

    async fn run_loop(self: Arc<Self>) {
        loop {
            let wait = if self.enabled() {
                match self.run_next().await {
                    Ok(Some(_)) => continue,
                    Ok(None) => Some(self.until_next().await),
                    Err(error) => {
                        warn!(%error, "自动切片：读取任务队列失败");
                        Some(Duration::from_secs(60))
                    }
                }
            } else {
                None
            };
            match wait {
                Some(wait) => {
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        () = self.wake.notified() => {}
                    }
                }
                None => self.wake.notified().await,
            }
        }
    }

    async fn until_next(&self) -> Duration {
        match jobs::next_queued(&self.pool).await {
            Ok(Some(job)) => Duration::from_millis((job.not_before - now_ms()).max(0) as u64)
                .min(self.options.idle_poll),
            _ => self.options.idle_poll,
        }
    }

    /// 取最早到点的排队任务跑完，返回任务 id 与结果；没有到点的任务返回 `None`。
    pub async fn run_next(&self) -> sqlx::Result<Option<(i64, Outcome)>> {
        let Some(next) = jobs::next_queued(&self.pool).await? else {
            return Ok(None);
        };
        if next.not_before > now_ms() {
            return Ok(None);
        }
        let Some(job) = jobs::claim(&self.pool, next.id, now_ms()).await? else {
            return Ok(None);
        };
        info!(
            job = job.id,
            session = job.session_id,
            trigger = ?job.trigger,
            resume = ?job.stage,
            "自动切片：开始任务"
        );
        let outcome = self.execute(&job).await;
        match &outcome {
            Outcome::Failed(error) => warn!(job = job.id, %error, "自动切片：任务失败"),
            outcome => info!(job = job.id, ?outcome, "自动切片：任务结束"),
        }
        Ok(Some((job.id, outcome)))
    }

    async fn execute(&self, job: &Job) -> Outcome {
        let files = SessionFiles::new(&self.options.root, job.session_id);
        let outcome = tokio::select! {
            outcome = self.work(job, &files) => outcome,
            () = self.canceled(job.id) => Outcome::Canceled,
        };
        let now = now_ms();
        let result = match &outcome {
            Outcome::Done => jobs::finish(&self.pool, job.id, None, now)
                .await
                .map(|_| ()),
            Outcome::Failed(error) => jobs::finish(&self.pool, job.id, Some(error.as_str()), now)
                .await
                .map(|_| ()),
            Outcome::Canceled | Outcome::Dropped => Ok(()),
        };
        if let Err(error) = result {
            warn!(%error, job = job.id, "自动切片：记录任务结果失败");
        }
        if outcome != Outcome::Dropped {
            files.remove_audio().await;
        }
        if let Err(error) = jobs::unpin_session(&self.pool, job.id).await {
            warn!(%error, job = job.id, "自动切片：撤销素材引用失败");
        }
        outcome
    }

    /// 任务在库里不再是运行中（被取消、被删）时返回。
    async fn canceled(&self, id: i64) {
        loop {
            tokio::time::sleep(self.options.cancel_poll).await;
            match jobs::state(&self.pool, id).await {
                Ok(Some(JobState::Running)) => {}
                Ok(_) => return,
                Err(error) => warn!(%error, job = id, "自动切片：查任务状态失败"),
            }
        }
    }

    async fn work(&self, job: &Job, files: &SessionFiles) -> Outcome {
        match self.try_work(job, files).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Failed(error),
        }
    }

    async fn try_work(&self, job: &Job, files: &SessionFiles) -> Result<Outcome, String> {
        let pool = &self.pool;
        let db = |error: sqlx::Error| format!("读写数据库出错：{error}");
        let Some(session) = store::session(pool, job.session_id).await.map_err(db)? else {
            jobs::delete(pool, job.id).await.map_err(db)?;
            return Ok(Outcome::Dropped);
        };
        let recording = session.ended_at.is_none() || live::is_recording(session.id);
        if recording && job.trigger == Trigger::Auto {
            info!(
                job = job.id,
                session = session.id,
                "自动切片：这一场又接着录了，删掉这条任务，等下次下播再排"
            );
            jobs::delete(pool, job.id).await.map_err(db)?;
            return Ok(Outcome::Dropped);
        }
        if recording {
            return Err("这一场还在录，下播后再生成".into());
        }
        let config = self
            .config
            .read()
            .unwrap()
            .auto_clip
            .clone()
            .unwrap_or_default();
        let endpoint = asr_endpoint(&config).ok_or(
            "没有配置转写接口：到设置页「自动切片（实验）」填好转写的接口地址和模型".to_string(),
        )?;
        let ffmpeg = crate::tools::ffmpeg_status().await;
        if !ffmpeg.available {
            return Err(format!(
                "FFmpeg 不可用，没法抽音频：{}",
                ffmpeg.error.unwrap_or_default()
            ));
        }
        jobs::pin_session(pool, job).await.map_err(db)?;
        if job.stage.is_none() {
            if !job.reuse_transcript {
                files.clear_transcript().await;
            }
            files.remove_audio().await;
        }
        let models = json!({
            "asr_model": endpoint.model(),
            "asr_host": display_host(endpoint.base_url()),
        });
        let mut warnings = Vec::new();

        let segments: Vec<SegmentRow> = store::session_segments(pool, session.id)
            .await
            .map_err(db)?
            .into_iter()
            .filter(transcribable)
            .collect();
        jobs::set_stage(pool, job.id, Stage::Audio, 0, segments.len() as i64)
            .await
            .map_err(db)?;
        let past_audio = !matches!(job.stage, Some(Stage::Audio) | None);
        let known: HashMap<i64, SegmentAudio> = if past_audio {
            files
                .load_silence()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|audio| (audio.segment_id, audio))
                .collect()
        } else {
            HashMap::new()
        };
        let mut audios = Vec::new();
        // 没有音轨的分段也记进静音表（时长 0），预估时就知道它不用转写
        let mut silent = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let cached = known
                .get(&segment.id)
                .filter(|audio| audio.start_ms == segment.start_ms && audio.duration_ms > 0)
                .cloned();
            let audio = match cached {
                Some(audio) => Ok(audio),
                None => self.segment_audio(files, segment).await,
            };
            match audio {
                Ok(audio) => audios.push(audio),
                Err(AudioError::NoAudio) => {
                    warnings.push(format!("分段 {} 没有音频，跳过", segment_name(segment)));
                    silent.push(SegmentAudio {
                        segment_id: segment.id,
                        start_ms: segment.start_ms,
                        duration_ms: 0,
                        silences: Vec::new(),
                    });
                }
                Err(error) => warnings.push(format!(
                    "分段 {} 抽音频失败，跳过：{error}",
                    segment_name(segment)
                )),
            }
            jobs::set_progress(pool, job.id, index as i64 + 1)
                .await
                .map_err(db)?;
        }
        jobs::set_details(pool, job.id, &models, &warnings)
            .await
            .map_err(db)?;
        if audios.is_empty() {
            return Err(if warnings.is_empty() {
                "这一场没有录完的分段，没有可以转写的音频".to_string()
            } else {
                format!("这一场没有可以转写的音频：{}", warnings.join("；"))
            });
        }
        let io = |error: std::io::Error| format!("写分析数据出错：{error}");
        let table: Vec<SegmentAudio> = audios.iter().chain(&silent).cloned().collect();
        files.save_silence(&table).await.map_err(io)?;

        let resumed_plan = match job.stage {
            Some(Stage::Audio) | None => None,
            Some(_) => files.load_plan().await,
        };
        let plan = match resumed_plan {
            Some(plan) => plan,
            None => {
                let max_ms = self.chunk_limit(&config).await;
                let plan: Vec<Chunk> = audios
                    .iter()
                    .flat_map(|audio| audio::plan_chunks(audio, max_ms))
                    .collect();
                files.replace_plan(&plan).await.map_err(io)?;
                plan
            }
        };
        let done = files.done_chunks().await;
        files.prune_transcript(&done).await.map_err(io)?;
        let pending: Vec<&Chunk> = plan.iter().filter(|c| !done.contains(&c.key)).collect();
        let planned_ms: i64 = pending.iter().map(|c| c.speech_ms()).sum();
        let recorded_ms: i64 = audios.iter().map(|a| a.duration_ms).sum();
        jobs::set_planned(pool, job.id, ceil_secs(planned_ms))
            .await
            .map_err(db)?;
        info!(
            job = job.id,
            recorded_min = minutes(recorded_ms),
            asr_min = minutes(planned_ms),
            chunks = pending.len(),
            "自动切片：静音跳过后要转写的时长"
        );
        let limit = config.max_asr_minutes();
        if planned_ms > (limit as i64).saturating_mul(60_000) {
            return Err(limit_message(planned_ms, limit));
        }

        let mut finished = (plan.len() - pending.len()) as i64;
        jobs::set_stage(pool, job.id, Stage::Asr, finished, plan.len() as i64)
            .await
            .map_err(db)?;
        let client = ModelClient::new(
            ClientOptions::builder()
                .chat_timeout(config.chat_timeout())
                .asr_timeout(config.asr_timeout())
                .maybe_backoff(self.options.backoff.clone())
                .build(),
        );
        let options = TranscribeOptions::builder()
            .maybe_language(config.asr_language.clone())
            .maybe_prompt(config.asr_prompt.clone())
            .build();
        let asr = Asr {
            client: &client,
            endpoint: &endpoint,
            options: &options,
            files,
        };
        for (index, chunk) in pending.iter().enumerate() {
            let Some(segment) = segments.iter().find(|s| s.id == chunk.segment_id) else {
                warnings.push(format!(
                    "分段 {} 已不在这一场，跳过它的转写",
                    chunk.segment_id
                ));
                jobs::set_details(pool, job.id, &models, &warnings)
                    .await
                    .map_err(db)?;
                files.commit_chunk(&chunk.key, &[]).await.map_err(io)?;
                continue;
            };
            self.segment_audio(files, segment)
                .await
                .map_err(|error| format!("分段 {} 抽音频失败：{error}", segment_name(segment)))?;
            let lines = asr
                .chunk(chunk, chunk.spans.clone(), chunk.key.clone(), 0)
                .await?;
            files.commit_chunk(&chunk.key, &lines).await.map_err(io)?;
            finished += 1;
            jobs::record_chunk(pool, job.id, finished, ceil_secs(chunk.speech_ms()))
                .await
                .map_err(db)?;
            if pending[index + 1..]
                .iter()
                .all(|later| later.segment_id != chunk.segment_id)
            {
                files.remove_segment_audio(chunk.segment_id).await;
            }
        }
        Ok(Outcome::Done)
    }

    /// 抽好的分段音频：缓存里有就用，没有就抽。
    async fn segment_audio(
        &self,
        files: &SessionFiles,
        segment: &SegmentRow,
    ) -> Result<SegmentAudio, AudioError> {
        if let Some(audio) = files.load_segment(segment.id).await
            && audio.start_ms == segment.start_ms
        {
            return Ok(audio);
        }
        tokio::fs::create_dir_all(files.audio_dir()).await?;
        let length = segment.end_ms.unwrap_or(segment.start_ms) - segment.start_ms;
        let (duration_ms, silences) = audio::extract(
            Path::new(&segment.path),
            &files.segment_audio(segment.id),
            length,
        )
        .await?;
        let audio = SegmentAudio {
            segment_id: segment.id,
            start_ms: segment.start_ms,
            duration_ms,
            silences,
        };
        files.save_segment(&audio).await?;
        Ok(audio)
    }

    /// 块长上限：连通性测试测出当前转写模型不给分句时间戳时用短块。
    async fn chunk_limit(&self, config: &AutoClipConfig) -> i64 {
        let no_segments = probe::load(&self.pool)
            .await
            .ok()
            .flatten()
            .filter(|report| report.matches_asr(config))
            .and_then(|report| report.asr.capable)
            == Some(false);
        if no_segments {
            audio::MAX_CHUNK_MS_WITHOUT_SEGMENTS
        } else {
            audio::MAX_CHUNK_MS
        }
    }
}

/// 转写一块要用的东西。
struct Asr<'a> {
    client: &'a ModelClient,
    endpoint: &'a Endpoint,
    options: &'a TranscribeOptions,
    files: &'a SessionFiles,
}

impl Asr<'_> {
    /// 切出块音频并转写；文件超过上传上限或服务回 413 时对半切开分别转写。
    fn chunk<'s>(
        &'s self,
        chunk: &'s Chunk,
        spans: Vec<Span>,
        name: String,
        splits: u32,
    ) -> Boxed<'s, Result<Vec<Line>, String>> {
        Box::pin(async move {
            let path = self.files.chunk_audio(&name);
            let size =
                audio::encode_chunk(&self.files.segment_audio(chunk.segment_id), &spans, &path)
                    .await
                    .map_err(|error| format!("切出音频块出错：{error}"))?;
            if size > audio::MAX_CHUNK_BYTES
                && splits < MAX_SPLITS
                && let Some((left, right)) = audio::halve(&spans)
            {
                let _ = tokio::fs::remove_file(&path).await;
                return self.halves(chunk, left, right, name, splits).await;
            }
            let bytes = tokio::fs::read(&path)
                .await
                .map_err(|error| format!("读音频块出错：{error}"));
            let _ = tokio::fs::remove_file(&path).await;
            let file = AudioFile {
                bytes: bytes?,
                file_name: format!("{name}.flac"),
                mime: "audio/flac".into(),
            };
            match self
                .client
                .transcribe(self.endpoint, &file, self.options)
                .await
            {
                Ok(transcript) => Ok(lines(chunk, &spans, &transcript)),
                Err(error) if error.kind == ErrorKind::PayloadTooLarge && splits < MAX_SPLITS => {
                    match audio::halve(&spans) {
                        Some((left, right)) => self.halves(chunk, left, right, name, splits).await,
                        None => Err(format!("转写失败：{error}")),
                    }
                }
                Err(error) => Err(format!("转写失败：{error}")),
            }
        })
    }

    async fn halves(
        &self,
        chunk: &Chunk,
        left: Vec<Span>,
        right: Vec<Span>,
        name: String,
        splits: u32,
    ) -> Result<Vec<Line>, String> {
        let mut lines = self
            .chunk(chunk, left, format!("{name}a"), splits + 1)
            .await?;
        lines.extend(
            self.chunk(chunk, right, format!("{name}b"), splits + 1)
                .await?,
        );
        Ok(lines)
    }
}

/// 转写结果换算成场次时间。没有分句时间戳时整块文字记在块的起止上。
fn lines(chunk: &Chunk, spans: &[Span], transcript: &Transcript) -> Vec<Line> {
    let at = |chunk_ms: i64| chunk.segment_start_ms + audio::to_segment_ms(spans, chunk_ms);
    let line = |from_ms: i64, to_ms: i64, text: &str| Line {
        chunk: chunk.key.clone(),
        from_ms,
        to_ms: to_ms.max(from_ms),
        text: text.trim().to_string(),
    };
    let lines: Vec<Line> = match &transcript.segments {
        Some(segments) => segments
            .iter()
            .map(|segment| {
                let from = (segment.start * 1000.0).round() as i64;
                let to = (segment.end * 1000.0).round() as i64;
                line(at(from), at((to - 1).max(from)) + 1, &segment.text)
            })
            .collect(),
        None => match (spans.first(), spans.last()) {
            (Some(first), Some(last)) => vec![line(
                chunk.segment_start_ms + first.from_ms,
                chunk.segment_start_ms + last.to_ms,
                &transcript.text,
            )],
            _ => Vec::new(),
        },
    };
    lines.into_iter().filter(|l| !l.text.is_empty()).collect()
}

#[cfg(test)]
mod tests;
