use super::*;
use crate::server::api::access::require_permission;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::permissions::Role;
use crate::server::infrastructure::users::{Backend, Credentials};
use crate::server::workbench::index::tests::build_flv;
use axum::Router;
use axum::extract::FromRef;
use axum::http::Request;
use axum::middleware::from_fn;
use axum::routing::{get, patch, post};
use axum_login::AuthManagerLayerBuilder;
use serde_json::{Value, json};
use std::time::Duration;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone, FromRef)]
struct AppState {
    pool: ConnectionPool,
    clips: Arc<ClipExports>,
}

struct Fixture {
    _dir: tempfile::TempDir,
    app: Router,
    pool: ConnectionPool,
    /// 录完的一场：两段 FLV `[0, 3965)`、`[3965, 7930)`
    ended: i64,
    /// 正在录的一场（登记了但还没有锚点）
    live: i64,
    _guard: live::LiveGuard,
}

async fn insert_session(pool: &ConnectionPool, ended_at: Option<i64>) -> i64 {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1, ?) RETURNING id",
    )
    .bind(ended_at)
    .fetch_one(pool)
    .await
    .unwrap();
    live::unique_session_id(pool, id).await
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let ended = insert_session(&pool, Some(2)).await;
    let live_id = insert_session(&pool, None).await;
    let guard = live::register(live_id, 1, None);
    for (name, start) in [("a.flv", 0i64), ("b.flv", 3965)] {
        let path = dir.path().join(name);
        std::fs::write(&path, build_flv(0, 100, 25, None).bytes).unwrap();
        sqlx::query(
            "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms)
             VALUES (?, ?, 'flv', 'finished', ?, ?)",
        )
        .bind(ended)
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
    let state = AppState {
        pool: pool.clone(),
        clips: Arc::new(ClipExports::new(pool.clone(), dir.path().join("clips"))),
    };
    let app = Router::new()
        .route("/v1/sessions/{id}/clips", get(list_clips).post(create_clip))
        .route(
            "/v1/sessions/{id}/clips/{cid}",
            patch(update_clip).delete(delete_clip),
        )
        .route("/v1/clips/{cid}", get(get_clip))
        .route("/v1/clips/{cid}/export", post(export_clip))
        .route("/v1/clips/{cid}/download", get(download_clip))
        .with_state(state)
        .route_layer(from_fn(require_permission))
        .merge(crate::server::api::auth::router())
        .layer(auth_layer);
    Fixture {
        _dir: dir,
        app,
        pool,
        ended,
        live: live_id,
        _guard: guard,
    }
}

