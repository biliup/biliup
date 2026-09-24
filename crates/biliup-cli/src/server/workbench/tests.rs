use super::index::tests::build_flv;
use super::recorder::{ClosedSegment, GAP_TOLERANCE_MS, SessionRecorder, SessionTarget, place};
use super::store::{self, OpenedSession, SegmentRow, SegmentState};
use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::StreamerInfo;
use chrono::{DateTime, Utc};
use ormlite::Model;
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
    (dir, pool)
}

fn info(title: &str, at_ms: i64) -> StreamerInfo {
    StreamerInfo::new(
        "a",
        "https://a",
        title,
        DateTime::<Utc>::from_timestamp_millis(at_ms).unwrap(),
        "",
    )
}

/// 监控循环检测到开播：插入或复用场次行。
async fn go_live(pool: &ConnectionPool, at: i64, merge_minutes: i64) -> OpenedSession {
    store::open_session(pool, 1, &info("标题", at), at, merge_minutes * 60_000)
        .await
        .unwrap()
}

fn target(session_id: i64) -> SessionTarget {
    SessionTarget {
        session_id,
        streamer_id: 1,
    }
}

fn write_flv(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, build_flv(0, 100, 25, None).bytes).unwrap();
    path
}

async fn sessions(pool: &ConnectionPool) -> Vec<(i64, Option<i64>, Option<i64>)> {
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
    assert_eq!(place(0, None), (0, 0));
    assert_eq!(place(4000, Some(-50)), (4000, 0));
    assert_eq!(place(4000, Some(200)), (4000, 0));
    assert_eq!(place(4000, Some(GAP_TOLERANCE_MS)), (4000, 0));
    assert_eq!(place(4000, Some(26_000)), (30_000, 26_000));
}

/// 同一连接按时间切段：每段墙钟比内容长一点（开段晚、关段晚），差值不能跨段累积成断流。
#[tokio::test]
async fn wall_clock_drift_between_segments_is_not_a_gap() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let session = go_live(&pool, t0, 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();
    handle.run_started_at(t0);
    let mut at = t0;
    for i in 0..10 {
        let path = write_flv(dir.path(), &format!("{i}.flv"));
        handle.opened_at(&path, at);
        // 内容 3965 ms，墙钟 4600 ms，下一段 150 ms 后才开写：每段偏出 785 ms
        at += 4600;
        handle.closed_at(&path, at, ClosedSegment::default());
        at += 150;
    }
    recorder.finish().await;

    let rows = segments(&pool, session.id).await;
    assert_eq!(rows.len(), 10);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.gap_before_ms, 0, "第 {i} 段");
        assert_eq!(row.start_ms, i as i64 * FLV_DURATION_MS);
    }
}

#[tokio::test]
async fn segments_are_laid_out_on_one_session_timeline() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let session = go_live(&pool, t0, 10).await;
    assert!(!session.resumed);
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();

    // mesio：开段 / 关段都有，关段带时长与字节数；报告的时长和内容对不上时以索引为准
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

    // 小于过滤阈值、关段时已被删掉的分段
    let d = write_flv(dir.path(), "d.flv");
    handle.opened_at(&d, t0 + 34_000);
    std::fs::remove_file(&d).unwrap();
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
    assert_eq!(
        started_at,
        Some(t0 + 500),
        "场次 0 点是第一个分段开写的墙钟"
    );
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
    // 断流从上一段关段（t0 + 8600）算到这一段开写（t0 + 30000）
    let c_gap = 30_000 - 8600;
    let c_start = 2 * FLV_DURATION_MS + c_gap;
    assert_eq!(
        summary,
        vec![
            (s(&a), SegmentState::Finished, 0, Some(FLV_DURATION_MS), 0),
            (
                s(&b),
                SegmentState::Finished,
                FLV_DURATION_MS,
                Some(2 * FLV_DURATION_MS),
                0
            ),
            (
                s(&c),
                SegmentState::Finished,
                c_start,
                Some(c_start + FLV_DURATION_MS),
                c_gap
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
}

#[tokio::test]
async fn reopening_within_the_merge_window_resumes_the_session() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let record = |merge: i64, title: &'static str, name: &str, at: i64| {
        let pool = pool.clone();
        let path = write_flv(dir.path(), name);
        async move {
            let session = store::open_session(&pool, 1, &info(title, at), at, merge * 60_000)
                .await
                .unwrap();
            let recorder = SessionRecorder::spawn(pool, target(session.id), None);
            let handle = recorder.handle();
            handle.run_started_at(at);
            handle.opened_at(&path, at);
            handle.closed_at(&path, at + 4000, ClosedSegment::default());
            recorder.finish().await;
            session
        }
    };

    let first = record(10, "第一次", "a.flv", t0).await;
    // 下播 5 分钟后又开播：复用同一行，断流从上次停下（t0 + 4000）算起
    let second = record(10, "第二次", "b.flv", t0 + 4000 + 5 * 60_000).await;
    assert!(second.resumed);
    assert_eq!(second.id, first.id);
    let all = sessions(&pool).await;
    assert_eq!(all.len(), 1);
    let rows = segments(&pool, first.id).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].gap_before_ms, 5 * 60_000);
    assert_eq!(rows[1].start_ms, FLV_DURATION_MS + 5 * 60_000);
    assert_eq!(all[0].1, Some(t0), "时间轴 0 点不变");
    assert_eq!(all[0].2, Some(t0 + 8000 + 5 * 60_000));
    // 直播历史里还是一条，标题和开播时间取第一次
    let history = StreamerInfo::select().fetch_all(&pool).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].title, "第一次");
    assert_eq!(history[0].date.timestamp_millis(), t0);

    // 超出窗口：新的一场
    let third = record(10, "第三次", "c.flv", t0 + 8000 + 16 * 60_000).await;
    assert!(!third.resumed);
    assert_eq!(sessions(&pool).await.len(), 2);
    // 窗口为 0：从不合并
    let fourth = record(0, "第四次", "d.flv", t0 + 12_000 + 16 * 60_000 + 1000).await;
    assert!(!fourth.resumed);
    let all = sessions(&pool).await;
    assert_eq!(all.len(), 3);
    assert_eq!(segments(&pool, fourth.id).await[0].start_ms, 0);
    let history = StreamerInfo::select().fetch_all(&pool).await.unwrap();
    assert_eq!(
        history.iter().map(|h| h.title.as_str()).collect::<Vec<_>>(),
        vec!["第一次", "第三次", "第四次"]
    );
}

