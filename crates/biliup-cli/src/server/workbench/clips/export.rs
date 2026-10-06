//! 导出任务：接口把切片标成 `exporting` 之后交给这里在后台做完，结果写回 `clips`。
//!
//! - 快速剪：[`remux::to_file`]，进程内按关键帧切；
//! - 精确剪：把覆盖入点、出点的关键帧区间照样接好，经管道喂给 ffmpeg，按入点、出点解码裁剪后
//!   重新编码成 MP4（`-ss` / `-to` 放在输入之后，逐帧精确）。跨分段、断流缺口、正在写的尾巴都由
//!   快速剪那一套处理，ffmpeg 只看到一条从 0 开始的连续流；
//! - 产物写在 `<root>/<场次>/<切片>.<扩展名>`（`root` 默认是工作目录下的 `clips`），先写 `.part`
//!   再改名；失败时删掉半成品，原因写进 `clips.error`，可以重试；
//! - 同时最多 [`QUICK_SLOTS`] 个快速剪、[`PRECISE_SLOTS`] 个精确剪，其余排队；
//! - 进度只在内存里（阶段 + 比例），列表接口带出去，前端轮询。

use super::plan::{self, Plan, PlanError};
use super::remux;
use super::{Clip, Exported, Mode};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::dvr::{self, flv};
use crate::server::workbench::index::Container;
use crate::server::workbench::recorder::now_ms;
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;
use tracing::{info, warn};

pub const QUICK_SLOTS: usize = 2;
pub const PRECISE_SLOTS: usize = 1;
/// 出点之后的关键帧最多等这么久。
pub const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
/// ffmpeg 报错时保留的 stderr 末尾长度。
const STDERR_TAIL: usize = 4096;
/// 国内平台在 FLV 里写 HEVC 用的视频 codec id（不是 Enhanced FLV），官方 FFmpeg 不认。
const FLV_CODEC_LEGACY_HEVC: u8 = 12;
/// 判断编码时读文件开头多少字节：够装下文件头、`onMetaData` 和序列头。
const CODEC_SNIFF_BYTES: usize = 512 * 1024;
const LEGACY_HEVC_HINT: &str = "这段录像是国内平台在 FLV 里写的 HEVC（codec id 12），服务器上的 FFmpeg 读不了；\
     换一个支持它的 FFmpeg（配置里的 ffmpeg_path），或者用快速剪、下载源格式";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Progress {
    pub phase: &'static str,
    /// 0～1；不知道总量时为 `None`。
    pub ratio: Option<f64>,
}

impl Progress {
    fn phase(phase: &'static str) -> Self {
        Self { phase, ratio: None }
    }
}

struct Job {
    token: u64,
    progress: Arc<Mutex<Progress>>,
    abort: Option<AbortHandle>,
}

pub struct ClipExports {
    pool: ConnectionPool,
    root: PathBuf,
    jobs: Mutex<HashMap<i64, Job>>,
    next_token: AtomicU64,
    quick: Semaphore,
    precise: Semaphore,
    /// 下载 MP4 时按需转封装，同一个切片同时只转一次。
    remuxing: tokio::sync::Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 写切片文件出错时给人看的话。
fn io_message(e: &io::Error) -> String {
    if e.kind() == io::ErrorKind::StorageFull || e.raw_os_error() == Some(28) {
        return "磁盘空间不足，写切片文件失败；清理磁盘后重试".into();
    }
    if e.kind() == io::ErrorKind::NotFound {
        return "源录像文件不见了（可能刚被清理），剪不了这一段".into();
    }
    format!("读写文件出错：{e}")
}

fn plan_message(e: PlanError) -> String {
    match e {
        PlanError::Unavailable(message) => message,
        PlanError::Io(e) => io_message(&e),
        PlanError::Db(e) => format!("读数据库出错：{e}"),
    }
}

/// ffmpeg 失败时从 stderr 里挑一句给人看。
pub(super) fn ffmpeg_message(stderr: &str, status: Option<std::process::ExitStatus>) -> String {
    if stderr.contains("No space left on device") {
        return "磁盘空间不足，写切片文件失败；清理磁盘后重试".into();
    }
    if stderr.contains("Unknown encoder 'libx264'") || stderr.contains("Encoder not found") {
        return "这个 FFmpeg 没有 libx264 编码器（常见于 LGPL 版），精确剪需要带 libx264 的 FFmpeg"
            .into();
    }
    // 动态链接器的警告（"no version information available"）不是失败原因。
    let last = stderr
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.contains("no version information available"))
        .unwrap_or_default();
    if last.is_empty() && status.is_some_and(interrupted) {
        return "FFmpeg 被中止了（可能是服务正在停止），重试即可".into();
    }
    match status {
        Some(status) if last.is_empty() => format!("FFmpeg 转码失败（{status}）"),
        _ if last.is_empty() => "FFmpeg 转码失败".into(),
        _ => format!("FFmpeg 转码失败：{last}"),
    }
}

