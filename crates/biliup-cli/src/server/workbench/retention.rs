//! 分段文件的生命周期：被引用就推迟删除、定时清理、磁盘水位兜底。
//!
//! - 删除点（后处理 `rm`、边录边传投稿后删临时文件、过滤删除小文件）都经过 [`remove`]：分段被引用
//!   （`pin_count > 0`）、所属场次在 `retain_until` 之前，或调用方要求再保留一段时间（「投稿后保留
//!   录像」）时，不立即删，把分段标成 `pending_delete`，文件原样留在盘上；
//! - [`spawn_sweeper`] 每分钟删掉到期且没人引用的 `pending_delete` 分段，并在录像所在磁盘的可用空间
//!   低于 `min_free_space` 时，按「没被引用的最旧 → 被引用的最旧」删已录完的分段；
//! - [`pin`] / [`unpin`]：给标记、切片等按场次时间区间登记引用，换算成分段的 `pin_count`；
//! - [`moved`]：后处理 `mv` 之后，关键帧索引跟着搬，分段的路径跟着改。
//!
//! 不在 `segments` 里的文件（工作台不支持的容器、升级前录的旧文件）一律按原来的方式立即删，
//! 清理任务和水位兜底也只处理有 `segments` 记录的文件。

use super::index;
use super::recorder::now_ms;
use crate::server::config::Config;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use sqlx::{Executor, Sqlite, SqliteConnection};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// 清理任务的间隔。
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

const HOUR_MS: i64 = 3_600_000;

/// 删除点要用到的上下文：数据库，以及删之前还要保留多久。
#[derive(Debug, Clone)]
pub struct Retention {
    pub pool: ConnectionPool,
    /// 大于 0 时分段一律先标 `pending_delete`，到期后由清理任务删除（「投稿后保留录像」）。
    pub keep_for_ms: i64,
}

impl Retention {
    /// 过滤删除之类不算「投稿后」的删除点：只看引用与场次保留期。
    pub fn without_delay(pool: ConnectionPool) -> Self {
        Self {
            pool,
            keep_for_ms: 0,
        }
    }

    /// 按配置里的 `retention_hours` 保留。
    pub fn after_upload(pool: ConnectionPool, config: &Config) -> Self {
        Self {
            pool,
            keep_for_ms: hours_to_ms(config.retention_hours),
        }
    }
}

fn hours_to_ms(hours: u64) -> i64 {
    i64::try_from(hours)
        .unwrap_or(i64::MAX)
        .saturating_mul(HOUR_MS)
}

/// [`remove`] 对每个路径的处理结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposal {
    /// 分段文件已删，行标 `deleted`。
    Deleted,
    /// 留在盘上等清理任务：分段行标 `pending_delete`，或是这样一个分段的弹幕文件。
    Deferred,
    /// 不是工作台记录的分段，按原来的方式删了。
    Untracked,
}

#[derive(Debug, sqlx::FromRow)]
struct Tracked {
    id: i64,
    danmaku_path: Option<String>,
    pin_count: i64,
    retain_until: Option<i64>,
}

async fn tracked(pool: &ConnectionPool, path: &Path) -> sqlx::Result<Option<Tracked>> {
    sqlx::query_as(
        "SELECT g.id, g.danmaku_path, g.pin_count, s.retain_until
         FROM segments g JOIN stream_sessions s ON s.id = g.session_id
         WHERE g.path = ? AND g.state IN ('recording', 'finished', 'missing', 'pending_delete')
         ORDER BY g.id DESC LIMIT 1",
    )
    .bind(path_string(path))
    .fetch_optional(pool)
    .await
}