/// 上一场还在录（或崩溃后还没收尾）时不去接它；别的主播的场次也不会被接上。
#[tokio::test]
async fn unfinished_or_foreign_sessions_are_not_resumed() {
    let (_dir, pool) = setup().await;
    sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (2, 'https://b', 'b')")
        .execute(&pool)
        .await
        .unwrap();
    let t0 = 1_700_000_000_000;
    let first = go_live(&pool, t0, 10).await;
    // 刚插入的行 ended_at 为空（正在录）
    let again = go_live(&pool, t0 + 1000, 10).await;
    assert_ne!(again.id, first.id);
    store::close_session(&pool, again.id, t0 + 2000)
        .await
        .unwrap();
    let other = store::open_session(&pool, 2, &info("b", t0 + 3000), t0 + 3000, 600_000)
        .await
        .unwrap();
    assert!(!other.resumed);
    assert_ne!(other.id, again.id);
    let streamer_ids: Vec<Option<i64>> =
        sqlx::query_scalar("SELECT streamer_id FROM stream_sessions ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(streamer_ids, vec![Some(1), Some(1), Some(2)]);
}

#[tokio::test]
async fn completion_only_downloaders_get_a_start_from_the_content() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let session = go_live(&pool, t0, 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();
    handle.run_started_at(t0);
    let a = write_flv(dir.path(), "a.flv");
    handle.closed_at(&a, t0 + 10_000, ClosedSegment::default());
    let b = write_flv(dir.path(), "b.flv");
    handle.closed_at(&b, t0 + 14_000, ClosedSegment::default());
    recorder.finish().await;

    let (session_id, started_at, _) = sessions(&pool).await[0];
    assert_eq!(started_at, Some(t0 + 10_000 - FLV_DURATION_MS));
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
async fn reported_duration_is_used_when_no_index_can_be_built() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let session = go_live(&pool, t0, 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();
    handle.run_started_at(t0);
    let a = dir.path().join("a.mkv");
    std::fs::write(&a, b"not indexable").unwrap();
    handle.opened_at(&a, t0);
    handle.closed_at(
        &a,
        t0 + 9_000,
        ClosedSegment {
            duration_ms: Some(7_500),
            ..Default::default()
        },
    );
    recorder.finish().await;

    let (session_id, _, _) = sessions(&pool).await[0];
    let rows = segments(&pool, session_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].start_ms, rows[0].end_ms), (0, Some(7_500)));
    assert_eq!(rows[0].index_path, None);
}

