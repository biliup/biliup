use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::infrastructure::models::hook_step::{HookStep, process_video};
use crate::server::workbench::store::{self, FinishedSegment, SegmentState};
use chrono::{DateTime, Utc};
use tempfile::TempDir;

const HOUR: i64 = HOUR_MS;

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

/// 新开一场，时间轴 0 点为 `started_at`。
async fn session(pool: &ConnectionPool, started_at: i64) -> i64 {
    let info = StreamerInfo::new(
        "a",
        "https://a",
        "标题",
        DateTime::<Utc>::from_timestamp_millis(started_at).unwrap(),
        "",
    );
    let id = store::open_session(pool, 1, &info, started_at, 0)
        .await
        .unwrap()
        .id;
    store::set_started_at(pool, id, started_at).await.unwrap();
    id
}

/// 在场次时间 `[start, end)` 写一个分段文件（`end` 为 `None` 表示还在录），带索引缓存与同名弹幕。
async fn segment(
    pool: &ConnectionPool,
    dir: &Path,
    session_id: i64,
    name: &str,
    start: i64,
    end: Option<i64>,
    bytes: usize,
) -> (i64, PathBuf) {
    let path = dir.join(name);
    std::fs::write(&path, vec![0u8; bytes]).unwrap();
    std::fs::write(index::index_path(&path), b"idx").unwrap();
    std::fs::write(path.with_extension("xml"), b"<i></i>").unwrap();
    let id = store::insert_segment(pool, session_id, &path_string(&path), "flv", start, 0)
        .await
        .unwrap();
    if let Some(end) = end {
        finish(pool, id, &path, end, SegmentState::Finished).await;
    }
    (id, path)
}

async fn finish(pool: &ConnectionPool, id: i64, path: &Path, end: i64, state: SegmentState) {
    store::finish_segment(
        pool,
        id,
        &FinishedSegment {
            path: path_string(path),
            state,
            end_ms: end,
            bytes: std::fs::metadata(path).ok().map(|m| m.len() as i64),
            index_path: Some(path_string(&index::index_path(path))),
            danmaku_path: None,
        },
    )
    .await
    .unwrap();
}

