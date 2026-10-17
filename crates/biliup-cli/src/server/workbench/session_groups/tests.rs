use super::*;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::workbench::store;
use chrono::{DateTime, Utc};

async fn fixture() -> ConnectionPool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query("INSERT INTO livestreamers(id,url,remark) VALUES(1,'https://a','a')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn source(pool: &ConnectionPool, start: i64, end: Option<i64>, duration: i64) -> i64 {
    let info = StreamerInfo::new(
        "a",
        "https://a",
        "title",
        DateTime::<Utc>::from_timestamp_millis(start).unwrap(),
        "",
    );
    let id = store::open_session(pool, 1, &info, start, 0)
        .await
        .unwrap()
        .id;
    store::set_started_at(pool, id, start).await.unwrap();
    let seg = store::insert_segment(pool, id, &format!("s{id}.flv"), "flv", 0, 0)
        .await
        .unwrap();
    sqlx::query("UPDATE segments SET end_ms=?,state='finished' WHERE id=?")
        .bind(duration)
        .bind(seg)
        .execute(pool)
        .await
        .unwrap();
    if let Some(end) = end {
        store::close_session(pool, id, end).await.unwrap();
    }
    id
}

#[tokio::test]
async fn merge_preserves_sources_and_actual_gap_with_fixed_offsets() {
    let pool = fixture().await;
    let first = source(&pool, 1_000, Some(11_000), 9_000).await;
    let second = source(&pool, 1_006_000, Some(1_020_000), 12_000).await;
    let group = create(&pool, &[second, first]).await.unwrap();
    let mapping = timeline(&pool, group).await.unwrap();
    assert_eq!(
        mapping,
        vec![
            TimelineMember {
                source_session_id: first,
                offset_ms: 0
            },
            TimelineMember {
                source_session_id: second,
                offset_ms: 1_004_000
            }
        ]
    );
    let segments = store::session_segments(&pool, group).await.unwrap();
    assert_eq!(segments[1].start_ms, 1_004_000);
    assert_eq!(segments[1].gap_before_ms, 995_000);
    assert_eq!(segments[1].session_id, second);
    assert_eq!(
        store::source_segments(&pool, second).await.unwrap()[0].start_ms,
        0
    );
    let (rows, total) = store::list_sessions(&pool, None, 20, 0).await.unwrap();
    assert_eq!(total, 1);
    assert_eq!(rows[0].id, group);
    assert_eq!(rows[0].end_ms, 1_016_000);
    assert_eq!(
        store::delete_session(&pool, second).await.unwrap(),
        store::DeleteSessionOutcome::GroupMember
    );
    assert_eq!(
        store::delete_session(&pool, group).await.unwrap(),
        store::DeleteSessionOutcome::Deleted
    );
    assert!(store::session(&pool, first).await.unwrap().is_some());
}

#[tokio::test]
async fn decline_is_durable_and_confirmation_can_append_to_existing_group() {
    let pool = fixture().await;
    let first = source(&pool, 1_000, Some(11_000), 9_000).await;
    let second = source(&pool, 20_000, Some(30_000), 8_000).await;
    let candidate = candidates(&pool).await.unwrap().pop().unwrap();
    assert!(dismiss(&pool, candidate.id).await.unwrap());
    assert!(candidates(&pool).await.unwrap().is_empty());
    let group = create(&pool, &[first, second]).await.unwrap();
    let third = source(&pool, 40_000, None, 3_000).await;
    let candidate = candidates(&pool).await.unwrap().pop().unwrap();
    assert_eq!(candidate.group_session_id, Some(group));
    assert_eq!(confirm(&pool, candidate.id).await.unwrap(), group);
    assert_eq!(
        timeline(&pool, group).await.unwrap().last().unwrap(),
        &TimelineMember {
            source_session_id: third,
            offset_ms: 36_000
        }
    );
    assert_eq!(
        store::session(&pool, group)
            .await
            .unwrap()
            .unwrap()
            .ended_at,
        None
    );
    assert!(confirm(&pool, candidate.id).await.is_err());
    assert!(create(&pool, &[group, third]).await.is_err());
}

