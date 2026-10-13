//! 候选接口：列表、接受（建切片草稿）、丢弃。候选直接写库，不调用模型。

use super::*;
use crate::server::auto_clip::candidates::{Candidate, Evidence};
use crate::server::auto_clip::suggestions;

async fn add_session(pool: &ConnectionPool) -> i64 {
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1000, 601000) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, '/nonexistent/a.flv', 'flv', 'finished', 0, 600000, 0)",
    )
    .bind(session)
    .execute(pool)
    .await
    .unwrap();
    session
}

fn candidate(in_s: i64, out_s: i64) -> Candidate {
    Candidate {
        in_ms: in_s * 1000,
        out_ms: out_s * 1000,
        title: "残血翻盘".into(),
        reason: "弹幕是基线 5 倍".into(),
        confidence: Some(0.8),
        tags: vec!["高能".into()],
        evidence: Evidence {
            asr_lines: 3,
            danmaku: 85,
            images: Vec::new(),
        },
    }
}

#[tokio::test]
async fn group_suggestions_keep_source_coordinates_and_accept_into_group() {
    let f = fixture(None).await;
    sqlx::query("INSERT INTO livestreamers(id,url,remark) VALUES(1,'https://a','a')")
        .execute(&f.pool)
        .await
        .unwrap();
    let first = add_session(&f.pool).await;
    let second = add_session(&f.pool).await;
    sqlx::query("UPDATE stream_sessions SET streamer_id=1 WHERE id IN (?,?)")
        .bind(first)
        .bind(second)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stream_sessions SET started_at=700000,ended_at=1300000 WHERE id=?")
        .bind(second)
        .execute(&f.pool)
        .await
        .unwrap();
    let mut proposed = candidate(3, 9);
    proposed.evidence.images = vec![5000];
    let old = suggestions::replace_pending(&f.pool, second, None, &[proposed], 1)
        .await
        .unwrap()
        .pop()
        .unwrap();
    let group = crate::server::workbench::session_groups::create(&f.pool, &[first, second])
        .await
        .unwrap();
    let list = json_of(
        call(
            &f.app,
            Some(&f.operator),
            "GET",
            &format!("/v1/sessions/{group}/suggestions"),
            None,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list["suggestions"][0]["session_id"], second);
    assert_eq!(list["suggestions"][0]["view_session_id"], group);
    assert_eq!(list["suggestions"][0]["in_ms"], 702000);
    assert_eq!(list["suggestions"][0]["evidence"]["images"][0], 704000);
    let accepted = json_of(
        call(
            &f.app,
            Some(&f.operator),
            "POST",
            &format!("/v1/sessions/{group}/suggestions/{}/accept", old.id),
            Some(json!({"in_ms":590000,"out_ms":709000})),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(accepted["clip"]["session_id"], group);
    assert_eq!(accepted["clip"]["out_ms"], 709000);
    assert_eq!(accepted["suggestion"]["view_session_id"], group);
    let canonical = suggestions::get(&f.pool, old.id).await.unwrap().unwrap();
    assert_eq!(canonical.in_ms, 3000);
    assert_eq!(canonical.session_id, second);
    let counts: Vec<i64> = sqlx::query_scalar(
        "SELECT pin_count FROM segments WHERE session_id IN (?,?) ORDER BY session_id",
    )
    .bind(first)
    .bind(second)
    .fetch_all(&f.pool)
    .await
    .unwrap();
    assert_eq!(counts, vec![1, 1]);
    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &format!("/v1/sessions/{group}/suggestions/{}/accept", old.id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn suggestion_routes_are_declared_in_the_policy() {
    for (method, route, permission) in [
        (
            Method::GET,
            "/v1/sessions/{id}/suggestions",
            Permission::FileView,
        ),
        (
            Method::POST,
            "/v1/sessions/{id}/suggestions/{sid}/accept",
            Permission::ClipEdit,
        ),
        (
            Method::POST,
            "/v1/sessions/{id}/suggestions/{sid}/dismiss",
            Permission::ClipEdit,
        ),
    ] {
        assert_eq!(
            RouteRequirement::of(&method, route, route),
            RouteRequirement::Permission(permission),
            "{method} {route}"
        );
    }
    assert_eq!(
        RouteRequirement::of(
            &Method::DELETE,
            "/v1/sessions/{id}/suggestions",
            "/v1/sessions/1/suggestions"
        ),
        RouteRequirement::AdminOnly
    );
}

#[tokio::test]
async fn accept_and_dismiss_suggestions() {
    let f = fixture(None).await;
    let session = add_session(&f.pool).await;
    let other = add_session(&f.pool).await;
    let saved = suggestions::replace_pending(
        &f.pool,
        session,
        None,
        &[candidate(10, 40), candidate(100, 160), candidate(300, 330)],
        now_ms(),
    )
    .await
    .unwrap();
    let list = format!("/v1/sessions/{session}/suggestions");
    let accept = |sid: i64| format!("/v1/sessions/{session}/suggestions/{sid}/accept");
    let dismiss = |sid: i64| format!("/v1/sessions/{session}/suggestions/{sid}/dismiss");

    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &list, None).await,
        StatusCode::OK,
    )
    .await;
    let items = body["suggestions"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["state"], "pending");
    assert_eq!(items[0]["confidence"], 0.8);
    assert_eq!(items[0]["evidence"]["danmaku"], 85);
    assert_eq!(items[0]["tags"], json!(["高能"]));
    assert!(items[0]["expires_at"].as_i64().is_some());

    // 看的人不能接受、丢弃
    let response = call(&f.app, Some(&f.viewer), "POST", &accept(saved[0].id), None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = call(&f.app, Some(&f.viewer), "POST", &dismiss(saved[0].id), None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 改过的范围不合法、标题有换行、字段不认识
    for body in [
        json!({"in_ms": 50_000, "out_ms": 40_000}),
        json!({"in_ms": -1}),
        json!({"title": "第一行\n第二行"}),
        json!({"export": "quick"}),
    ] {
        let response = call(
            &f.app,
            Some(&f.operator),
            "POST",
            &accept(saved[0].id),
            Some(body.clone()),
        )
        .await;
        assert!(response.status().is_client_error(), "{body}");
    }
    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &format!("/v1/sessions/{other}/suggestions/{}/accept", saved[0].id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "别的场次的候选");

    let body = json_of(
        call(
            &f.app,
            Some(&f.operator),
            "POST",
            &accept(saved[0].id),
            Some(json!({"out_ms": 45_000, "title": "  改过的标题 "})),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(body["suggestion"]["state"], "accepted");
    let clip = &body["clip"];
    assert_eq!(clip["state"], "draft");
    assert_eq!(
        (clip["in_ms"].as_i64(), clip["out_ms"].as_i64()),
        (Some(10_000), Some(45_000))
    );
    assert_eq!(clip["title"], "改过的标题");
    assert_eq!(body["suggestion"]["clip_id"], clip["id"]);
    let operator: i64 = sqlx::query_scalar("SELECT id FROM web_users WHERE username = 'op'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(clip["created_by"], operator);

    let clips = json_of(
        call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &format!("/v1/sessions/{session}/clips"),
            None,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        clips["clips"].as_array().unwrap().len(),
        1,
        "切片 Tab 多一条草稿"
    );

    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &accept(saved[0].id),
        None,
    )
    .await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("已经接受"),
        "{body}"
    );

    // 不带请求体：用候选自己的范围和标题
    let body = json_of(
        call(&f.app, Some(&f.admin), "POST", &accept(saved[1].id), None).await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(body["clip"]["title"], "残血翻盘");
    assert_eq!(body["clip"]["out_ms"], 160_000);

    let body = json_of(
        call(
            &f.app,
            Some(&f.operator),
            "POST",
            &dismiss(saved[2].id),
            None,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["state"], "dismissed");
    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &dismiss(saved[2].id),
        None,
    )
    .await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("已经丢弃"),
        "{body}"
    );
    let response = call(&f.app, Some(&f.operator), "POST", &dismiss(9_999), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = call(
        &f.app,
        Some(&f.viewer),
        "GET",
        "/v1/sessions/9999/suggestions",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let pins: Vec<String> = sqlx::query_scalar("SELECT owner FROM segment_pins ORDER BY owner")
        .fetch_all(&f.pool)
        .await
        .unwrap();
    assert!(
        pins.iter().all(|owner| owner.starts_with("clip:")),
        "处理过的候选不再引用素材：{pins:?}"
    );
    assert_eq!(pins.len(), 2);
}
