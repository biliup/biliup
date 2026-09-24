//! 切片工作台的标记接口：`/v1/sessions/{id}/markers[/{mid}]`。
//!
//! 看列表要 `file.view`，增删改要 `clip.edit`（只读观察者没有）。只按场次 id、标记 id 寻址。

use crate::server::api::access::Caller;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::markers::{
    self, DEFAULT_LOOKBACK_MS, MAX_LABEL_CHARS, MAX_MARKERS_PER_SESSION, MAX_RANGE_MS, Marker,
    MarkerChanges, NewMarker, Timing,
};
use crate::server::workbench::{live, recorder, store};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{debug, warn};

fn internal(error: impl std::fmt::Display) -> Response {
    warn!(%error, "标记接口出错");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

fn bad_request(message: String) -> Response {
    (StatusCode::BAD_REQUEST, message).into_response()
}

fn session_not_found() -> Response {
    (StatusCode::NOT_FOUND, "场次不存在").into_response()
}

fn marker_not_found() -> Response {
    (StatusCode::NOT_FOUND, "标记不存在，可能已经被删掉了").into_response()
}

#[derive(Debug, Serialize)]
pub struct MarkerList {
    pub markers: Vec<Marker>,
}

/// `GET /v1/sessions/{id}/markers`
pub async fn list_markers(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    }
    match markers::list(&pool, id).await {
        Ok(markers) => Json(MarkerList { markers }).into_response(),
        Err(e) => internal(e),
    }
}

/// `POST /v1/sessions/{id}/markers` 的请求体。
///
/// 给了 `at_ms` 就按它记（回看时播放器位置本身就是场次时间）；否则按「现在」换算，要求这一场
/// 正在录：`pressed_at` / `client_now` 是客户端按下标记、发出请求时自己的时钟（Unix 毫秒，只用
/// 两者之差），`latency_ms` 是播放器缓冲里还没播出的时长。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMarker {
    pub at_ms: Option<i64>,
    pub client_now: Option<i64>,
    pub pressed_at: Option<i64>,
    pub latency_ms: Option<i64>,
    #[serde(default)]
    pub label: String,
    pub color: Option<String>,
    pub lookback_ms: Option<i64>,
    pub lookahead_ms: Option<i64>,
}

fn check_label(label: &str) -> Result<String, String> {
    let label = label.trim();
    if label.chars().count() > MAX_LABEL_CHARS {
        return Err(format!("标记名最多 {MAX_LABEL_CHARS} 个字"));
    }
    if label.chars().any(char::is_control) {
        return Err("标记名里不能有换行或控制字符".into());
    }
    Ok(label.to_string())
}

/// 颜色只收 `#rrggbb`；空串表示不要颜色。
fn check_color(color: &str) -> Result<Option<String>, String> {
    let color = color.trim();
    if color.is_empty() {
        return Ok(None);
    }
    let hex = color.strip_prefix('#').unwrap_or_default();
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("颜色要写成 #rrggbb，例如 #ff6600".into());
    }
    Ok(Some(color.to_ascii_lowercase()))
}

fn check_range(name: &str, value: i64) -> Result<i64, String> {
    if (0..=MAX_RANGE_MS).contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "{name} 要在 0 到 {MAX_RANGE_MS} 毫秒（10 分钟）之间"
        ))
    }
}

fn check_at(at_ms: i64) -> Result<i64, String> {
    if at_ms >= 0 {
        Ok(at_ms)
    } else {
        Err("at_ms 不能是负数".into())
    }
}