async fn row(pool: &ConnectionPool, id: i64) -> (String, i64, Option<i64>, Option<String>) {
    sqlx::query_as("SELECT state, pin_count, delete_after, danmaku_path FROM segments WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn state(pool: &ConnectionPool, id: i64) -> String {
    row(pool, id).await.0
}

async fn pins(pool: &ConnectionPool, session_id: i64) -> Vec<i64> {
    sqlx::query_scalar("SELECT pin_count FROM segments WHERE session_id = ? ORDER BY start_ms")
        .bind(session_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

fn gone(path: &Path) -> bool {
    !path.exists() && !index::index_path(path).exists()
}

fn retention(pool: &ConnectionPool, keep_for_ms: i64) -> Retention {
    Retention {
        pool: pool.clone(),
        keep_for_ms,
    }
}

#[tokio::test]
async fn pins_follow_time_ranges_including_the_live_tail() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    segment(&pool, dir.path(), s, "b.flv", 1000, Some(2000), 10).await;
    let (c, c_path) = segment(&pool, dir.path(), s, "c.flv", 2000, None, 10).await;
    let mut conn = pool.acquire().await.unwrap();

    pin(&mut conn, "marker:1", s, 500, 1500).await.unwrap();
    assert_eq!(pins(&pool, s).await, vec![1, 1, 0]);
    // 伸进仍在录的尾部：还没结束的分段算到无穷远
    pin(&mut conn, "marker:2", s, 1800, 9000).await.unwrap();
    assert_eq!(pins(&pool, s).await, vec![1, 2, 1]);
    // 同一个引用方再登记是改区间，不重复计数；起止写反了也一样
    pin(&mut conn, "marker:1", s, 2600, 2500).await.unwrap();
    assert_eq!(pins(&pool, s).await, vec![0, 1, 2]);

    // 尾部之后新写出来的分段落在 marker:2 的区间里，同样被引用
    finish(&pool, c, &c_path, 3000, SegmentState::Finished).await;
    segment(&pool, dir.path(), s, "d.flv", 3000, None, 10).await;
    assert_eq!(pins(&pool, s).await, vec![0, 1, 2, 1]);

    assert!(unpin(&mut conn, "marker:2").await.unwrap());
    assert_eq!(pins(&pool, s).await, vec![0, 0, 1, 0]);
    assert!(!unpin(&mut conn, "marker:2").await.unwrap());
    // 区间端点正好落在分段边界：只算后一段
    pin(&mut conn, "marker:3", s, 1000, 1000).await.unwrap();
    assert_eq!(pins(&pool, s).await, vec![0, 1, 1, 0]);
}

#[tokio::test]
async fn pin_moves_between_sessions_and_accepts_a_transaction() {
    let (dir, pool) = setup().await;
    let s1 = session(&pool, 1_000_000).await;
    segment(&pool, dir.path(), s1, "a.flv", 0, Some(1000), 10).await;
    store::close_session(&pool, s1, 1_001_000).await.unwrap();
    let s2 = session(&pool, 9_000_000).await;
    segment(&pool, dir.path(), s2, "b.flv", 0, Some(1000), 10).await;

    let mut tx = pool.begin().await.unwrap();
    pin(&mut tx, "clip:1", s1, 0, 100).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(pins(&pool, s1).await, vec![0]);

    let mut tx = pool.begin().await.unwrap();
    pin(&mut tx, "clip:1", s1, 0, 100).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(pins(&pool, s1).await, vec![1]);

    let mut conn = pool.acquire().await.unwrap();
    pin(&mut conn, "clip:1", s2, 0, 100).await.unwrap();
    assert_eq!(pins(&pool, s1).await, vec![0]);
    assert_eq!(pins(&pool, s2).await, vec![1]);
}

/// 保留期为 0、没有引用：`rm` 与以前完全一样，立即删视频、弹幕和索引；分段行标 `deleted`。
#[tokio::test]
async fn unreferenced_segments_are_removed_immediately_without_retention() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let xml = a_path.with_extension("xml");
    let untracked = dir.path().join("old.flv");
    std::fs::write(&untracked, b"v").unwrap();
    std::fs::write(index::index_path(&untracked), b"i").unwrap();

    let outcome = remove(&retention(&pool, 0), &[&a_path, &xml, &untracked])
        .await
        .unwrap();

    assert_eq!(
        outcome,
        vec![Disposal::Deleted, Disposal::Untracked, Disposal::Untracked]
    );
    assert!(gone(&a_path) && !xml.exists() && gone(&untracked));
    assert_eq!(state(&pool, a).await, "deleted");
    assert_eq!(
        sweep_pending(&pool, now_ms() + 1000 * HOUR).await.unwrap(),
        0
    );
}

/// 同一份后处理配置，给不给数据库结果相同：不用工作台、保留期 0 的用户行为不变。
#[tokio::test]
async fn rm_step_matches_legacy_behaviour_when_retention_is_off() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (_, tracked) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let legacy = dir.path().join("legacy.flv");
    std::fs::write(&legacy, b"v").unwrap();
    std::fs::write(index::index_path(&legacy), b"i").unwrap();
    std::fs::write(legacy.with_extension("xml"), b"d").unwrap();
    let rm = [HookStep::Remove("rm".into())];

    process_video(
        &[&tracked, &tracked.with_extension("xml")],
        &rm,
        Some(&Retention::after_upload(pool.clone(), &Config::default())),
    )
    .await
    .unwrap();
    process_video(&[&legacy, &legacy.with_extension("xml")], &rm, None)
        .await
        .unwrap();

    for path in [&tracked, &legacy] {
        assert!(gone(path) && !path.with_extension("xml").exists());
    }
    // 删不掉（文件已不在）时照旧报错
    assert!(
        process_video(&[&legacy], &rm, Some(&retention(&pool, 0)))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn retention_period_defers_until_the_sweeper_runs_after_expiry() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let xml = a_path.with_extension("xml");
    let before = now_ms();

    let outcome = remove(&retention(&pool, 24 * HOUR), &[&a_path, &xml])
        .await
        .unwrap();

    assert_eq!(outcome, vec![Disposal::Deferred, Disposal::Deferred]);
    let (state, _, delete_after, danmaku) = row(&pool, a).await;
    assert_eq!(state, "pending_delete");
    let delete_after = delete_after.unwrap();
    assert!(delete_after >= before + 24 * HOUR && delete_after <= now_ms() + 24 * HOUR);
    assert_eq!(danmaku.as_deref(), Some(path_string(&xml).as_str()));
    assert!(a_path.exists() && xml.exists() && index::index_path(&a_path).exists());

    assert_eq!(sweep_pending(&pool, delete_after - 1).await.unwrap(), 0);
    assert!(a_path.exists());
    assert_eq!(sweep_pending(&pool, delete_after).await.unwrap(), 1);
    assert!(gone(&a_path) && !xml.exists());
    assert_eq!(
        super::super::store::session_segments(&pool, s)
            .await
            .unwrap()[0]
            .state,
        SegmentState::Deleted
    );
}

#[tokio::test]
async fn pinned_segments_wait_for_the_last_reference() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let (b, b_path) = segment(&pool, dir.path(), s, "b.flv", 1000, Some(2000), 10).await;
    let mut conn = pool.acquire().await.unwrap();
    pin(&mut conn, "marker:1", s, 200, 300).await.unwrap();
    pin(&mut conn, "clip:1", s, 100, 900).await.unwrap();

    let outcome = remove(&retention(&pool, 0), &[&a_path, &b_path])
        .await
        .unwrap();
    assert_eq!(outcome, vec![Disposal::Deferred, Disposal::Deleted]);
    assert_eq!(row(&pool, a).await.2, None, "只等引用，不按时间等");
    assert!(a_path.exists() && gone(&b_path));
    assert_eq!(state(&pool, b).await, "deleted");

    let later = now_ms() + 1000 * HOUR;
    unpin(&mut conn, "marker:1").await.unwrap();
    assert_eq!(sweep_pending(&pool, later).await.unwrap(), 0);
    assert!(a_path.exists());
    unpin(&mut conn, "clip:1").await.unwrap();
    assert_eq!(sweep_pending(&pool, now_ms()).await.unwrap(), 1);
    assert!(gone(&a_path));
    assert!(
        a_path.with_extension("xml").exists(),
        "弹幕文件不在这次 rm 的列表里，也没有记在分段上，不删"
    );
    assert_eq!(state(&pool, a).await, "deleted");
}

