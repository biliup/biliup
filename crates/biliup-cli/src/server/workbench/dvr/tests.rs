use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::workbench::index::tests::{build_flv, build_ts};
use crate::server::workbench::store::FinishedSegment;
use biliup::downloader::util::ByteCounter;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

async fn setup() -> (TempDir, ConnectionPool, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (1, 'https://a', 'a')")
        .execute(&pool)
        .await
        .unwrap();
    let at = 1_700_000_000_000;
    let info = StreamerInfo::new(
        "a",
        "https://a",
        "标题",
        DateTime::<Utc>::from_timestamp_millis(at).unwrap(),
        "",
    );
    let session = store::open_session(&pool, 1, &info, at, 0).await.unwrap();
    let session = live::unique_session_id(&pool, session.id).await;
    store::set_started_at(&pool, session, at).await.unwrap();
    (dir, pool, session)
}

async fn add_segment(
    pool: &ConnectionPool,
    session_id: i64,
    path: &Path,
    start_ms: i64,
    gap_before_ms: i64,
    end_ms: Option<i64>,
) -> i64 {
    let container = store::container_of(path).unwrap();
    let path = path.to_string_lossy().into_owned();
    let id = store::insert_segment(pool, session_id, &path, container, start_ms, gap_before_ms)
        .await
        .unwrap();
    if let Some(end_ms) = end_ms {
        finish(pool, id, &path, end_ms).await;
    }
    id
}

async fn finish(pool: &ConnectionPool, id: i64, path: &str, end_ms: i64) {
    store::finish_segment(
        pool,
        id,
        &FinishedSegment {
            path: path.to_string(),
            state: SegmentState::Finished,
            end_ms,
            bytes: None,
            index_path: None,
            danmaku_path: None,
        },
    )
    .await
    .unwrap();
}

fn write(dir: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

async fn collect(dvr: Dvr) -> Vec<u8> {
    let mut stream = Box::pin(dvr.into_stream());
    let mut out = Vec::new();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("回看流不应卡住")
    {
        out.extend_from_slice(&chunk.unwrap());
    }
    out
}

/// `(tag 类型, 时间戳, body 第一个字节)`
fn flv_tags(data: &[u8]) -> Vec<(u8, u32, u8)> {
    assert_eq!(&data[..3], b"FLV");
    let mut rest = &data[13..];
    let mut tags = Vec::new();
    while rest.len() >= 11 {
        let size = u32::from_be_bytes([0, rest[1], rest[2], rest[3]]) as usize;
        let ts = u32::from_be_bytes([rest[7], rest[4], rest[5], rest[6]]);
        tags.push((rest[0], ts, rest[11]));
        rest = &rest[11 + size + 4..];
    }
    assert!(rest.is_empty(), "输出里不应有不完整的 tag");
    tags
}

/// 两段 FLV：第一段时间戳从 0 起（mesio），第二段从 17 s 起（stream-gears 的绝对时间戳）。
async fn two_flv_segments(gap_before_ms: i64, second_seq: u8) -> (TempDir, ConnectionPool, i64) {
    let (dir, pool, session) = setup().await;
    let a = build_flv(0, 100, 25, None);
    let mut b = build_flv(17_000, 100, 25, None).bytes;
    let seq = [0x17, 0x00, 0, 0, 0, 1, 2, 3];
    let at = b.windows(seq.len()).position(|w| w == seq).unwrap();
    b[at + 7] = second_seq;
    let a_path = write(&dir, "a.flv", &a.bytes);
    let b_path = write(&dir, "b.flv", &b);
    add_segment(&pool, session, &a_path, 0, 0, Some(3965)).await;
    add_segment(
        &pool,
        session,
        &b_path,
        3965 + gap_before_ms,
        gap_before_ms,
        Some(3965 + gap_before_ms + 3965),
    )
    .await;
    (dir, pool, session)
}

#[tokio::test]
async fn flv_starts_at_the_keyframe_before_from_and_joins_the_next_segment() {
    let (_dir, pool, session) = two_flv_segments(0, 3).await;
    let dvr = open(&pool, session, 1500).await.unwrap();
    assert_eq!((dvr.start_ms, dvr.content_type()), (1000, "video/x-flv"));
    let tags = flv_tags(&collect(dvr).await);

    // 序列头（时间戳记为起播时刻）→ 起播关键帧；onMetaData 不发
    assert!(tags.iter().all(|t| t.0 != 18));
    assert_eq!(tags[0], (9, 2000, 0x17));
    assert_eq!(tags[1], (8, 2000, 0xAF));
    let media = &tags[2..];
    assert_eq!(media[0], (9, 2000, 0x17), "第一帧是 1000 ms 处的关键帧");
    let video: Vec<u32> = media.iter().filter(|t| t.0 == 9).map(|t| t.1).collect();
    assert_eq!(video.len(), 75 + 100);
    assert!(
        video.windows(2).all(|w| w[1] > w[0]),
        "跨段后视频时间戳必须严格递增"
    );
    // 第二段接在场次 3965 ms 处，顺延到上一帧（4960 + 40）之后
    assert_eq!(video[75], 5000);
    assert_eq!(*video.last().unwrap(), 5000 + 99 * 40);
}

#[tokio::test]
async fn flv_response_ends_at_a_gap_or_a_codec_change() {
    for (gap, seq) in [(5_000, 3), (0, 4)] {
        let (_dir, pool, session) = two_flv_segments(gap, seq).await;
        let dvr = open(&pool, session, 0).await.unwrap();
        let tags = flv_tags(&collect(dvr).await);
        let video = tags[2..].iter().filter(|t| t.0 == 9).count();
        assert_eq!(video, 100, "gap={gap} seq={seq}：只回放第一段");
    }
    // 从第二段里起播：直接从第二段开始
    let (_dir, pool, session) = two_flv_segments(5_000, 3).await;
    let dvr = open(&pool, session, 9_000).await.unwrap();
    assert_eq!(dvr.start_ms, 8_965);
    let tags = flv_tags(&collect(dvr).await);
    assert_eq!(tags[2], (9, 8_965 + 1000, 0x17));
}

async fn next_within<S: futures::Stream + Unpin>(
    stream: &mut S,
    ms: u64,
) -> Result<Option<S::Item>, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_millis(ms), stream.next()).await
}

