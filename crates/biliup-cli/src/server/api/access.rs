//! 业务路由的访问控制：登录校验 + 按路由表的权限校验（默认拒绝），
//! 以及长连接在握手之后的定期复查。

use crate::server::infrastructure::permissions::Permission;
use crate::server::infrastructure::policy::{Field, RouteRequirement, Subject};
use crate::server::infrastructure::users::{AuthSession, Backend};
use axum::Json;
use axum::body::Body;
use axum::extract::{FromRequestParts, MatchedPath, Request};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_login::{AuthUser, AuthnBackend};
use futures::StreamExt;
use futures::future::BoxFuture;
use std::time::Duration;

/// 长连接（WS、SSE、`/live` 流）握手之后不再经过中间件，每隔这么久复查一次用户状态与权限。
#[cfg(not(test))]
pub const RECHECK_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(test)]
pub const RECHECK_INTERVAL: Duration = Duration::from_millis(100);

/// 当前请求的调用者。由访问控制中间件放进请求扩展，处理函数据此决定脱敏与字段保护；
/// 判断本身都在 [`crate::server::infrastructure::policy`]。
#[derive(Clone)]
pub struct Caller {
    pub subject: Subject,
    watch: Option<SessionWatch>,
}

impl Caller {
    /// `--auth` 关闭时：零鉴权，视为超管。
    pub fn unrestricted() -> Self {
        Caller {
            subject: Subject::unrestricted(),
            watch: None,
        }
    }

    pub fn can(&self, permission: Permission) -> bool {
        self.subject.can(permission)
    }

    pub fn can_access(&self, field: Field) -> bool {
        self.subject.can_access(field)
    }

    /// 会话失效（禁用、删除、改密、强制下线）或失去这条路由的权限时完成；`--auth` 关闭时永不完成。
    pub fn revoked(&self) -> BoxFuture<'static, ()> {
        match self.watch.clone() {
            Some(watch) => Box::pin(watch.revoked()),
            None => Box::pin(std::future::pending()),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Caller {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 访问控制层挂在所有业务路由上；拿不到说明路由绕过了它，宁可失败也不放行。
        parts
            .extensions
            .get::<Caller>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[derive(Clone)]
struct SessionWatch {
    backend: Backend,
    user_id: i64,
    auth_hash: Vec<u8>,
    requirement: RouteRequirement,
}

impl SessionWatch {
    async fn still_valid(&self) -> Result<bool, crate::server::infrastructure::users::Error> {
        let user = self.backend.get_user(&self.user_id).await?;
        Ok(user.is_some_and(|user| {
            user.session_auth_hash() == self.auth_hash.as_slice()
                && Subject::user(user.id, user.role).satisfies(self.requirement)
        }))
    }

    async fn revoked(self) {
        loop {
            tokio::time::sleep(RECHECK_INTERVAL).await;
            match self.still_valid().await {
                Ok(true) => {}
                Ok(false) => return,
                // 数据库一时不可用不代表会话失效，下一轮再查
                Err(error) => {
                    tracing::warn!(error = ?error, "failed to re-check a long-lived session")
                }
            }
        }
    }
}

pub fn forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "message": "没有权限执行此操作" })),
    )
        .into_response()
}

/// `--auth` 开启时的访问控制层。
pub async fn require_permission(
    auth_session: AuthSession,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_owned())
        .unwrap_or_else(|| request.uri().path().to_owned());
    let requirement = RouteRequirement::of(request.method(), &route, request.uri().path());
    let subject = Subject::user(user.id, user.role);
    if !subject.satisfies(requirement) {
        return forbidden();
    }
    let watch = SessionWatch {
        backend: auth_session.backend.clone(),
        user_id: user.id,
        auth_hash: user.session_auth_hash().to_vec(),
        requirement,
    };
    let caller = Caller {
        subject,
        watch: Some(watch),
    };
    request.extensions_mut().insert(caller.clone());
    let response = next.run(request).await;
    cut_off_when_revoked(response, &caller)
}

/// `--auth` 关闭时：不做任何校验，只放一个超管身份进去，让处理函数的逻辑保持一致。
pub async fn unrestricted(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(Caller::unrestricted());
    next.run(request).await
}

