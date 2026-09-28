//! `data/pair-outbox.json`：配对两台各自的同步账本与待发队列。
//!
//! 只在配对生效时才建，解除配对时删掉；不配对的机器上没有这个文件。队列里只记键与序号，
//! 内容在发的那一刻按本机现状取（所以凭据原文从不落进这个文件）。同一个键排了几次只留最后一次；
//! 对端应答 `upto` 之后，序号不大于它的都出队。重连后整个队列重发一遍，对端按版本认出重放，不会重复生效。
//! 要加入配对的本地行（[`super::adopt`]）也记在这里，重启后接着加入。

use super::adopt::Adoption;
use super::sync::Book;
use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub const FILE_NAME: &str = "pair-outbox.json";
const FILE_VERSION: u32 = 1;

/// 与 `node.json` / `fleet.sqlite3` 放在同一个目录
pub fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queued {
    pub seq: u64,
    pub key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairFile {
    pub version: u32,
    /// 配对的对端：节点上是控制面的 EndpointId，控制面上是 `node:<节点 id>`。对不上时整份作废
    pub peer: String,
    /// 最后用掉的序号
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub book: Book,
    #[serde(default)]
    pub queue: Vec<Queued>,
    #[serde(default)]
    pub adoption: Adoption,
}

impl PairFile {
    pub fn new(peer: &str) -> Self {
        PairFile {
            version: FILE_VERSION,
            peer: peer.to_string(),
            ..PairFile::default()
        }
    }

    /// 读上次留下的；没有、坏了、版本不认识或对端换了时返回 `None`
    pub fn load(path: &Path, peer: &str) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str::<PairFile>(&text) {
            Ok(file) if file.version == FILE_VERSION && file.peer == peer => Some(file),
            Ok(file) if file.version == FILE_VERSION => {
                info!(
                    "{} belongs to another pair ({}), starting over",
                    path.display(),
                    file.peer
                );
                None
            }
            Ok(file) => {
                warn!(
                    version = file.version,
                    "{} has an unsupported version, ignoring it",
                    path.display()
                );
                None
            }
            Err(e) => {
                warn!(error = %e, "{} is not valid, ignoring it", path.display());
                None
            }
        }
    }

    pub fn save(&self, path: &Path) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).change_context(AppError::Unknown)?;
        }
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_vec_pretty(self).change_context(AppError::Unknown)?;
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&tmp)
                .change_context(AppError::Unknown)
                .attach_with(|| format!("could not write {}", tmp.display()))?;
            file.write_all(&body).change_context(AppError::Unknown)?;
            file.sync_all().change_context(AppError::Unknown)?;
        }
        std::fs::rename(&tmp, path)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not write {}", path.display()))
    }

    /// 解除配对或离开控制面时删掉
    pub fn forget(path: &Path) {
        match std::fs::remove_file(path) {
            Ok(()) => info!("{} removed", path.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!(error = %e, "could not remove {}", path.display()),
        }
    }

    /// 排一条待发；同一个键之前排的作废。返回新序号
    pub fn enqueue(&mut self, key: &str) -> u64 {
        self.queue.retain(|queued| queued.key != key);
        self.seq += 1;
        self.queue.push(Queued {
            seq: self.seq,
            key: key.to_string(),
        });
        self.seq
    }

    /// 对端处理完了 `upto` 及之前的；返回出队了几条
    pub fn acked(&mut self, upto: u64) -> usize {
        let before = self.queue.len();
        self.queue.retain(|queued| queued.seq > upto);
        before - self.queue.len()
    }

    /// 序号大于 `after` 的待发，按排队顺序
    pub fn pending(&self, after: u64) -> impl Iterator<Item = &Queued> {
        self.queue.iter().filter(move |queued| queued.seq > after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::sync::{Side, Verdict, config_key, room_key};

    #[test]
    fn the_queue_coalesces_per_key_and_acks_are_cumulative() {
        let mut file = PairFile::new("node:5");
        assert_eq!(file.enqueue(&config_key("a")), 1);
        assert_eq!(file.enqueue(&room_key("7")), 2);
        assert_eq!(file.enqueue(&config_key("a")), 3);
        let keys: Vec<_> = file.pending(0).map(|q| q.key.as_str()).collect();
        assert_eq!(keys, ["room/7", "config/a"], "同一个键只留最后一次");
        assert_eq!(file.pending(2).count(), 1);
        assert_eq!(file.acked(2), 1);
        // 重复的应答不出错
        assert_eq!(file.acked(2), 0);
        assert_eq!(file.acked(3), 1);
        assert!(file.queue.is_empty());
        assert_eq!(file.enqueue(&config_key("b")), 4, "序号不回头");
    }

    #[test]
    fn the_file_survives_a_restart_and_replay_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path());
        let mut file = PairFile::new("abc");
        let stamp = file.book.write(
            &config_key("segment_time"),
            Side::Node,
            1_000,
            Some("d".into()),
        );
        file.enqueue(&config_key("segment_time"));
        file.save(&path).unwrap();

        assert!(PairFile::load(&path, "other").is_none(), "换了对端就作废");
        let mut back = PairFile::load(&path, "abc").unwrap();
        assert_eq!(back, file);
        assert_eq!(back.pending(0).count(), 1);
        // 对端收了两次（第一次的应答丢了）：第二次认出是同一个版本
        let mut peer = Book::default();
        assert_eq!(
            peer.judge(&config_key("segment_time"), &stamp, Side::Controller),
            Verdict::Take
        );
        peer.put(&config_key("segment_time"), stamp, Some("d2".into()));
        assert_eq!(
            peer.judge(&config_key("segment_time"), &stamp, Side::Controller),
            Verdict::Same
        );
        back.acked(1);
        back.save(&path).unwrap();
        assert!(PairFile::load(&path, "abc").unwrap().queue.is_empty());

        std::fs::write(&path, "{").unwrap();
        assert!(PairFile::load(&path, "abc").is_none());
        PairFile::forget(&path);
        assert!(!path.exists());
        PairFile::forget(&path);
    }
}