/// 删除点：删掉 `paths`（视频与弹幕文件混在一起，与后处理拿到的列表相同）。
///
/// 工作台记录的分段在被引用、场次保留期内或 `keep_for_ms > 0` 时推迟删除，同一批里它的弹幕文件
/// 跟着留下；其余的立即删，分段行标 `deleted`，关键帧索引一起删。数据库出错时按原来的方式立即删。
/// 删文件出错时返回错误，与原来的 `rm` 一样停在出错的那个文件。
pub async fn remove(retention: &Retention, paths: &[&Path]) -> io::Result<Vec<Disposal>> {
    let now = now_ms();
    let mut outcome: Vec<Option<Disposal>> = vec![None; paths.len()];
    let mut kept_danmaku: Vec<PathBuf> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let segment = match tracked(&retention.pool, path).await {
            Ok(segment) => segment,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "查询分段记录失败，按原方式直接删除");
                None
            }
        };
        let Some(segment) = segment else {
            continue;
        };
        let referenced = segment.pin_count > 0 || segment.retain_until.is_some_and(|t| t > now);
        if referenced || retention.keep_for_ms > 0 {
            let danmaku = danmaku_in_batch(path, segment.danmaku_path.as_deref(), paths);
            match defer(
                &retention.pool,
                segment.id,
                retention.keep_for_ms,
                now,
                danmaku.as_deref(),
            )
            .await
            {
                Ok(()) => {
                    info!(
                        path = %path.display(),
                        referenced,
                        keep_hours = retention.keep_for_ms / HOUR_MS,
                        "分段被引用或在保留期内，暂不删除，到期后由清理任务删除"
                    );
                    outcome[i] = Some(Disposal::Deferred);
                    kept_danmaku.extend(danmaku);
                    continue;
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "标记推迟删除失败，按原方式直接删除")
                }
            }
        }
        delete_video(path).await?;
        if let Err(e) = set_deleted(&retention.pool, segment.id).await {
            warn!(path = %path.display(), error = %e, "分段已删除，但更新分段状态失败");
        }
        outcome[i] = Some(Disposal::Deleted);
    }
    for (i, path) in paths.iter().enumerate() {
        if outcome[i].is_some() {
            continue;
        }
        if kept_danmaku.iter().any(|kept| kept == path) {
            outcome[i] = Some(Disposal::Deferred);
            continue;
        }
        delete_video(path).await?;
        outcome[i] = Some(Disposal::Untracked);
    }
    Ok(outcome.into_iter().flatten().collect())
}

/// 同一批里属于这个分段的弹幕文件：分段记下的弹幕路径，或同名 `.xml`。
fn danmaku_in_batch(video: &Path, recorded: Option<&str>, paths: &[&Path]) -> Option<PathBuf> {
    let sibling = video.with_extension("xml");
    paths
        .iter()
        .find(|p| **p != video && (recorded.is_some_and(|r| Path::new(r) == **p) || **p == sibling))
        .map(|p| p.to_path_buf())
}

