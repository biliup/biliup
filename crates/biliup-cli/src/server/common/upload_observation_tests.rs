use super::*;

async fn pool() -> ConnectionPool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE activity_events(event_key TEXT PRIMARY KEY,kind TEXT NOT NULL,ts INTEGER NOT NULL,text TEXT NOT NULL,sub TEXT,session_id INTEGER)")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE upload_observation_tasks(id TEXT PRIMARY KEY,source TEXT NOT NULL,scheduler_group TEXT NOT NULL,task_order INTEGER NOT NULL,state TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,snapshot TEXT NOT NULL)")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE upload_observation_files(id TEXT PRIMARY KEY,task_id TEXT NOT NULL REFERENCES upload_observation_tasks(id) ON DELETE CASCADE,file_order INTEGER NOT NULL,snapshot TEXT NOT NULL)")
        .execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn records_confirmed_ranges_partial_results_and_restart_recovery() {
    let pool = pool().await;
    let observations = UploadObservations::new(pool.clone());
    observations.initialize().await.unwrap();
    let guard = observations.register(NewTask {
        source: "sync",
        scheduler_group: "sync",
        title: "live",
        streamer_name: Some("room"),
        session_id: Some(7),
        template_name: Some("template"),
        fifo: true,
    });
    let task = guard.observation();
    let first = task.file("first.mp4", Some(100));
    first.confirmed_range(0, 80);
    first.confirmed_range(40, 60);
    first.finish(None);
    let second = task.file("second.mp4", Some(50));
    second.confirmed_range(0, 10);
    second.finish(Some("upload failed"));
    guard.finish("succeeded", None, Some("BV123"));
    observations.flush().await;

    let detail = observations.detail(task.id()).await.unwrap().unwrap();
    assert_eq!(detail.task.state, "partial");
    assert_eq!(detail.task.bytes_confirmed, 110);
    assert_eq!(detail.files[0].bytes_confirmed, 100);
    assert_eq!(detail.files[1].state, "failed");
    assert_eq!(detail.task.files_succeeded, 1);
    assert_eq!(detail.task.files_failed, 1);

    let restarted = UploadObservations::new(pool);
    restarted.initialize().await.unwrap();
    let recovered = restarted.detail(task.id()).await.unwrap().unwrap();
    assert_eq!(recovered.task.state, "partial");
    assert_eq!(recovered.files[0].bytes_confirmed, 100);
}

#[tokio::test]
async fn interruption_during_submission_is_unknown_and_never_retried() {
    let pool = pool().await;
    let observations = UploadObservations::new(pool.clone());
    observations.initialize().await.unwrap();
    let guard = observations.register(NewTask {
        source: "manual",
        scheduler_group: "manual",
        title: "manual upload",
        streamer_name: None,
        session_id: None,
        template_name: None,
        fifo: false,
    });
    let id = guard.observation().id().to_owned();
    guard.observation().submitting().await;
    std::mem::forget(guard);
    observations.flush().await;

    let restarted = UploadObservations::new(pool);
    restarted.initialize().await.unwrap();
    let task = restarted.detail(&id).await.unwrap().unwrap();
    assert_eq!(task.task.state, "unknown");
    assert!(task.task.error.unwrap().contains("不会重复投稿"));
}

#[tokio::test]
async fn explicit_ha_skip_stays_skipped_when_upload_pipeline_returns_ok() {
    let pool = pool().await;
    let observations = UploadObservations::new(pool);
    observations.initialize().await.unwrap();
    let guard = observations.register(NewTask {
        source: "ha",
        scheduler_group: "automatic-pool",
        title: "standby",
        streamer_name: None,
        session_id: Some(1),
        template_name: None,
        fifo: false,
    });
    let id = guard.observation().id().to_owned();
    guard
        .observation()
        .finish("skipped", Some("已交由备机处理"), None);
    guard.finish_result(&Ok::<(), String>(()));
    assert_eq!(
        observations.detail(&id).await.unwrap().unwrap().task.state,
        "skipped"
    );

    let guard = observations.register(NewTask {
        source: "ha",
        scheduler_group: "automatic-pool",
        title: "standby drop",
        streamer_name: None,
        session_id: Some(2),
        template_name: None,
        fifo: false,
    });
    let drop_id = guard.observation().id().to_owned();
    guard
        .observation()
        .finish("skipped", Some("备机处理"), None);
    drop(guard);
    assert_eq!(
        observations
            .detail(&drop_id)
            .await
            .unwrap()
            .unwrap()
            .task
            .state,
        "skipped"
    );

    let guard = observations.register(NewTask {
        source: "ha",
        scheduler_group: "ha-pool",
        title: "fenced",
        streamer_name: None,
        session_id: Some(3),
        template_name: None,
        fifo: false,
    });
    let fenced_id = guard.observation().id().to_owned();
    guard
        .observation()
        .finish("skipped", Some("提交前已由备机接手"), None);
    guard.finish_result(&Err::<(), _>("pipeline error"));
    assert_eq!(
        observations
            .detail(&fenced_id)
            .await
            .unwrap()
            .unwrap()
            .task
            .state,
        "skipped"
    );
}

#[tokio::test]
async fn old_history_is_paginated_from_sql_without_growing_the_live_cache() {
    let pool = pool().await;
    let observations = UploadObservations::new(pool);
    observations.initialize().await.unwrap();
    let mut first = observations.register(NewTask {
        source: "clip",
        scheduler_group: "clip-publish",
        title: "retryable history",
        streamer_name: None,
        session_id: None,
        template_name: None,
        fifo: true,
    });
    first.pin();
    let first_id = first.observation().id().to_owned();
    first.finish_and_keep("failed", Some("try again"), None);
    for _ in 1..503 {
        let guard = observations.register(NewTask {
            source: "manual",
            scheduler_group: "manual",
            title: "history",
            streamer_name: None,
            session_id: None,
            template_name: None,
            fifo: false,
        });
        guard.finish("succeeded", None, None);
    }
    observations.flush().await;
    assert_eq!(
        lock(&observations.inner).tasks.len(),
        CACHED_TERMINAL_TASKS + 1
    );
    let old = observations.detail(&first_id).await.unwrap().unwrap();
    assert_eq!(old.task.state, "failed");
    let page = observations.list(None, None, 10, 500).await;
    assert_eq!(page.total, 503);
    assert_eq!(page.tasks.len(), 3);
    first.reopen();
    first.observation().requeue();
    observations.flush().await;
    assert_eq!(
        observations
            .detail(&first_id)
            .await
            .unwrap()
            .unwrap()
            .task
            .state,
        "queued"
    );
}

#[test]
fn scrub_hides_upos_credentials_and_url_userinfo() {
    for message in [
        r#"{"auth":"secret-value","message":"failed"}"#,
        r#"ResponseData { token_info: { access_token: "secret-value" } }"#,
        "upload failed at https://user:password@example.test/video.mp4",
    ] {
        let scrubbed = scrub(message);
        assert!(!scrubbed.contains("secret-value"));
        assert!(!scrubbed.contains("password@example"));
        assert!(scrubbed.contains("已隐藏"));
    }
}
