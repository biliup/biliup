//! 正在录的场次：DVR 回看跟随正在写的分段时，靠这里拿到两种通知，不轮询。
//!
//! - 分段表变了（开段、关段、分段被删、录制任务结束）：场次记录器写完库之后 [`LiveGuard::changed`]；
//! - 又写了新数据：下载任务的写盘计数器（[`ByteCounter`]）累加时唤醒订阅者。
//!
//! 下载器写盘计数的位置在数据进入写盘缓冲之前（mesio 甚至在修复管线之前），所以被唤醒时新数据
//! 不一定已经落盘；读取方读不到新内容就接着等下一次唤醒，最后一点缓冲在关段时由分段表变更唤醒。
//! 边录边传、yt-dlp 没有写盘计数，场次照样登记（`/v1/streamers` 要用场次 id），但读取方拿不到
//! 字节通知，只回看到最后一个写完的分段。
//!
//! 打标记时也靠这里把墙钟换算成场次时间。场次时间轴段内按容器时间走、段与段之间按墙钟接续（见
//! [`super::recorder`]），和墙钟之间没有固定的换算，要从观测里得到「现在收到的内容在场次时间
//! 轴上的哪里」：
//!
//! - 场次记录器每次开段、关段记下一对「场次时间 ↔ 墙钟」（[`LiveGuard::anchor`]）；
//! - 录制期间每秒看一眼正在写的分段（[`LiveGuard::sample_segment`]）：边写边建索引的分段取索引任务扫到的
//!   时长和扫到那里的墙钟（[`index::live::written`]），其余扫盘，已写内容的时长对应最后一次写盘的墙钟。
//!
//! 每个观测都是下界——内容总是先收到、后写盘。差多少看下载器：刚连上时 CDN 先补发一个 GOP 的缓存，
//! 第一个分段的场次 0 早于开写的墙钟；mesio 的 FLV 修复管线攒满一个 GOP 才往下写，写盘又经过
//! 1 MiB 的缓冲（低码率的流十来秒才落一次盘），而中转预览在管线之前就拿到了数据。所以换算取最近
//! 一段时间里最靠前的观测（[`written_anchor`]）；最近一直没有观测（断流、卡住）时才从最后的开段 /
//! 关段锚点外推（[`anchor`]）。

use super::index::{self, KeyframeIndex};
use biliup::downloader::util::{ByteCounter, ByteWatch};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// 多久看一次正在写的分段。
const WATCH_EVERY: Duration = Duration::from_secs(1);
/// 盘上观测保留多久；取这段时间里最靠前的一个。窗口越长越贴近真实位置，但断流后内容比墙钟
/// 慢下来时，要过这么久才跟上。
pub const WRITTEN_WINDOW_MS: i64 = 30_000;

/// 场次时间轴上的 `session_ms` 对应墙钟 `wall_ms`（Unix 毫秒）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub session_ms: i64,
    pub wall_ms: i64,
}

impl Anchor {
    /// 墙钟 `wall_ms` 对应的场次时间（不早于 0）。
    pub fn session_ms_at(&self, wall_ms: i64) -> i64 {
        (self.session_ms + (wall_ms - self.wall_ms)).max(0)
    }
}

struct Entry {
    streamer_id: i64,
    bytes: Option<ByteCounter>,
    changes: watch::Sender<u64>,
    anchor: Option<Anchor>,
    /// 最近的观测 `(墙钟, 场次时间 − 墙钟)`：盘上观测和开段 / 关段锚点。
    written: VecDeque<(i64, i64)>,
    /// 同一场被合并进来的新任务会重新登记；旧任务的 guard 晚 drop 时不能把新登记删掉。
    generation: u64,
}

impl Entry {
    fn observe(&mut self, wall_ms: i64, session_ms: i64) {
        self.written.push_back((wall_ms, session_ms - wall_ms));
        let newest = self
            .written
            .iter()
            .map(|&(w, _)| w)
            .max()
            .unwrap_or(wall_ms);
        self.written
            .retain(|&(w, _)| newest - w <= WRITTEN_WINDOW_MS);
    }
}

static LIVE: LazyLock<Mutex<(u64, HashMap<i64, Entry>)>> =
    LazyLock::new(|| Mutex::new((0, HashMap::new())));

/// 录制任务持有：drop 时注销场次并唤醒所有读取方。
pub struct LiveGuard {
    session_id: i64,
    generation: u64,
    watcher: Option<JoinHandle<()>>,
}

fn with_entry(session_id: i64, generation: u64, f: impl FnOnce(&mut Entry)) {
    let mut live = LIVE.lock().unwrap();
    if let Some(entry) = live.1.get_mut(&session_id)
        && entry.generation == generation
    {
        f(entry);
    }
}

impl LiveGuard {
    /// 分段表已经改完（事务已提交），叫醒读取方重新查库。
    pub fn changed(&self) {
        let live = LIVE.lock().unwrap();
        if let Some(entry) = live.1.get(&self.session_id)
            && entry.generation == self.generation
        {
            entry.changes.send_modify(|v| *v += 1);
        }
    }

