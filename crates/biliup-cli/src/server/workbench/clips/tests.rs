use super::plan::{self, Attempt, PlanError};
use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::workbench::index::tests::{build_flv, build_fmp4, build_ts};
use crate::server::workbench::index::{self, Container};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

/// `build_flv(0, 100, 25)`：关键帧在段内 0 / 1000 / 2000 / 3000 ms，时长 3965 ms。
const FLV_MS: i64 = 3965;

pub(super) async fn setup() -> (TempDir, ConnectionPool, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1, 2) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    (dir, pool, id)
}

pub(super) async fn add_segment(
    pool: &ConnectionPool,
    session_id: i64,
    path: &Path,
    state: &str,
    start_ms: i64,
    end_ms: Option<i64>,
) -> i64 {
    let container = store_container(path);
    sqlx::query_scalar(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, ?, ?, ?, ?, ?, 0) RETURNING id",
    )
    .bind(session_id)
    .bind(path.to_string_lossy().into_owned())
    .bind(container)
    .bind(state)
    .bind(start_ms)
    .bind(end_ms)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn store_container(path: &Path) -> &'static str {
    crate::server::workbench::store::container_of(path).unwrap()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// 三段 FLV：`[0, 3965)`、`[3965, 7930)`、断流、`[30000, 33965)`；第二段的时间戳是绝对的。
pub(super) async fn flv_session() -> (TempDir, ConnectionPool, i64, Vec<i64>) {
    let (dir, pool, session) = setup().await;
    let mut ids = Vec::new();
    for (name, base, start) in [
        ("a.flv", 0, 0),
        ("b.flv", 5_000_000, FLV_MS),
        ("c.flv", 0, 30_000),
    ] {
        let path = write(dir.path(), name, &build_flv(base, 100, 25, None).bytes);
        ids.push(
            add_segment(
                &pool,
                session,
                &path,
                "finished",
                start,
                Some(start + FLV_MS),
            )
            .await,
        );
    }
    (dir, pool, session, ids)
}

async fn ready(pool: &ConnectionPool, session: i64, in_ms: i64, out_ms: i64) -> plan::Plan {
    match plan::compute(pool, session, in_ms, out_ms).await.unwrap() {
        Attempt::Ready(plan) => plan,
        Attempt::Wait => panic!("不该等待"),
    }
}

fn spans(plan: &plan::Plan) -> Vec<(i64, i64, i64)> {
    plan.pieces
        .iter()
        .map(|p| (p.start_ms, p.end_ms, p.duration_ms))
        .collect()
}

#[tokio::test]
async fn cuts_snap_to_keyframes_like_the_keyframe_endpoint() {
    let (_dir, pool, session, ids) = flv_session().await;
    let flv = build_flv(0, 100, 25, None);

    // 段内：入点取之前的关键帧，出点取之后的关键帧（不含）
    let plan = ready(&pool, session, 1500, 2500).await;
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (1000, 3000));
    assert_eq!(spans(&plan), vec![(1000, 3000, 2000)]);
    assert_eq!(plan.pieces[0].from, flv.keyframes[1].1);
    assert_eq!(plan.pieces[0].to, flv.keyframes[3].1);
    assert_eq!(plan.pieces[0].segment_id, ids[0]);
    assert_eq!(plan.pieces[0].keyframes.len(), 2);

    // 已经在关键帧上：原样
    let plan = ready(&pool, session, 1000, 2000).await;
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (1000, 2000));

    // 跨段：第一段读到末尾，第二段从第一个关键帧起
    let plan = ready(&pool, session, 3500, 4500).await;
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (3000, FLV_MS + 1000));
    assert_eq!(
        spans(&plan),
        vec![(3000, FLV_MS, 965), (FLV_MS, FLV_MS + 1000, 1000)]
    );
    assert_eq!(plan.pieces[0].to, flv.bytes.len() as u64);
    assert_eq!(plan.duration_ms(), 1965);

    // 跨断流缺口：缺口不占产物时长
    let plan = ready(&pool, session, 7000, 31_500).await;
    assert_eq!(
        spans(&plan),
        vec![(FLV_MS + 3000, 2 * FLV_MS, 965), (30_000, 32_000, 2000)]
    );
    assert_eq!(plan.output_ms(7000), 35);
    assert_eq!(plan.output_ms(20_000), 965);
    assert_eq!(plan.output_ms(31_500), 965 + 1500);

    // 入点在缺口里：从下一段第一个关键帧起；出点晚于最后一段：到末尾
    let plan = ready(&pool, session, 10_000, 99_000).await;
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (30_000, 30_000 + FLV_MS));
    assert_eq!(spans(&plan), vec![(30_000, 30_000 + FLV_MS, FLV_MS)]);

    // 出点在段末之后的缺口里：到这一段末尾
    let plan = ready(&pool, session, 5000, 20_000).await;
    assert_eq!(plan.cut_out_ms, 2 * FLV_MS);
    assert_eq!(plan.pieces.len(), 1);

    // 整段落在缺口里
    let err = plan::compute(&pool, session, 10_000, 20_000)
        .await
        .unwrap_err();
    assert!(matches!(err, PlanError::Unavailable(_)), "{err}");
}

