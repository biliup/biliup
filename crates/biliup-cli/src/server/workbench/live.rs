//! 正在录的场次：DVR 回看跟随正在写的分段时，靠这里拿到两种通知，不轮询。
//!
//! - 分段表变了（开段、关段、分段被删、录制任务结束）：场次记录器写完库之后 [`LiveGuard::changed`]；
//! - 又写了新数据：下载任务的写盘计数器（[`ByteCounter`]）累加时唤醒订阅者。
//!
//! 下载器写盘计数的位置在数据进入写盘缓冲之前（mesio 甚至在修复管线之前），所以被唤醒时新数据
//! 不一定已经落盘；读取方读不到新内容就接着等下一次唤醒，最后一点缓冲在关段时由分段表变更唤醒。
//! 边录边传、yt-dlp 没有写盘计数，场次照样登记（`/v1/streamers` 要用场次 id），但读取方拿不到
//! 字节通知，只回看到最后一个写完的分段。

use biliup::downloader::util::{ByteCounter, ByteWatch};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::watch;

struct Entry {
    streamer_id: i64,
    bytes: Option<ByteCounter>,
    changes: watch::Sender<u64>,
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
}