/// 没有 `Content-Length` 的成功响应是流（SSE 弹幕、`/live` 媒体流）：会话失效后截断。
fn cut_off_when_revoked(response: Response, caller: &Caller) -> Response {
    if !response.status().is_success() || response.headers().contains_key(header::CONTENT_LENGTH) {
        return response;
    }
    let (parts, body) = response.into_parts();
    let stream = body.into_data_stream().take_until(caller.revoked());
    Response::from_parts(parts, Body::from_stream(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::permissions::Role;
    use crate::server::infrastructure::users::{Credentials, UserChanges};
    use axum::Router;
    use axum::http::Method;
    use axum::middleware::from_fn;
    use axum::routing::{MethodRouter, delete, get, patch, post, put};
    use axum_login::AuthManagerLayerBuilder;
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    /// 与 `router.rs` 同形的全部业务路由，处理函数一律返回 200，只看访问控制层的裁决。
    const ROUTES: &[(&str, &str)] = &[
        ("GET", "/v1/streamers"),
        ("POST", "/v1/streamers"),
        ("PUT", "/v1/streamers"),
        ("DELETE", "/v1/streamers/1"),
        ("PUT", "/v1/streamers/1/pause"),
        ("GET", "/v1/streamers/1/cover"),
        ("GET", "/v1/streamers/1/avatar"),
        ("GET", "/v1/streamers/1/live"),
        ("GET", "/v1/streamers/1/live-url"),
        ("GET", "/v1/streamers/1/danmaku"),
        ("GET", "/v1/danmaku"),
        ("GET", "/v1/ws/live-rates"),
        ("GET", "/v1/live-rates"),
        ("GET", "/v1/configuration"),
        ("PUT", "/v1/configuration"),
        ("GET", "/v1/streamer-info"),
        ("GET", "/v1/streamer-info/files/1"),
        ("PATCH", "/v1/sessions/1"),
        ("GET", "/v1/upload/streamers"),
        ("POST", "/v1/upload/streamers"),
        ("GET", "/v1/upload/streamers/1"),
        ("DELETE", "/v1/upload/streamers/1"),
        ("GET", "/v1/users"),
        ("POST", "/v1/users"),
        ("GET", "/v1/users/1"),
        ("DELETE", "/v1/users/1"),
        ("GET", "/v1/users/1/archives"),
        ("GET", "/bili/archive/pre"),
        ("GET", "/v1/get_qrcode"),
        ("POST", "/v1/login_by_qrcode"),
        ("GET", "/v1/videos"),
        ("GET", "/v1/status"),
        ("GET", "/v1/tools"),
        ("POST", "/v1/uploads"),
        ("GET", "/static/ds_update.log"),
        ("GET", "/static/a.flv"),
        ("GET", "/v1/ws/logs"),
        ("GET", "/v1/web-users"),
        ("GET", "/v1/web-users/roles"),
        ("POST", "/v1/web-users"),
        ("PUT", "/v1/web-users/1"),
        ("DELETE", "/v1/web-users/1"),
        ("POST", "/v1/web-users/1/logout-all"),
    ];

    /// 已批准的矩阵（#1717「按倾向」）：`(方法, 路径) → (操作员, 只读)` 能否访问；超管全部可访问。
    fn expected(method: &str, path: &str) -> (bool, bool) {
        let view = (true, true);
        let operate = (true, false);
        let admin = (false, false);
        match (method, path) {
            ("GET", "/v1/streamers")
            | ("GET", "/v1/streamers/1/cover")
            | ("GET", "/v1/streamers/1/avatar")
            | ("GET", "/v1/streamers/1/live")
            | ("GET", "/v1/streamers/1/live-url")
            | ("GET", "/v1/streamers/1/danmaku")
            | ("GET", "/v1/danmaku")
            | ("GET", "/v1/ws/live-rates")
            | ("GET", "/v1/live-rates")
            | ("GET", "/v1/configuration")
            | ("GET", "/v1/streamer-info")
            | ("GET", "/v1/streamer-info/files/1")
            | ("GET", "/v1/upload/streamers")
            | ("GET", "/v1/upload/streamers/1")
            | ("GET", "/v1/videos")
            | ("GET", "/v1/status")
            | ("GET", "/v1/tools")
            | ("GET", "/static/ds_update.log")
            | ("GET", "/static/a.flv")
            | ("GET", "/v1/ws/logs") => view,
            ("POST", "/v1/streamers")
            | ("PUT", "/v1/streamers")
            | ("DELETE", "/v1/streamers/1")
            | ("PUT", "/v1/streamers/1/pause")
            | ("PATCH", "/v1/sessions/1")
            | ("POST", "/v1/upload/streamers")
            | ("DELETE", "/v1/upload/streamers/1")
            | ("GET", "/v1/users")
            | ("GET", "/v1/users/1")
            | ("GET", "/bili/archive/pre")
            | ("POST", "/v1/uploads") => operate,
            _ => admin,
        }
    }

    fn ok() -> MethodRouter {
        get(|| async { StatusCode::OK })
    }

    fn stub_router() -> Router {
        let any = || {
            get(|| async { StatusCode::OK })
                .post(|| async { StatusCode::OK })
                .put(|| async { StatusCode::OK })
                .delete(|| async { StatusCode::OK })
        };
        Router::new()
            .route("/v1/streamers", any())
            .route("/v1/streamers/{id}", delete(|| async { StatusCode::OK }))
            .route("/v1/streamers/{id}/pause", put(|| async { StatusCode::OK }))
            .route("/v1/streamers/{id}/cover", ok())
            .route("/v1/streamers/{id}/avatar", ok())
            .route("/v1/streamers/{id}/live", ok())
            .route("/v1/streamers/{id}/live-url", ok())
            .route("/v1/streamers/{id}/danmaku", ok())
            .route("/v1/danmaku", ok())
            .route("/v1/ws/live-rates", ok())
            .route("/v1/live-rates", ok())
            .route("/v1/configuration", any())
            .route("/v1/streamer-info", ok())
            .route("/v1/streamer-info/files/{id}", ok())
            .route("/v1/sessions/{id}", patch(|| async { StatusCode::OK }))
            .route("/v1/upload/streamers", any())
            .route("/v1/upload/streamers/{id}", any())
            .route("/v1/users", any())
            .route("/v1/users/{id}", any())
            .route("/v1/users/{id}/archives", ok())
            .route("/bili/archive/pre", ok())
            .route("/v1/get_qrcode", ok())
            .route("/v1/login_by_qrcode", post(|| async { StatusCode::OK }))
            .route("/v1/videos", ok())
            .route("/v1/status", ok())
            .route("/v1/tools", ok())
            .route("/v1/uploads", post(|| async { StatusCode::OK }))
            .route("/static/{path}", ok())
            .route("/v1/ws/logs", ok())
            .route("/v1/web-users", any())
            .route("/v1/web-users/roles", ok())
            .route("/v1/web-users/{id}", any())
            .route(
                "/v1/web-users/{id}/logout-all",
                post(|| async { StatusCode::OK }),
            )
            .route_layer(from_fn(require_permission))
    }

    async fn app() -> (tempfile::TempDir, Backend, Router) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();
        let backend = Backend::new(pool);
        let auth_layer = AuthManagerLayerBuilder::new(
            backend.clone(),
            SessionManagerLayer::new(session_store).with_secure(false),
        )
        .build();
        let app = stub_router()
            .merge(crate::server::api::auth::router())
            .layer(auth_layer);
        (dir, backend, app)
    }

    async fn login(app: &Router, username: &str, password: &str) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "username": username, "password": password })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "login {username}");
        response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    async fn status(app: &Router, cookie: Option<&str>, method: &str, path: &str) -> StatusCode {
        let mut request = Request::builder()
            .method(Method::from_bytes(method.as_bytes()).unwrap())
            .uri(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    async fn seed(backend: &Backend) {
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
    }

    #[tokio::test]
    async fn three_roles_against_every_route() {
        let (_dir, backend, app) = app().await;
        seed(&backend).await;
        let admin = login(&app, "biliup", "admin-password").await;
        let operator = login(&app, "op", "operator-password").await;
        let viewer = login(&app, "ro", "viewer-password").await;

        for (method, path) in ROUTES {
            assert_eq!(
                status(&app, None, method, path).await,
                StatusCode::UNAUTHORIZED,
                "anonymous {method} {path}"
            );
            assert_eq!(
                status(&app, Some(&admin), method, path).await,
                StatusCode::OK,
                "admin {method} {path}"
            );
            let (operator_ok, viewer_ok) = expected(method, path);
            let code = |ok: bool| {
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                }
            };
            assert_eq!(
                status(&app, Some(&operator), method, path).await,
                code(operator_ok),
                "operator {method} {path}"
            );
            assert_eq!(
                status(&app, Some(&viewer), method, path).await,
                code(viewer_ok),
                "viewer {method} {path}"
            );
        }
    }

    #[tokio::test]
    async fn role_changes_and_disabling_apply_to_the_next_request() {
        let (_dir, backend, app) = app().await;
        seed(&backend).await;
        let viewer = login(&app, "ro", "viewer-password").await;
        let ro = backend.find_by_username("ro").await.unwrap().unwrap();

        assert_eq!(
            status(&app, Some(&viewer), "PUT", "/v1/streamers/1/pause").await,
            StatusCode::FORBIDDEN
        );
        let promote = UserChanges {
            role: Some(Role::Operator),
            ..Default::default()
        };
        backend.update_user(ro.id, promote).await.unwrap();
        assert_eq!(
            status(&app, Some(&viewer), "PUT", "/v1/streamers/1/pause").await,
            StatusCode::OK
        );
        let disable = UserChanges {
            disabled: Some(true),
            ..Default::default()
        };
        backend.update_user(ro.id, disable).await.unwrap();
        assert_eq!(
            status(&app, Some(&viewer), "GET", "/v1/status").await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn forced_logout_invalidates_existing_sessions() {
        let (_dir, backend, app) = app().await;
        seed(&backend).await;
        let operator = login(&app, "op", "operator-password").await;
        let op = backend.find_by_username("op").await.unwrap().unwrap();
        assert_eq!(
            status(&app, Some(&operator), "GET", "/v1/status").await,
            StatusCode::OK
        );
        backend.logout_everywhere(op.id).await.unwrap();
        assert_eq!(
            status(&app, Some(&operator), "GET", "/v1/status").await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn long_lived_sessions_are_rechecked() {
        let (_dir, backend, _app) = app().await;
        seed(&backend).await;
        let op = backend.find_by_username("op").await.unwrap().unwrap();
        let watch = SessionWatch {
            backend: backend.clone(),
            user_id: op.id,
            auth_hash: op.session_auth_hash().to_vec(),
            requirement: RouteRequirement::Permission(Permission::PreviewView),
        };
        assert!(watch.still_valid().await.unwrap());

        let demote = UserChanges {
            role: Some(Role::Viewer),
            ..Default::default()
        };
        backend.update_user(op.id, demote).await.unwrap();
        assert!(watch.still_valid().await.unwrap(), "只读也能看预览");

        let log_watch = SessionWatch {
            requirement: RouteRequirement::Permission(Permission::RecordingControl),
            ..watch.clone()
        };
        assert!(!log_watch.still_valid().await.unwrap(), "失去该路由的权限");

        backend.logout_everywhere(op.id).await.unwrap();
        assert!(!watch.still_valid().await.unwrap(), "强制下线");
    }

    #[tokio::test]
    async fn streaming_bodies_end_when_the_session_is_revoked() {
        let (_dir, backend, _app) = app().await;
        seed(&backend).await;
        let op = backend.find_by_username("op").await.unwrap().unwrap();
        let caller = Caller {
            subject: Subject::user(op.id, op.role),
            watch: Some(SessionWatch {
                backend: backend.clone(),
                user_id: op.id,
                auth_hash: op.session_auth_hash().to_vec(),
                requirement: RouteRequirement::Permission(Permission::PreviewView),
            }),
        };
        let endless = futures::stream::unfold((), |()| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Some((Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"x")), ()))
        });
        let response = Response::new(Body::from_stream(endless));
        let response = cut_off_when_revoked(response, &caller);
        let mut stream = response.into_body().into_data_stream();

        assert!(stream.next().await.is_some());
        backend.logout_everywhere(op.id).await.unwrap();
        let mut chunks = 0;
        while stream.next().await.is_some() {
            chunks += 1;
            assert!(chunks < 100, "会话失效后流应在一两个复查周期内结束");
        }
    }
}
