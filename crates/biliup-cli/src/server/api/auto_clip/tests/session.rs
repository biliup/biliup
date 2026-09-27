//! `/v1/sessions/{id}/auto-clip`：预估、入队、取消。测试里没登记调度器，入队的任务不会跑，
//! 也不会调用转写。

use super::*;

async fn add_session(pool: &ConnectionPool, ended_at: Option<i64>) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1000, ?) RETURNING id",
    )
    .bind(ended_at)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 两段录完的（各 10 分钟）加一段已删的；文件不需要存在，预估只看库里的时长。
async fn add_segments(pool: &ConnectionPool, session: i64) {
    for (name, state, start) in [
        ("a.flv", "finished", 0),
        ("b.ts", "pending_delete", 600_000),
        ("c.flv", "deleted", 1_200_000),
    ] {
        sqlx::query(
            "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
             VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(session)
        .bind(format!("/nonexistent/{name}"))
        .bind(if name.ends_with(".ts") { "ts" } else { "flv" })
        .bind(state)
        .bind(start)
        .bind(start + 600_000)
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn job_count(pool: &ConnectionPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM auto_clip_jobs")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn uri(session: i64) -> String {
    format!("/v1/sessions/{session}/auto-clip")
}

#[test]
fn session_routes_are_declared_in_the_policy() {
    let route = "/v1/sessions/{id}/auto-clip";
    for (method, permission) in [
        (Method::GET, Permission::FileView),
        (Method::POST, Permission::ClipEdit),
        (Method::DELETE, Permission::ClipEdit),
    ] {
        let requirement = RouteRequirement::of(&method, route, "/v1/sessions/1/auto-clip");
        assert_eq!(
            requirement,
            RouteRequirement::Permission(permission),
            "{method}"
        );
    }
    assert_eq!(
        RouteRequirement::of(&Method::PUT, route, "/v1/sessions/1/auto-clip"),
        RouteRequirement::AdminOnly
    );
}

#[tokio::test]
async fn without_auto_clip_there_is_nothing_to_start() {
    let f = fixture(None).await;
    let session = add_session(&f.pool, Some(1_200_000)).await;
    add_segments(&f.pool, session).await;

    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        body,
        json!({"enabled": false, "job": null, "estimate": null})
    );
    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        &uri(session),
        Some(json!({"confirm": true})),
    )
    .await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("没有开启"),
        "{body}"
    );
    assert_eq!(job_count(&f.pool).await, 0);
}

