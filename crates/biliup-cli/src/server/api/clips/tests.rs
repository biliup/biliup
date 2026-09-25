use super::*;
use crate::server::api::access::require_permission;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::permissions::Role;
use crate::server::infrastructure::users::{Backend, Credentials};
use crate::server::workbench::index::tests::build_flv;
use crate::server::workbench::live;
use axum::Router;
use axum::body::Body;
use axum::extract::FromRef;
use axum::http::{Request, header};
use axum::middleware::from_fn;
use axum::routing::{get, patch};
use axum_login::AuthManagerLayerBuilder;
use serde_json::{Value, json};
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone, FromRef)]
struct AppState {
    pool: ConnectionPool,
}

struct Fixture {
    _dir: tempfile::TempDir,
    app: Router,
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
    let state = AppState { pool: pool.clone() };
    let app = Router::new()
        .route("/v1/sessions/{id}/clips", get(list_clips).post(create_clip))
        .route(
            "/v1/sessions/{id}/clips/{cid}",
            patch(update_clip).delete(delete_clip),
        )
        .route("/v1/clips/{cid}", get(get_clip))
        .with_state(state)
        .route_layer(from_fn(require_permission))
        .merge(crate::server::api::auth::router())
        .layer(auth_layer);
    Fixture {
        _dir: dir,
        app,
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

/// 切片路由的要求都由策略层按路由表给出：看列表、查一个要 `file.view`，建 / 改 / 删要
/// `clip.edit`（Viewer 没有）；表里没登记的方法只给超管。处理函数里不判断角色。
#[test]
fn clip_routes_are_declared_in_the_policy() {
    use crate::server::infrastructure::permissions::Permission;
    use crate::server::infrastructure::policy::{RouteRequirement, Subject};
    use axum::http::Method;
    let list = ("/v1/sessions/{id}/clips", "/v1/sessions/1/clips");
    let one = ("/v1/sessions/{id}/clips/{cid}", "/v1/sessions/1/clips/2");
    let clip = ("/v1/clips/{cid}", "/v1/clips/2");
    for (method, (route, raw), permission) in [
        (Method::GET, list, Permission::FileView),
        (Method::POST, list, Permission::ClipEdit),
        (Method::PATCH, one, Permission::ClipEdit),
        (Method::DELETE, one, Permission::ClipEdit),
        (Method::GET, clip, Permission::FileView),
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
    ] {
        assert_eq!(
            RouteRequirement::of(&method, route, raw),
            RouteRequirement::AdminOnly,
            "{method} {route}"
        );
    }
}

#[tokio::test]
async fn anonymous_is_401_and_viewer_can_only_look() {
    let f = fixture().await;
    let list = format!("/v1/sessions/{}/clips", f.ended);
    let one = format!("{list}/1");
    for (method, uri) in [
        ("GET", list.as_str()),
        ("POST", list.as_str()),
        ("PATCH", one.as_str()),
        ("DELETE", one.as_str()),
        ("GET", "/v1/clips/1"),
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
    ] {
        let response = call(&f.app, Some(&viewer), method, uri, Some(json!({}))).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method} {uri}");
    }
    let response = call(&f.app, Some(&viewer), "GET", "/v1/clips/1", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn operator_saves_edits_and_deletes_a_clip() {
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
    assert_eq!(renamed["title"], "新标题");
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
    assert_eq!(
        (moved["in_ms"].as_i64(), moved["out_ms"].as_i64()),
        (Some(4000), Some(5000))
    );
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
async fn clip_requests_are_validated() {
    let f = fixture().await;
    let admin = login(&f.app, "biliup", "admin-password").await;
    let ended = format!("/v1/sessions/{}/clips", f.ended);
    for body in [
        json!({ "in_ms": -1, "out_ms": 10 }),
        json!({ "in_ms": 10, "out_ms": 10 }),
        json!({ "in_ms": 0, "out_ms": MAX_CLIP_MS + 1 }),
        json!({ "in_ms": 0 }),
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
        "/v1/sessions/999/clips",
        Some(json!({ "in_ms": 0, "out_ms": 10 })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let clip = json_of(
        call(
            &f.app,
            Some(&admin),
            "POST",
            &ended,
            Some(json!({ "in_ms": 0, "out_ms": 7000 })),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    let id = clip["id"].as_i64().unwrap();
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