/// `POST /v1/sessions/{id}/markers`
pub async fn create_marker(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Json(body): Json<CreateMarker>,
) -> Response {
    let fields = (|| {
        Ok::<_, String>((
            check_label(&body.label)?,
            body.color
                .as_deref()
                .map(check_color)
                .transpose()?
                .flatten(),
            check_range(
                "lookback_ms",
                body.lookback_ms.unwrap_or(DEFAULT_LOOKBACK_MS),
            )?,
            check_range("lookahead_ms", body.lookahead_ms.unwrap_or(0))?,
            body.at_ms.map(check_at).transpose()?,
        ))
    })();
    let (label, color, lookback_ms, lookahead_ms, at_ms) = match fields {
        Ok(fields) => fields,
        Err(message) => return bad_request(message),
    };
    let session = match store::session(&pool, id).await {
        Ok(Some(session)) => session,
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    };
    let now = recorder::now_ms();
    let at_ms = match at_ms {
        Some(at_ms) => at_ms,
        None => {
            let timing = Timing {
                client_now: body.client_now,
                pressed_at: body.pressed_at,
                latency_ms: body.latency_ms,
            };
            match markers::watched_at_ms(&pool, id, now, timing).await {
                Some(at_ms) => {
                    debug!(session = id, at_ms, ?timing, "按当前画面换算标记时间");
                    at_ms
                }
                None if live::is_recording(id) || session.ended_at.is_none() => {
                    return (
                        StatusCode::CONFLICT,
                        "这次录制还没开出分段，等画面开始写盘后再标记",
                    )
                        .into_response();
                }
                None => {
                    return (
                        StatusCode::CONFLICT,
                        "这一场没有在录，不能按当前画面标记；请在工作台里按时间标记",
                    )
                        .into_response();
                }
            }
        }
    };
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM markers WHERE session_id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
    {
        Ok(n) if n >= MAX_MARKERS_PER_SESSION => {
            return (
                StatusCode::CONFLICT,
                format!("这一场的标记已达上限（{MAX_MARKERS_PER_SESSION} 个），先删掉一些再标"),
            )
                .into_response();
        }
        Ok(_) => {}
        Err(e) => return internal(e),
    }
    let marker = NewMarker {
        at_ms,
        label,
        color,
        created_by: caller.subject.user_id,
        created_at: now,
        lookback_ms,
        lookahead_ms,
    };
    match markers::insert(&pool, id, &marker).await {
        Ok(marker) => (StatusCode::CREATED, Json(marker)).into_response(),
        Err(e) => internal(e),
    }
}

/// 区分「没给这个键」（`None`）和「给了 `null`」（`Some(None)`）。
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// `PATCH /v1/sessions/{id}/markers/{mid}` 的请求体：只改给出的字段；`color: null` 或 `""` 清掉颜色。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMarker {
    pub at_ms: Option<i64>,
    pub label: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub color: Option<Option<String>>,
    pub lookback_ms: Option<i64>,
    pub lookahead_ms: Option<i64>,
}

/// `PATCH /v1/sessions/{id}/markers/{mid}`
pub async fn update_marker(
    State(pool): State<ConnectionPool>,
    Path((id, mid)): Path<(i64, i64)>,
    Json(body): Json<UpdateMarker>,
) -> Response {
    let changes = (|| {
        Ok::<_, String>(MarkerChanges {
            at_ms: body.at_ms.map(check_at).transpose()?,
            label: body.label.as_deref().map(check_label).transpose()?,
            color: match &body.color {
                None => None,
                Some(None) => Some(None),
                Some(Some(color)) => Some(check_color(color)?),
            },
            lookback_ms: body
                .lookback_ms
                .map(|v| check_range("lookback_ms", v))
                .transpose()?,
            lookahead_ms: body
                .lookahead_ms
                .map(|v| check_range("lookahead_ms", v))
                .transpose()?,
        })
    })();
    let changes = match changes {
        Ok(changes) => changes,
        Err(message) => return bad_request(message),
    };
    match markers::update(&pool, id, mid, &changes).await {
        Ok(Some(marker)) => Json(marker).into_response(),
        Ok(None) => marker_not_found(),
        Err(e) => internal(e),
    }
}