#[tokio::test]
async fn group_endpoints_expose_compatible_summary_and_preserve_sources() {
    use crate::server::api::{session_groups as api, sessions};
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use tower::ServiceExt;
    let pool = fixture().await;
    let first = source(&pool, 1000, Some(11000), 9000).await;
    let second = source(&pool, 20000, Some(30000), 8000).await;
    let app = Router::new()
        .route("/v1/session-merge-candidates", get(api::list_candidates))
        .route(
            "/v1/session-merge-candidates/{id}/confirm",
            post(api::confirm_candidate),
        )
        .route(
            "/v1/session-merge-candidates/{id}/dismiss",
            post(api::dismiss_candidate),
        )
        .route("/v1/session-groups", post(api::create_group))
        .route("/v1/sessions/{id}", get(sessions::get_session))
        .with_state(pool.clone());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/session-merge-candidates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["items"][0]["gap_ms"], 9000);
    let id = body["items"][0]["id"].as_i64().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/session-merge-candidates/{id}/confirm"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let group = body["id"].as_i64().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/sessions/{group}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["kind"], "group");
    assert_eq!(
        body["source_session_ids"],
        serde_json::json!([first, second])
    );
    assert_eq!(body["segments"][1]["source_session_id"], second);
    assert_eq!(body["members"][1]["offset_ms"], 18000);
    assert_eq!(
        body["gaps"][0],
        serde_json::json!({"from_ms":9000,"to_ms":18000})
    );
    assert_eq!(
        store::source_segments(&pool, second).await.unwrap()[0].start_ms,
        0
    );
    let repeated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/session-merge-candidates/{id}/confirm"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated.status(), StatusCode::NOT_FOUND);
    let malformed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/session-groups")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    "{{\"source_session_ids\":[{first},{first}]}}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn groups_reject_overlap_cross_streamer_and_non_tail_recording() {
    let pool = fixture().await;
    let first = source(&pool, 1000, Some(12000), 9000).await;
    let overlap = source(&pool, 11000, Some(15000), 3000).await;
    assert!(matches!(
        create(&pool, &[first, overlap]).await,
        Err(MergeError::Invalid(_))
    ));
    let live = source(&pool, 20000, None, 4000).await;
    let after = source(&pool, 30000, Some(35000), 3000).await;
    assert!(matches!(
        create(&pool, &[live, after]).await,
        Err(MergeError::Invalid(_))
    ));
    sqlx::query("INSERT INTO livestreamers(id,url,remark) VALUES(2,'https://b','b')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stream_sessions SET streamer_id=2 WHERE id=?")
        .bind(after)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        create(&pool, &[first, after]).await,
        Err(MergeError::Invalid(_))
    ));
    assert!(
        candidates(&pool)
            .await
            .unwrap()
            .iter()
            .all(|c| c.source_session_id != overlap)
    );
}

#[tokio::test]
async fn member_notifications_do_not_wake_other_sessions_or_databases() {
    let pool = fixture().await;
    let other = fixture().await;
    let first = source(&pool, 1000, Some(11000), 9000).await;
    let second = source(&pool, 20000, Some(30000), 8000).await;
    let group = create(&pool, &[first, second]).await.unwrap();
    let mut own = updates(&pool, group);
    let unrelated = updates(&pool, first);
    let other_database = updates(&other, group);
    let third = source(&pool, 40000, Some(50000), 9000).await;
    create(&pool, &[second, third]).await.unwrap();
    own.changed().await.unwrap();
    assert!(!unrelated.has_changed().unwrap());
    assert!(!other_database.has_changed().unwrap());
}

#[tokio::test]
async fn equal_offsets_preserve_member_order_independent_of_imported_ids() {
    let pool = fixture().await;
    let tail = source(&pool, 30000, None, 0).await;
    let previous = source(&pool, 10000, Some(30000), 0).await;
    let group = create(&pool, &[tail, previous]).await.unwrap();
    assert_eq!(
        timeline(&pool, group).await.unwrap(),
        vec![
            TimelineMember {
                source_session_id: previous,
                offset_ms: 0
            },
            TimelineMember {
                source_session_id: tail,
                offset_ms: 0
            }
        ]
    );
}

#[tokio::test]
async fn suggestions_only_append_after_the_existing_group_tail() {
    let pool = fixture().await;
    let first = source(&pool, 1000, Some(10000), 9000).await;
    let middle = source(&pool, 20000, Some(25000), 5000).await;
    let tail = source(&pool, 30000, Some(40000), 10000).await;
    create(&pool, &[first, tail]).await.unwrap();
    assert!(
        candidates(&pool)
            .await
            .unwrap()
            .iter()
            .all(|c| c.source_session_id != middle)
    );
    let next = source(&pool, 50000, None, 1000).await;
    assert_eq!(
        candidates(&pool)
            .await
            .unwrap()
            .iter()
            .map(|c| c.source_session_id)
            .collect::<Vec<_>>(),
        vec![next]
    );
}
