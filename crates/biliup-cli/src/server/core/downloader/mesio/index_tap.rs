//! mesio 的关键帧索引旁路。
//!
//! 修复管线会重排 tag、改时间戳、补头，writer 还会把 onMetaData 换成定长版本，所以条目的落盘
//! 偏移只能在管线之后、按 writer 自己的字节计数来算：管线输出先经 [`forward`]，把每个条目要进
//! 索引的部分记进队列再交给 writer；writer 每写完一个条目回调一次进度（阈值为 0），用
//! `bytes_written_total` 的增量从队列里取出刚写的那个条目，按当前文件已写字节得到它的偏移，
//! 交给 [`FileTap`]。
//!
//! 队列里不放媒体数据：FLV tag 在 [`forward`] 里就地判定，只留时间戳、长度和判定结果；HLS 分片
//! 放条目自己的 [`Bytes`]（引用计数，不复制）。writer 的回调只给字节计数、不给条目，所以条目要在
//! 进 writer 之前记下来。
//!
//! 回调在 writer 线程上运行，只做非阻塞的 `try_recv` / `try_send`。增量与条目长度对不上时标记
//! 这个文件，交给关段时的扫盘，不影响写盘。

use super::WriterEvent;
use crate::server::workbench::index::classify_flv_tag;
use biliup::downloader::index_tap::{FileTap, FlvTagKind, IndexTap};
use bytes::Bytes;
use flv::FlvData;
use flv_fix::FlvWriter;
use hls::HlsData;
use hls_fix::HlsWriter;
use pipeline_common::{
    PipelineError, PipelineReceiver, ProgressConfig, SplitReason, WriterProgress,
};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

/// FLV 文件头 + PreviousTagSize0，writer 在每个文件的第一个 tag 之前写出。
const FLV_FILE_HEAD: u64 = 13;
/// tag 头 + PreviousTagSize。
const FLV_TAG_OVERHEAD: u64 = 15;

/// 一个会写出字节的管线条目；`Split` / `EndMarker` 这类控制项不进队列。
pub(super) enum Entry {
    Tag {
        timestamp: u32,
        data_size: u32,
        kind: FlvTagKind,
        script: bool,
    },
    Bytes(Bytes),
}

pub(super) trait Indexable {
    fn index_entry(&self) -> Option<Entry>;
}

impl Indexable for FlvData {
    fn index_entry(&self) -> Option<Entry> {
        let FlvData::Tag(tag) = self else {
            return None;
        };
        let tag_type = u8::from(tag.tag_type()) | if tag.is_filtered() { 0x20 } else { 0 };
        Some(Entry::Tag {
            timestamp: tag.timestamp_ms,
            data_size: tag.data().len() as u32,
            kind: classify_flv_tag(tag_type, tag.data()),
            script: tag.is_script_tag(),
        })
    }
}

impl Indexable for HlsData {
    fn index_entry(&self) -> Option<Entry> {
        self.data()
            .filter(|data| !data.is_empty())
            .map(|data| Entry::Bytes(data.clone()))
    }
}

type OpenHook = Box<dyn Fn(&Path, u32) + Send + Sync>;
type CloseHook = Box<dyn Fn(&Path, u32, f64, u64, Option<&SplitReason>) + Send + Sync>;

/// FLV / HLS writer 共有的三个回调。
pub(super) trait WriterHooks {
    fn on_open(&mut self, hook: OpenHook);
    fn on_close(&mut self, hook: CloseHook);
    fn on_progress(&mut self, hook: Box<dyn Fn(WriterProgress) + Send + Sync>);
}

macro_rules! writer_hooks {
    ($writer:ty) => {
        impl WriterHooks for $writer {
            fn on_open(&mut self, hook: OpenHook) {
                self.set_on_segment_start_callback(hook);
            }
            fn on_close(&mut self, hook: CloseHook) {
                self.set_on_segment_complete_callback(hook);
            }
            fn on_progress(&mut self, hook: Box<dyn Fn(WriterProgress) + Send + Sync>) {
                self.set_progress_callback_with_config(
                    hook,
                    ProgressConfig {
                        bytes_interval: 0,
                        time_interval_ms: 0,
                    },
                );
            }
        }
    };
}

