use super::*;
use crate::server::api::access::require_permission;
use crate::server::api::clips::{create_clip, delete_clip, get_clip, update_clip};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::infrastructure::permissions::Role;
use crate::server::infrastructure::users::{Backend, Credentials};
use crate::server::workbench::clips::publish::queue::{
    Bilibili, Connection, Failure, RATE_LIMITED, Submitted,
};
use crate::server::workbench::index::tests::build_flv;
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::FromRef;
use axum::http::Request;
use axum::middleware::from_fn;
use axum::routing::{delete, get, patch, post};
use axum_login::AuthManagerLayerBuilder;
use biliup::bilibili::{Studio, Video};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Default)]
struct Fake {
    submitted: Mutex<Vec<Value>>,
    upload_failures: Mutex<VecDeque<Failure>>,
}

struct FakeConnection(Arc<Fake>);

#[async_trait]
impl Bilibili for Arc<Fake> {
    async fn connect(&self, _: &UploadStreamer) -> Result<Box<dyn Connection>, Failure> {
        Ok(Box::new(FakeConnection(self.clone())))
    }
}

#[async_trait]
impl Connection for FakeConnection {
    async fn upload(
        &self,
        path: &std::path::Path,
        progress: &(dyn Fn(usize) + Send + Sync),
    ) -> Result<Video, Failure> {
        tokio::time::sleep(Duration::from_millis(20)).await;
        if let Some(failure) = self.0.upload_failures.lock().unwrap().pop_front() {
            return Err(failure);
        }
        progress(std::fs::metadata(path).unwrap().len() as usize);
        Ok(Video::new("n1"))
    }

    async fn cover(&self, path: &std::path::Path) -> Result<String, Failure> {
        assert!(path.exists());
        Ok("https://i0.hdslb.com/c.jpg".into())
    }

    async fn submit(&self, studio: &Studio) -> Result<Submitted, Failure> {
        let mut submitted = self.0.submitted.lock().unwrap();
        submitted.push(serde_json::to_value(studio).unwrap());
        Ok(Submitted {
            bvid: format!("BV1api{}", submitted.len()),
        })
    }
}

#[derive(Clone, FromRef)]
struct AppState {
    pool: ConnectionPool,
    clips: Arc<ClipExports>,
    publisher: Arc<ClipPublisher>,
    client: StatelessClient,
}

struct Fixture {
    dir: tempfile::TempDir,
    app: Router,
    pool: ConnectionPool,
    fake: Arc<Fake>,
    session: i64,
    template: i64,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let template: i64 = sqlx::query_scalar(
        r#"INSERT INTO uploadstreamers (template_name, title, tid, copyright, tags, user_cookie)
           VALUES ('模板', '{streamer} 录播 %Y', 171, 1, '["直播"]', 'cookies.json') RETURNING id"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let streamer: i64 = sqlx::query_scalar(
        "INSERT INTO livestreamers (url, remark, upload_streamers_id)
         VALUES ('https://live.bilibili.com/9', '主播', ?) RETURNING id",
    )
    .bind(template)
    .fetch_one(&pool)
    .await
    .unwrap();
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at, streamer_id)
         VALUES ('主播', 'https://live.bilibili.com/9', '直播', '2026-09-24 00:00:00', '', 1790000000000, 2, ?)
         RETURNING id",
    )
    .bind(streamer)
    .fetch_one(&pool)
    .await
    .unwrap();
    for (name, start) in [("a.flv", 0i64), ("b.flv", 3965)] {
        let path = dir.path().join(name);
        std::fs::write(&path, build_flv(0, 100, 25, None).bytes).unwrap();
        sqlx::query(
            "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms)
             VALUES (?, ?, 'flv', 'finished', ?, ?)",
        )
        .bind(session)
        .bind(path.to_string_lossy().into_owned())
        .bind(start)
        .bind(start + 3965)
        .execute(&pool)
        .await
        .unwrap();
    }

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let backend = Backend::new(pool.clone());
    backend
        .bootstrap_admin(Credentials {
            username: "biliup".into(),
            password: "admin-password".into(),
            next: None,
        })
        .await
        .unwrap();
    backend
        .create_user("op", "operator-password".into(), Role::Operator)
        .await
        .unwrap();
    backend
        .create_user("ro", "viewer-password".into(), Role::Viewer)
        .await
        .unwrap();
    let auth_layer = AuthManagerLayerBuilder::new(
        backend.clone(),
        SessionManagerLayer::new(session_store).with_secure(false),
    )
    .build();
    let clips = Arc::new(ClipExports::new(pool.clone(), dir.path().join("clips")));
    let fake = Arc::new(Fake::default());
    let publisher = Arc::new(ClipPublisher::new(
        pool.clone(),
        clips.clone(),
        Arc::new(fake.clone()),
    ));
    publisher.spawn();
    let state = AppState {
        pool: pool.clone(),
        clips,
        publisher,
        client: StatelessClient::default(),
    };
    let app = Router::new()
        .route("/v1/sessions/{id}/clips", post(create_clip))
        .route(
            "/v1/sessions/{id}/clips/{cid}",
            patch(update_clip).delete(delete_clip),
        )
        .route("/v1/clips/{cid}", get(get_clip))
        .route("/v1/clips/{cid}/publish", post(publish_clip))
        .route(
            "/v1/clips/{cid}/cover",
            get(get_clip_cover)
                .put(put_clip_cover)
                .delete(delete_clip_cover),
        )
        .route("/v1/sessions/{id}/thumb", get(get_session_thumb))
        .route(
            "/v1/publish-jobs",
            get(list_publish_jobs).post(publish_batch),
        )
        .route("/v1/publish-jobs/preview", post(preview_publish))
        .route("/v1/publish-jobs/resume", post(resume_publish))
        .route("/v1/publish-jobs/{jid}/retry", post(retry_publish))
        .route("/v1/publish-jobs/{jid}", delete(remove_publish))
        .with_state(state)
        .route_layer(from_fn(require_permission))
        .merge(crate::server::api::auth::router())
        .layer(auth_layer);
    Fixture {
        dir,
        app,
        pool,
        fake,
        session,
        template,
    }
}

