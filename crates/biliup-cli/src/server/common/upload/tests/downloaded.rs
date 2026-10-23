use super::*;
use crate::server::common::downloaded::{self, HookFile};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::context::Worker;

async fn context(dir: &Path, hook: Option<&str>, post: Option<Vec<HookStep>>) -> Context {
    let base = test_context(Config::default());
    let mut room = base.live_streamer().clone();
    room.downloaded_processor = hook.map(|run| {
        vec![HookStep::Run {
            run: run.into(),
            json: true,
        }]
    });
    room.postprocessor = post;
    let pool = ConnectionManager::new_pool(dir.join("db.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    sqlx::query("INSERT INTO stream_sessions (id,name,url,title,date,live_cover_path) VALUES (1,'test','https://www.douyu.com/1','test','2026-10-23 00:00:00','')")
        .execute(&pool).await.unwrap();
    Context::new(
        1,
        Arc::new(Worker::new(
            room,
            None,
            Arc::new(std::sync::RwLock::new(Config::default())),
            Default::default(),
        )),
        pool,
        base.live_stream().clone(),
    )
}

fn echo(text: &str) -> String {
    if cfg!(windows) {
        format!("echo {text}")
    } else {
        format!("printf '%s\\n' '{}'", text.replace('\'', "'\"'\"'"))
    }
}

#[tokio::test]
async fn session_hook_waits_for_all_segments_and_runs_once() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("calls.txt");
    let cmd = format!("{} >> \"{}\"", echo("called"), marker.display());
    let ctx = context(dir.path(), Some(&cmd), Some(vec![])).await;
    let a = dir.path().join("a.mp4");
    let b = dir.path().join("b.webm");
    std::fs::write(&a, b"video").unwrap();
    std::fs::write(&b, b"video").unwrap();
    let (tx, rx) = async_channel::unbounded();
    let (uploaded_tx, uploaded_rx) = async_channel::unbounded();
    let task_ctx = ctx.clone();
    let task = tokio::spawn(async move {
        pipeline_upload_videos(rx, &[], &task_ctx, |path| {
            uploaded_tx.try_send(path.clone()).unwrap();
            async move { Ok(Video::new(&path.to_string_lossy())) }
        })
        .await
        .unwrap()
    });
    tx.send(SegmentInfo::new(a.clone(), None, None, 0))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), uploaded_rx.recv())
            .await
            .is_err()
    );
    assert!(
        !marker.exists(),
        "the first segment is not a session-end event"
    );
    tx.send(SegmentInfo::new(b.clone(), None, None, 1))
        .await
        .unwrap();
    ctx.download_end
        .finish(chrono::Utc::now().timestamp(), true);
    drop(tx);
    let result = task.await.unwrap();
    assert_eq!(result.paths, [a.clone(), b]);
    let duplicate = downloaded::run(
        vec![HookFile {
            video: a,
            danmaku: None,
        }],
        &ctx,
    )
    .await
    .unwrap();
    assert!(
        duplicate.is_empty(),
        "duplicate completion must not upload old inputs"
    );
    assert_eq!(std::fs::read_to_string(marker).unwrap().lines().count(), 1);
}

#[tokio::test]
async fn payload_matches_python_contract_and_uses_recording_end_time() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path(), None, Some(vec![])).await;
    let end = ctx.streamer_info().date.timestamp() + 10;
    ctx.download_end.finish(end, true);
    let payload: serde_json::Value = serde_json::from_slice(&downloaded::payload(
        &[
            HookFile {
                video: "a.mp4".into(),
                danmaku: Some("a.xml".into()),
            },
            HookFile {
                video: "b.mp4".into(),
                danmaku: None,
            },
        ],
        &ctx,
    ))
    .unwrap();
    assert_eq!(payload["url"], ctx.streamer_info().url);
    assert_eq!(payload["end_time"], end);
    assert_eq!(payload["file_list"], serde_json::json!(["a.mp4", "b.mp4"]));
    assert_eq!(payload.as_object().unwrap().len(), 6);
}

#[tokio::test]
async fn hook_reports_exact_replacement_and_unrelated_files_are_never_uploaded() {
    let dir = tempfile::tempdir().unwrap();
    let before = dir.path().join("input.webm");
    let after = dir.path().join("result.mp4");
    let unrelated = dir.path().join("unrelated.mp4");
    for path in [&before, &after, &unrelated] {
        std::fs::write(path, b"video").unwrap();
    }
    let report = format!(
        "BILIUP_OUTPUT={}",
        serde_json::json!({"files":[{"video":after}]})
    );
    let ctx = context(dir.path(), Some(&echo(&report)), Some(vec![])).await;
    let result = pipeline_upload_videos(
        futures::stream::iter([SegmentInfo::new(before.clone(), None, None, 0)]),
        &[],
        &ctx,
        |path| {
            assert_eq!(path, after);
            async move { Ok(Video::new(&path.to_string_lossy())) }
        },
    )
    .await
    .unwrap();
    assert_eq!(result.paths, [after]);
    assert!(before.exists() && unrelated.exists());
}

#[tokio::test]
async fn only_an_absent_postprocessor_gets_default_cleanup() {
    for post in [
        None,
        Some(vec![]),
        Some(vec![HookStep::Run {
            run: echo("notified"),
            json: false,
        }]),
        Some(vec![HookStep::Remove("rm".into())]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let delete = post.is_none()
            || post
                .as_ref()
                .unwrap()
                .iter()
                .any(|s| matches!(s, HookStep::Remove(_)));
        let ctx = context(dir.path(), None, post).await;
        let video = dir.path().join("video.mp4");
        std::fs::write(&video, b"video").unwrap();
        execute_postprocessor(vec![video.clone()], &ctx)
            .await
            .unwrap();
        assert_eq!(video.exists(), !delete);
    }
}

#[tokio::test]
async fn empty_and_sync_sessions_still_get_one_completion_hook() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("calls.txt");
    let cmd = format!("{} >> \"{}\"", echo("called"), marker.display());
    let ctx = context(dir.path(), Some(&cmd), Some(vec![])).await;
    let session = Arc::new(tokio::sync::Mutex::new(
        crate::server::common::sync::SyncSession::default(),
    ));
    for _ in 0..2 {
        crate::server::common::sync::finish_downloaded_session(&session, &ctx)
            .await
            .unwrap();
    }
    assert_eq!(std::fs::read_to_string(marker).unwrap().lines().count(), 1);
}

#[tokio::test]
async fn malformed_result_keeps_original_files_and_prevents_upload() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path(), Some(&echo("BILIUP_OUTPUT={bad}")), Some(vec![])).await;
    let video = dir.path().join("video.mp4");
    std::fs::write(&video, b"video").unwrap();
    let result = pipeline_upload_videos(
        futures::stream::iter([SegmentInfo::new(video.clone(), None, None, 0)]),
        &[],
        &ctx,
        |_| async { panic!("invalid hook output must not be uploaded") },
    )
    .await;
    assert!(result.is_err());
    assert!(video.exists());
}