#[tokio::test]
async fn follows_the_segment_being_written_without_polling() {
    let (dir, pool, session) = setup().await;
    let flv = build_flv(0, 100, 25, None);
    // 先落盘前两个 GOP（第三个关键帧 tag 之前），模拟录制中
    let cut = flv.keyframes[2].1 as usize;
    let path = write(&dir, "live.flv", &flv.bytes[..cut]);
    let id = add_segment(&pool, session, &path, 0, 0, None).await;
    let counter = ByteCounter::new();
    let guard = live::register(session, 1, Some(counter.clone()));

    let dvr = open(&pool, session, 0).await.unwrap();
    let mut stream = Box::pin(dvr.into_stream());
    let mut received = Vec::new();
    while let Ok(Some(chunk)) = next_within(&mut stream, 300).await {
        received.extend_from_slice(&chunk.unwrap());
    }
    let before = flv_tags(&received).iter().filter(|t| t.0 == 9).count();
    assert_eq!(before, 1 + 50, "视频序列头 + 已落盘的两个 GOP");

    // 写入端追加剩下的内容并累加写盘计数：读取方被唤醒
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(&flv.bytes[cut..]).unwrap();
    counter.add((flv.bytes.len() - cut) as u64);
    while let Ok(Some(chunk)) = next_within(&mut stream, 300).await {
        received.extend_from_slice(&chunk.unwrap());
    }
    let after = flv_tags(&received).iter().filter(|t| t.0 == 9).count();
    assert_eq!(after, 1 + 100);

    // 关段 + 录制结束：响应正常结束
    finish(&pool, id, &path.to_string_lossy(), 3965).await;
    guard.changed();
    drop(guard);
    let end = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(chunk) = stream.next().await {
            chunk.unwrap();
        }
    })
    .await;
    assert!(end.is_ok(), "录制结束后响应应当结束");
}

