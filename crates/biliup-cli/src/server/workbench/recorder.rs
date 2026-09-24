//! 录制侧：把下载器的「开段 / 关段」事件按顺序写进 `stream_sessions` / `segments`。
//!
//! 下载器的回调只往无界通道里丢一个事件（不 await、不碰数据库），由每个下载任务自己的
//! 写入任务按顺序落库、关段后建关键帧索引，录制热路径不会被 SQLite 或扫盘拖住；
//! 数据库出错只记日志，不影响录制与上传。
//!
//! 进程内写盘的下载器另有一个索引任务（[`index::live`]）边写边建关键帧索引；写入任务在改名、
//! 续扫、删除索引之前先等它处理完已发出的事件（[`IndexTap::sync`]），关段时的续扫只剩兜底。
//!
//! 场次时间轴：场次第一个分段开写（第一个关键帧到达）的墙钟为 t = 0；段内按容器时间戳走
//! （段长取索引扫出的内容时长，建不出索引才用下载器报告的时长，再没有才用墙钟差）。段与段之间
//! 看墙钟：新段开写比上一段关段（断流合并接上的一场，是上一次录制停下的时刻）晚出
//! [`GAP_TOLERANCE_MS`] 以上才算断流，时间轴跳过这段空白并记进 `gap_before_ms`；否则紧接上一段。
//! 只比相邻两段的墙钟，容器时长与墙钟的偏差不会跨段累积成假断流。

use super::index;
use super::store::{self, FinishedSegment, SegmentState};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use biliup::downloader::index_tap::IndexTap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// 开段墙钟比上一段末尾晚不到这么多毫秒时视为紧接着录，不记断流。
pub const GAP_TOLERANCE_MS: i64 = 1000;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// 这次录制记在哪一场。
#[derive(Debug, Clone)]
pub struct SessionTarget {
    /// 开播时监控循环插入或复用的 `stream_sessions.id`
    pub session_id: i64,
    /// `livestreamers.id`
    pub streamer_id: i64,
}

/// 关段时下载器能给的信息。
#[derive(Debug, Clone, Default)]
pub struct ClosedSegment {
    /// 下载器报告的段长（目前只有 mesio）。建不出索引时才用它。
    pub duration_ms: Option<u64>,
    /// 下载器报告的字节数（目前只有 mesio）。
    pub bytes: Option<u64>,
    pub danmaku_path: Option<PathBuf>,
    /// 分段会被丢弃（小于 `filtering_threshold` 被过滤删除等），记为 `deleted`，不建索引。
    pub discard: bool,
}

#[derive(Debug)]
enum Event {
    RunStarted {
        at: i64,
    },
    Opened {
        path: PathBuf,
        at: i64,
    },
    Closed {
        path: PathBuf,
        at: i64,
        info: ClosedSegment,
    },
    Settle(oneshot::Sender<()>),
    Finish,
}

/// 往写入任务发事件的句柄，可随意克隆；所有方法都立即返回。
#[derive(Debug, Clone)]
pub struct RecorderHandle {
    tx: UnboundedSender<Event>,
    index: Option<IndexTap>,
}

impl RecorderHandle {
    /// 交给下载器的关键帧索引旁路（见 [`DownloadConfig::index_tap`]）。
    ///
    /// [`DownloadConfig::index_tap`]: crate::server::core::downloader::DownloadConfig::index_tap
    pub fn index_tap(&self) -> Option<IndexTap> {
        self.index.clone()
    }

    /// 下载器新一轮拉流开始（断流重连后）。
    pub fn run_started(&self) {
        self.run_started_at(now_ms());
    }

    /// 开始写一个分段文件（可以是带 `.part` 的临时名）。
    pub fn opened(&self, path: &Path) {
        self.opened_at(path, now_ms());
    }

    /// 一个分段写完（`path` 为最终文件名）。
    pub fn closed(&self, path: &Path, info: ClosedSegment) {
        self.closed_at(path, now_ms(), info);
    }

