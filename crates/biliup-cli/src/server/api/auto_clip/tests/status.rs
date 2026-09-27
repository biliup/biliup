//! 预估只在入队时算一次、多场任务一次取回、直播间「下播后自动生成候选」对操作员只读。

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

async fn queue(pool: &ConnectionPool, session: i64) -> jobs::Job {
    let job = jobs::NewJob::builder()
        .session_id(session)
        .trigger(jobs::Trigger::Manual)
        .not_before(0)
        .created_at(0)
        .build();
    jobs::insert(pool, &job).await.unwrap().unwrap()
}

fn uri(session: i64) -> String {
    format!("/v1/sessions/{session}/auto-clip")
}

#[test]
fn the_job_list_is_declared_in_the_policy() {
    let route = "/v1/auto-clip/jobs";
    assert_eq!(
        RouteRequirement::of(&Method::GET, route, route),
        RouteRequirement::Permission(Permission::FileView)
    );
    assert_eq!(
        RouteRequirement::of(&Method::POST, route, route),
        RouteRequirement::AdminOnly
    );
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

#[tokio::test]
async fn one_request_returns_the_latest_job_of_each_session() {
    let f = fixture(None).await;
    let (a, b, c) = (
        add_session(&f.pool, 10).await,
        add_session(&f.pool, 10).await,
        add_session(&f.pool, 10).await,
    );
    queue(&f.pool, a).await;
    jobs::cancel(&f.pool, a, 1).await.unwrap();
    let latest_a = queue(&f.pool, a).await;
    let job_c = queue(&f.pool, c).await;

    let body = json_of(
        call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &format!("/v1/auto-clip/jobs?session_ids={c},{b},{a},{c}"),
            None,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let jobs = body["jobs"].as_array().unwrap();
    let ids: Vec<(i64, i64)> = jobs
        .iter()
        .map(|job| {
            (
                job["session_id"].as_i64().unwrap(),
                job["id"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ids,
        vec![(a, latest_a.id), (c, job_c.id)],
        "没有任务的场次不出现"
    );
    assert_eq!(jobs[0]["state"], "queued");
    assert!(jobs[0].get("estimate").is_none());

    let response = call(
        &f.app,
        None,
        "GET",
        "/v1/auto-clip/jobs?session_ids=1",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for query in ["", "?session_ids=", "?session_ids=1,x", "?session_ids=1,,2"] {
        let response = call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &format!("/v1/auto-clip/jobs{query}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query:?}");
    }
    let many = |n: i64| {
        let ids: Vec<String> = (1..=n).map(|id| id.to_string()).collect();
        format!("/v1/auto-clip/jobs?session_ids={}", ids.join(","))
    };
    let limit = MAX_JOB_SESSIONS as i64;
    let response = call(&f.app, Some(&f.viewer), "GET", &many(limit), None).await;
    assert_eq!(
        json_of(response, StatusCode::OK).await["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let response = call(&f.app, Some(&f.viewer), "GET", &many(limit + 1), None).await;
    let body = json_of(response, StatusCode::BAD_REQUEST).await;
    assert!(body["message"].as_str().unwrap().contains("最多"), "{body}");
}

async fn add_streamer(pool: &ConnectionPool, url: &str, override_cfg: Option<Value>) {
    sqlx::query("INSERT INTO livestreamers (url, remark, override) VALUES (?, 'r', ?)")
        .bind(url)
        .bind(override_cfg.map(|value| value.to_string()))
        .execute(pool)
        .await
        .unwrap();
}

async fn after_live(f: &Fixture, cookie: &str) -> Vec<Option<Value>> {
    let body = json_of(
        call(&f.app, Some(cookie), "GET", "/v1/streamers", None).await,
        StatusCode::OK,
    )
    .await;
    body.as_array()
        .unwrap()
        .iter()
        .map(|streamer| streamer.get("auto_clip_after_live").cloned())
        .collect()
}

#[tokio::test]
async fn operators_see_the_after_live_switch_but_not_the_override() {
    let seed = async |f: &Fixture| {
        add_streamer(
            &f.pool,
            "https://live.example/on",
            Some(json!({"auto_clip_after_live": true})),
        )
        .await;
        add_streamer(&f.pool, "https://live.example/off", None).await;
    };

    let f = fixture(None).await;
    seed(&f).await;
    for cookie in [&f.admin, &f.operator, &f.viewer] {
        assert_eq!(
            after_live(&f, cookie).await,
            vec![None, None],
            "没配置自动切片时不出现"
        );
    }

    let f = fixture(Some(stored("https://api.example.com/v1"))).await;
    seed(&f).await;
    let on_off = vec![Some(json!(true)), Some(json!(false))];
    assert_eq!(after_live(&f, &f.admin).await, on_off);
    assert_eq!(after_live(&f, &f.operator).await, on_off);
    assert_eq!(
        after_live(&f, &f.viewer).await,
        vec![None, None],
        "没有 clip.edit 不给"
    );

    // 覆写本身仍只给有 streamer.hooks 的人
    let rows = json_of(
        call(&f.app, Some(&f.operator), "GET", "/v1/streamers", None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(rows[0]["override"], Value::Null);
}
