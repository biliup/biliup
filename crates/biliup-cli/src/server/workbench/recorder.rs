//! 录制侧：把下载器的「开段 / 关段」事件按顺序写进 `stream_sessions` / `segments`。
//!
//! 下载器的回调只往无界通道里丢一个事件（不 await、不碰数据库），由每个下载任务自己的
//! 写入任务按顺序落库、关段后建关键帧索引，录制热路径不会被 SQLite 或扫盘拖住；
//! 数据库出错只记日志，不影响录制与上传。
//!
//! 场次时间轴：场次第一个分段开写（第一个关键帧到达）的墙钟为 t = 0；段内按容器时间戳走
//! （段长取下载器报告的时长，没有就取索引扫出的时长，再没有才用墙钟差）；段与段之间用
//! 开段时的墙钟接上，比上一段末尾晚出 [`GAP_TOLERANCE_MS`] 以上才算断流，记进
//! `gap_before_ms`，否则紧接上一段（吸收容器时长与墙钟的小偏差，不让它累积）。

use super::index;
use super::store::{self, FinishedSegment, SegmentState};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// 开段墙钟比上一段末尾晚不到这么多毫秒时视为紧接着录，不记断流。
pub const GAP_TOLERANCE_MS: i64 = 1000;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// 这次录制归到哪个主播、挂哪行 streamerinfo。
#[derive(Debug, Clone)]
pub struct SessionTarget {
    /// `livestreamers.id`
    pub streamer_id: i64,
    /// 本次下载任务的 `streamerinfo.id`
    pub streamerinfo_id: i64,
    pub title: String,
    /// 下播后多久内再开播算同一场；0 = 从不合并。
    pub merge_window_ms: i64,
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
    Deleted {
        path: PathBuf,
    },
    Finish,
}

/// 往写入任务发事件的句柄，可随意克隆；所有方法都立即返回。
#[derive(Debug, Clone)]
pub struct RecorderHandle {
    tx: UnboundedSender<Event>,
}

impl RecorderHandle {
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

    /// 已记录的分段文件被删掉了（边录边传投稿后清理临时文件等）。
    pub fn deleted(&self, path: &Path) {
        let _ = self.tx.send(Event::Deleted {
            path: path.to_path_buf(),
        });
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
    pub fn spawn(pool: ConnectionPool, target: SessionTarget) -> Self {
        let (tx, rx) = unbounded_channel();
        let task = tokio::spawn(Writer::new(pool, target).run(rx));
        Self {
            handle: RecorderHandle { tx },
            task,
        }
    }

    pub fn handle(&self) -> RecorderHandle {
        self.handle.clone()
    }

    /// 处理完已发出的事件，收尾没关上的分段，写入场次 `ended_at`。
    /// 之后仍在别处（如边录边传会话）留着的句柄再发事件会被丢弃。
    pub async fn finish(self) {
        let _ = self.handle.tx.send(Event::Finish);
        if let Err(e) = self.task.await {
            warn!(error = %e, "切片工作台场次记录任务异常退出");
        }
    }
}

struct Session {
    id: i64,
    started_at: i64,
    last_end_ms: i64,
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
    session: Option<Session>,
    open: Option<OpenSegment>,
    run_started_at: i64,
    run_has_segment: bool,
    last_close_at: Option<i64>,
}

impl Writer {
    fn new(pool: ConnectionPool, target: SessionTarget) -> Self {
        Self {
            pool,
            target,
            session: None,
            open: None,
            run_started_at: now_ms(),
            run_has_segment: false,
            last_close_at: None,
        }
    }

    async fn run(mut self, mut rx: UnboundedReceiver<Event>) {
        while let Some(event) = rx.recv().await {
            debug!(?event, "切片工作台分段事件");
            let result = match event {
                Event::RunStarted { at } => self.on_run_started(at).await,
                Event::Opened { path, at } => self.on_opened(path, at).await,
                Event::Closed { path, at, info } => self.on_closed(path, at, info).await,
                Event::Deleted { path } => self.on_deleted(&path).await,
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
        let state = if info.discard {
            SegmentState::Deleted
        } else if exists {
            SegmentState::Finished
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

    async fn on_deleted(&mut self, path: &Path) -> sqlx::Result<()> {
        index::remove(path);
        let Some(session) = &self.session else {
            return Ok(());
        };
        sqlx::query("UPDATE segments SET state = 'deleted' WHERE session_id = ? AND path = ?")
            .bind(session.id)
            .bind(path_string(path))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn on_finish(&mut self) -> sqlx::Result<()> {
        let now = now_ms();
        self.finalize_open(now).await?;
        if let Some(session) = &self.session
            && !store::delete_session_if_empty(&self.pool, session.id).await?
        {
            store::close_session(&self.pool, session.id, self.last_close_at.unwrap_or(now)).await?;
        }
        Ok(())
    }

    fn segment_done(&mut self, end_ms: i64, at: i64) {
        if let Some(session) = self.session.as_mut() {
            session.last_end_ms = session.last_end_ms.max(end_ms);
        }
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
        let session = self.ensure_session(opened_at).await?;
        let (session_id, (start_ms, gap)) = (
            session.id,
            place(session.last_end_ms, opened_at - session.started_at),
        );
        let id = store::insert_segment(
            &self.pool,
            session_id,
            &path_string(path),
            container,
            start_ms,
            gap,
        )
        .await?;
        Ok((id, start_ms))
    }

    async fn ensure_session(&mut self, at: i64) -> sqlx::Result<&Session> {
        if self.session.is_none() {
            let opened = store::open_session(
                &self.pool,
                self.target.streamer_id,
                Some(self.target.streamerinfo_id),
                &self.target.title,
                at,
                self.target.merge_window_ms,
            )
            .await?;
            if opened.resumed {
                tracing::info!(
                    session = opened.id,
                    last_end_ms = opened.last_end_ms,
                    "切片工作台：下播后很快又开播，接着记在上一场"
                );
            }
            self.session = Some(Session {
                id: opened.id,
                started_at: opened.started_at,
                last_end_ms: opened.last_end_ms,
            });
        }
        Ok(self.session.as_ref().unwrap())
    }

    /// 收尾没等到关段事件的分段（下载器出错退出、进程被停止）：按盘上的文件补齐；
    /// 文件没生成或是空的就删掉这一行。
    async fn finalize_open(&mut self, at: i64) -> sqlx::Result<()> {
        let Some(open) = self.open.take() else {
            return Ok(());
        };
        let Some((path, len)) = [open.path.clone(), strip_part(&open.path)]
            .into_iter()
            .find_map(|p| {
                let len = std::fs::metadata(&p).ok()?.len();
                (len > 0).then_some((p, len))
            })
        else {
            index::remove(&open.path);
            return store::delete_segment(&self.pool, open.id).await;
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

/// 新分段在时间轴上的位置：`wall_pos` 为开段墙钟相对场次 0 点的毫秒数。
/// 返回 `(start_ms, gap_before_ms)`。
pub(crate) fn place(last_end_ms: i64, wall_pos: i64) -> (i64, i64) {
    if wall_pos - last_end_ms > GAP_TOLERANCE_MS {
        (wall_pos, wall_pos - last_end_ms)
    } else {
        (last_end_ms, 0)
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