    /// 等写入任务处理完此前发出的所有事件（任务已结束时立即完成）。
    ///
    /// 关段时的删除点要先等它：否则分段行可能还没写进去、或还是 `.part` 路径，删除点认不出是
    /// 工作台的分段，就不管引用和保留直接删了。
    pub fn settled(&self) -> impl Future<Output = ()> + Send + 'static {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Event::Settle(tx));
        async move {
            let _ = rx.await;
        }
    }

    pub(crate) fn run_started_at(&self, at: i64) {
        let _ = self.tx.send(Event::RunStarted { at });
    }

    pub(crate) fn opened_at(&self, path: &Path, at: i64) {
        let _ = self.tx.send(Event::Opened {
            path: path.to_path_buf(),
            at,
        });
    }

    pub(crate) fn closed_at(&self, path: &Path, at: i64, info: ClosedSegment) {
        let _ = self.tx.send(Event::Closed {
            path: path.to_path_buf(),
            at,
            info,
        });
    }
}

/// 一个下载任务的场次记录器。任务结束时调用 [`SessionRecorder::finish`]。
pub struct SessionRecorder {
    handle: RecorderHandle,
    task: JoinHandle<()>,
}

impl SessionRecorder {
    /// `index`：边写边建关键帧索引的任务（进程内写盘的下载器才有），见 [`index::live::spawn`]。
    pub fn spawn(pool: ConnectionPool, target: SessionTarget, index: Option<IndexTap>) -> Self {
        let (tx, rx) = unbounded_channel();
        let task = tokio::spawn(Writer::new(pool, target, index.clone()).run(rx));
        Self {
            handle: RecorderHandle { tx, index },
            task,
        }
    }

    pub fn handle(&self) -> RecorderHandle {
        self.handle.clone()
    }

    /// 处理完已发出的事件，收尾没关上的分段，写入场次 `ended_at`。
    /// 之后仍在别处（如边录边传会话）留着的句柄再发事件会被丢弃。
    /// 不调用就 drop 时，写入任务在所有句柄都释放后照样收尾。
    pub async fn finish(self) {
        let _ = self.handle.tx.send(Event::Finish);
        if let Err(e) = self.task.await {
            warn!(error = %e, "切片工作台场次记录任务异常退出");
        }
    }
}

struct OpenSegment {
    id: i64,
    path: PathBuf,
    start_ms: i64,
    opened_at: i64,
}

struct Writer {
    pool: ConnectionPool,
    target: SessionTarget,
    index: Option<IndexTap>,
    /// 时间轴 0 点；这一场还没有分段时为 `None`。
    started_at: Option<i64>,
    last_end_ms: i64,
    open: Option<OpenSegment>,
    run_started_at: i64,
    run_has_segment: bool,
    /// 本次任务里上一段关段的墙钟。
    last_close_at: Option<i64>,
    /// 断流合并接上的一场：上一次录制停下的墙钟，给本次第一段算断流用。
    resumed_after: Option<i64>,
}

impl Writer {
    fn new(pool: ConnectionPool, target: SessionTarget, index: Option<IndexTap>) -> Self {
        Self {
            pool,
            target,
            index,
            started_at: None,
            last_end_ms: 0,
            open: None,
            run_started_at: now_ms(),
            run_has_segment: false,
            last_close_at: None,
            resumed_after: None,
        }
    }

    async fn run(mut self, mut rx: UnboundedReceiver<Event>) {
        match store::begin_recording(&self.pool, self.target.session_id).await {
            Ok(start) => {
                if start.resumed_after.is_some() {
                    tracing::info!(
                        session = self.target.session_id,
                        last_end_ms = start.last_end_ms,
                        "切片工作台：下播后很快又开播，接着记在上一场"
                    );
                }
                self.started_at = start.started_at;
                self.last_end_ms = start.last_end_ms;
                self.resumed_after = start.resumed_after;
            }
            Err(e) => warn!(error = %e, "切片工作台读取场次失败，不影响录制"),
        }
        while let Some(event) = rx.recv().await {
            debug!(?event, "切片工作台分段事件");
            let result = match event {
                Event::RunStarted { at } => self.on_run_started(at).await,
                Event::Opened { path, at } => self.on_opened(path, at).await,
                Event::Closed { path, at, info } => self.on_closed(path, at, info).await,
                Event::Settle(done) => {
                    let _ = done.send(());
                    Ok(())
                }
                Event::Finish => break,
            };
            if let Err(e) = result {
                warn!(error = %e, "切片工作台写场次 / 分段失败，不影响录制");
            }
        }
        if let Err(e) = self.on_finish().await {
            warn!(error = %e, "切片工作台收尾场次失败");
        }
    }

    async fn on_run_started(&mut self, at: i64) -> sqlx::Result<()> {
        self.finalize_open(at).await?;
        self.run_started_at = at;
        self.run_has_segment = false;
        Ok(())
    }