#[tokio::test]
async fn estimate_then_confirm_then_cancel() {
    let server = FakeServer::start(Scenario::Ok).await;
    let f = fixture(Some(stored(server.base_url()))).await;
    let session = add_session(&f.pool, Some(1_200_000)).await;
    add_segments(&f.pool, session).await;

    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["enabled"], true);
    assert_eq!(body["job"], Value::Null);
    let mut estimate = body["estimate"].clone();
    let chat = estimate.as_object_mut().unwrap().remove("chat").unwrap();
    assert_eq!(
        estimate,
        json!({
            "recorded_seconds": 1200,
            "asr_seconds": 1200,
            "transcribed_seconds": 0,
            "basis": "duration",
            "max_asr_minutes": 300,
            "over_limit": false,
            "message": null,
            "max_chat_tokens": 300_000,
            "chat_over_limit": false,
            "chat_message": null,
        })
    );
    // 还没转写：20 分钟语音按每分钟 260 token 估，加一窗的固定部分与预留的回复
    assert_eq!(chat["basis"], "duration");
    assert_eq!(chat["windows"], 1);
    assert_eq!(chat["images"], 0, "没做连通性测试时缩图不开");
    let tokens = chat["tokens"].as_i64().unwrap();
    assert!(
        (20 * 260 + 1_000..20 * 260 + 3_000).contains(&tokens),
        "{chat}"
    );

    // 看的人不能生成
    let response = call(&f.app, Some(&f.viewer), "POST", &uri(session), None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 不带 confirm 只回预估
    let body = json_of(
        call(&f.app, Some(&f.operator), "POST", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["estimate"]["asr_seconds"], 1200);
    assert_eq!(job_count(&f.pool).await, 0);

    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &uri(session),
        Some(json!({"confirm": true, "reuse_transcript": false})),
    )
    .await;
    let body = json_of(response, StatusCode::CREATED).await;
    let job = &body["job"];
    assert_eq!(job["state"], "queued");
    assert_eq!(job["trigger"], "manual");
    assert_eq!(job["reuse_transcript"], false);
    let operator: i64 = sqlx::query_scalar("SELECT id FROM web_users WHERE username = 'op'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(job["created_by"], operator);

    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        &uri(session),
        Some(json!({"confirm": true})),
    )
    .await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("已经有"),
        "{body}"
    );
    assert_eq!(job_count(&f.pool).await, 1);

    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["job"]["state"], "queued");

    let response = call(&f.app, Some(&f.viewer), "DELETE", &uri(session), None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = json_of(
        call(&f.app, Some(&f.operator), "DELETE", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["state"], "canceled");
    let response = call(&f.app, Some(&f.operator), "DELETE", &uri(session), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    assert!(server.requests().is_empty(), "入队、取消都不调用转写");
}

#[tokio::test]
async fn requests_that_cannot_run_are_refused() {
    let server = FakeServer::start(Scenario::Ok).await;
    let f = fixture(Some(stored(server.base_url()))).await;
    let confirm = Some(json!({"confirm": true}));

    let response = call(&f.app, Some(&f.admin), "POST", &uri(999), confirm.clone()).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = call(&f.app, Some(&f.admin), "GET", &uri(999), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let live = add_session(&f.pool, None).await;
    add_segments(&f.pool, live).await;
    let response = call(&f.app, Some(&f.admin), "POST", &uri(live), confirm.clone()).await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("还在录"),
        "{body}"
    );

    let empty = add_session(&f.pool, Some(1)).await;
    let response = call(&f.app, Some(&f.admin), "POST", &uri(empty), confirm.clone()).await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("没有录完的分段"),
        "{body}"
    );

    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        &uri(live),
        Some(json!({"confirm": true, "force": true})),
    )
    .await;
    assert!(response.status().is_client_error(), "不认识的字段拒绝");

    // 没填转写模型
    let ended = add_session(&f.pool, Some(1_200_000)).await;
    add_segments(&f.pool, ended).await;
    f.config
        .write()
        .unwrap()
        .auto_clip
        .as_mut()
        .unwrap()
        .asr_model = None;
    let response = call(&f.app, Some(&f.admin), "POST", &uri(ended), confirm).await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("转写接口"),
        "{body}"
    );

    // 没填 chat 模型
    {
        let mut config = f.config.write().unwrap();
        let auto_clip = config.auto_clip.as_mut().unwrap();
        auto_clip.asr_model = Some("whisper-1".into());
        auto_clip.chat_model = None;
    }
    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        &uri(ended),
        Some(json!({"confirm": true})),
    )
    .await;
    let body = json_of(response, StatusCode::CONFLICT).await;
    assert!(
        body["message"].as_str().unwrap().contains("chat 接口"),
        "{body}"
    );
    assert_eq!(job_count(&f.pool).await, 0);
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn the_estimate_warns_when_the_recording_is_over_the_limit() {
    let server = FakeServer::start(Scenario::Ok).await;
    let mut config = stored(server.base_url());
    config.max_asr_minutes = Some(15);
    let f = fixture(Some(config)).await;
    let session = add_session(&f.pool, Some(1_200_000)).await;
    add_segments(&f.pool, session).await;

    let body = json_of(
        call(&f.app, Some(&f.admin), "POST", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    let estimate = &body["estimate"];
    assert_eq!(estimate["over_limit"], true);
    assert_eq!(estimate["basis"], "duration");
    let message = estimate["message"].as_str().unwrap();
    assert!(
        message.contains("约 20.0 分钟") && message.contains("上限 15 分钟"),
        "{message}"
    );

    // 按录像时长估的还可能在跳过静音后降到上限以下，允许入队；跑的时候再按静音表把关
    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        &uri(session),
        Some(json!({"confirm": true})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

/// 排队的任务占住整场素材：这时整场投稿后删除录像，分段只标 `pending_delete`，清理任务也不删；
/// 取消后引用撤销，下一轮清理照常删。
#[tokio::test]
async fn a_queued_job_holds_the_footage_until_it_is_canceled() {
    use crate::server::workbench::recorder::now_ms;
    use crate::server::workbench::retention::{self, Disposal, Retention};

    let server = FakeServer::start(Scenario::Ok).await;
    let f = fixture(Some(stored(server.base_url()))).await;
    let session = add_session(&f.pool, Some(600_000)).await;
    let path = f._dir.path().join("a.flv");
    std::fs::write(&path, b"FLV").unwrap();
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, ?, 'flv', 'finished', 0, 600000, 0)",
    )
    .bind(session)
    .bind(path.to_string_lossy().into_owned())
    .execute(&f.pool)
    .await
    .unwrap();
    let state = async || -> String {
        sqlx::query_scalar("SELECT state FROM segments WHERE session_id = ?")
            .bind(session)
            .fetch_one(&f.pool)
            .await
            .unwrap()
    };

    let response = call(
        &f.app,
        Some(&f.operator),
        "POST",
        &uri(session),
        Some(json!({"confirm": true})),
    )
    .await;
    let body = json_of(response, StatusCode::CREATED).await;
    assert_eq!(body["job"]["state"], "queued");

    let retention = Retention::without_delay(f.pool.clone());
    assert_eq!(
        retention::remove(&retention, &[path.as_path()])
            .await
            .unwrap(),
        vec![Disposal::Deferred]
    );
    assert_eq!(state().await, "pending_delete");
    assert_eq!(
        retention::sweep_pending(&f.pool, now_ms()).await.unwrap(),
        0
    );
    assert!(path.exists(), "排队期间录像留着");

    json_of(
        call(&f.app, Some(&f.operator), "DELETE", &uri(session), None).await,
        StatusCode::OK,
    )
    .await;
    let pins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segment_pins")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, 0, "取消即撤销引用");
    assert_eq!(
        retention::sweep_pending(&f.pool, now_ms()).await.unwrap(),
        1
    );
    assert_eq!(state().await, "deleted");
    assert!(!path.exists());
}
