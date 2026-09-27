//! 预估只在入队时算一次。

use super::*;

async fn add_session(pool: &ConnectionPool, minutes: i64) -> i64 {
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1000, 1) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    add_segment(pool, session, 0, minutes).await;
    session
}

async fn add_segment(pool: &ConnectionPool, session: i64, start_min: i64, minutes: i64) {
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, ?, 'flv', 'finished', ?, ?, 0)",
    )
    .bind(session)
    .bind(format!("/nonexistent/{session}-{start_min}.flv"))
    .bind(start_min * 60_000)
    .bind((start_min + minutes) * 60_000)
    .execute(pool)
    .await
    .unwrap();
}

fn uri(session: i64) -> String {
    format!("/v1/sessions/{session}/auto-clip")
}

/// 入队时的预估存进任务行；之后录像变了，状态接口仍回那一份，不重新读分段。
#[tokio::test]
async fn the_status_returns_the_estimate_stored_at_queue_time() {
    let server = FakeServer::start(Scenario::Ok).await;
    let f = fixture(Some(stored(server.base_url()))).await;
    let session = add_session(&f.pool, 20).await;

    let body = json_of(
        call(
            &f.app,
            Some(&f.operator),
            "POST",
            &uri(session),
            Some(json!({"confirm": true})),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(body["estimate"]["asr_seconds"], 1200);
    assert!(
        body["job"].get("estimate").is_none(),
        "预估不随任务重复一份"
    );
    let confirmed = body["estimate"].clone();

    add_segment(&f.pool, session, 20, 40).await;
    for _ in 0..3 {
        let body = json_of(
            call(&f.app, Some(&f.viewer), "GET", &uri(session), None).await,
            StatusCode::OK,
        )
        .await;
        assert_eq!(body["job"]["state"], "queued");
        assert_eq!(body["estimate"], confirmed, "录像多了 40 分钟也不重算");
    }

    // 任务结束后再手动预估才会按现在的录像算
    jobs::cancel(&f.pool, session, 1).await.unwrap();
    let body = json_of(
        call(&f.app, Some(&f.operator), "POST", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["estimate"]["asr_seconds"], 3600);
    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["estimate"], confirmed, "只预估不入队，不改任务行");
    assert!(server.requests().is_empty());
}
