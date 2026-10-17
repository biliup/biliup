use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use axum::body::to_bytes;
use sqlx::sqlite::SqlitePoolOptions;

/// A database from before the workbench migration, holding the recordings an
/// installation of that era would have. Applying the migration must turn them
/// into activity history, so every test here starts from a real upgrade.
async fn migration_fixture() -> ConnectionPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let mut through_14 = sqlx::migrate!();
    through_14.migrations = through_14
        .migrations
        .iter()
        .filter(|migration| migration.version <= 14)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    through_14.run(&pool).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path,started_at,ended_at) VALUES
            (1,'旧主播','https://example.test/1','旧主播','2020-01-01','',1577836800000,1577836810000),
            (2,'第二场','https://example.test/2','第二场','2020-01-01','',1577836800000,1577836810000),
            (3,'无时间','https://example.test/3','无时间','bad-date','',NULL,NULL);
        INSERT INTO segments(id,session_id,path,container,state,start_ms,end_ms) VALUES
            (1,1,'原始录像.flv','flv','finished',0,5000),
            (2,2,'第二场.flv','flv','finished',0,5000);",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn backfill_uses_business_times_and_excludes_recordings_without_one() {
    let pool = migration_fixture().await;
    let events = recent(&pool, 12).await.unwrap();
    assert_eq!(events.len(), 6);
    assert!(
        events
            .iter()
            .all(|event| matches!(event.session_id, Some(1) | Some(2)))
    );
    // Both segments finished at the same timestamp, so select by name.
    let mut files: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == "file")
        .filter_map(|event| event.sub.as_deref())
        .collect();
    files.sort_unstable();
    assert_eq!(files, ["原始录像.flv", "第二场.flv"]);
    let file = events
        .iter()
        .find(|event| event.sub.as_deref() == Some("原始录像.flv"))
        .unwrap();
    assert_eq!(file.ts, 1577836805000);
    // A session whose start time cannot be derived contributes nothing.
    assert!(events.iter().all(|event| event.session_id != Some(3)));
    let response = list(State(pool), Query(ListQuery::default())).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(json["events"].as_array().unwrap().len(), 6);
    // Old activity remains visible well beyond the former 24-hour window.
    assert_eq!(json["events"][0]["ts"], 1577836810000i64);
}

#[tokio::test]
async fn triggers_ignore_progress_updates_and_deduplicate_finished_segments() {
    let pool = migration_fixture().await;
    sqlx::raw_sql(
        "INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path) \
            VALUES(4,'新主播','https://example.test/4','新主播','2026-10-13','');
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=4;
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=4;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=4;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=4;
        INSERT INTO segments(id,session_id,path,container,state,start_ms) \
            VALUES(4,4,'新分段.mp4','mp4','recording',0);
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=4;
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=4;
        UPDATE segments SET state='missing' WHERE id=4;
        UPDATE segments SET state='finished' WHERE id=4;
        INSERT INTO render_jobs(id,session_id,state,phase,spec_json,created_at,updated_at) \
            VALUES(4,4,'queued','queued','{}',1900000010000,1900000010000);
        UPDATE render_jobs SET state='running',updated_at=1900000011000 WHERE id=4;
        UPDATE render_jobs SET phase='编码',ratio=0.5,updated_at=1900000012000 WHERE id=4;
        UPDATE render_jobs SET state='failed',updated_at=1900000013000,finished_at=1900000013000 WHERE id=4;
        UPDATE stream_sessions SET started_at=1577836800000,ended_at=1577836810000 WHERE id=2;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let events = recent(&pool, 100).await.unwrap();
    let own: Vec<_> = events
        .iter()
        .filter(|event| event.session_id == Some(4))
        .collect();
    assert_eq!(own.len(), 6);
    assert_eq!(own.iter().filter(|event| event.kind == "file").count(), 1);
    assert_eq!(own.iter().filter(|event| event.kind == "render").count(), 3);
    // Repeating a session's own times changes nothing; the backfilled history
    // stays as it was written.
    assert_eq!(recent(&pool, 100).await.unwrap().len(), 12);
    // Activity survives deletion of the underlying business record.
    sqlx::query("DELETE FROM stream_sessions WHERE id=4")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(recent(&pool, 100).await.unwrap().len(), 12);
}

#[tokio::test]
async fn history_survives_reopening_the_working_directory_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("data.sqlite3");
    let pool = ConnectionManager::new_pool(db.to_str().unwrap())
        .await
        .unwrap();
    for n in 0..15 {
        sqlx::query(
            "INSERT INTO activity_events(event_key,kind,ts,text) VALUES(?,'file',?,'file')",
        )
        .bind(format!("test:{n}"))
        .bind(1000 + n)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;
    let reopened = ConnectionManager::new_pool(db.to_str().unwrap())
        .await
        .unwrap();
    let events = recent(&reopened, 12).await.unwrap();
    assert_eq!(events.len(), 12);
    assert_eq!(events[0].ts, 1014);
    assert_eq!(events[11].ts, 1003);
    let other = ConnectionManager::new_pool(dir.path().join("another.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    assert!(recent(&other, 12).await.unwrap().is_empty());
    for limit in [0, -1, 101] {
        assert_eq!(
            list(
                State(reopened.clone()),
                Query(ListQuery { limit: Some(limit) })
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
}