writer_hooks!(FlvWriter);
writer_hooks!(HlsWriter);

struct State {
    file: Option<FileTap>,
    /// 当前文件已写字节。
    file_pos: u64,
    last_total: u64,
    queue: Receiver<Entry>,
}

/// 给 writer 装上开段 / 关段回调（`tap` 为 `None` 时只转发分段事件），返回条目队列的发送端，
/// 交给 [`forward`]。
pub(super) fn install<W: WriterHooks>(
    writer: &mut W,
    seg_tx: UnboundedSender<WriterEvent>,
    tap: Option<IndexTap>,
) -> Option<Sender<Entry>> {
    let start = super::segment_start_hook(seg_tx.clone());
    let complete = super::segment_complete_hook(seg_tx);
    let Some(tap) = tap else {
        writer.on_open(Box::new(start));
        writer.on_close(Box::new(complete));
        return None;
    };
    let (queue_tx, queue) = channel();
    let state = Arc::new(Mutex::new(State {
        file: None,
        file_pos: 0,
        last_total: 0,
        queue,
    }));

    let st = state.clone();
    writer.on_open(Box::new(move |path, index| {
        {
            let mut st = st.lock().unwrap();
            st.file = Some(tap.open(path));
            st.file_pos = 0;
        }
        start(path, index);
    }));

    let st = state.clone();
    writer.on_close(Box::new(move |path, index, duration, size, reason| {
        {
            let mut st = st.lock().unwrap();
            if let Some(file) = st.file.take() {
                if st.file_pos != size {
                    file.mark_lost();
                }
                // 先于分段事件发出：录制器收到关段时，索引任务队列里已有这个文件的全部事件
                file.closed(size);
            }
        }
        complete(path, index, duration, size, reason);
    }));

    writer.on_progress(Box::new(move |progress| {
        let mut st = state.lock().unwrap();
        let delta = progress.bytes_written_total.saturating_sub(st.last_total);
        st.last_total = progress.bytes_written_total;
        if delta == 0 {
            return;
        }
        let entry = st.queue.try_recv().ok();
        let pos = st.file_pos;
        st.file_pos += delta;
        if let Some(file) = &st.file {
            report(file, pos, delta, entry);
        }
    }));
    Some(queue_tx)
}

/// 刚写完的条目从当前文件的 `pos` 开始，占 `delta` 字节。
fn report(file: &FileTap, pos: u64, delta: u64, entry: Option<Entry>) {
    match entry {
        Some(Entry::Tag {
            timestamp,
            data_size,
            kind,
            script,
        }) => {
            let head = if pos == 0 { FLV_FILE_HEAD } else { 0 };
            let offset = pos + head;
            if script {
                // onMetaData 可能被换成了定长版本，长度以实际写出的为准
                match delta.checked_sub(head + FLV_TAG_OVERHEAD) {
                    Some(size) => file.flv_tag_kind(offset, timestamp, size as u32, kind),
                    None => file.mark_lost(),
                }
            } else if delta == head + FLV_TAG_OVERHEAD + data_size as u64 {
                file.flv_tag_kind(offset, timestamp, data_size, kind);
            } else {
                file.mark_lost();
            }
        }
        Some(Entry::Bytes(data)) if delta == data.len() as u64 => file.bytes(pos, &data),
        _ => file.mark_lost(),
    }
}

/// 在管线输出与 writer 之间插一站：每个条目先把要进索引的部分记进 `queue`，再原样交给 writer。
pub(super) fn forward<T: Indexable + Send + 'static>(
    mut upstream: PipelineReceiver<T>,
    queue: Sender<Entry>,
    capacity: usize,
) -> PipelineReceiver<T> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<T, PipelineError>>(capacity);
    tokio::spawn(async move {
        while let Some(item) = upstream.recv().await {
            if let Ok(item) = &item
                && let Some(entry) = item.index_entry()
            {
                let _ = queue.send(entry);
            }
            if tx.send(item).await.is_err() {
                break;
            }
        }
    });
    PipelineReceiver::from_items(rx)
}
