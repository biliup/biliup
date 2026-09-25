use super::export::ClipExports;
use super::plan::{self, Attempt, PlanError};
use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::workbench::index::tests::{build_flv, build_fmp4, build_ts};
use crate::server::workbench::index::{self, Container};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
async fn pins_follow_the_clip_range_and_are_released() {
    let (_dir, pool, session, ids) = flv_session().await;
    let pins = |pool: ConnectionPool| async move {
        sqlx::query_scalar::<_, i64>("SELECT pin_count FROM segments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap()
    };
    let clip = insert(
        &pool,
        session,
        &NewClip {
            marker_id: None,
            in_ms: 1000,
            out_ms: 2000,
            title: "t".into(),
            created_by: None,
            created_at: 5,
        },
    )
    .await
    .unwrap();
    assert_eq!(clip.state, State::Draft);
    assert_eq!(pins(pool.clone()).await, vec![1, 0, 0]);

    let changes = ClipChanges {
        out_ms: Some(31_000),
        ..Default::default()
    };
    let UpdateOutcome::Updated(clip) = update(&pool, session, clip.id, &changes, 6).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(clip.out_ms, 31_000);
    assert_eq!(pins(pool.clone()).await, vec![1, 1, 1]);

    // 导出中不能改范围，改标题可以
    begin_export(&pool, clip.id, Mode::Quick, 7)
        .await
        .unwrap()
        .unwrap();
    assert!(
        begin_export(&pool, clip.id, Mode::Quick, 7)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        update(
            &pool,
            session,
            clip.id,
            &ClipChanges {
                in_ms: Some(0),
                ..Default::default()
            },
            8
        )
        .await
        .unwrap(),
        UpdateOutcome::Busy
    );
    assert!(matches!(
        update(
            &pool,
            session,
            clip.id,
            &ClipChanges {
                title: Some("x".into()),
                ..Default::default()
            },
            8
        )
        .await
        .unwrap(),
        UpdateOutcome::Updated(_)
    ));
    assert_eq!(
        update(
            &pool,
            session,
            clip.id,
            &ClipChanges {
                out_ms: Some(0),
                ..Default::default()
            },
            8
        )
        .await
        .unwrap(),
        UpdateOutcome::BadRange
    );

    // 快速剪的入点早于所选入点、落在前一段里：引用跟着扩大
    let done = Exported {
        cut_in_ms: 0,
        cut_out_ms: 32_000,
        output_path: "clips/1/1.flv".into(),
        output_bytes: 1,
        duration_ms: 1,
    };
    assert!(finish_export(&pool, clip.id, &done, 9).await.unwrap());
    let from: i64 = sqlx::query_scalar("SELECT from_ms FROM segment_pins WHERE owner = ?")
        .bind(pin_owner(clip.id))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(from, 0);

    // 改范围：之前的结果作废
    let UpdateOutcome::Updated(changed) = update(
        &pool,
        session,
        clip.id,
        &ClipChanges {
            in_ms: Some(30_500),
            ..Default::default()
        },
        10,
    )
    .await
    .unwrap() else {
        panic!()
    };
    assert_eq!(changed.state, State::Draft);
    assert_eq!(changed.output_path, None);
    assert_eq!(pins(pool.clone()).await, vec![0, 0, 1]);

    assert!(release(&pool, clip.id).await.unwrap());
    assert_eq!(pins(pool.clone()).await, vec![0, 0, 0]);
    assert!(delete(&pool, session, clip.id).await.unwrap().is_some());
    assert!(get(&pool, clip.id).await.unwrap().is_none());
    let _ = ids;
}

#[tokio::test]
async fn interrupted_exports_become_retryable_failures_at_startup() {
    let (_dir, pool, session, _) = flv_session().await;
    let new = NewClip {
        marker_id: None,
        in_ms: 0,
        out_ms: 1000,
        title: String::new(),
        created_by: None,
        created_at: 1,
    };
    let clip = insert(&pool, session, &new).await.unwrap();
    begin_export(&pool, clip.id, Mode::Precise, 2)
        .await
        .unwrap();
    assert_eq!(recover(&pool, 3).await.unwrap(), 1);
    let clip = get(&pool, clip.id).await.unwrap().unwrap();
    assert_eq!(clip.state, State::Failed);
    assert_eq!(clip.error.as_deref(), Some(INTERRUPTED));
    assert_eq!(clip.mode, Some(Mode::Precise));
    assert!(
        begin_export(&pool, clip.id, Mode::Quick, 4)
            .await
            .unwrap()
            .is_some()
    );
}

async fn wait_done(pool: &ConnectionPool, id: i64) -> Clip {
    for _ in 0..500 {
        let clip = get(pool, id).await.unwrap().unwrap();
        if clip.state != State::Exporting {
            return clip;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("导出没有结束");
}

#[tokio::test]
async fn quick_export_runs_in_the_background_and_failures_are_retryable() {
    let (dir, pool, session, ids) = flv_session().await;
    let exports = Arc::new(ClipExports::new(pool.clone(), dir.path().join("clips")));
    let new = NewClip {
        marker_id: None,
        in_ms: 3500,
        out_ms: 31_000,
        title: String::new(),
        created_by: None,
        created_at: 1,
    };
    let clip = insert(&pool, session, &new).await.unwrap();

    sqlx::query("UPDATE segments SET state = 'missing' WHERE id = ?")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    let exporting = begin_export(&pool, clip.id, Mode::Quick, 2)
        .await
        .unwrap()
        .unwrap();
    exports.start(exporting);
    let failed = wait_done(&pool, clip.id).await;
    assert_eq!(failed.state, State::Failed);
    assert!(
        failed.error.as_deref().unwrap().contains("文件丢失"),
        "{failed:?}"
    );
    assert!(exports.progress(clip.id).is_none());

    sqlx::query("UPDATE segments SET state = 'finished' WHERE id = ?")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    let exporting = begin_export(&pool, clip.id, Mode::Quick, 3)
        .await
        .unwrap()
        .unwrap();
    exports.start(exporting);
    let done = wait_done(&pool, clip.id).await;
    assert_eq!(done.state, State::Ready, "{done:?}");
    assert_eq!(
        (done.cut_in_ms, done.cut_out_ms),
        (Some(3000), Some(31_000))
    );
    assert_eq!(done.error, None);
    let path = PathBuf::from(done.output_path.as_deref().unwrap());
    assert_eq!(path, exports.dir(session).join(format!("{}.flv", clip.id)));
    assert_eq!(done.file_name(), Some(format!("{}.flv", clip.id).as_str()));
    assert_eq!(
        std::fs::metadata(&path).unwrap().len() as i64,
        done.output_bytes.unwrap()
    );
    // 索引的段长算到最后一个 tag 的时间戳（音频，比最后一帧视频晚 5 ms），视频要再多一帧：
    // 每个接缝处后一段往后让 35 ms，保证视频时间戳严格递增
    assert_eq!(done.duration_ms, Some(965 + FLV_MS + 1000 + 2 * 35));
    assert!(!path.with_extension("flv.part").exists());

    exports.remove_outputs(session, clip.id).await;
    assert!(!path.exists());
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

#[tokio::test]
async fn recognizes_flv_that_carries_hevc_as_codec_12() {
    let dir = tempfile::tempdir().unwrap();
    let flv = |codec: u8| {
        let body = [0x10 | codec, 0, 0, 0, 0, 1, 2, 3];
        let mut out = b"FLV\x01\x01\x00\x00\x00\x09\x00\x00\x00\x00".to_vec();
        out.push(9);
        out.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        out.extend_from_slice(&[0; 7]);
        out.extend_from_slice(&body);
        out.extend_from_slice(&(11 + body.len() as u32).to_be_bytes());
        out
    };
    let hevc = write(dir.path(), "hevc.flv", &flv(12));
    let avc = write(dir.path(), "avc.flv", &flv(7));
    assert!(export::legacy_hevc_flv(&hevc).await);
    assert!(!export::legacy_hevc_flv(&avc).await);
    assert!(!export::legacy_hevc_flv(&dir.path().join("missing.flv")).await);
}

#[cfg(unix)]
#[test]
fn ffmpeg_failure_skips_loader_noise_and_names_interruptions() {
    use super::export::ffmpeg_message;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    let noise =
        "ffmpeg: /lib/libncursesw.so.6: no version information available (required by ffmpeg)\n";
    let interrupted = ffmpeg_message(noise, Some(ExitStatus::from_raw(255 << 8)));
    assert!(interrupted.contains("被中止"), "{interrupted}");
    let killed = ffmpeg_message(noise, Some(ExitStatus::from_raw(9)));
    assert!(killed.contains("被中止"), "{killed}");
    let real = ffmpeg_message(
        &format!("{noise}pipe:: Invalid data found when processing input\n"),
        Some(ExitStatus::from_raw(1 << 8)),
    );
    assert_eq!(
        real,
        "FFmpeg 转码失败：pipe:: Invalid data found when processing input"
    );
}