#[tokio::test]
async fn segments_that_never_reached_the_disk_leave_no_rows() {
    let (dir, pool) = setup().await;
    let session = go_live(&pool, recorder::now_ms(), 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();
    handle.run_started();
    handle.opened(&dir.path().join("never.flv.part"));
    handle.run_started();
    let empty = dir.path().join("empty.flv");
    std::fs::write(&empty, b"").unwrap();
    handle.opened(&empty);
    recorder.finish().await;
    // 场次行照样留着（直播历史里有这一场），只是没有时间轴
    let all = sessions(&pool).await;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].1, None);
    assert!(all[0].2.is_some());
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
    let session = go_live(&pool, t0, 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
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
    let session = go_live(&pool, t0, 10).await;
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), None);
    let handle = recorder.handle();
    for (name, at) in [("a.flv", t0), ("b.flv", t0 + 3965), ("c.flv", t0 + 30_000)] {
        let path = write_flv(dir.path(), name);
        handle.opened_at(&path, at);
        handle.closed_at(&path, at + FLV_DURATION_MS, ClosedSegment::default());
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
    let opened = go_live(&pool, started_at, 10).await;
    store::begin_recording(&pool, opened.id).await.unwrap();
    store::set_started_at(&pool, opened.id, started_at)
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
    // 2. 录制中的 `.part` 已经被改名；最后一次写盘在 started_at + 20 s
    let renamed = write_flv(dir.path(), "renamed.flv");
    let last_write = started_at + 20_000;
    std::fs::File::options()
        .write(true)
        .open(&renamed)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(last_write as u64))
        .unwrap();
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
    // 另一场的分段文件已经被搬走：按时间轴算
    let moved_id: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path)
         VALUES ('c', 'https://c', 't', '2026-09-24T08:00:00Z', '') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    store::set_started_at(&pool, moved_id, started_at)
        .await
        .unwrap();
    let moved_seg = store::insert_segment(&pool, moved_id, "moved-away.flv", "flv", 0, 0)
        .await
        .unwrap();
    store::finish_segment(
        &pool,
        moved_seg,
        &store::FinishedSegment {
            path: "moved-away.flv".into(),
            state: SegmentState::Finished,
            end_ms: 7000,
            bytes: Some(1),
            index_path: None,
            danmaku_path: None,
        },
    )
    .await
    .unwrap();
    // 再一场（别的主播）一个分段都没有
    let empty_date = "2026-09-24T08:00:00.123456789+00:00";
    let empty_id: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path)
         VALUES ('b', 'https://b', 't', ?, '') RETURNING id",
    )
    .bind(empty_date)
    .fetch_one(&pool)
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
    assert_eq!(all.len(), 3, "没有分段的场次也保留（直播历史里有它）");
    let ended_at = last_write;
    assert_eq!(all[0].2, Some(ended_at), "取最后一个分段文件的修改时间");
    assert_eq!(
        all[1],
        (moved_id, Some(started_at), Some(started_at + 7000))
    );
    assert_eq!(
        all[2],
        (
            empty_id,
            None,
            Some(
                DateTime::parse_from_rfc3339(empty_date)
                    .unwrap()
                    .timestamp_millis()
            )
        ),
        "没有分段的记为开播时间"
    );

    // 收尾过的场次可以被很快开播的下一次录制接上
    let resumed = go_live(&pool, ended_at + 60_000, 10).await;
    assert!(resumed.resumed);
    assert_eq!(resumed.id, opened.id);
    let start = store::begin_recording(&pool, resumed.id).await.unwrap();
    assert_eq!(start.started_at, Some(started_at));
    assert_eq!(start.last_end_ms, 10_000 + FLV_DURATION_MS);
    assert_eq!(start.resumed_after, Some(ended_at));
    assert_eq!(sessions(&pool).await[0].2, None, "开始录就清空 ended_at");
}

/// stream-gears 边写 `.part` 边建索引：改名后录制器先等索引任务处理完已发出的事件，
/// 再让 `.idx` 跟着改名；段长取流式建好的索引，关段续扫只做兜底。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streamed_index_follows_the_rename_at_close() {
    let (dir, pool) = setup().await;
    let t0 = 1_700_000_000_000;
    let session = go_live(&pool, t0, 10).await;
    let tap = index::live::spawn();
    let recorder = SessionRecorder::spawn(pool.clone(), target(session.id), Some(tap));
    let handle = recorder.handle();
    let tap = handle.index_tap().expect("index tap");
    handle.run_started_at(t0);

    let part = dir.path().join("a.flv.part");
    let flv = build_flv(0, 100, 25, None);
    std::fs::write(&part, &flv.bytes).unwrap();
    handle.opened_at(&part, t0);
    let file = tap.open(&part);
    let mut offset = 13;
    while offset < flv.bytes.len() {
        let h = &flv.bytes[offset..offset + 11];
        let size = u32::from_be_bytes([0, h[1], h[2], h[3]]) as usize;
        let ts = u32::from_be_bytes([h[7], h[4], h[5], h[6]]);
        let body = bytes::Bytes::copy_from_slice(&flv.bytes[offset + 11..offset + 11 + size]);
        file.flv_tag(offset as u64, h[0], ts, &body);
        offset += 15 + size;
    }
    file.closed(flv.bytes.len() as u64);
    let done = dir.path().join("a.flv");
    std::fs::rename(&part, &done).unwrap();
    handle.closed_at(&done, t0 + 4200, ClosedSegment::default());
    recorder.finish().await;

    let rows = segments(&pool, session.id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].end_ms, Some(FLV_DURATION_MS));
    assert_eq!(rows[0].index_path, Some(s(&index::index_path(&done))));
    assert!(!index::index_path(&part).exists());
    let cached = index::load(&done).unwrap();
    assert!(cached.complete);
    assert_eq!(cached.keyframes.len(), 4);
    assert_eq!(cached.source_len, flv.bytes.len() as u64);
}
