use super::index::tests::build_flv;
use super::recorder::{ClosedSegment, GAP_TOLERANCE_MS, SessionRecorder, SessionTarget, place};
use super::store::{self, SegmentRow, SegmentState};
use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use tempfile::TempDir;

/// 100 帧、每 25 帧一个关键帧的 FLV：关键帧在 0 / 1000 / 2000 / 3000 ms，时长 3965 ms。
const FLV_DURATION_MS: i64 = 3965;

async fn setup() -> (TempDir, ConnectionPool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (1, 'https://a', 'a')")
        .execute(&pool)
        .await
        .unwrap();
    for id in 1..=4 {
        sqlx::query(
            "INSERT INTO streamerinfo (id, name, url, title, date, live_cover_path)
             VALUES (?, 'a', 'https://a', '标题', '2026-09-24 00:00:00', '')",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    }
    (dir, pool)
}

fn target(streamerinfo_id: i64, merge_minutes: i64) -> SessionTarget {
    SessionTarget {
        streamer_id: 1,
        streamerinfo_id,
        title: "标题".into(),
        merge_window_ms: merge_minutes * 60_000,
    }
}

fn write_flv(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, build_flv(0, 100, 25, None).bytes).unwrap();
    path
}

async fn sessions(pool: &ConnectionPool) -> Vec<(i64, i64, Option<i64>)> {
    sqlx::query_as("SELECT id, started_at, ended_at FROM stream_sessions ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn segments(pool: &ConnectionPool, session_id: i64) -> Vec<SegmentRow> {
    store::session_segments(pool, session_id).await.unwrap()
}

fn s(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn small_gaps_are_absorbed_and_real_ones_recorded() {
    assert_eq!(place(0, 0), (0, 0));
    assert_eq!(place(4000, 3800), (4000, 0));
    assert_eq!(place(4000, 4000 + GAP_TOLERANCE_MS), (4000, 0));
    assert_eq!(place(4000, 30_000), (30_000, 26_000));
}

#[tokio::test]
async fn segments_are_laid_out_on_one_session_timeline() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let recorder = SessionRecorder::spawn(pool.clone(), target(1, 10));
    let handle = recorder.handle();

    // mesio：开段 / 关段都有，关段带时长与字节数
    handle.run_started_at(t0);
    let a = write_flv(dir.path(), "a.flv");
    handle.opened_at(&a, t0 + 500);
    handle.closed_at(
        &a,
        t0 + 4500,
        ClosedSegment {
            duration_ms: Some(4000),
            bytes: Some(123),
            danmaku_path: Some(dir.path().join("a.xml")),
            discard: false,
        },
    );
    // 紧接着的下一段，没有报告时长：取索引扫出的时长
    let b = write_flv(dir.path(), "b.flv");
    handle.opened_at(&b, t0 + 4500);
    handle.closed_at(&b, t0 + 8600, ClosedSegment::default());

    // 断流重连（stream-gears：`.part` 开写，改名后关段）
    handle.run_started_at(t0 + 20_000);
    let c_part = write_flv(dir.path(), "c.flv.part");
    handle.opened_at(&c_part, t0 + 30_000);
    let c = dir.path().join("c.flv");
    std::fs::rename(&c_part, &c).unwrap();
    handle.closed_at(&c, t0 + 34_000, ClosedSegment::default());

    // 小于过滤阈值、会被删掉的分段
    let d = write_flv(dir.path(), "d.flv");
    handle.opened_at(&d, t0 + 34_000);
    handle.closed_at(
        &d,
        t0 + 34_100,
        ClosedSegment {
            discard: true,
            ..Default::default()
        },
    );
    recorder.finish().await;

    let all = sessions(&pool).await;
    assert_eq!(all.len(), 1);
    let (session_id, started_at, ended_at) = all[0];
    assert_eq!(started_at, t0 + 500, "场次 0 点是第一个分段开写的墙钟");
    assert_eq!(ended_at, Some(t0 + 34_100));

    let rows = segments(&pool, session_id).await;
    let summary: Vec<(String, SegmentState, i64, Option<i64>, i64)> = rows
        .iter()
        .map(|r| {
            (
                r.path.clone(),
                r.state,
                r.start_ms,
                r.end_ms,
                r.gap_before_ms,
            )
        })
        .collect();
    let c_start = 30_000 - 500;
    assert_eq!(
        summary,
        vec![
            (s(&a), SegmentState::Finished, 0, Some(4000), 0),
            (
                s(&b),
                SegmentState::Finished,
                4000,
                Some(4000 + FLV_DURATION_MS),
                0
            ),
            (
                s(&c),
                SegmentState::Finished,
                c_start,
                Some(c_start + FLV_DURATION_MS),
                c_start - 4000 - FLV_DURATION_MS
            ),
            (
                s(&d),
                SegmentState::Deleted,
                c_start + FLV_DURATION_MS,
                Some(c_start + FLV_DURATION_MS + 100),
                0
            ),
        ]
    );
    assert_eq!(rows[0].bytes, Some(123));
    assert_eq!(rows[0].danmaku_path, Some(s(&dir.path().join("a.xml"))));
    assert_eq!(
        rows[1].bytes,
        Some(std::fs::metadata(&b).unwrap().len() as i64)
    );
    for (row, path) in rows.iter().zip([&a, &b, &c]) {
        assert_eq!(row.container, "flv");
        assert_eq!(row.index_path, Some(s(&index::index_path(path))));
        assert!(index::index_path(path).exists());
    }
    assert_eq!(rows[3].index_path, None);
    assert!(!index::index_path(&d).exists());
    assert!(!index::index_path(&c_part).exists());

    let links: Vec<(i64, i64)> =
        sqlx::query_as("SELECT session_id, streamerinfo_id FROM session_streamerinfo")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(links, vec![(session_id, 1)]);
}

#[tokio::test]
async fn reopening_within_the_merge_window_resumes_the_session() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let record = |streamerinfo_id: i64, merge: i64, name: &str, at: i64| {
        let pool = pool.clone();
        let path = write_flv(dir.path(), name);
        async move {
            let recorder = SessionRecorder::spawn(pool, target(streamerinfo_id, merge));
            let handle = recorder.handle();
            handle.run_started_at(at);
            handle.opened_at(&path, at);
            handle.closed_at(&path, at + 4000, ClosedSegment::default());
            recorder.finish().await;
        }
    };

    record(1, 10, "a.flv", t0).await;
    // 下播 5 分钟后又开播：同一场，中间记为断流
    record(2, 10, "b.flv", t0 + 4000 + 5 * 60_000).await;
    let all = sessions(&pool).await;
    assert_eq!(all.len(), 1);
    let rows = segments(&pool, all[0].0).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].start_ms, 4000 + 5 * 60_000);
    assert_eq!(rows[1].gap_before_ms, 4000 + 5 * 60_000 - FLV_DURATION_MS);
    assert_eq!(all[0].2, Some(t0 + 8000 + 5 * 60_000));
    let links: Vec<i64> =
        sqlx::query_scalar("SELECT streamerinfo_id FROM session_streamerinfo ORDER BY 1")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(links, vec![1, 2]);

    // 超出窗口：新的一场
    record(3, 10, "c.flv", t0 + 8000 + 16 * 60_000).await;
    assert_eq!(sessions(&pool).await.len(), 2);
    // 窗口为 0：从不合并
    record(4, 0, "d.flv", t0 + 12_000 + 16 * 60_000 + 1000).await;
    let all = sessions(&pool).await;
    assert_eq!(all.len(), 3);
    assert_eq!(segments(&pool, all[2].0).await[0].start_ms, 0);
}