    async fn on_opened(&mut self, path: PathBuf, at: i64) -> sqlx::Result<()> {
        self.finalize_open(at).await?;
        let Some(container) = store::container_of(&path) else {
            debug!(path = %path.display(), "容器不在切片工作台支持范围内，不记录");
            return Ok(());
        };
        let (id, start_ms) = self.insert_segment(&path, container, at).await?;
        self.open = Some(OpenSegment {
            id,
            path,
            start_ms,
            opened_at: at,
        });
        Ok(())
    }

    async fn on_closed(&mut self, path: PathBuf, at: i64, info: ClosedSegment) -> sqlx::Result<()> {
        if let Some(open) = &self.open
            && !same_segment(&open.path, &path)
        {
            self.finalize_open(at).await?;
        }
        let Some(container) = store::container_of(&path) else {
            return Ok(());
        };
        self.sync_index().await;
        let open = match self.open.take() {
            Some(open) => {
                move_index(&open.path, &path);
                open
            }
            // 只报关段的下载器（ffmpeg 内部分段、yt-dlp）：先按内容算出段长，再往前推开段时刻
            None => {
                let estimate = if self.run_has_segment {
                    self.last_close_at.unwrap_or(self.run_started_at)
                } else {
                    self.run_started_at
                };
                let probe = if info.discard {
                    None
                } else {
                    scan_index(&path).await
                };
                let duration = probe
                    .filter(|d| *d > 0)
                    .map(i64::from)
                    .or(info.duration_ms.map(|d| d as i64))
                    .unwrap_or(0);
                let opened_at = (at - duration).max(estimate).min(at);
                let (id, start_ms) = self.insert_segment(&path, container, opened_at).await?;
                OpenSegment {
                    id,
                    path: path.clone(),
                    start_ms,
                    opened_at,
                }
            }
        };

        let exists = path.exists();
        let (index_path, index_duration) = if info.discard || !exists {
            index::remove(&path);
            (None, None)
        } else {
            match refresh_index(&path).await {
                Some(duration) => (Some(index::index_path(&path)), Some(duration)),
                None => (None, None),
            }
        };
        // 段长以索引扫出的内容时长为准，这样 locate 给出的关键帧一定落在 [start_ms, end_ms) 内；
        // mesio 的 HLS 统计按 EXTINF 累加，遇到 EXTINF 偏小的源会比真实内容短很多。
        let duration = index_duration
            .filter(|d| *d > 0)
            .map(i64::from)
            .or(info.duration_ms.filter(|d| *d > 0).map(|d| d as i64))
            .unwrap_or((at - open.opened_at).max(0));
        let bytes = info
            .bytes
            .or_else(|| std::fs::metadata(&path).ok().map(|m| m.len()))
            .map(|b| b as i64);
        // 过滤删除的分段由删除点定状态（被引用时推迟删除），这里只按盘上还有没有文件记，
        // 免得先落库的 `deleted` 让删除点认不出这个分段而直接删掉。
        let state = if exists {
            SegmentState::Finished
        } else if info.discard {
            SegmentState::Deleted
        } else {
            SegmentState::Missing
        };
        let end_ms = open.start_ms + duration;
        store::finish_segment(
            &self.pool,
            open.id,
            &FinishedSegment {
                path: path_string(&path),
                state,
                end_ms,
                bytes,
                index_path: index_path.as_deref().map(path_string),
                danmaku_path: info.danmaku_path.as_deref().map(path_string),
            },
        )
        .await?;
        self.segment_done(end_ms, at);
        Ok(())
    }

    async fn on_finish(&mut self) -> sqlx::Result<()> {
        let now = now_ms();
        self.finalize_open(now).await?;
        store::close_session(
            &self.pool,
            self.target.session_id,
            self.last_close_at.unwrap_or(now),
        )
        .await
    }

    /// 等索引任务处理完已发出的事件：关段事件之前的写入都已进 `.idx`，接下来可以改名、续扫。
    async fn sync_index(&self) {
        if let Some(index) = &self.index {
            index.sync().await;
        }
    }

    fn segment_done(&mut self, end_ms: i64, at: i64) {
        self.last_end_ms = self.last_end_ms.max(end_ms);
        self.run_has_segment = true;
        self.last_close_at = Some(at);
    }