#[tokio::test]
async fn keep_this_session_holds_segments_until_retain_until() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let until = now_ms() + HOUR;
    assert!(set_session_retention(&pool, s, Some(until)).await.unwrap());
    assert!(
        !set_session_retention(&pool, 999, Some(until))
            .await
            .unwrap()
    );

    assert_eq!(
        remove(&retention(&pool, 0), &[&a_path]).await.unwrap(),
        vec![Disposal::Deferred]
    );
    assert_eq!(sweep_pending(&pool, until - 1).await.unwrap(), 0);
    assert_eq!(sweep_pending(&pool, until).await.unwrap(), 1);
    assert_eq!(state(&pool, a).await, "deleted");

    // 取消保留后，下一轮清理就删
    let (b, b_path) = segment(&pool, dir.path(), s, "b.flv", 1000, Some(2000), 10).await;
    set_session_retention(&pool, s, Some(i64::MAX))
        .await
        .unwrap();
    remove(&retention(&pool, 0), &[&b_path]).await.unwrap();
    assert_eq!(sweep_pending(&pool, now_ms()).await.unwrap(), 0);
    set_session_retention(&pool, s, None).await.unwrap();
    assert_eq!(sweep_pending(&pool, now_ms()).await.unwrap(), 1);
    assert_eq!(state(&pool, b).await, "deleted");
}