async fn login(app: &Router, username: &str, password: &str) -> String {
    let response = send(
        app,
        None,
        "POST",
        "/v1/users/login",
        Some(json!({ "username": username, "password": password })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers().get(header::SET_COOKIE).unwrap();
    cookie
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn send(
    app: &Router,
    cookie: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn send_bytes(app: &Router, cookie: &str, uri: &str, kind: &str, body: Vec<u8>) -> Response {
    let request = Request::builder()
        .method("PUT")
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, kind)
        .body(Body::from(body))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn body_of(response: Response, status: StatusCode) -> Vec<u8> {
    let actual = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(actual, status, "{}", String::from_utf8_lossy(&body));
    body.to_vec()
}

async fn json_of(response: Response, status: StatusCode) -> Value {
    serde_json::from_slice(&body_of(response, status).await).unwrap()
}

async fn text_of(response: Response, status: StatusCode) -> String {
    String::from_utf8(body_of(response, status).await).unwrap()
}

impl Fixture {
    async fn clip(&self, cookie: &str, in_ms: i64, out_ms: i64, title: &str) -> i64 {
        let clip = json_of(
            send(
                &self.app,
                Some(cookie),
                "POST",
                &format!("/v1/sessions/{}/clips", self.session),
                Some(json!({ "in_ms": in_ms, "out_ms": out_ms, "title": title })),
            )
            .await,
            StatusCode::CREATED,
        )
        .await;
        clip["id"].as_i64().unwrap()
    }

    async fn jobs(&self, cookie: &str) -> Value {
        json_of(
            send(
                &self.app,
                Some(cookie),
                "GET",
                &format!("/v1/publish-jobs?session={}", self.session),
                None,
            )
            .await,
            StatusCode::OK,
        )
        .await
    }

    async fn wait_job(&self, cookie: &str, id: u64, state: &str) -> Value {
        for _ in 0..2000 {
            let view = self.jobs(cookie).await;
            if let Some(job) = view["jobs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|j| j["id"] == id)
                && job["state"] == state
            {
                return job.clone();
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("任务 {id} 没有到 {state}：{}", self.jobs(cookie).await);
    }
}

/// 发布相关路由都由策略层按路由表判断：发布、预览、继续、重试、移出要 `upload.submit`，
/// 看队列、取帧、看封面要 `file.view`，改封面要 `clip.edit`。
#[test]
fn publish_routes_are_declared_in_the_policy() {
    use crate::server::infrastructure::permissions::Permission::{self, *};
    use crate::server::infrastructure::policy::{RouteRequirement, Subject};
    use axum::http::Method;
    let table: [(Method, &str, &str, Permission); 11] = [
        (
            Method::POST,
            "/v1/clips/{cid}/publish",
            "/v1/clips/1/publish",
            UploadSubmit,
        ),
        (
            Method::POST,
            "/v1/publish-jobs",
            "/v1/publish-jobs",
            UploadSubmit,
        ),
        (
            Method::POST,
            "/v1/publish-jobs/preview",
            "/v1/publish-jobs/preview",
            UploadSubmit,
        ),
        (
            Method::POST,
            "/v1/publish-jobs/resume",
            "/v1/publish-jobs/resume",
            UploadSubmit,
        ),
        (
            Method::POST,
            "/v1/publish-jobs/{jid}/retry",
            "/v1/publish-jobs/1/retry",
            UploadSubmit,
        ),
        (
            Method::DELETE,
            "/v1/publish-jobs/{jid}",
            "/v1/publish-jobs/1",
            UploadSubmit,
        ),
        (
            Method::GET,
            "/v1/publish-jobs",
            "/v1/publish-jobs",
            FileView,
        ),
        (
            Method::GET,
            "/v1/sessions/{id}/thumb",
            "/v1/sessions/1/thumb",
            FileView,
        ),
        (
            Method::GET,
            "/v1/clips/{cid}/cover",
            "/v1/clips/1/cover",
            FileView,
        ),
        (
            Method::PUT,
            "/v1/clips/{cid}/cover",
            "/v1/clips/1/cover",
            ClipEdit,
        ),
        (
            Method::DELETE,
            "/v1/clips/{cid}/cover",
            "/v1/clips/1/cover",
            ClipEdit,
        ),
    ];
    for (method, route, raw, permission) in table {
        let requirement = RouteRequirement::of(&method, route, raw);
        assert_eq!(
            requirement,
            RouteRequirement::Permission(permission),
            "{method} {route}"
        );
        for role in Role::ALL {
            assert_eq!(
                Subject::user(1, role).satisfies(requirement),
                role.has(permission),
                "{method} {route} {role:?}"
            );
        }
    }
    assert!(Role::Operator.has(UploadSubmit));
    assert!(!Role::Viewer.has(UploadSubmit));
    for (method, route, raw) in [
        (
            Method::GET,
            "/v1/clips/{cid}/publish",
            "/v1/clips/1/publish",
        ),
        (Method::GET, "/v1/publish-jobs/{jid}", "/v1/publish-jobs/1"),
        (Method::PUT, "/v1/publish-jobs", "/v1/publish-jobs"),
    ] {
        assert_eq!(
            RouteRequirement::of(&method, route, raw),
            RouteRequirement::AdminOnly,
            "{method} {route}"
        );
    }
}

#[tokio::test]
async fn publishing_one_clip_runs_every_step_as_a_reprint() {
    let f = fixture().await;
    let op = login(&f.app, "op", "operator-password").await;
    let viewer = login(&f.app, "ro", "viewer-password").await;
    let id = f.clip(&op, 1000, 2000, "名场面").await;

    let response = send(
        &f.app,
        Some(&viewer),
        "POST",
        &format!("/v1/clips/{id}/publish"),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(f.jobs(&viewer).await["jobs"], json!([]));

    let job = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            &format!("/v1/clips/{id}/publish"),
            Some(json!({ "studio_override": { "tags": ["切片", "游戏"] } })),
        )
        .await,
        StatusCode::ACCEPTED,
    )
    .await;
    let jid = job["jobs"][0]["id"].as_u64().unwrap();
    assert_eq!(job["jobs"][0]["clip_ids"], json!([id]));
    let done = f.wait_job(&op, jid, "done").await;
    assert_eq!(done["bvid"], "BV1api1");
    assert_eq!(done["title"], "名场面");

    let studio = f.fake.submitted.lock().unwrap()[0].clone();
    assert_eq!(studio["copyright"], 2);
    assert_eq!(studio["source"], "https://live.bilibili.com/9");
    assert_eq!(studio["tag"], "切片,游戏");
    assert_eq!(studio["videos"][0]["title"], "名场面");

    let clip = json_of(
        send(
            &f.app,
            Some(&viewer),
            "GET",
            &format!("/v1/clips/{id}"),
            None,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(clip["state"], "published");
    assert_eq!(clip["archive_bvid"], "BV1api1");
    assert!(clip["published_at"].is_i64());
    assert_eq!(clip["studio_override"], json!({ "tags": ["切片", "游戏"] }));

    let again = send(
        &f.app,
        Some(&op),
        "POST",
        &format!("/v1/clips/{id}/publish"),
        Some(json!({})),
    )
    .await;
    assert!(
        text_of(again, StatusCode::CONFLICT)
            .await
            .contains("已经发布过了")
    );
}

#[tokio::test]
async fn preview_shows_the_final_fields_and_bad_batches_are_refused() {
    let f = fixture().await;
    let op = login(&f.app, "op", "operator-password").await;
    let a = f.clip(&op, 1000, 2000, "开场").await;
    let b = f.clip(&op, 3000, 5000, "").await;

    let preview = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            "/v1/publish-jobs/preview",
            Some(json!({ "clip_ids": [b, a] })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let archives = preview["archives"].as_array().unwrap();
    assert_eq!(archives.len(), 2, "默认每个切片一个稿件");
    assert_eq!(archives[0]["clip_ids"], json!([a]));
    let first = &archives[0]["rendered"];
    assert_eq!(first["title"], "开场");
    assert_eq!(first["copyright"], 2);
    assert_eq!(first["source"], "https://live.bilibili.com/9");
    assert_eq!(first["template_self_made"], true);
    assert_eq!(archives[0]["template_id"], f.template);
    assert!(
        archives[1]["rendered"]["title"]
            .as_str()
            .unwrap()
            .starts_with("主播 2026-")
    );

    let combined = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            "/v1/publish-jobs/preview",
            Some(json!({
                "clip_ids": [a, b],
                "combine": true,
                "studio_override": { "title": "{streamer} 今日高光", "desc": "{url}" },
            })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let rendered = &combined["archives"][0]["rendered"];
    assert_eq!(rendered["title"], "主播 今日高光");
    assert_eq!(rendered["desc"], "https://live.bilibili.com/9");
    assert_eq!(rendered["part_titles"].as_array().unwrap().len(), 2);

    for (body, status, hint) in [
        (
            json!({ "clip_ids": [a, b], "studio_override": { "title": "x" } }),
            StatusCode::BAD_REQUEST,
            "合成一个稿件",
        ),
        (
            json!({ "clip_ids": [] }),
            StatusCode::BAD_REQUEST,
            "没有选切片",
        ),
        (
            json!({ "clip_ids": [999] }),
            StatusCode::NOT_FOUND,
            "不存在",
        ),
        (
            json!({ "clip_ids": [a], "studio_override": { "tags": ["a,b"] } }),
            StatusCode::BAD_REQUEST,
            "逗号",
        ),
    ] {
        let response = send(&f.app, Some(&op), "POST", "/v1/publish-jobs", Some(body)).await;
        assert!(text_of(response, status).await.contains(hint), "{hint}");
    }
    let response = send(
        &f.app,
        Some(&op),
        "POST",
        "/v1/publish-jobs",
        Some(json!({ "clip_ids": [a], "copyright": 1 })),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "不接受版权字段"
    );

    // 模板没有标签、也没覆盖：入队前就拒绝，说清楚怎么办
    sqlx::query("UPDATE uploadstreamers SET tags = '[]'")
        .execute(&f.pool)
        .await
        .unwrap();
    let response = send(
        &f.app,
        Some(&op),
        "POST",
        "/v1/publish-jobs",
        Some(json!({ "clip_ids": [a] })),
    )
    .await;
    assert!(
        text_of(response, StatusCode::CONFLICT)
            .await
            .contains("至少一个标签")
    );
    assert_eq!(f.jobs(&op).await["jobs"], json!([]));
}

#[tokio::test]
async fn rate_limit_pauses_and_clips_in_the_queue_are_locked() {
    let f = fixture().await;
    let op = login(&f.app, "op", "operator-password").await;
    let a = f.clip(&op, 1000, 2000, "a").await;
    let b = f.clip(&op, 3000, 5000, "b").await;
    f.fake
        .upload_failures
        .lock()
        .unwrap()
        .push_back(Failure::RateLimited(format!("{RATE_LIMITED}（601）")));
    let job = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            "/v1/publish-jobs",
            Some(json!({ "clip_ids": [a, b], "combine": true })),
        )
        .await,
        StatusCode::ACCEPTED,
    )
    .await;
    let jid = job["jobs"][0]["id"].as_u64().unwrap();
    let paused = f.wait_job(&op, jid, "paused").await;
    assert!(paused["error"].as_str().unwrap().starts_with(RATE_LIMITED));
    assert!(
        f.jobs(&op).await["paused"]
            .as_str()
            .unwrap()
            .starts_with(RATE_LIMITED)
    );

    let one = format!("/v1/sessions/{}/clips/{a}", f.session);
    let response = send(
        &f.app,
        Some(&op),
        "PATCH",
        &one,
        Some(json!({ "in_ms": 500 })),
    )
    .await;
    assert!(
        text_of(response, StatusCode::CONFLICT)
            .await
            .contains("发布队列")
    );
    let response = send(&f.app, Some(&op), "DELETE", &one, None).await;
    assert!(
        text_of(response, StatusCode::CONFLICT)
            .await
            .contains("移出队列")
    );
    // 标题等不影响已经排好的任务，可以改
    let response = send(
        &f.app,
        Some(&op),
        "PATCH",
        &one,
        Some(json!({ "title": "a2" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = send(&f.app, Some(&op), "POST", "/v1/publish-jobs/resume", None).await;
    assert_eq!(
        json_of(response, StatusCode::OK).await["paused"],
        Value::Null
    );
    let done = f.wait_job(&op, jid, "done").await;
    assert_eq!(done["uploaded"], 2);
    let studio = f.fake.submitted.lock().unwrap()[0].clone();
    assert_eq!(studio["videos"].as_array().unwrap().len(), 2);
    assert_eq!(studio["videos"][0]["title"], "a2");
    assert_eq!(studio["copyright"], 2);

    let response = send(
        &f.app,
        Some(&op),
        "POST",
        &format!("/v1/publish-jobs/{jid}/retry"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let response = send(
        &f.app,
        Some(&op),
        "DELETE",
        &format!("/v1/publish-jobs/{jid}"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = send(&f.app, Some(&op), "DELETE", "/v1/publish-jobs/99", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

fn png() -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[0; 32]);
    bytes
}

#[tokio::test]
async fn covers_and_publish_settings_are_stored_on_the_clip() {
    let f = fixture().await;
    let op = login(&f.app, "op", "operator-password").await;
    let viewer = login(&f.app, "ro", "viewer-password").await;
    let id = f.clip(&op, 1000, 2000, "a").await;
    let cover = format!("/v1/clips/{id}/cover");

    let response = send_bytes(&f.app, &viewer, &cover, "image/png", png()).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = send_bytes(&f.app, &op, &cover, "image/gif", b"GIF89a".to_vec()).await;
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let too_big = vec![0xff; thumb::MAX_JPEG_BYTES + 1];
    let response = send_bytes(&f.app, &op, &cover, "image/jpeg", too_big).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let over = json_of(
        send_bytes(&f.app, &op, &cover, "image/png", png()).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(over["cover"], json!({ "source": "upload" }));
    let response = send(&f.app, Some(&viewer), "GET", &cover, None).await;
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(body_of(response, StatusCode::OK).await, png());
    let file = f
        .dir
        .path()
        .join(format!("clips/{}/{id}-cover.jpg", f.session));
    assert!(file.exists());

    // 这一场没记下直播间封面
    let response = send(
        &f.app,
        Some(&op),
        "PUT",
        &cover,
        Some(json!({ "live": true })),
    )
    .await;
    assert!(
        text_of(response, StatusCode::CONFLICT)
            .await
            .contains("直播间封面")
    );
    let response = send(&f.app, Some(&op), "PUT", &cover, Some(json!({}))).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // PATCH 发布设置：覆盖整体替换，封面跟着切片走
    let one = format!("/v1/sessions/{}/clips/{id}", f.session);
    let clip = json_of(
        send(
            &f.app,
            Some(&op),
            "PATCH",
            &one,
            Some(json!({
                "template_id": f.template,
                "studio_override": { "title": "{clip_title}!", "cover": { "source": "upload" } },
            })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(clip["template_id"], f.template);
    assert_eq!(clip["studio_override"]["title"], "{clip_title}!");
    let preview = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            "/v1/publish-jobs/preview",
            Some(json!({ "clip_ids": [id] })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(preview["archives"][0]["rendered"]["title"], "a!");
    assert_eq!(
        preview["archives"][0]["cover"],
        json!({ "source": "upload" })
    );

    let response = send(
        &f.app,
        Some(&op),
        "PATCH",
        &one,
        Some(json!({ "studio_override": { "tags": vec!["x"; 13] } })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let clip = json_of(
        send(
            &f.app,
            Some(&op),
            "PATCH",
            &one,
            Some(json!({ "template_id": null, "studio_override": null })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(clip["template_id"], Value::Null);
    assert_eq!(clip["studio_override"], json!({}));

    // 封面文件被删了：发布前就拒绝
    sqlx::query("UPDATE clips SET studio_override = '{\"cover\":{\"source\":\"upload\"}}'")
        .execute(&f.pool)
        .await
        .unwrap();
    std::fs::remove_file(&file).unwrap();
    let response = send(
        &f.app,
        Some(&op),
        "POST",
        &format!("/v1/clips/{id}/publish"),
        Some(json!({})),
    )
    .await;
    assert!(
        text_of(response, StatusCode::CONFLICT)
            .await
            .contains("重新选一次封面")
    );

    send_bytes(&f.app, &op, &cover, "image/png", png()).await;
    assert!(file.exists());
    let response = send(&f.app, Some(&op), "DELETE", &cover, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!file.exists());
    let response = send(&f.app, Some(&op), "GET", &cover, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 删切片时封面文件一起删
    send_bytes(&f.app, &op, &cover, "image/png", png()).await;
    let response = send(&f.app, Some(&op), "DELETE", &one, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!file.exists());
}

/// 用系统的 FFmpeg 编一段真的 H.264 FLV，取帧、存成封面。没有 FFmpeg 时跳过。
#[tokio::test]
async fn frames_are_grabbed_from_real_recordings() {
    if !crate::tools::ffmpeg_status().await.available {
        eprintln!("没有 FFmpeg，跳过取帧测试");
        return;
    }
    let f = fixture().await;
    let path: PathBuf = f.dir.path().join("real.flv");
    let status = crate::tools::ffmpeg_command()
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg("testsrc=size=640x360:rate=25:duration=3")
        .args(["-c:v", "libx264", "-g", "25", "-pix_fmt", "yuv420p", "-y"])
        .arg(&path)
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('b', 'https://b', 't', '2026-09-24 00:00:00', '', 1, 2) RETURNING id",
    )
    .fetch_one(&f.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms)
         VALUES (?, ?, 'flv', 'finished', 0, 3000)",
    )
    .bind(session)
    .bind(path.to_string_lossy().into_owned())
    .execute(&f.pool)
    .await
    .unwrap();

    let viewer = login(&f.app, "ro", "viewer-password").await;
    let response = send(
        &f.app,
        Some(&viewer),
        "GET",
        &format!("/v1/sessions/{session}/thumb?t=1500&w=320"),
        None,
    )
    .await;
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
    let jpeg = body_of(response, StatusCode::OK).await;
    assert!(jpeg.starts_with(&[0xff, 0xd8, 0xff]));
    assert!(jpeg.len() > 1000);

    let response = send(
        &f.app,
        Some(&viewer),
        "GET",
        &format!("/v1/sessions/{session}/thumb?t=99000"),
        None,
    )
    .await;
    let message = text_of(response, StatusCode::CONFLICT).await;
    assert!(message.contains("换个时间点再取"), "{message}");
    assert!(!message.contains("范围"), "{message}");
    let response = send(
        &f.app,
        Some(&viewer),
        "GET",
        "/v1/sessions/999/thumb?t=0",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let op = login(&f.app, "op", "operator-password").await;
    let clip = json_of(
        send(
            &f.app,
            Some(&op),
            "POST",
            &format!("/v1/sessions/{session}/clips"),
            Some(json!({ "in_ms": 500, "out_ms": 2500 })),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    let id = clip["id"].as_i64().unwrap();
    let over = json_of(
        send(
            &f.app,
            Some(&op),
            "PUT",
            &format!("/v1/clips/{id}/cover"),
            Some(json!({ "t": 2000 })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(over["cover"], json!({ "source": "frame", "t_ms": 2000 }));
    let saved =
        std::fs::read(f.dir.path().join(format!("clips/{session}/{id}-cover.jpg"))).unwrap();
    assert!(saved.starts_with(&[0xff, 0xd8, 0xff]));
}
