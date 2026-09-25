//! 关键帧索引旁路：进程内写盘的下载器（stream-gears、mesio）把「写进了哪个文件、从什么偏移开始、
//! 写的是什么」交给索引任务，由它边录边建 `<分段>.idx`，分段关闭时不用再把整个文件读一遍。
//!
//! 挂点必须是写盘处，不能是预览的解析点：预览拿到的 tag 还没进 GOP 缓存（stream-gears），
//! 或还没经过修复管线（mesio 会重排 tag、改时间戳、补头），与落盘的字节和偏移对不上。
//!
//! 不复制媒体数据：FLV tag 在写盘处就地判定（[`FlvClassifier`]），只发偏移、时间戳、长度和判定结果；
//! TS / fMP4 要跨写入拼 PES、读 `moof`，交出去的是写入端本来就持有的 [`Bytes`]（引用计数，不拷贝）。
//!
//! 热路径只调 [`FileTap`] 的方法，它们只做一次 `try_send`：不 await、不返回错误。
//! 通道满或索引任务已退出时丢掉这个事件，并把该文件标记为 [`TapFile::lost`]，此后不再为它发事件；
//! 索引任务保留已处理的部分，分段关闭时由扫盘从那里补齐。录制本身不受任何影响。

use bytes::Bytes;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, oneshot};

/// 索引对一个 FLV tag 的判定。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlvTagKind {
    /// 音频或视频 tag。
    pub media: bool,
    /// 音视频序列头。
    pub sequence_header: bool,
    /// 能从这里开始解码的视频关键帧。
    pub keyframe: bool,
}

/// 按 tag 头第一个字节和完整的 body 判定一个 FLV tag。
pub type FlvClassifier = fn(tag_type: u8, body: &Bytes) -> FlvTagKind;

/// 正在写的一个分段文件。
#[derive(Debug)]
pub struct TapFile {
    path: PathBuf,
    lost: AtomicBool,
}

impl TapFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 有事件没送到（或写入端发现字节对不上），这个文件的流式索引不完整。
    pub fn lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub enum IndexEvent {
    Opened(Arc<TapFile>),
    /// 一个 FLV tag 从 `offset`（tag 头起点）开始写进了文件，占 `11 + data_size + 4` 字节。
    FlvTag {
        file: Arc<TapFile>,
        offset: u64,
        timestamp: u32,
        data_size: u32,
        kind: FlvTagKind,
    },
    /// 一段字节从 `offset` 开始原样写进了文件（TS 分片、fMP4 的 init / 分片）。
    Bytes {
        file: Arc<TapFile>,
        offset: u64,
        data: Bytes,
    },
    /// 文件写完，共 `len` 字节。
    Closed {
        file: Arc<TapFile>,
        len: u64,
    },
    /// 索引任务处理到这里时回复：在它之前发出的事件都已处理完。
    Sync(oneshot::Sender<()>),
}

/// 发往一个索引任务的句柄，可随意克隆。
#[derive(Debug, Clone)]
pub struct IndexTap {
    tx: mpsc::Sender<IndexEvent>,
    classify: FlvClassifier,
}

impl IndexTap {
    /// `classify` 在写入端就地判定每个 FLV tag，须与索引任务扫盘时的判定一致。
    pub fn channel(capacity: usize, classify: FlvClassifier) -> (Self, mpsc::Receiver<IndexEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        (Self { tx, classify }, rx)
    }

    /// 开始写 `path`。
    pub fn open(&self, path: &Path) -> FileTap {
        let tap = FileTap {
            tx: self.tx.clone(),
            classify: self.classify,
            file: Arc::new(TapFile {
                path: path.to_path_buf(),
                lost: AtomicBool::new(false),
            }),
        };
        tap.send(IndexEvent::Opened(tap.file.clone()));
        tap
    }

    /// 等索引任务处理完此前发出的所有事件（索引任务已退出时立即返回）。不在录制热路径上用。
    pub async fn sync(&self) {
        let (reply, done) = oneshot::channel();
        if self.tx.send(IndexEvent::Sync(reply)).await.is_ok() {
            let _ = done.await;
        }
    }
}

/// 一个分段文件的写入端，方法都只做一次 `try_send`。
#[derive(Debug)]
pub struct FileTap {
    tx: mpsc::Sender<IndexEvent>,
    classify: FlvClassifier,
    file: Arc<TapFile>,
}

impl FileTap {
    pub fn path(&self) -> &Path {
        &self.file.path
    }

    /// 见 [`TapFile::lost`]。
    pub fn lost(&self) -> bool {
        self.file.lost()
    }

    /// 一个 FLV tag 从 `offset` 开始写进了文件。`tag_type` 为 tag 头第一个字节。
    pub fn flv_tag(&self, offset: u64, tag_type: u8, timestamp: u32, body: &Bytes) {
        if self.file.lost() {
            return;
        }
        let kind = (self.classify)(tag_type, body);
        self.flv_tag_kind(offset, timestamp, body.len() as u32, kind);
    }

    /// 同 [`Self::flv_tag`]，判定已由调用方做过（或 body 写进文件时换了长度，如 mesio 的 onMetaData）。
    pub fn flv_tag_kind(&self, offset: u64, timestamp: u32, data_size: u32, kind: FlvTagKind) {
        self.send(IndexEvent::FlvTag {
            file: self.file.clone(),
            offset,
            timestamp,
            data_size,
            kind,
        });
    }

    /// `data` 从 `offset` 开始原样写进了文件。
    pub fn bytes(&self, offset: u64, data: &Bytes) {
        self.send(IndexEvent::Bytes {
            file: self.file.clone(),
            offset,
            data: data.clone(),
        });
    }

    /// 文件写完，共 `len` 字节。
    pub fn closed(&self, len: u64) {
        self.send(IndexEvent::Closed {
            file: self.file.clone(),
            len,
        });
    }

    /// 写入端自己发现偏移对不上：放弃这个文件的流式索引，关段时扫盘。
    pub fn mark_lost(&self) {
        self.file.lost.store(true, Ordering::Release);
    }

    fn send(&self, event: IndexEvent) {
        if self.file.lost() {
            return;
        }
        if self.tx.try_send(event).is_err() {
            self.mark_lost();
        }
    }
}
