use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use axum::body::to_bytes;
use sqlx::sqlite::SqlitePoolOptions;

async fn migration_fixture() -> ConnectionPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE stream_sessions(id INTEGER PRIMARY KEY,name TEXT,date TEXT,kind TEXT,started_at INTEGER,ended_at INTEGER);
        CREATE TABLE segments(id INTEGER PRIMARY KEY,session_id INTEGER,path TEXT,state TEXT,start_ms INTEGER,end_ms INTEGER);
        CREATE TABLE render_jobs(id INTEGER PRIMARY KEY,session_id INTEGER,state TEXT,phase TEXT,ratio REAL,created_at INTEGER,updated_at INTEGER,finished_at INTEGER);
        INSERT INTO stream_sessions VALUES(1,'旧主播','2020-01-01','source',1577836800000,1577836810000);
        INSERT INTO stream_sessions VALUES(2,'合并场次','2020-01-01','group',1577836800000,1577836810000);
        INSERT INTO stream_sessions VALUES(3,'无时间','bad-date','source',NULL,NULL);
        INSERT INTO segments VALUES(1,1,'原始录像.flv','finished',0,5000);
        INSERT INTO segments VALUES(2,2,'group.mp4','finished',0,5000);
        INSERT INTO render_jobs VALUES(1,1,'ready','ready',1,1577836811000,1577836819000,1577836819000);")
        .execute(&pool).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/23_activity_events.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn backfill_uses_business_times_and_excludes_virtual_recordings() {
    let pool = migration_fixture().await;
    let events = recent(&pool, 12).await.unwrap();
    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|event| event.session_id == Some(1)));
    let file = events.iter().find(|event| event.kind == "file").unwrap();
    assert_eq!(file.ts, 1577836805000);
    assert_eq!(file.sub.as_deref(), Some("原始录像.flv"));
    let response = list(State(pool), Query(ListQuery::default())).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(json["events"].as_array().unwrap().len(), 4);
    // Old activity remains visible well beyond the former 24-hour window.
    assert_eq!(json["events"][0]["ts"], 1577836819000i64);
}

#[tokio::test]
async fn triggers_ignore_progress_updates_and_deduplicate_finished_segments() {
    let pool = migration_fixture().await;
    sqlx::raw_sql("INSERT INTO stream_sessions VALUES(4,'新主播','2026-10-13','source',NULL,NULL);
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=4;
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=4;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=4;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=4;
        INSERT INTO segments VALUES(4,4,'新分段.mp4','recording',0,NULL);
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=4;
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=4;
        UPDATE segments SET state='missing' WHERE id=4;
        UPDATE segments SET state='finished' WHERE id=4;
        INSERT INTO render_jobs VALUES(4,4,'queued','queued',NULL,1900000010000,1900000010000,NULL);
        UPDATE render_jobs SET state='running',updated_at=1900000011000 WHERE id=4;
        UPDATE render_jobs SET phase='编码',ratio=0.5,updated_at=1900000012000 WHERE id=4;
        UPDATE render_jobs SET state='failed',updated_at=1900000013000,finished_at=1900000013000 WHERE id=4;
        UPDATE stream_sessions SET started_at=1900000000000,ended_at=1900000009000 WHERE id=2;")
        .execute(&pool).await.unwrap();
    let events = recent(&pool, 100).await.unwrap();
    let own: Vec<_> = events
        .iter()
        .filter(|event| event.session_id == Some(4))
        .collect();
    assert_eq!(own.len(), 6);
    assert_eq!(own.iter().filter(|event| event.kind == "file").count(), 1);
    assert_eq!(own.iter().filter(|event| event.kind == "render").count(), 3);
    assert!(events.iter().all(|event| event.session_id != Some(2)));
    // Activity survives deletion of the underlying business record.
    sqlx::query("DELETE FROM stream_sessions WHERE id=4")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(recent(&pool, 100).await.unwrap().len(), 10);
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