    /// 在场次时间轴上放一个新分段并插行，返回 `(id, start_ms)`。
    async fn insert_segment(
        &mut self,
        path: &Path,
        container: &str,
        opened_at: i64,
    ) -> sqlx::Result<(i64, i64)> {
        let (start_ms, gap) = match self.started_at {
            None => {
                self.started_at = Some(
                    store::set_started_at(&self.pool, self.target.session_id, opened_at).await?,
                );
                (0, 0)
            }
            Some(_) => place(
                self.last_end_ms,
                self.last_close_at
                    .or(self.resumed_after)
                    .map(|closed| opened_at - closed),
            ),
        };
        let id = store::insert_segment(
            &self.pool,
            self.target.session_id,
            &path_string(path),
            container,
            start_ms,
            gap,
        )
        .await?;
        Ok((id, start_ms))
    }

    /// 收尾没等到关段事件的分段（下载器出错退出、进程被停止）：按盘上的文件补齐；
    /// 文件没生成或是空的就删掉这一行。
    async fn finalize_open(&mut self, at: i64) -> sqlx::Result<()> {
        let Some(open) = self.open.take() else {
            return Ok(());
        };
        self.sync_index().await;
        let Some((path, len)) = [open.path.clone(), strip_part(&open.path)]
            .into_iter()
            .find_map(|p| {
                let len = std::fs::metadata(&p).ok()?.len();
                (len > 0).then_some((p, len))
            })
        else {
            index::remove(&open.path);
            store::delete_segment(&self.pool, open.id).await?;
            if store::clear_started_at_if_empty(&self.pool, self.target.session_id).await? {
                self.started_at = None;
            }
            return Ok(());
        };
        move_index(&open.path, &path);
        let index_duration = refresh_index(&path).await;
        let duration = index_duration
            .filter(|d| *d > 0)
            .map(i64::from)
            .unwrap_or((at - open.opened_at).max(0));
        let end_ms = open.start_ms + duration;
        store::finish_segment(
            &self.pool,
            open.id,
            &FinishedSegment {
                path: path_string(&path),
                state: SegmentState::Finished,
                end_ms,
                bytes: Some(len as i64),
                index_path: index_duration.map(|_| path_string(&index::index_path(&path))),
                danmaku_path: None,
            },
        )
        .await?;
        self.segment_done(end_ms, at);
        Ok(())
    }
}

/// 新分段在时间轴上的位置：`since_close` 为开段墙钟距上一段关段的毫秒数（不知道时为 `None`）。
/// 返回 `(start_ms, gap_before_ms)`。
pub(crate) fn place(last_end_ms: i64, since_close: Option<i64>) -> (i64, i64) {
    match since_close {
        Some(gap) if gap > GAP_TOLERANCE_MS => (last_end_ms + gap, gap),
        _ => (last_end_ms, 0),
    }
}

/// 建（续扫）已写完分段的索引，返回段长毫秒；不支持的容器或扫描失败返回 `None`。
async fn refresh_index(path: &Path) -> Option<u32> {
    let owned = path.to_path_buf();
    match tokio::task::spawn_blocking(move || index::refresh(&owned, true)).await {
        Ok(Ok(index)) => Some(index.duration_ms),
        Ok(Err(e)) => {
            if e.kind() != std::io::ErrorKind::Unsupported {
                warn!(path = %path.display(), error = %e, "建关键帧索引失败");
            }
            index::remove(path);
            None
        }
        Err(e) => {
            warn!(path = %path.display(), error = %e, "建关键帧索引的任务异常退出");
            None
        }
    }
}

async fn scan_index(path: &Path) -> Option<u32> {
    refresh_index(path).await.filter(|d| *d > 0)
}

/// 录制时按临时名建过的索引，跟着改名。
fn move_index(from: &Path, to: &Path) {
    if from == to {
        return;
    }
    let from_index = index::index_path(from);
    if from_index.exists()
        && let Err(e) = std::fs::rename(&from_index, index::index_path(to))
    {
        debug!(error = %e, "索引缓存改名失败，稍后重新扫描");
        let _ = std::fs::remove_file(&from_index);
    }
}

fn strip_part(path: &Path) -> PathBuf {
    let s = path.as_os_str().to_string_lossy();
    PathBuf::from(s.strip_suffix(".part").unwrap_or(&s).to_string())
}

fn same_segment(open: &Path, closed: &Path) -> bool {
    open == closed || strip_part(open) == closed
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
