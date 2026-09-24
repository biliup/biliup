use super::super::tests::{build_flv, build_fmp4, build_ts};
use super::super::{load, refresh};
use super::*;
use biliup::downloader::index_tap::FileTap;

/// 把收到的事件全部交给 `indexer` 并扫一轮（不起线程，便于逐步断言）。
fn drain(indexer: &mut Indexer, rx: &mut mpsc::Receiver<IndexEvent>) {
    while let Ok(event) = rx.try_recv() {
        indexer.handle(event);
    }
    indexer.scan_dirty();
}

/// 像 stream-gears 的 `FlvFile` 那样逐 tag 报告：偏移、tag 类型、时间戳、body。
fn feed_flv_tags(tap: &FileTap, bytes: &[u8]) {
    let mut offset = 13;
    while offset + 15 <= bytes.len() {
        let head = &bytes[offset..offset + 11];
        let size = u32::from_be_bytes([0, head[1], head[2], head[3]]) as usize;
        let ts = u32::from_be_bytes([head[7], head[4], head[5], head[6]]);
        let body = Bytes::copy_from_slice(&bytes[offset + 11..offset + 11 + size]);
        tap.flv_tag(offset as u64, head[0], ts, &body);
        offset += 15 + size;
    }
}

/// 按伪随机大小的块原样报告（HTTP 分块 / HLS 分片的边界与容器单元无关）。
fn feed_chunks(tap: &FileTap, bytes: &[u8], seed: u64) {
    let mut state = seed;
    let mut offset = 0;
    while offset < bytes.len() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let len = (1 + (state >> 33) as usize % 3000).min(bytes.len() - offset);
        tap.bytes(
            offset as u64,
            &Bytes::copy_from_slice(&bytes[offset..offset + len]),
        );
        offset += len;
    }
}

/// 流式建出的缓存与对同一份字节从头扫盘的结果逐项一致，关段时的兜底续扫不再读新字节。
fn assert_same_as_disk_scan(path: &Path, bytes: &[u8]) {
    let streamed = load(path).expect("流式索引已落盘");
    assert!(!streamed.complete);
    assert_eq!(streamed.source_len, bytes.len() as u64);

    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join(path.file_name().unwrap());
    std::fs::write(&copy, bytes).unwrap();
    let scanned = refresh(&copy, true).unwrap();
    assert_eq!(streamed.keyframes, scanned.keyframes);
    assert_eq!(streamed.header_len, scanned.header_len);
    assert_eq!(streamed.base_ts, scanned.base_ts);
    assert_eq!(streamed.timescale, scanned.timescale);
    assert_eq!(streamed.duration_ms, scanned.duration_ms);
    assert_eq!(streamed.track, scanned.track);
    assert_eq!(streamed.scanned_upto, scanned.scanned_upto);

    let finished = refresh(path, true).unwrap();
    assert!(finished.complete);
    assert_eq!(
        KeyframeIndex {
            source_len: 0,
            complete: false,
            ..finished
        },
        KeyframeIndex {
            source_len: 0,
            complete: false,
            ..scanned
        }
    );
}

#[test]
fn flv_tags_build_the_same_index_as_a_disk_scan() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(90_000, 300, 25, None);
    let path = dir.path().join("a.flv.part");
    std::fs::write(&path, &flv.bytes).unwrap();

    let (tap, mut rx) = IndexTap::channel(1 << 16);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    feed_flv_tags(&file, &flv.bytes);
    drain(&mut indexer, &mut rx);
    assert!(is_live(&path));
    // 录制中已经落过盘，查询不用扫盘
    let during = load(&path).unwrap();
    assert_eq!(during.keyframes.len(), flv.keyframes.len());

    file.closed(flv.bytes.len() as u64);
    drain(&mut indexer, &mut rx);
    assert!(!is_live(&path));
    assert!(indexer.files.is_empty());
    assert_same_as_disk_scan(&path, &flv.bytes);
}