#[tokio::test]
async fn unreadable_or_mismatched_segments_are_reported_not_skipped() {
    let (dir, pool, session, ids) = flv_session().await;
    sqlx::query("UPDATE segments SET state = 'deleted' WHERE id = ?")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    let err = plan::compute(&pool, session, 1000, 5000).await.unwrap_err();
    assert!(err.to_string().contains("已经被清理"), "{err}");
    // 范围挪开这一段就能剪
    ready(&pool, session, 1000, 2000).await;

    // 等着被删的分段文件还在，照样能剪
    sqlx::query("UPDATE segments SET state = 'pending_delete' WHERE id = ?")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    ready(&pool, session, 1000, 5000).await;

    // 文件丢了
    std::fs::remove_file(dir.path().join("b.flv")).unwrap();
    index::remove(&dir.path().join("b.flv"));
    let err = plan::compute(&pool, session, 1000, 5000).await.unwrap_err();
    assert!(err.to_string().contains("不见了"), "{err}");

    // 换了序列头（分辨率变了）
    let mut other = build_flv(0, 100, 25, None).bytes;
    let seq = other
        .windows(8)
        .position(|w| w == [0x17, 0x00, 0, 0, 0, 1, 2, 3])
        .unwrap();
    other[seq + 7] = 9;
    std::fs::write(dir.path().join("b.flv"), other).unwrap();
    let err = plan::compute(&pool, session, 1000, 5000).await.unwrap_err();
    assert!(err.to_string().contains("编码参数不同"), "{err}");
}

#[tokio::test]
async fn the_growing_tail_is_waited_for_until_a_keyframe_follows_the_out_point() {
    let (dir, pool, session) = setup().await;
    let flv = build_flv(0, 100, 25, None);
    let cut = flv.keyframes[2].1 as usize + 20;
    let path = write(dir.path(), "live.flv.part", &flv.bytes[..cut]);
    add_segment(&pool, session, &path, "recording", 0, None).await;

    // 出点 1500 之后的关键帧（2000）只写了一半
    assert!(matches!(
        plan::compute(&pool, session, 500, 1500).await.unwrap(),
        Attempt::Wait
    ));
    let waited = plan::resolve(&pool, session, 500, 1500, Duration::ZERO, || {}).await;
    assert!(waited.unwrap_err().to_string().contains("还没有写出关键帧"));

    std::fs::write(&path, &flv.bytes).unwrap();
    let plan = ready(&pool, session, 500, 1500).await;
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (0, 2000));
    assert_eq!(plan.pieces[0].to, flv.keyframes[2].1);

    // 录制中的分段读不到段尾：出点在最后一个关键帧之后就等
    assert!(matches!(
        plan::compute(&pool, session, 3100, 3900).await.unwrap(),
        Attempt::Wait
    ));
}

#[tokio::test]
async fn ts_and_fmp4_sessions_plan_in_their_own_containers() {
    let (dir, pool, session) = setup().await;
    let ts = build_ts(900_000, 100, false, false);
    let path = write(dir.path(), "a.ts", &ts.bytes);
    add_segment(&pool, session, &path, "finished", 0, Some(3300)).await;
    let plan = ready(&pool, session, 1500, 2500).await;
    assert_eq!(plan.container, Container::Ts);
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (1000, 3000));
    assert_eq!(plan.pieces[0].from, ts.keyframes[1].1);

    let (dir2, pool2, session2) = setup().await;
    let (bytes, frames, _) = build_fmp4(6);
    let path = write(dir2.path(), "a.mp4", &bytes);
    add_segment(&pool2, session2, &path, "finished", 0, Some(6000)).await;
    let plan = ready(&pool2, session2, 2500, 3000).await;
    assert_eq!(plan.container, Container::Fmp4);
    assert_eq!((plan.cut_in_ms, plan.cut_out_ms), (2000, 4000));
    assert_eq!(plan.pieces[0].from, frames[1].1);
    assert_eq!(plan.pieces[0].to, frames[2].1);
    drop(dir);
}