#[tokio::test]
async fn completion_only_downloaders_get_a_start_from_the_content() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let recorder = SessionRecorder::spawn(pool.clone(), target(1, 10));
    let handle = recorder.handle();
    handle.run_started_at(t0);
    let a = write_flv(dir.path(), "a.flv");
    handle.closed_at(&a, t0 + 10_000, ClosedSegment::default());
    let b = write_flv(dir.path(), "b.flv");
    handle.closed_at(&b, t0 + 14_000, ClosedSegment::default());
    recorder.finish().await;

    let (session_id, started_at, _) = sessions(&pool).await[0];
    assert_eq!(started_at, t0 + 10_000 - FLV_DURATION_MS);
    let rows = segments(&pool, session_id).await;
    assert_eq!(
        rows.iter()
            .map(|r| (r.start_ms, r.end_ms, r.gap_before_ms))
            .collect::<Vec<_>>(),
        vec![
            (0, Some(FLV_DURATION_MS), 0),
            (FLV_DURATION_MS, Some(2 * FLV_DURATION_MS), 0)
        ]
    );
}

#[tokio::test]
async fn segments_that_never_reached_the_disk_leave_no_rows() {
    let (dir, pool) = setup().await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(1, 10));
    let handle = recorder.handle();
    handle.run_started();
    handle.opened(&dir.path().join("never.flv.part"));
    handle.run_started();
    let empty = dir.path().join("empty.flv");
    std::fs::write(&empty, b"").unwrap();
    handle.opened(&empty);
    recorder.finish().await;
    assert!(sessions(&pool).await.is_empty());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn open_segment_is_finalized_from_disk_when_the_run_ends() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let recorder = SessionRecorder::spawn(pool.clone(), target(1, 10));
    let handle = recorder.handle();
    handle.run_started_at(t0);
    // 下载器出错退出，没等到关段事件
    let part = write_flv(dir.path(), "a.flv.part");
    handle.opened_at(&part, t0);
    handle.run_started_at(t0 + 60_000);
    recorder.finish().await;

    let (session_id, _, ended_at) = sessions(&pool).await[0];
    let rows = segments(&pool, session_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, s(&part));
    assert_eq!(rows[0].state, SegmentState::Finished);
    assert_eq!(rows[0].end_ms, Some(FLV_DURATION_MS));
    assert_eq!(ended_at, Some(t0 + 60_000));
}