/// ffmpeg 收到 SIGINT/SIGTERM 时以 255 退出；被 SIGKILL 之类直接杀掉时没有退出码。
fn interrupted(status: std::process::ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if status.signal().is_some() {
            return true;
        }
    }
    status.code() == Some(255)
}

/// `path` 是不是用 codec id 12 写 HEVC 的 FLV。只在 ffmpeg 失败后用来解释原因：打过补丁的 ffmpeg 能读。
pub(super) async fn legacy_hevc_flv(path: &Path) -> bool {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let Ok(region) = dvr::read_at(&mut file, 0, CODEC_SNIFF_BYTES).await else {
        return false;
    };
    flv::Header::parse(&region).is_ok_and(|header| {
        header.sequence_headers.iter().any(|tag| {
            tag.tag_type == flv::TAG_VIDEO
                && tag
                    .body()
                    .first()
                    .is_some_and(|b| b & 0x0f == FLV_CODEC_LEGACY_HEVC)
        })
    })
}

async fn ffmpeg_unavailable() -> Option<String> {
    let status = crate::tools::ffmpeg_status().await;
    (!status.available).then(|| status.error.unwrap_or_else(|| "找不到 FFmpeg".into()))
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("切片还没导出好")]
    NotReady,
    #[error("切片文件不见了，重新导出一次")]
    Missing,
    #[error("{0}")]
    Ffmpeg(String),
    #[error("{0}")]
    Io(String),
}