/// 下一段由索引任务边写边建：还没有关键帧时等索引缓存落盘的通知，不靠写盘计数器、不扫盘。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn joins_a_new_segment_when_its_streaming_index_is_saved() {
    let (dir, pool, session) = setup().await;
    let a = build_flv(0, 100, 25, None);
    let a_path = write(&dir, "a.flv", &a.bytes);
    add_segment(&pool, session, &a_path, 0, 0, Some(3965)).await;
    let b = build_flv(17_000, 100, 25, None);
    let head = b.keyframes[0].1 as usize;
    let b_path = write(&dir, "b.flv", &b.bytes[..head]);
    add_segment(&pool, session, &b_path, 3965, 0, None).await;
    let _guard = live::register(session, 1, Some(ByteCounter::new()));
    let tap = index::live::spawn();
    let b_tap = tap.open(&b_path);
    tokio::time::timeout(Duration::from_secs(2), async {
        while !index::live::is_live(&b_path) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let mut stream = Box::pin(open(&pool, session, 0).await.unwrap().into_stream());
    let mut received = Vec::new();
    while let Ok(Some(chunk)) = next_within(&mut stream, 300).await {
        received.extend_from_slice(&chunk.unwrap());
    }
    let video = flv_tags(&received).iter().filter(|t| t.0 == 9).count();
    assert_eq!(video, 1 + 100, "第二段还没有关键帧，停在第一段末尾");

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&b_path)
        .unwrap();
    file.write_all(&b.bytes[head..]).unwrap();
    index::live::tests::feed_flv_tags(&b_tap, &b.bytes);
    while let Ok(Some(chunk)) = next_within(&mut stream, 1000).await {
        received.extend_from_slice(&chunk.unwrap());
    }
    let tags = flv_tags(&received);
    let video: Vec<u32> = tags.iter().filter(|t| t.0 == 9).map(|t| t.1).collect();
    assert_eq!(video.len(), 1 + 200, "索引落盘后接上第二段");
    assert!(video[1..].windows(2).all(|w| w[1] > w[0]));
    drop((b_tap, tap));
}

#[test]
fn empty_wakes_back_off_to_a_cap() {
    let steps: Vec<u64> = (0..8).map(empty_wake_step).collect();
    let k = 1024;
    assert_eq!(
        steps,
        [
            0,
            16 * k,
            32 * k,
            64 * k,
            128 * k,
            256 * k,
            256 * k,
            256 * k
        ]
    );
    assert_eq!(empty_wake_step(u32::MAX), 256 * k);
}

#[tokio::test]
async fn counter_growth_that_is_not_on_disk_yet_backs_off() {
    let (dir, pool, session) = setup().await;
    let flv = build_flv(0, 100, 25, None);
    let cut = flv.keyframes[2].1 as usize;
    let path = write(&dir, "live.flv", &flv.bytes[..cut]);
    add_segment(&pool, session, &path, 0, 0, None).await;
    let counter = ByteCounter::new();
    let _guard = live::register(session, 1, Some(counter.clone()));
    let mut stream = Box::pin(open(&pool, session, 0).await.unwrap().into_stream());
    while let Ok(Some(chunk)) = next_within(&mut stream, 300).await {
        chunk.unwrap();
    }

    // 计数器涨了但数据还在写入端的缓冲里：读取方醒一次、读空
    counter.add(100);
    assert!(next_within(&mut stream, 200).await.is_err());
    // 数据落盘了，但计数器只再涨一点：还没到落空后的门槛，不会再去读
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(&flv.bytes[cut..]).unwrap();
    counter.add(1000);
    assert!(next_within(&mut stream, 300).await.is_err());
    // 过了门槛：读到新内容
    counter.add(16 * 1024);
    let chunk = next_within(&mut stream, 1000)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!chunk.is_empty());
}

#[tokio::test]
async fn ts_rewrites_pts_to_session_time() {
    let (dir, pool, session) = setup().await;
    let ts = build_ts((1 << 33) - 45_000, 90, false, false);
    let path = write(&dir, "a.ts", &ts.bytes);
    add_segment(&pool, session, &path, 10_000, 0, Some(12_967)).await;
    let dvr = open(&pool, session, 11_100).await.unwrap();
    assert_eq!((dvr.start_ms, dvr.content_type()), (11_000, "video/mp2t"));
    let out = collect(dvr).await;
    assert_eq!(out.len() % 188, 0);
    // PAT、PMT，然后是 1000 ms 处关键帧的 PES，PTS = (11_000 + 1000) × 90
    assert_eq!(out[188 * 2 + 1] & 0x40, 0x40);
    let first_video = &out[188 * 2..188 * 3];
    let payload = 4 + if first_video[3] & 0x20 != 0 {
        1 + first_video[4] as usize
    } else {
        0
    };
    let pts = &first_video[payload + 9..payload + 14];
    let v = (((pts[0] >> 1) & 7) as i64) << 30
        | (pts[1] as i64) << 22
        | ((pts[2] >> 1) as i64) << 15
        | (pts[3] as i64) << 7
        | (pts[4] >> 1) as i64;
    assert_eq!(v, 12_000 * 90);
}

#[tokio::test]
async fn unknown_session_and_empty_session_are_reported() {
    let (_dir, pool, session) = setup().await;
    assert!(matches!(
        open(&pool, session + 1, 0).await,
        Err(OpenError::NotFound)
    ));
    assert!(matches!(
        open(&pool, session, 0).await,
        Err(OpenError::NoMedia)
    ));
}