#[test]
fn ts_chunks_build_the_same_index_as_a_disk_scan() {
    let dir = tempfile::tempdir().unwrap();
    let ts = build_ts((1 << 33) - 90_000, 400, false, false);
    let path = dir.path().join("a.ts");
    std::fs::write(&path, &ts.bytes).unwrap();

    let (tap, mut rx) = IndexTap::channel(1 << 16);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    feed_chunks(&file, &ts.bytes, 7);
    file.closed(ts.bytes.len() as u64);
    drain(&mut indexer, &mut rx);
    assert_eq!(load(&path).unwrap().header_len, ts.header_len);
    assert_same_as_disk_scan(&path, &ts.bytes);
}

#[test]
fn fmp4_chunks_build_the_same_index_as_a_disk_scan() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, frames, _) = build_fmp4(40);
    let path = dir.path().join("a.mp4");
    std::fs::write(&path, &bytes).unwrap();

    let (tap, mut rx) = IndexTap::channel(1 << 16);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    feed_chunks(&file, &bytes, 11);
    file.closed(bytes.len() as u64);
    drain(&mut indexer, &mut rx);
    assert_eq!(load(&path).unwrap().keyframes.len(), frames.len());
    assert_same_as_disk_scan(&path, &bytes);
}

/// 队列满时写入端丢事件并标记；索引任务保存已建好的部分，关段时扫盘从那里补齐。
#[test]
fn a_full_queue_hands_the_rest_over_to_the_close_time_scan() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 300, 25, None);
    let path = dir.path().join("a.flv");
    std::fs::write(&path, &flv.bytes).unwrap();

    let (tap, mut rx) = IndexTap::channel(200);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    feed_flv_tags(&file, &flv.bytes);
    file.closed(flv.bytes.len() as u64);
    assert!(file.lost());
    drain(&mut indexer, &mut rx);
    indexer.sweep_lost();
    assert!(indexer.files.is_empty());
    assert!(!is_live(&path));

    let partial = load(&path).expect("已建好的部分已保存");
    assert!(partial.keyframes.len() < flv.keyframes.len());
    assert!(partial.scanned_upto < flv.bytes.len() as u64);

    let dir2 = tempfile::tempdir().unwrap();
    let copy = dir2.path().join("a.flv");
    std::fs::write(&copy, &flv.bytes).unwrap();
    assert_eq!(
        refresh(&path, true).unwrap().keyframes,
        refresh(&copy, true).unwrap().keyframes
    );
}

#[test]
fn a_gap_in_the_offsets_stops_streaming_for_that_file() {
    let dir = tempfile::tempdir().unwrap();
    let ts = build_ts(0, 60, false, false);
    let path = dir.path().join("a.ts");
    let (tap, mut rx) = IndexTap::channel(64);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    file.bytes(0, &Bytes::copy_from_slice(&ts.bytes[..188 * 10]));
    file.bytes(188 * 11, &Bytes::copy_from_slice(&ts.bytes[188 * 11..]));
    drain(&mut indexer, &mut rx);
    assert!(indexer.files.is_empty());
    assert!(!is_live(&path));
}

/// 写入端报告的长度与收到的字节对不上（例如 mesio 的 writer 多写了东西）时不保存成完整结果。
#[test]
fn a_length_mismatch_at_close_is_left_to_the_disk_scan() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, _, _) = build_fmp4(6);
    let path = dir.path().join("a.mp4");
    let (tap, mut rx) = IndexTap::channel(64);
    let mut indexer = Indexer::default();
    let file = tap.open(&path);
    file.bytes(0, &Bytes::copy_from_slice(&bytes));
    file.closed(bytes.len() as u64 + 1);
    drain(&mut indexer, &mut rx);
    assert!(indexer.files.is_empty());
    assert!(!is_live(&path));
}

/// `sync` 返回时，之前发出的关段事件已经处理完、`.idx` 已落盘，录制器可以接着改名、续扫。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_returns_after_earlier_events_are_indexed() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 200, 25, None);
    let path = dir.path().join("b.flv.part");
    std::fs::write(&path, &flv.bytes).unwrap();

    let tap = spawn();
    let file = tap.open(&path);
    feed_flv_tags(&file, &flv.bytes);
    file.closed(flv.bytes.len() as u64);
    tap.sync().await;
    assert!(!is_live(&path));
    assert_same_as_disk_scan(&path, &flv.bytes);
}