    /// 记下场次时间 `session_ms` 对应的墙钟（开段、关段时）。
    pub fn anchor(&self, session_ms: i64, wall_ms: i64) {
        with_entry(self.session_id, self.generation, |entry| {
            entry.anchor = Some(Anchor {
                session_ms,
                wall_ms,
            });
            entry.observe(wall_ms, session_ms);
        });
    }

    /// 开始每秒观测正在写的分段（从场次时间 `start_ms` 起）；换段时替换上一个。
    pub fn sample_segment(&mut self, path: PathBuf, start_ms: i64) {
        self.stop_sampling();
        let (session_id, generation) = (self.session_id, self.generation);
        self.watcher = Some(tokio::spawn(async move {
            let mut ticks = tokio::time::interval(WATCH_EVERY);
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut seen_len = None;
            let mut seen_tapped = None;
            let mut index = None;
            loop {
                ticks.tick().await;
                let written = if index::live::is_live(&path) {
                    let tapped = index::live::written(&path);
                    if tapped.is_none() || tapped == seen_tapped {
                        continue;
                    }
                    seen_tapped = tapped;
                    tapped
                } else {
                    let Ok(len) = tokio::fs::metadata(&path).await.map(|m| m.len()) else {
                        continue;
                    };
                    if seen_len == Some(len) {
                        continue;
                    }
                    seen_len = Some(len);
                    let file = path.clone();
                    let Ok((scanned, written)) =
                        tokio::task::spawn_blocking(move || written_upto(&file, index)).await
                    else {
                        return;
                    };
                    index = scanned;
                    written
                };
                if let Some((written_ms, wall_ms)) = written {
                    with_entry(session_id, generation, |entry| {
                        entry.observe(wall_ms, start_ms + i64::from(written_ms));
                    });
                }
            }
        }));
    }

    /// 分段写完了，停止观测。
    pub fn stop_sampling(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            watcher.abort();
        }
    }
}

/// 分段已写内容的时长（毫秒，按关键帧索引续扫）与写到那里时的墙钟；连同续扫到的索引一起返回，
/// 下次从这里接着扫。
///
/// 扫描前后文件长度一致时，墙钟取文件的修改时间（最后一次写盘）；扫描期间又写了就重来，
/// 一直在写的下载器取扫完的时刻。
fn written_upto(
    path: &Path,
    mut index: Option<KeyframeIndex>,
) -> (Option<KeyframeIndex>, Option<(u32, i64)>) {
    let mut latest = None;
    for _ in 0..3 {
        let Ok(before) = std::fs::metadata(path) else {
            break;
        };
        let Ok(scanned) = index::rescan(path, index.take()) else {
            break;
        };
        let (len, written_ms, keyed) = (
            scanned.source_len,
            scanned.duration_ms,
            scanned.base_ts.is_some(),
        );
        index = Some(scanned);
        if !keyed {
            break;
        }
        if len == before.len() {
            let modified = before.modified().ok();
            let wall = modified.and_then(|m| m.duration_since(UNIX_EPOCH).ok());
            return (index, wall.map(|w| (written_ms, w.as_millis() as i64)));
        }
        latest = Some((written_ms, super::recorder::now_ms()));
    }
    (index, latest)
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.stop_sampling();
        let mut live = LIVE.lock().unwrap();
        if live
            .1
            .get(&self.session_id)
            .is_some_and(|e| e.generation == self.generation)
        {
            // 发送端随条目一起 drop，读取方的 `changed()` 返回错误，据此知道录制已结束
            live.1.remove(&self.session_id);
        }
    }
}

/// 场次开始（或断流合并后重新开始）录制时登记。
pub fn register(session_id: i64, streamer_id: i64, bytes: Option<ByteCounter>) -> LiveGuard {
    let mut live = LIVE.lock().unwrap();
    live.0 += 1;
    let generation = live.0;
    let changes = match live.1.remove(&session_id) {
        Some(previous) => previous.changes,
        None => watch::channel(0).0,
    };
    changes.send_modify(|v| *v += 1);
    live.1.insert(
        session_id,
        Entry {
            streamer_id,
            bytes,
            changes,
            anchor: None,
            written: VecDeque::new(),
            generation,
        },
    );
    LiveGuard {
        session_id,
        generation,
        watcher: None,
    }
}

/// 主播当前正在录的场次。
pub fn session_of_streamer(streamer_id: i64) -> Option<i64> {
    let live = LIVE.lock().unwrap();
    live.1
        .iter()
        .filter(|(_, e)| e.streamer_id == streamer_id)
        .map(|(id, _)| *id)
        .max()
}

pub fn is_recording(session_id: i64) -> bool {
    LIVE.lock().unwrap().1.contains_key(&session_id)
}

/// 正在录的场次最近的开段 / 关段锚点；没在录，或这次录制还没开出分段时为 `None`。
pub fn anchor(session_id: i64) -> Option<Anchor> {
    LIVE.lock().unwrap().1.get(&session_id)?.anchor
}