async fn login(app: &Router, username: &str, password: &str) -> String {
    let response = call(
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

async fn call(
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

async fn json_of(response: Response, status: StatusCode) -> Value {
    assert_eq!(response.status(), status);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn text_of(response: Response, status: StatusCode) -> String {
    assert_eq!(response.status(), status);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

/// 切片路由的要求都由策略层按路由表给出：看列表、查一个、下载要 `file.view`，建 / 改 / 删 /
/// 导出要 `clip.edit`（Viewer 没有）；表里没登记的方法只给超管。处理函数里不判断角色。
#[test]
fn clip_routes_are_declared_in_the_policy() {
    use crate::server::infrastructure::permissions::Permission;
    use crate::server::infrastructure::policy::{RouteRequirement, Subject};
    use axum::http::Method;
    let list = ("/v1/sessions/{id}/clips", "/v1/sessions/1/clips");
    let one = ("/v1/sessions/{id}/clips/{cid}", "/v1/sessions/1/clips/2");
    let clip = ("/v1/clips/{cid}", "/v1/clips/2");
    let export = ("/v1/clips/{cid}/export", "/v1/clips/2/export");
    let download = ("/v1/clips/{cid}/download", "/v1/clips/2/download");
    for (method, (route, raw), permission) in [
        (Method::GET, list, Permission::FileView),
        (Method::POST, list, Permission::ClipEdit),
        (Method::PATCH, one, Permission::ClipEdit),
        (Method::DELETE, one, Permission::ClipEdit),
        (Method::GET, clip, Permission::FileView),
        (Method::POST, export, Permission::ClipEdit),
        (Method::GET, download, Permission::FileView),
    ] {
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
    for (method, (route, raw)) in [
        (Method::PUT, list),
        (Method::GET, one),
        (Method::DELETE, clip),
        (Method::GET, export),
        (Method::POST, download),
    ] {
        assert_eq!(
            RouteRequirement::of(&method, route, raw),
            RouteRequirement::AdminOnly,
            "{method} {route}"
        );
    }
}

#[tokio::test]
async fn anonymous_is_401_and_viewer_can_only_look_and_download() {
    let f = fixture().await;
    let list = format!("/v1/sessions/{}/clips", f.ended);
    let one = format!("{list}/1");
    for (method, uri) in [
        ("GET", list.as_str()),
        ("POST", list.as_str()),
        ("PATCH", one.as_str()),
        ("DELETE", one.as_str()),
        ("GET", "/v1/clips/1"),
        ("POST", "/v1/clips/1/export"),
        ("GET", "/v1/clips/1/download"),
    ] {
        let response = call(&f.app, None, method, uri, Some(json!({}))).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
    let viewer = login(&f.app, "ro", "viewer-password").await;
    let body = json_of(
        call(&f.app, Some(&viewer), "GET", &list, None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["clips"], json!([]));
    for (method, uri) in [
        ("POST", list.as_str()),
        ("PATCH", one.as_str()),
        ("DELETE", one.as_str()),
        ("POST", "/v1/clips/1/export"),
    ] {
        let response = call(&f.app, Some(&viewer), method, uri, Some(json!({}))).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method} {uri}");
    }
    let response = call(&f.app, Some(&viewer), "GET", "/v1/clips/1/download", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn wait_exported(f: &Fixture, cookie: &str, id: i64) -> Value {
    for _ in 0..500 {
        let clip = json_of(
            call(
                &f.app,
                Some(cookie),
                "GET",
                &format!("/v1/clips/{id}"),
                None,
            )
            .await,
            StatusCode::OK,
        )
        .await;
        if clip["state"] != "exporting" {
            return clip;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("导出没有结束");
}

#[tokio::test]
async fn operator_saves_exports_downloads_and_deletes_a_clip() {
    let f = fixture().await;
    let operator = login(&f.app, "op", "operator-password").await;
    let list = format!("/v1/sessions/{}/clips", f.ended);

    let clip = json_of(
        call(
            &f.app,
            Some(&operator),
            "POST",
            &list,
            Some(json!({ "in_ms": 1500, "out_ms": 5000, "title": "  团战 / 翻盘  " })),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(clip["state"], "draft");
    assert_eq!(clip["title"], "团战 / 翻盘");
    assert_eq!(clip["file_name"], Value::Null);
    assert!(clip.get("output_path").is_none(), "不把路径发给前端");
    let id = clip["id"].as_i64().unwrap();
    let one = format!("{list}/{id}");
    let pins: Vec<i64> = sqlx::query_scalar("SELECT pin_count FROM segments ORDER BY id")
        .fetch_all(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, vec![1, 1], "建切片就引用源录像");

    let response = call(
        &f.app,
        Some(&operator),
        "GET",
        &format!("/v1/clips/{id}/download"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT, "还没导出");

    let started = json_of(
        call(
            &f.app,
            Some(&operator),
            "POST",
            &format!("/v1/clips/{id}/export"),
            Some(json!({ "mode": "quick" })),
        )
        .await,
        StatusCode::ACCEPTED,
    )
    .await;
    assert_eq!(started["state"], "exporting");
    assert_eq!(started["mode"], "quick");
    let done = wait_exported(&f, &operator, id).await;
    assert_eq!(done["state"], "ready", "{done}");
    assert_eq!(done["cut_in_ms"], 1000);
    assert_eq!(done["cut_out_ms"], 3965 + 2000);
    assert_eq!(done["file_name"], format!("{id}.flv"));

    let response = call(
        &f.app,
        Some(&operator),
        "GET",
        &format!("/v1/clips/{id}/download"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let disposition = response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(
        disposition.contains(&*urlencoding::encode("团战 _ 翻盘.flv")),
        "{disposition}"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.len() as i64, done["output_bytes"].as_i64().unwrap());
    assert_eq!(&body[..3], b"FLV");

    // 改标题不影响导出结果；改范围作废，要重新导出
    let renamed = json_of(
        call(
            &f.app,
            Some(&operator),
            "PATCH",
            &one,
            Some(json!({ "title": "新标题" })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(renamed["state"], "ready");
    let moved = json_of(
        call(
            &f.app,
            Some(&operator),
            "PATCH",
            &one,
            Some(json!({ "in_ms": 4000 })),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(moved["state"], "draft");
    assert_eq!(moved["file_name"], Value::Null);
    let pins: Vec<i64> = sqlx::query_scalar("SELECT pin_count FROM segments ORDER BY id")
        .fetch_all(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, vec![0, 1]);
    let text = text_of(
        call(
            &f.app,
            Some(&operator),
            "PATCH",
            &one,
            Some(json!({ "out_ms": 100 })),
        )
        .await,
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert!(text.contains("出点要在入点之后"), "{text}");

    let listed = json_of(
        call(&f.app, Some(&operator), "GET", &list, None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(listed["clips"].as_array().unwrap().len(), 1);

    let response = call(&f.app, Some(&operator), "DELETE", &one, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let pins: Vec<i64> = sqlx::query_scalar("SELECT pin_count FROM segments ORDER BY id")
        .fetch_all(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, vec![0, 0], "删掉切片就撤销引用");
    let response = call(&f.app, Some(&operator), "DELETE", &one, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = call(
        &f.app,
        Some(&operator),
        "GET",
        &format!("/v1/clips/{id}"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn validation_and_live_cuts_need_a_recording_session() {
    let f = fixture().await;
    let admin = login(&f.app, "biliup", "admin-password").await;
    let ended = format!("/v1/sessions/{}/clips", f.ended);
    for body in [
        json!({ "in_ms": -1, "out_ms": 10 }),
        json!({ "in_ms": 10, "out_ms": 10 }),
        json!({ "in_ms": 0, "out_ms": MAX_CLIP_MS + 1 }),
        json!({ "in_ms": 0 }),
        json!({ "in_ms": 0, "out_ms": 10, "last_ms": 30_000 }),
        json!({ "last_ms": 500 }),
        json!({ "in_ms": 0, "out_ms": 10, "title": "a\nb" }),
        json!({ "in_ms": 0, "out_ms": 10, "title": "长".repeat(MAX_TITLE_CHARS + 1) }),
        json!({ "in_ms": 0, "out_ms": 10, "marker_id": 999 }),
    ] {
        let response = call(&f.app, Some(&admin), "POST", &ended, Some(body.clone())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
    }
    let response = call(
        &f.app,
        Some(&admin),
        "POST",
        &ended,
        Some(json!({ "in_ms": 0, "out_ms": 10, "export": "slow" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let text = text_of(
        call(
            &f.app,
            Some(&admin),
            "POST",
            &ended,
            Some(json!({ "last_ms": 60_000 })),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert!(text.contains("没有在录"), "{text}");
    let live = format!("/v1/sessions/{}/clips", f.live);
    let text = text_of(
        call(
            &f.app,
            Some(&admin),
            "POST",
            &live,
            Some(json!({ "last_ms": 60_000 })),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert!(text.contains("还没开出分段"), "{text}");
    let response = call(
        &f.app,
        Some(&admin),
        "POST",
        "/v1/sessions/999/clips",
        Some(json!({ "in_ms": 0, "out_ms": 10 })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 建好就导出；导出中不能再导出、不能改范围
    let clip = json_of(
        call(
            &f.app,
            Some(&admin),
            "POST",
            &ended,
            Some(json!({ "in_ms": 0, "out_ms": 7000, "export": "quick" })),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(clip["state"], "exporting");
    let id = clip["id"].as_i64().unwrap();
    let done = wait_exported(&f, &admin, id).await;
    assert_eq!(done["state"], "ready");
    let response = call(
        &f.app,
        Some(&admin),
        "POST",
        "/v1/clips/999/export",
        Some(json!({ "mode": "quick" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    // 别的场次删不到
    let response = call(
        &f.app,
        Some(&admin),
        "DELETE",
        &format!("/v1/sessions/{}/clips/{id}", f.live),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
