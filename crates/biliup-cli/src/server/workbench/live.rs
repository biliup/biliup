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
//! [`super::recorder`]），和墙钟之间没有固定的换算。场次记录器每次开段、关段都记下一对「场次
//! 时间 ↔ 墙钟」（[`Anchor`]），换算时从最近的一对按墙钟外推。锚点比收到内容的时刻晚：刚连上时
//! CDN 先发的 GOP 缓存让第一个分段的段首内容早于开段墙钟（到关段时由关段锚点消掉）；mesio 的
//! FLV 修复管线攒满一个 GOP 才写盘，开段、关段都晚一个 GOP。打标记优先按盘上写到的位置换算，
//! 这里只作后备（见 [`super::markers`]）。

use biliup::downloader::util::{ByteCounter, ByteWatch};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::watch;

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
    /// 同一场被合并进来的新任务会重新登记；旧任务的 guard 晚 drop 时不能把新登记删掉。
    generation: u64,
}

static LIVE: LazyLock<Mutex<(u64, HashMap<i64, Entry>)>> =
    LazyLock::new(|| Mutex::new((0, HashMap::new())));

/// 录制任务持有：drop 时注销场次并唤醒所有读取方。
pub struct LiveGuard {
    session_id: i64,
    generation: u64,
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
        let mut live = LIVE.lock().unwrap();
        if let Some(entry) = live.1.get_mut(&self.session_id)
            && entry.generation == self.generation
        {
            entry.anchor = Some(Anchor {
                session_ms,
                wall_ms,
            });
        }
    }
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
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
            generation,
        },
    );
    LiveGuard {
        session_id,
        generation,
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
    fn session_time_never_goes_negative() {
        let a = Anchor {
            session_ms: 500,
            wall_ms: 10_000,
        };
        assert_eq!(a.session_ms_at(9_000), 0);
        assert_eq!(a.session_ms_at(10_200), 700);
    }
}
