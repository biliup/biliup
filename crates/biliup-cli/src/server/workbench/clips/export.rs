//! 导出任务：接口把切片标成 `exporting` 之后交给这里在后台做完，结果写回 `clips`。
//!
//! - 快速剪：[`remux::to_file`]，进程内按关键帧切；
//! - 产物写在 `<root>/<场次>/<切片>.<扩展名>`（`root` 默认是工作目录下的 `clips`），先写 `.part`
//!   再改名；失败时删掉半成品，原因写进 `clips.error`，可以重试；
//! - 同时最多 [`QUICK_SLOTS`] 个快速剪，其余排队；
//! - 进度只在内存里（阶段 + 比例），列表接口带出去，前端轮询。

use super::plan::{self, Plan, PlanError};
use super::remux;
use super::{Clip, Exported, Mode};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::recorder::now_ms;
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;
use tracing::{info, warn};

pub const QUICK_SLOTS: usize = 2;
/// 出点之后的关键帧最多等这么久。
pub const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
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

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("切片还没导出好")]
    NotReady,
    #[error("切片文件不见了，重新导出一次")]
    Missing,
}

impl ClipExports {
    pub fn new(pool: ConnectionPool, root: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            root: root.into(),
            jobs: Mutex::new(HashMap::new()),
            next_token: AtomicU64::new(1),
            quick: Semaphore::new(QUICK_SLOTS),
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
        };
        let _permit = slots.acquire().await.map_err(|e| e.to_string())?;
        *lock(progress) = Progress::phase("准备中");
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
}