/// 最近 [`WRITTEN_WINDOW_MS`] 里最靠前的观测，换成墙钟 `now_ms` 处的锚点；没有这样的观测时为 `None`。
pub fn written_anchor(session_id: i64, now_ms: i64) -> Option<Anchor> {
    let live = LIVE.lock().unwrap();
    let lead = live
        .1
        .get(&session_id)?
        .written
        .iter()
        .filter(|&&(w, _)| w <= now_ms + 1_000 && now_ms - w <= WRITTEN_WINDOW_MS)
        .map(|&(_, lead)| lead)
        .max()?;
    Some(Anchor {
        session_ms: now_ms + lead,
        wall_ms: now_ms,
    })
}

/// 读取方的订阅。
pub struct LiveWatch {
    pub changes: watch::Receiver<u64>,
    /// `None`：这一路没有写盘计数，正在写的分段不能跟随。
    pub bytes: Option<ByteWatch>,
}

/// 场次正在录时订阅它的变更与写入通知；没在录返回 `None`。
pub fn watch(session_id: i64) -> Option<LiveWatch> {
    let live = LIVE.lock().unwrap();
    live.1.get(&session_id).map(|entry| LiveWatch {
        changes: entry.changes.subscribe(),
        bytes: entry.bytes.as_ref().map(ByteCounter::watch),
    })
}

/// 登记表是进程级的，而每个测试库的场次 id 都从 1 开始：并行的测试把场次改成进程内唯一的 id，
/// 免得互相顶掉登记。
#[cfg(test)]
pub(crate) async fn unique_session_id(
    pool: &crate::server::infrastructure::connection_pool::ConnectionPool,
    session_id: i64,
) -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static NEXT: AtomicI64 = AtomicI64::new(1_000_000);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    sqlx::query("UPDATE stream_sessions SET id = ?1 WHERE id = ?2")
        .bind(id)
        .bind(session_id)
        .execute(pool)
        .await
        .unwrap();
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn merged_task_keeps_the_newer_registration() {
        let old = register(9_000_001, 7, None);
        let mut watch = watch(9_000_001).unwrap();
        let new = register(9_000_001, 7, Some(ByteCounter::new()));
        assert!(watch.changes.has_changed().unwrap());
        watch.changes.mark_unchanged();
        drop(old);
        assert!(
            is_recording(9_000_001),
            "旧任务晚 drop 不能注销新任务的登记"
        );
        assert_eq!(session_of_streamer(7), Some(9_000_001));
        new.changed();
        assert!(watch.changes.has_changed().unwrap());
        watch.changes.mark_unchanged();
        drop(new);
        assert!(!is_recording(9_000_001));
        assert!(watch.changes.changed().await.is_err());
    }

    #[test]
    fn merged_task_keeps_its_own_anchor() {
        let old = register(9_100_001, 7, None);
        old.anchor(1_000, 50_000);
        let new = register(9_100_001, 7, None);
        assert_eq!(anchor(9_100_001), None, "新登记从没有锚点开始");
        old.anchor(2_000, 60_000);
        assert_eq!(anchor(9_100_001), None, "旧任务不能改新登记的锚点");
        drop(old);
        new.anchor(3_000, 70_000);
        assert_eq!(
            anchor(9_100_001).map(|a| a.session_ms_at(71_500)),
            Some(4_500)
        );
        drop(new);
        assert_eq!(anchor(9_100_001), None);
    }

    #[test]
    fn written_anchor_takes_the_most_advanced_recent_observation() {
        let guard = register(9_100_002, 8, None);
        assert_eq!(written_anchor(9_100_002, 100_000), None);
        with_entry(9_100_002, guard.generation, |e| {
            // 写盘缓冲让观测参差不齐：同一时刻的真实位置是 场次 = 墙钟 − 40_000
            e.observe(100_000, 58_500);
            e.observe(102_000, 62_000);
            e.observe(104_000, 62_900);
        });
        assert_eq!(
            written_anchor(9_100_002, 105_000).map(|a| a.session_ms_at(105_000)),
            Some(65_000),
            "取最靠前的观测"
        );
        // 关段时写缓冲整个落盘，关段锚点往往是最靠前的一个
        guard.anchor(66_000, 105_000);
        assert_eq!(
            written_anchor(9_100_002, 105_000).map(|a| a.session_ms_at(105_000)),
            Some(66_000)
        );
        assert_eq!(
            written_anchor(9_100_002, 134_500).map(|a| a.session_ms_at(134_500)),
            Some(134_500 - 39_000),
            "窗口外的观测不算"
        );
        assert_eq!(written_anchor(9_100_002, 140_000), None, "太久没写盘");
        drop(guard);
        assert_eq!(written_anchor(9_100_002, 105_000), None);
    }

    #[test]
    fn session_time_never_goes_negative() {
        let a = Anchor {
            session_ms: 500,
            wall_ms: 10_000,
        };
        assert_eq!(a.session_ms_at(9_000), 0);
        assert_eq!(a.session_ms_at(10_200), 700);
    }
}