async fn locate_fixture() -> (TempDir, ConnectionPool, i64, Vec<SegmentRow>) {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let recorder = SessionRecorder::spawn(pool.clone(), target(1, 10));
    let handle = recorder.handle();
    for (name, at) in [("a.flv", t0), ("b.flv", t0 + 3965), ("c.flv", t0 + 30_000)] {
        let path = write_flv(dir.path(), name);
        handle.opened_at(&path, at);
        handle.closed_at(&path, at + 4000, ClosedSegment::default());
    }
    recorder.finish().await;
    let session_id = sessions(&pool).await[0].0;
    let rows = segments(&pool, session_id).await;
    assert_eq!(
        rows.iter().map(|r| r.start_ms).collect::<Vec<_>>(),
        vec![0, 3965, 30_000]
    );
    (dir, pool, session_id, rows)
}

#[tokio::test]
async fn locate_maps_session_time_to_a_keyframe_offset() {
    let (_dir, pool, session_id, rows) = locate_fixture().await;
    let flv = build_flv(0, 100, 25, None);

    let hit = locate(&pool, session_id, 1500).await.unwrap().unwrap();
    assert_eq!(hit.segment_id, rows[0].id);
    assert_eq!(hit.keyframe_ms, 1000);
    assert_eq!(hit.offset, flv.keyframes[1].1);
    assert_eq!(hit.header_len, flv.header_len);
    assert_eq!(hit.container, Container::Flv);
    assert_eq!(hit.base_ts, Some(0));

    let hit = locate(&pool, session_id, 3965 + 2500)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hit.segment_id, rows[1].id);
    assert_eq!(hit.keyframe_ms, 3965 + 2000);
    assert_eq!(hit.offset, flv.keyframes[2].1);

    // 落在断流空档：前进到下一段的第一个关键帧
    let hit = locate(&pool, session_id, 10_000).await.unwrap().unwrap();
    assert_eq!(hit.segment_id, rows[2].id);
    assert_eq!(hit.keyframe_ms, 30_000);
    assert_eq!(hit.offset, flv.keyframes[0].1);

    // 晚于最后一段：最后一个关键帧
    let hit = locate(&pool, session_id, 1_000_000).await.unwrap().unwrap();
    assert_eq!(hit.keyframe_ms, 33_000);

    // 从偏移起读就是一个关键帧 tag
    let bytes = std::fs::read(&hit.path).unwrap();
    assert_eq!(bytes[hit.offset as usize], 9);
    assert_eq!(bytes[hit.offset as usize + 11], 0x17);

    assert!(locate(&pool, session_id + 1, 0).await.unwrap().is_none());
}