/// 过滤删除和关段几乎同时发生：删除点先处理、关段事件后落库时，删除点定下的状态不被覆盖。
#[tokio::test]
async fn late_close_event_keeps_the_deletion_decision() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, None, 10).await;
    let (b, b_path) = segment(&pool, dir.path(), s, "b.flv", 5000, None, 10).await;
    let mut conn = pool.acquire().await.unwrap();
    pin(&mut conn, "marker:1", s, 100, 200).await.unwrap();

    let filter = Retention::without_delay(pool.clone());
    assert_eq!(
        remove(&filter, &[&a_path]).await.unwrap(),
        vec![Disposal::Deferred]
    );
    finish(&pool, a, &a_path, 1000, SegmentState::Deleted).await;
    assert_eq!(state(&pool, a).await, "pending_delete");

    unpin(&mut conn, "marker:1").await.unwrap();
    assert_eq!(
        remove(&filter, &[&b_path]).await.unwrap(),
        vec![Disposal::Deleted]
    );
    finish(&pool, b, &b_path, 6000, SegmentState::Missing).await;
    assert_eq!(state(&pool, b).await, "deleted");
}

#[tokio::test]
async fn mv_moves_the_index_and_updates_segment_paths() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    let xml = a_path.with_extension("xml");
    sqlx::query("UPDATE segments SET danmaku_path = ? WHERE id = ?")
        .bind(path_string(&xml))
        .bind(a)
        .execute(&pool)
        .await
        .unwrap();
    let target = dir.path().join("archive");
    let mv = HookStep::Move {
        mv: target.display().to_string(),
    };

    mv.execute_with_retention(&[&a_path, &xml], Some(&retention(&pool, 0)))
        .await
        .unwrap();

    let moved_video = target.join("a.flv");
    let row = &store::session_segments(&pool, s).await.unwrap()[0];
    assert_eq!(row.path, path_string(&moved_video));
    assert_eq!(
        row.index_path.as_deref(),
        Some(path_string(&index::index_path(&moved_video)).as_str())
    );
    assert_eq!(
        row.danmaku_path.as_deref(),
        Some(path_string(&target.join("a.xml")).as_str())
    );
    assert_eq!(row.state, SegmentState::Finished);
    assert!(gone(&a_path) && !xml.exists());
    assert_eq!(
        std::fs::read(index::index_path(&moved_video)).unwrap(),
        b"idx"
    );

    // 搬走之后再 rm，按新路径找到分段
    remove(&retention(&pool, 0), &[&moved_video]).await.unwrap();
    assert_eq!(state(&pool, a).await, "deleted");
}

/// 可用空间 = 容量 - 目录里还在的文件大小。
fn fake_disk(capacity: u64) -> impl FnMut(&Path) -> io::Result<u64> {
    move |dir: &Path| {
        let used: u64 = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok()?.metadata().ok())
            .map(|m| m.len())
            .sum();
        Ok(capacity.saturating_sub(used))
    }
}