async fn defer(
    pool: &ConnectionPool,
    id: i64,
    keep_for_ms: i64,
    now: i64,
    danmaku: Option<&Path>,
) -> sqlx::Result<()> {
    let delete_after = (keep_for_ms > 0).then(|| now.saturating_add(keep_for_ms));
    sqlx::query(
        "UPDATE segments SET state = 'pending_delete',
             delete_after = CASE WHEN ?1 IS NULL THEN delete_after
                                 ELSE MAX(COALESCE(delete_after, 0), ?1) END,
             danmaku_path = COALESCE(danmaku_path, ?2)
         WHERE id = ?3",
    )
    .bind(delete_after)
    .bind(danmaku.map(path_string))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn set_deleted(pool: &ConnectionPool, id: i64) -> sqlx::Result<()> {
    set_state(pool, id, "deleted").await
}

async fn set_state(pool: &ConnectionPool, id: i64, state: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE segments SET state = ? WHERE id = ?")
        .bind(state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 删视频（或弹幕）文件并删掉它的关键帧索引缓存。
async fn delete_video(path: &Path) -> io::Result<()> {
    info!("删除 - Removing: {}", path.display());
    tokio::fs::remove_file(path).await?;
    remove_index(path);
    Ok(())
}

/// 删 `video` 的关键帧索引缓存。索引任务还在边写边建时不删：它关段时还会再存一次，删了会留下
/// 孤儿 `.idx`；这种分段由录制器在索引任务处理完之后删（过滤删除的分段关段时带 `discard`）。
fn remove_index(video: &Path) {
    if !index::live::is_live(video) {
        let _ = std::fs::remove_file(index::index_path(video));
    }
}

/// 后处理 `mv` 把 `from` 搬到了 `to`：关键帧索引跟着搬（搬不过去就删掉，用到时重建）；
/// 给了数据库时，分段的 `path` / `index_path`，或记着这个弹幕文件的分段的 `danmaku_path` 跟着改。
pub async fn moved(pool: Option<&ConnectionPool>, from: &Path, to: &Path) {
    let from_index = index::index_path(from);
    let to_index = index::index_path(to);
    // 后处理只在录制任务结束后执行，这时不该还有索引任务在写；万一有，不去抢它的 `.idx`
    let live = index::live::is_live(from);
    let index_moved = match tokio::fs::metadata(&from_index).await {
        Ok(_) if live => false,
        Ok(_) => match move_file(&from_index, &to_index).await {
            Ok(()) => true,
            Err(e) => {
                debug!(error = %e, "关键帧索引没能跟着搬，删掉，用到时重建");
                let _ = tokio::fs::remove_file(&from_index).await;
                false
            }
        },
        Err(_) => false,
    };
    let Some(pool) = pool else {
        return;
    };
    if let Err(e) = update_moved(pool, from, to, index_moved.then_some(&to_index)).await {
        warn!(from = %from.display(), to = %to.display(), error = %e, "文件已移动，但更新分段路径失败");
    }
}

async fn update_moved(
    pool: &ConnectionPool,
    from: &Path,
    to: &Path,
    index: Option<&PathBuf>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE segments SET path = ?, index_path = ? WHERE path = ? AND state != 'deleted'",
    )
    .bind(path_string(to))
    .bind(index.map(|p| path_string(p)))
    .bind(path_string(from))
    .execute(pool)
    .await?;
    sqlx::query("UPDATE segments SET danmaku_path = ? WHERE danmaku_path = ?")
        .bind(path_string(to))
        .bind(path_string(from))
        .execute(pool)
        .await?;
    Ok(())
}

async fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match tokio::fs::rename(from, to).await {
        Ok(()) => Ok(()),
        Err(_) => {
            tokio::fs::copy(from, to).await?;
            tokio::fs::remove_file(from).await
        }
    }
}

// ===== 引用 =====

/// 以 `owner`（如 `marker:12`、`clip:5`）的名义引用场次时间 `[from_ms, to_ms]`。
///
/// 同一个 `owner` 再次调用是改区间，不会重复计数；区间可以伸进仍在录的尾部，之后写出来的分段同样
/// 算被引用。与区间重叠的分段 `pin_count` 加一，被引用的分段在删除点推迟删除，水位兜底时也排在最后。
/// 可以传事务（`&mut *tx`），与调用方自己的写入一起提交。
pub async fn pin(
    conn: &mut SqliteConnection,
    owner: &str,
    session_id: i64,
    from_ms: i64,
    to_ms: i64,
) -> sqlx::Result<()> {
    let (from_ms, to_ms) = (from_ms.min(to_ms), from_ms.max(to_ms));
    let previous: Option<i64> =
        sqlx::query_scalar("SELECT session_id FROM segment_pins WHERE owner = ?")
            .bind(owner)
            .fetch_optional(&mut *conn)
            .await?;
    sqlx::query(
        "INSERT INTO segment_pins (owner, session_id, from_ms, to_ms) VALUES (?, ?, ?, ?)
         ON CONFLICT (owner) DO UPDATE SET
             session_id = excluded.session_id, from_ms = excluded.from_ms, to_ms = excluded.to_ms",
    )
    .bind(owner)
    .bind(session_id)
    .bind(from_ms)
    .bind(to_ms)
    .execute(&mut *conn)
    .await?;
    if let Some(previous) = previous.filter(|p| *p != session_id) {
        refresh_pin_counts(&mut *conn, previous).await?;
    }
    refresh_pin_counts(&mut *conn, session_id).await
}

/// 撤销 `owner` 的引用，返回之前是否有。引用全部释放的 `pending_delete` 分段由下一轮清理任务删除。
pub async fn unpin(conn: &mut SqliteConnection, owner: &str) -> sqlx::Result<bool> {
    let session: Option<i64> =
        sqlx::query_scalar("DELETE FROM segment_pins WHERE owner = ? RETURNING session_id")
            .bind(owner)
            .fetch_optional(&mut *conn)
            .await?;
    match session {
        Some(session_id) => {
            refresh_pin_counts(&mut *conn, session_id).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// 按 `segment_pins` 重算场次里每个分段的 `pin_count`。仍在录的分段（`end_ms` 为空）算到无穷远。
pub(crate) async fn refresh_pin_counts<'e, E>(executor: E, session_id: i64) -> sqlx::Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "UPDATE segments SET pin_count = (
             SELECT COUNT(*) FROM segment_pins p
             WHERE p.session_id = segments.session_id
               AND p.to_ms >= segments.start_ms
               AND (segments.end_ms IS NULL OR p.from_ms < segments.end_ms))
         WHERE session_id = ?",
    )
    .bind(session_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// 「保留这场」：`retain_until`（Unix 毫秒）之前，这一场的分段在删除点一律推迟删除；`None` 取消。
/// 场次不存在返回 `false`。
pub async fn set_session_retention(
    pool: &ConnectionPool,
    session_id: i64,
    retain_until: Option<i64>,
) -> sqlx::Result<bool> {
    let done = sqlx::query("UPDATE stream_sessions SET retain_until = ? WHERE id = ?")
        .bind(retain_until)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() > 0)
}

// ===== 清理任务 =====

#[derive(Debug, sqlx::FromRow)]
struct Doomed {
    id: i64,
    path: String,
    index_path: Option<String>,
    danmaku_path: Option<String>,
}

/// 删一个分段的文件：视频、关键帧索引、弹幕。视频本来就不在了也算删掉；其它错误（Windows 上
/// 文件正被读取等）返回错误，留到下一轮。
fn delete_segment_files(segment: &Doomed) -> io::Result<()> {
    let video = Path::new(&segment.path);
    match std::fs::remove_file(video) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    remove_index(video);
    if let Some(index_path) = &segment.index_path
        && Path::new(index_path) != index::index_path(video)
    {
        let _ = std::fs::remove_file(index_path);
    }
    if let Some(danmaku) = &segment.danmaku_path {
        let _ = std::fs::remove_file(danmaku);
    }
    Ok(())
}

/// 删掉到期（`delete_after` 已过）、没有引用、场次也不在保留期内的 `pending_delete` 分段，
/// 返回删了几个。
pub async fn sweep_pending(pool: &ConnectionPool, now: i64) -> sqlx::Result<usize> {
    let due: Vec<Doomed> = sqlx::query_as(
        "SELECT g.id, g.path, g.index_path, g.danmaku_path
         FROM segments g JOIN stream_sessions s ON s.id = g.session_id
         WHERE g.state = 'pending_delete' AND g.pin_count = 0
           AND (g.delete_after IS NULL OR g.delete_after <= ?1)
           AND (s.retain_until IS NULL OR s.retain_until <= ?1)
         ORDER BY g.id",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
    let mut deleted = 0;
    for segment in &due {
        if index::live::is_live(Path::new(&segment.path)) {
            continue;
        }
        match delete_segment_files(segment) {
            Ok(()) => {
                set_deleted(pool, segment.id).await?;
                info!(path = segment.path, "保留期已过且没有引用，已删除分段");
                deleted += 1;
            }
            Err(e) => warn!(path = segment.path, error = %e, "删除到期分段失败，下一轮再试"),
        }
    }
    Ok(deleted)
}

#[derive(Debug, sqlx::FromRow)]
struct Candidate {
    id: i64,
    path: String,
    index_path: Option<String>,
    danmaku_path: Option<String>,
    referenced: bool,
}

/// 磁盘水位兜底：分段所在磁盘的可用空间低于 `min_free` 字节时，按「没被引用的最旧 → 被引用的
/// 最旧」删已录完（`finished` / `pending_delete`）的分段，直到可用空间回到阈值以上。正在录的分段、
/// 索引任务还没处理完的分段不删；不同磁盘各自判断。`available` 返回某个目录所在磁盘的可用字节数。返回删了几个。
pub async fn enforce_free_space<F>(
    pool: &ConnectionPool,
    min_free: u64,
    now: i64,
    mut available: F,
) -> sqlx::Result<usize>
where
    F: FnMut(&Path) -> io::Result<u64>,
{
    let candidates: Vec<Candidate> = sqlx::query_as(
        "SELECT g.id, g.path, g.index_path, g.danmaku_path,
                (g.pin_count > 0 OR COALESCE(s.retain_until, 0) > ?) AS referenced
         FROM segments g JOIN stream_sessions s ON s.id = g.session_id
         WHERE g.state IN ('finished', 'pending_delete')
         ORDER BY referenced, COALESCE(s.started_at, 0) + g.start_ms, g.id",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
    // 同一轮里只有删了文件才会多出空间，其间按目录缓存查询结果
    let mut free: HashMap<PathBuf, u64> = HashMap::new();
    let mut deleted = 0;
    for candidate in &candidates {
        let video = Path::new(&candidate.path);
        if index::live::is_live(video) {
            continue;
        }
        let dir = match video.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let space = match free.get(&dir) {
            Some(space) => *space,
            None => match available(&dir) {
                Ok(space) => *free.entry(dir.clone()).or_insert(space),
                Err(e) => {
                    debug!(dir = %dir.display(), error = %e, "查询磁盘可用空间失败，跳过");
                    continue;
                }
            },
        };
        if space >= min_free {
            continue;
        }
        if std::fs::metadata(video).is_err() {
            set_state(pool, candidate.id, "missing").await?;
            continue;
        }
        let doomed = Doomed {
            id: candidate.id,
            path: candidate.path.clone(),
            index_path: candidate.index_path.clone(),
            danmaku_path: candidate.danmaku_path.clone(),
        };
        match delete_segment_files(&doomed) {
            Ok(()) => {
                set_deleted(pool, candidate.id).await?;
                warn!(
                    path = candidate.path,
                    referenced = candidate.referenced,
                    available_mib = space / 1024 / 1024,
                    min_free_mib = min_free / 1024 / 1024,
                    "磁盘可用空间低于阈值，已删除最旧的分段"
                );
                deleted += 1;
                free.clear();
            }
            Err(e) => warn!(path = candidate.path, error = %e, "磁盘水位兜底删除分段失败"),
        }
    }
    Ok(deleted)
}

/// 目录所在磁盘对当前用户可用的字节数。
#[cfg(unix)]
pub fn available_space(dir: &Path) -> io::Result<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: c_path 是以 NUL 结尾的路径，stat 是足够大的输出缓冲
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    #[allow(clippy::unnecessary_cast)]
    Ok((stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
}

/// 目录所在磁盘对当前用户可用的字节数。
#[cfg(windows)]
pub fn available_space(dir: &Path) -> io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            directory: *const u16,
            free_to_caller: *mut u64,
            total: *mut u64,
            total_free: *mut u64,
        ) -> i32;
    }
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free_to_caller = 0u64;
    // SAFETY: wide 以 NUL 结尾；不需要的输出参数传空指针是 API 允许的
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_to_caller,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(free_to_caller)
}

#[cfg(not(any(unix, windows)))]
pub fn available_space(_dir: &Path) -> io::Result<u64> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// 跑一轮：先删到期的 `pending_delete`，再按 `min_free_space` 做水位兜底。
pub async fn sweep_once(pool: &ConnectionPool, min_free_space: Option<u64>) -> sqlx::Result<()> {
    let now = now_ms();
    sweep_pending(pool, now).await?;
    if let Some(min_free) = min_free_space.filter(|v| *v > 0) {
        enforce_free_space(pool, min_free, now, available_space).await?;
    }
    Ok(())
}

/// 启动每分钟一次的清理任务。返回的句柄 drop 时任务停止。
pub fn spawn_sweeper(pool: ConnectionPool, config: Arc<RwLock<Config>>) -> SweeperGuard {
    let task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let min_free_space = config.read().unwrap().min_free_space;
            if let Err(e) = sweep_once(&pool, min_free_space).await {
                warn!(error = %e, "分段清理任务出错，下一分钟再试");
            }
        }
    });
    SweeperGuard(task)
}

/// [`spawn_sweeper`] 的句柄，drop 时停掉清理任务。
pub struct SweeperGuard(tokio::task::JoinHandle<()>);

impl Drop for SweeperGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