#[tokio::test]
async fn session_keyframes_span_segments_in_order() {
    let (_dir, pool, session_id, rows) = locate_fixture().await;
    let keys = session_keyframes(&pool, session_id, 2000, 31_000)
        .await
        .unwrap();
    assert_eq!(
        keys.iter()
            .map(|k| (k.t_ms, k.segment_id))
            .collect::<Vec<_>>(),
        vec![
            (2000, rows[0].id),
            (3000, rows[0].id),
            (3965, rows[1].id),
            (4965, rows[1].id),
            (5965, rows[1].id),
            (6965, rows[1].id),
            (30_000, rows[2].id),
            (31_000, rows[2].id),
        ]
    );
}

#[tokio::test]
async fn startup_recovery_finalizes_leftover_recording_segments() {
    let (dir, pool) = setup().await;
    let started_at = 1_700_000_000_000;
    let opened = store::open_session(&pool, 1, Some(1), "标题", started_at, 600_000)
        .await
        .unwrap();

    // 1. 被 kill -9 的 mesio 分段：文件写到一半，索引缓存比文件新
    let flv = build_flv(0, 100, 25, None);
    let killed = dir.path().join("killed.flv");
    std::fs::write(&killed, &flv.bytes).unwrap();
    index::refresh(&killed, false).unwrap();
    let cut = flv.keyframes[3].1 + 20;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&killed)
        .unwrap()
        .set_len(cut)
        .unwrap();
    let killed_id = store::insert_segment(&pool, opened.id, &s(&killed), "flv", 0, 0)
        .await
        .unwrap();
    // 2. 录制中的 `.part` 已经被改名
    let renamed = write_flv(dir.path(), "renamed.flv");
    let renamed_id = store::insert_segment(
        &pool,
        opened.id,
        &format!("{}.part", s(&renamed)),
        "flv",
        10_000,
        5000,
    )
    .await
    .unwrap();
    // 3. 文件从没生成
    let ghost_id = store::insert_segment(
        &pool,
        opened.id,
        &s(&dir.path().join("ghost.flv")),
        "flv",
        20_000,
        0,
    )
    .await
    .unwrap();
    // 另一场一个分段都没有
    sqlx::query(
        "INSERT INTO stream_sessions (streamer_id, title, started_at, created_at)
         VALUES (1, 't', 1, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    recover(&pool).await.unwrap();

    let rows = segments(&pool, opened.id).await;
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![killed_id, renamed_id]
    );
    assert!(!rows.iter().any(|r| r.id == ghost_id));
    let killed_row = &rows[0];
    assert_eq!(killed_row.state, SegmentState::Finished);
    assert_eq!(killed_row.bytes, Some(cut as i64));
    // 截断后最后一个完整 tag 在第三个关键帧之后、第四个之前
    let index = index::load(&killed).unwrap();
    assert!(index.complete);
    assert_eq!(index.keyframes.len(), 3);
    assert!(index.keyframes.iter().all(|k| k.offset < cut));
    assert_eq!(killed_row.end_ms, Some(index.duration_ms as i64));
    assert!(killed_row.end_ms.unwrap() < 3000 + 40);

    let renamed_row = &rows[1];
    assert_eq!(renamed_row.path, s(&renamed));
    assert_eq!(renamed_row.state, SegmentState::Finished);
    assert_eq!(renamed_row.end_ms, Some(10_000 + FLV_DURATION_MS));
    assert_eq!(
        renamed_row.index_path,
        Some(s(&index::index_path(&renamed)))
    );

    let all = sessions(&pool).await;
    assert_eq!(all.len(), 1, "空场次被删掉");
    assert_eq!(all[0].2, Some(started_at + 10_000 + FLV_DURATION_MS));

    // 收尾过的场次可以被很快开播的下一次录制接上
    let resumed = store::open_session(
        &pool,
        1,
        Some(2),
        "标题",
        started_at + 10_000 + FLV_DURATION_MS + 60_000,
        600_000,
    )
    .await
    .unwrap();
    assert!(resumed.resumed);
    assert_eq!(resumed.id, opened.id);
    assert_eq!(resumed.last_end_ms, 10_000 + FLV_DURATION_MS);
}