/// `DELETE /v1/sessions/{id}/markers/{mid}`
pub async fn delete_marker(
    State(pool): State<ConnectionPool>,
    Path((id, mid)): Path<(i64, i64)>,
) -> Response {
    match markers::delete(&pool, id, mid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => marker_not_found(),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::access::require_permission;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::permissions::Role;
    use crate::server::infrastructure::users::{Backend, Credentials};
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, header};
    use axum::middleware::from_fn;
    use axum::routing::{get, patch};
    use axum_login::AuthManagerLayerBuilder;
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    struct Fixture {
        _dir: tempfile::TempDir,
        backend: Backend,
        app: Router,
        /// 正在录的一场
        live: i64,
        /// 已经结束的一场
        ended: i64,
        guard: live::LiveGuard,
    }

    async fn insert_session(pool: &ConnectionPool, ended_at: Option<i64>) -> i64 {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
             VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1, ?)
             RETURNING id",
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
        let live = insert_session(&pool, None).await;
        let ended = insert_session(&pool, Some(2)).await;
        let guard = live::register(live, 1, None);

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
        let app = Router::new()
            .route(
                "/v1/sessions/{id}/markers",
                get(list_markers).post(create_marker),
            )
            .route(
                "/v1/sessions/{id}/markers/{mid}",
                patch(update_marker).delete(delete_marker),
            )
            .with_state(pool)
            .route_layer(from_fn(require_permission))
            .merge(crate::server::api::auth::router())
            .layer(auth_layer);
        Fixture {
            _dir: dir,
            backend,
            app,
            live,
            ended,
            guard,
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

    /// 标记路由的要求都由策略层按路由表给出：列出跟场次详情一样要 `file.view`，打 / 改 / 删要
    /// `clip.edit`（Viewer 没有）；表里没登记的方法按默认拒绝只给超管。处理函数里不再判断角色。
    #[test]
    fn marker_routes_are_declared_in_the_policy() {
        use crate::server::infrastructure::permissions::Permission;
        use crate::server::infrastructure::policy::{RouteRequirement, Subject};
        use axum::http::Method;
        let list = ("/v1/sessions/{id}/markers", "/v1/sessions/1/markers");
        let one = (
            "/v1/sessions/{id}/markers/{mid}",
            "/v1/sessions/1/markers/2",
        );
        for (method, (route, raw), permission) in [
            (Method::GET, list, Permission::FileView),
            (Method::POST, list, Permission::ClipEdit),
            (Method::PATCH, one, Permission::ClipEdit),
            (Method::DELETE, one, Permission::ClipEdit),
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
            assert!(Subject::unrestricted().satisfies(requirement));
        }
        assert!(
            !Subject::user(1, Role::Viewer).satisfies(RouteRequirement::of(
                &Method::POST,
                list.0,
                list.1
            ))
        );
        for (method, (route, raw)) in [(Method::PUT, list), (Method::GET, one)] {
            assert_eq!(
                RouteRequirement::of(&method, route, raw),
                RouteRequirement::AdminOnly,
                "{method} {route}"
            );
        }
    }

    #[tokio::test]
    async fn anonymous_is_401_and_viewer_can_only_list() {
        let f = fixture().await;
        let list = format!("/v1/sessions/{}/markers", f.live);
        let one = format!("/v1/sessions/{}/markers/1", f.live);
        for (method, uri) in [
            ("GET", &list),
            ("POST", &list),
            ("PATCH", &one),
            ("DELETE", &one),
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
        assert_eq!(body["markers"], json!([]));
        for (method, uri) in [("POST", &list), ("PATCH", &one), ("DELETE", &one)] {
            let response = call(&f.app, Some(&viewer), method, uri, Some(json!({}))).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method} {uri}");
        }
        let response = call(&f.app, Some(&viewer), "GET", "/v1/sessions/1/markers", None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn operator_marks_the_live_picture_renames_and_undoes() {
        let f = fixture().await;
        let operator = login(&f.app, "op", "operator-password").await;
        let op_id = f.backend.find_by_username("op").await.unwrap().unwrap().id;
        let list = format!("/v1/sessions/{}/markers", f.live);

        // 场次时间 30 s 处是 10 s 前录下来的；播放器落后 2 s，按下 0.3 s 后才发出请求
        f.guard.anchor(30_000, recorder::now_ms() - 10_000);
        let body = json!({ "client_now": 5_000_300, "pressed_at": 5_000_000, "latency_ms": 2_000 });
        let marker = json_of(
            call(&f.app, Some(&operator), "POST", &list, Some(body)).await,
            StatusCode::CREATED,
        )
        .await;
        let at_ms = marker["at_ms"].as_i64().unwrap();
        assert!(
            (37_700..37_700 + 1_000).contains(&at_ms),
            "30 s + 10 s − 2 s − 0.3 s，实际 {at_ms}"
        );
        assert_eq!(marker["lookback_ms"], DEFAULT_LOOKBACK_MS);
        assert_eq!(marker["lookahead_ms"], 0);
        assert_eq!(marker["label"], "");
        assert_eq!(marker["created_by"], op_id);
        let id = marker["id"].as_i64().unwrap();
        let one = format!("{list}/{id}");

        let renamed = json_of(
            call(
                &f.app,
                Some(&operator),
                "PATCH",
                &one,
                Some(json!({ "label": "  团战  ", "color": "#FF6600" })),
            )
            .await,
            StatusCode::OK,
        )
        .await;
        assert_eq!(renamed["label"], "团战");
        assert_eq!(renamed["color"], "#ff6600");
        assert_eq!(renamed["at_ms"], at_ms);
        let cleared = json_of(
            call(
                &f.app,
                Some(&operator),
                "PATCH",
                &one,
                Some(json!({ "color": null })),
            )
            .await,
            StatusCode::OK,
        )
        .await;
        assert_eq!(cleared["color"], Value::Null);
        assert_eq!(cleared["label"], "团战");

        let listed = json_of(
            call(&f.app, Some(&operator), "GET", &list, None).await,
            StatusCode::OK,
        )
        .await;
        assert_eq!(listed["markers"].as_array().unwrap().len(), 1);

        let response = call(&f.app, Some(&operator), "DELETE", &one, None).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let response = call(&f.app, Some(&operator), "DELETE", &one, None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = call(
            &f.app,
            Some(&operator),
            "PATCH",
            &one,
            Some(json!({ "label": "x" })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn explicit_time_validation_and_sessions_not_recording() {
        let f = fixture().await;
        let admin = login(&f.app, "biliup", "admin-password").await;
        let ended = format!("/v1/sessions/{}/markers", f.ended);

        let text = text_of(
            call(&f.app, Some(&admin), "POST", &ended, Some(json!({}))).await,
            StatusCode::CONFLICT,
        )
        .await;
        assert!(text.contains("没有在录"), "{text}");
        let marker = json_of(
            call(
                &f.app,
                Some(&admin),
                "POST",
                &ended,
                Some(json!({ "at_ms": 123_456, "label": "回看时标的", "lookback_ms": 30_000, "lookahead_ms": 5_000 })),
            )
            .await,
            StatusCode::CREATED,
        )
        .await;
        assert_eq!(marker["at_ms"], 123_456);
        assert_eq!(marker["lookback_ms"], 30_000);
        assert_eq!(marker["lookahead_ms"], 5_000);

        let live = format!("/v1/sessions/{}/markers", f.live);
        let text = text_of(
            call(&f.app, Some(&admin), "POST", &live, Some(json!({}))).await,
            StatusCode::CONFLICT,
        )
        .await;
        assert!(text.contains("还没开出分段"), "登记了但还没有锚点：{text}");

        for body in [
            json!({ "at_ms": -1 }),
            json!({ "at_ms": 0, "label": "长".repeat(MAX_LABEL_CHARS + 1) }),
            json!({ "at_ms": 0, "label": "a\nb" }),
            json!({ "at_ms": 0, "color": "red" }),
            json!({ "at_ms": 0, "lookback_ms": MAX_RANGE_MS + 1 }),
            json!({ "at_ms": 0, "lookahead_ms": -1 }),
        ] {
            let response = call(&f.app, Some(&admin), "POST", &ended, Some(body.clone())).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
        }
        let response = call(
            &f.app,
            Some(&admin),
            "POST",
            &ended,
            Some(json!({ "at_ms": 0, "typo": 1 })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let response = call(
            &f.app,
            Some(&admin),
            "POST",
            "/v1/sessions/1/markers",
            Some(json!({ "at_ms": 0 })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let id = marker["id"].as_i64().unwrap();
        let response = call(
            &f.app,
            Some(&admin),
            "DELETE",
            &format!("{live}/{id}"),
            None,
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "别的场次的标记删不到"
        );
    }
}