impl ClipExports {
    pub fn new(pool: ConnectionPool, root: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            root: root.into(),
            jobs: Mutex::new(HashMap::new()),
            next_token: AtomicU64::new(1),
            quick: Semaphore::new(QUICK_SLOTS),
            precise: Semaphore::new(PRECISE_SLOTS),
            remuxing: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn dir(&self, session_id: i64) -> PathBuf {
        self.root.join(session_id.to_string())
    }

    /// 正在导出的切片的进度。
    pub fn progress(&self, id: i64) -> Option<Progress> {
        lock(&self.jobs)
            .get(&id)
            .map(|job| lock(&job.progress).clone())
    }

    /// 在后台导出 `clip`（调用方已经把它标成 `exporting`）。
    pub fn start(self: &Arc<Self>, clip: Clip) {
        let token = self.next_token.fetch_add(1, Ordering::Relaxed);
        let progress = Arc::new(Mutex::new(Progress::phase("排队中")));
        let id = clip.id;
        lock(&self.jobs).insert(
            id,
            Job {
                token,
                progress: progress.clone(),
                abort: None,
            },
        );
        let this = self.clone();
        let handle = tokio::spawn(async move {
            this.run(clip, progress).await;
            let mut jobs = lock(&this.jobs);
            if jobs.get(&id).is_some_and(|job| job.token == token) {
                jobs.remove(&id);
            }
        });
        if let Some(job) = lock(&self.jobs).get_mut(&id)
            && job.token == token
        {
            job.abort = Some(handle.abort_handle());
        }
    }

    /// 停掉切片的导出任务（删切片时），返回之前是否在导出。
    pub fn cancel(&self, id: i64) -> bool {
        match lock(&self.jobs).remove(&id) {
            Some(job) => {
                if let Some(abort) = job.abort {
                    abort.abort();
                }
                true
            }
            None => false,
        }
    }

    /// 删掉切片的全部产物（含半成品和下载用的 MP4）。
    pub async fn remove_outputs(&self, session_id: i64, id: i64) {
        let dir = self.dir(session_id);
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            return;
        };
        let prefix = format!("{id}.");
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.file_name().to_string_lossy().starts_with(&prefix)
                && let Err(e) = tokio::fs::remove_file(entry.path()).await
            {
                warn!(path = %entry.path().display(), error = %e, "删除切片文件失败");
            }
        }
    }

    async fn run(&self, clip: Clip, progress: Arc<Mutex<Progress>>) {
        let mode = clip.mode.unwrap_or(Mode::Quick);
        let started = std::time::Instant::now();
        self.remove_outputs(clip.session_id, clip.id).await;
        let result = self.export(&clip, mode, &progress).await;
        let now = now_ms();
        match result {
            Ok(done) => {
                info!(
                    clip = clip.id,
                    mode = mode.as_str(),
                    bytes = done.output_bytes,
                    duration_ms = done.duration_ms,
                    secs = started.elapsed().as_secs(),
                    "切片导出完成"
                );
                if let Err(e) = super::finish_export(&self.pool, clip.id, &done, now).await {
                    warn!(clip = clip.id, error = %e, "切片已导出，但写回数据库失败");
                }
            }
            Err(message) => {
                warn!(clip = clip.id, mode = mode.as_str(), %message, "切片导出失败");
                self.remove_outputs(clip.session_id, clip.id).await;
                if let Err(e) = super::fail_export(&self.pool, clip.id, &message, now).await {
                    warn!(clip = clip.id, error = %e, "记录切片导出失败原因时出错");
                }
            }
        }
    }

    async fn export(
        &self,
        clip: &Clip,
        mode: Mode,
        progress: &Arc<Mutex<Progress>>,
    ) -> Result<Exported, String> {
        let slots = match mode {
            Mode::Quick => &self.quick,
            Mode::Precise => &self.precise,
        };
        let _permit = slots.acquire().await.map_err(|e| e.to_string())?;
        *lock(progress) = Progress::phase("准备中");
        if mode == Mode::Precise
            && let Some(error) = ffmpeg_unavailable().await
        {
            return Err(format!("精确剪要用 FFmpeg 转码，但{error}"));
        }
        let plan = plan::resolve(
            &self.pool,
            clip.session_id,
            clip.in_ms,
            clip.out_ms,
            WAIT_TIMEOUT,
            || *lock(progress) = Progress::phase("等出点之后的关键帧写到盘上"),
        )
        .await
        .map_err(plan_message)?;
        let dir = self.dir(clip.session_id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| io_message(&e))?;
        match mode {
            Mode::Quick => self.quick_cut(clip, &plan, &dir, progress).await,
            Mode::Precise => self.precise_cut(clip, &plan, &dir, progress).await,
        }
    }

    async fn quick_cut(
        &self,
        clip: &Clip,
        plan: &Plan,
        dir: &Path,
        progress: &Arc<Mutex<Progress>>,
    ) -> Result<Exported, String> {
        let name = format!("{}.{}", clip.id, remux::extension(plan.container));
        let part = dir.join(format!("{name}.part"));
        let target = dir.join(&name);
        let total = plan.bytes().max(1);
        *lock(progress) = Progress {
            phase: "剪切中",
            ratio: Some(0.0),
        };
        let mut report = |read: u64| {
            lock(progress).ratio = Some((read as f64 / total as f64).min(1.0));
        };
        let done = remux::to_file(plan, &part, &mut report)
            .await
            .map_err(|e| io_message(&e))?;
        tokio::fs::rename(&part, &target)
            .await
            .map_err(|e| io_message(&e))?;
        Ok(Exported {
            cut_in_ms: plan.cut_in_ms,
            cut_out_ms: plan.cut_out_ms,
            output_path: target.to_string_lossy().into_owned(),
            output_bytes: done.bytes as i64,
            duration_ms: done.duration_ms,
        })
    }

    async fn precise_cut(
        &self,
        clip: &Clip,
        plan: &Plan,
        dir: &Path,
        progress: &Arc<Mutex<Progress>>,
    ) -> Result<Exported, String> {
        let cut_in = clip.in_ms.max(plan.cut_in_ms);
        let cut_out = clip.out_ms.min(plan.cut_out_ms);
        let from = plan.output_ms(cut_in);
        let to = plan.output_ms(cut_out);
        if to <= from {
            return Err("所选范围里没有录像画面，换个范围再剪".into());
        }
        let name = format!("{}.mp4", clip.id);
        let part = dir.join(format!("{name}.part"));
        let target = dir.join(&name);
        *lock(progress) = Progress {
            phase: "转码中",
            ratio: Some(0.0),
        };
        let secs = |ms: i64| format!("{}.{:03}", ms / 1000, ms % 1000);
        let mut child = crate::tools::ffmpeg_command()
            .args(["-hide_banner", "-loglevel", "error", "-nostats"])
            .args(["-progress", "pipe:1"])
            .args(["-f", remux::ffmpeg_format(plan.container), "-i", "pipe:0"])
            .args(["-ss", &secs(from), "-to", &secs(to)])
            .args(["-map", "0:v:0", "-map", "0:a:0?"])
            .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"])
            .args(["-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k"])
            .args(["-movflags", "+faststart", "-f", "mp4", "-y"])
            .arg(&part)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("启动 FFmpeg 失败：{e}"))?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let span = (to - from) as f64;
        let watch = {
            let progress = progress.clone();
            async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(us) = line
                        .strip_prefix("out_time_us=")
                        .and_then(|v| v.trim().parse::<f64>().ok())
                    {
                        lock(&progress).ratio = Some((us / 1000.0 / span).clamp(0.0, 1.0));
                    }
                }
            }
        };
        let errors = async move {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            let start = buf.len().saturating_sub(STDERR_TAIL);
            String::from_utf8_lossy(&buf[start..]).into_owned()
        };
        let feed = async move {
            let mut ignore = |_: u64| {};
            let fed = remux::to_pipe(plan, &mut stdin, &mut ignore).await;
            drop(stdin);
            fed
        };
        let (fed, (), stderr) = tokio::join!(feed, watch, errors);
        let status = child.wait().await.ok();
        if !status.is_some_and(|s| s.success()) {
            if plan.container == Container::Flv && legacy_hevc_flv(&plan.pieces[0].path).await {
                return Err(format!("精确剪失败：{LEGACY_HEVC_HINT}"));
            }
            return Err(ffmpeg_message(&stderr, status));
        }
        if let Err(e) = fed
            && e.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(io_message(&e));
        }
        tokio::fs::rename(&part, &target)
            .await
            .map_err(|e| io_message(&e))?;
        let bytes = tokio::fs::metadata(&target)
            .await
            .map_err(|e| io_message(&e))?
            .len();
        Ok(Exported {
            cut_in_ms: cut_in,
            cut_out_ms: cut_out,
            output_path: target.to_string_lossy().into_owned(),
            output_bytes: bytes as i64,
            duration_ms: to - from,
        })
    }

    /// 下载用的 MP4：产物本来就是 MP4 直接给；否则用 ffmpeg `-c copy` 转封装一份缓存在旁边。
    pub async fn mp4(&self, clip: &Clip) -> Result<PathBuf, DownloadError> {
        let source = clip
            .output_path
            .as_deref()
            .filter(|_| clip.state == super::State::Ready || clip.state == super::State::Published)
            .map(PathBuf::from)
            .ok_or(DownloadError::NotReady)?;
        if !tokio::fs::try_exists(&source).await.unwrap_or(false) {
            return Err(DownloadError::Missing);
        }
        if source
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
        {
            return Ok(source);
        }
        let target = source.with_extension("mp4");
        let gate = self
            .remuxing
            .lock()
            .await
            .entry(clip.id)
            .or_default()
            .clone();
        let _guard = gate.lock().await;
        if tokio::fs::try_exists(&target).await.unwrap_or(false) {
            return Ok(target);
        }
        if let Some(error) = ffmpeg_unavailable().await {
            return Err(DownloadError::Ffmpeg(format!(
                "转成 MP4 要用 FFmpeg，但{error}；可以先下载源格式"
            )));
        }
        let part = source.with_extension("mp4.part");
        let output = crate::tools::ffmpeg_command()
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
            .arg(&source)
            .args(["-map", "0:v", "-map", "0:a?", "-c", "copy"])
            .args(["-movflags", "+faststart", "-f", "mp4", "-y"])
            .arg(&part)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| DownloadError::Ffmpeg(format!("启动 FFmpeg 失败：{e}")))?;
        if !output.status.success() {
            let _ = tokio::fs::remove_file(&part).await;
            if legacy_hevc_flv(&source).await {
                return Err(DownloadError::Ffmpeg(format!(
                    "转成 MP4 失败：{LEGACY_HEVC_HINT}"
                )));
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DownloadError::Ffmpeg(
                ffmpeg_message(&stderr, Some(output.status)).replace("转码", "转封装"),
            ));
        }
        tokio::fs::rename(&part, &target)
            .await
            .map_err(|e| DownloadError::Io(io_message(&e)))?;
        self.remuxing.lock().await.remove(&clip.id);
        Ok(target)
    }
}