#[tokio::test]
async fn low_disk_deletes_unreferenced_oldest_first_and_never_the_live_segment() {
    let (dir, pool) = setup().await;
    let disk = dir.path().join("disk");
    std::fs::create_dir(&disk).unwrap();
    const MB: usize = 1 << 20;
    // 场次 1（最早）：录制中的分段最旧，其后一段被引用，再一段没被引用
    let s1 = session(&pool, 1_000_000).await;
    let (live, live_path) = segment(&pool, &disk, s1, "s1-live.flv", 0, None, 10 * MB).await;
    let (pinned, pinned_path) = segment(
        &pool,
        &disk,
        s1,
        "s1-pinned.flv",
        60_000,
        Some(120_000),
        10 * MB,
    )
    .await;
    let (old, old_path) = segment(
        &pool,
        &disk,
        s1,
        "s1-old.flv",
        120_000,
        Some(180_000),
        10 * MB,
    )
    .await;
    // 场次 2（更晚）：两段都没被引用，其中一段已在等保留期
    let s2 = session(&pool, 5_000_000).await;
    let (newer, newer_path) = segment(&pool, &disk, s2, "s2-a.flv", 0, Some(60_000), 10 * MB).await;
    let (newest, newest_path) =
        segment(&pool, &disk, s2, "s2-b.flv", 60_000, Some(120_000), 10 * MB).await;
    remove(&retention(&pool, 24 * HOUR), &[&newest_path])
        .await
        .unwrap();
    // 没有分段记录的文件不碰
    let untracked = disk.join("untracked.flv");
    std::fs::write(&untracked, vec![0u8; 10 * MB]).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    pin(&mut conn, "clip:1", s1, 70_000, 80_000).await.unwrap();
    assert_eq!(
        row(&pool, live).await.1,
        1,
        "引用伸到录制中的分段上也不删它"
    );
    // 容量 70 MB，用了约 60 MB，要求至少 25 MB 可用：得删两段
    let capacity = 70 * MB as u64;
    let deleted = enforce_free_space(&pool, 25 * MB as u64, now_ms(), fake_disk(capacity))
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    assert!(gone(&old_path) && gone(&newer_path));
    assert_eq!(state(&pool, old).await, "deleted");
    assert_eq!(state(&pool, newer).await, "deleted");
    assert!(
        pinned_path.exists() && newest_path.exists() && live_path.exists() && untracked.exists()
    );

    // 还不够时才动被引用的，录制中的始终不删
    let deleted = enforce_free_space(&pool, 60 * MB as u64, now_ms(), fake_disk(capacity))
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    assert!(gone(&newest_path) && gone(&pinned_path));
    assert_eq!(state(&pool, pinned).await, "deleted");
    assert_eq!(state(&pool, newest).await, "deleted");
    assert!(live_path.exists() && untracked.exists());
    assert_eq!(state(&pool, live).await, "recording");
}

#[tokio::test]
async fn low_disk_order_counts_kept_sessions_as_referenced() {
    let (dir, pool) = setup().await;
    let disk = dir.path().join("disk");
    std::fs::create_dir(&disk).unwrap();
    let s1 = session(&pool, 1_000_000).await;
    let (_, kept) = segment(&pool, &disk, s1, "kept.flv", 0, Some(1000), 1000).await;
    set_session_retention(&pool, s1, Some(i64::MAX))
        .await
        .unwrap();
    let s2 = session(&pool, 2_000_000).await;
    let (_, later) = segment(&pool, &disk, s2, "later.flv", 0, Some(1000), 1000).await;
    // 两段各约 1 KB，容量 2500 B、要求 600 B 可用：删一段就够
    let deleted = enforce_free_space(&pool, 600, now_ms(), fake_disk(2500))
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(kept.exists() && gone(&later));
}

/// 默认配置（保留期 0、不设水位）下清理任务什么都不删。
#[tokio::test]
async fn default_config_sweep_touches_nothing() {
    let (dir, pool) = setup().await;
    let config = Config::default();
    assert_eq!(config.retention_hours, 0);
    assert_eq!(config.min_free_space, None);
    let s = session(&pool, 1_000_000).await;
    let (a, a_path) = segment(&pool, dir.path(), s, "a.flv", 0, Some(1000), 10).await;
    sweep_once(&pool, config.min_free_space).await.unwrap();
    assert!(a_path.exists());
    assert_eq!(state(&pool, a).await, "finished");
}

#[test]
fn available_space_reports_the_current_disk() {
    assert!(available_space(Path::new(".")).unwrap() > 0);
}
