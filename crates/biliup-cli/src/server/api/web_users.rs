//! Web 用户管理与「我」的接口。前缀避开 `/v1/users`（那是 B 站投稿账号）。

use crate::server::api::access::Caller;
use crate::server::fleet::FleetCapability;
use crate::server::infrastructure::permissions::Role;
use crate::server::infrastructure::policy::Subject;
use crate::server::infrastructure::users::{
    AuthSession, CreateUserError, UpdateUserError, UserChanges, UserSummary,
};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use serde_json::json;

/// 用户管理（`user.manage`，挂在访问控制层之内）。
pub fn admin_router() -> Router<()> {
    Router::new()
        .route("/v1/web-users", get(list_users).post(create_user))
        .route("/v1/web-users/roles", get(list_roles))
        .route("/v1/web-users/{id}", put(update_user).delete(delete_user))
        .route("/v1/web-users/{id}/logout-all", post(logout_all))
}

/// `--auth` 开启时：只要求登录，不看权限点。
pub fn me_router() -> Router<()> {
    Router::new()
        .route("/v1/me", get(me))
        .route("/v1/me/password", put(change_password))
}

/// `--auth` 关闭时：`/v1/me` 仍然可用，告诉前端当前是零鉴权、视为超管。
pub fn unrestricted_me_router() -> Router<()> {
    Router::new().route("/v1/me", get(unrestricted_me))
}

fn message(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

fn internal(error: impl std::fmt::Debug) -> Response {
    tracing::error!(error = ?error, "Web user management failed");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// 权限点由授权决策点算出（含环境属性），前端只按它显隐，不再自己推导。
/// `fleet_controller`：本进程以 `--controller` 运行，前端据此显示「节点」菜单。
/// `fleet_node`：只有被控制面托管的节点才有，前端据此把托管的主播与模板显示为只读。
fn me_body(username: Option<&str>, subject: Subject, fleet: Option<FleetCapability>) -> Response {
    let fleet = fleet.unwrap_or_default();
    let mut body = json!({
        "id": subject.user_id,
        "username": username,
        "role": subject.role,
        "permissions": subject.permissions(),
        "auth_enabled": subject.auth_enabled,
        "fleet_controller": fleet.controller,
    });
    if let Some(node) = fleet.managed_view() {
        body["fleet_node"] = node;
    }
    Json(body).into_response()
}

async fn me(auth_session: AuthSession, fleet: Option<Extension<FleetCapability>>) -> Response {
    match auth_session.user {
        Some(user) => me_body(
            Some(&user.username),
            Subject::user(user.id, user.role),
            fleet.map(|Extension(fleet)| fleet),
        ),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn unrestricted_me(fleet: Option<Extension<FleetCapability>>) -> Response {
    me_body(
        None,
        Subject::unrestricted(),
        fleet.map(|Extension(fleet)| fleet),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangePassword {
    old_password: String,
    new_password: String,
}

async fn change_password(
    mut auth_session: AuthSession,
    Json(body): Json<ChangePassword>,
) -> Response {
    let Some(user) = auth_session.user.clone() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let updated = match auth_session
        .backend
        .change_own_password(user.id, body.old_password, body.new_password)
        .await
    {
        Ok(Some(updated)) => updated,
        Ok(None) => return message(StatusCode::BAD_REQUEST, "当前密码不正确"),
        Err(UpdateUserError::Invalid(reason)) => return message(StatusCode::BAD_REQUEST, reason),
        Err(error) => return internal(error),
    };
    // 会话版本已递增，其它设备全部下线；当前这个浏览器用新的校验值重新登录，不必再输一遍密码。
    if auth_session.login(&updated).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

/// 各角色实际拥有的权限点，用户管理页据此说明角色，前端不再手写角色与权限的对应。
async fn list_roles() -> Response {
    let roles: Vec<_> = Role::ALL
        .into_iter()
        .map(|role| {
            // 这个接口只在 `--auth` 开启时存在，按开启时的环境算
            let subject = Subject {
                user_id: None,
                role,
                auth_enabled: true,
            };
            json!({ "role": role, "permissions": subject.permissions() })
        })
        .collect();
    Json(roles).into_response()
}

async fn list_users(auth_session: AuthSession) -> Response {
    match auth_session.backend.list_users().await {
        Ok(users) => Json(users.iter().map(UserSummary::from).collect::<Vec<_>>()).into_response(),
        Err(error) => internal(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewUser {
    username: String,
    password: String,
    role: Role,
}

async fn create_user(auth_session: AuthSession, Json(body): Json<NewUser>) -> Response {
    match auth_session
        .backend
        .create_user(&body.username, body.password, body.role)
        .await
    {
        Ok(user) => (StatusCode::CREATED, Json(UserSummary::from(&user))).into_response(),
        Err(CreateUserError::Invalid(reason)) => message(StatusCode::BAD_REQUEST, reason),
        Err(CreateUserError::AlreadyExists) => message(StatusCode::CONFLICT, "用户名已存在"),
        Err(error) => internal(error),
    }
}

fn update_error(error: UpdateUserError) -> Response {
    match error {
        UpdateUserError::Invalid(reason) => message(StatusCode::BAD_REQUEST, reason),
        UpdateUserError::NotFound => message(StatusCode::NOT_FOUND, "用户不存在"),
        UpdateUserError::LastAdmin => {
            message(StatusCode::CONFLICT, "至少要保留一个启用中的超级管理员")
        }
        error => internal(error),
    }
}

async fn update_user(
    caller: Caller,
    auth_session: AuthSession,
    Path(id): Path<i64>,
    Json(changes): Json<UserChanges>,
) -> Response {
    if caller.subject.user_id == Some(id)
        && (changes.role.is_some() || changes.disabled == Some(true))
    {
        return message(StatusCode::BAD_REQUEST, "不能修改自己的角色或禁用自己");
    }
    match auth_session.backend.update_user(id, changes).await {
        Ok(user) => Json(UserSummary::from(&user)).into_response(),
        Err(error) => update_error(error),
    }
}

async fn delete_user(caller: Caller, auth_session: AuthSession, Path(id): Path<i64>) -> Response {
    if caller.subject.user_id == Some(id) {
        return message(StatusCode::BAD_REQUEST, "不能删除自己");
    }
    match auth_session.backend.delete_user(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => update_error(error),
    }
}

async fn logout_all(auth_session: AuthSession, Path(id): Path<i64>) -> Response {
    match auth_session.backend.logout_everywhere(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => update_error(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::server::api::{access, auth};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::permissions::Role;
    use crate::server::infrastructure::users::Backend;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::middleware::from_fn;
    use axum_login::AuthManagerLayerBuilder;
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    async fn app() -> (tempfile::TempDir, Router) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();
        let auth_layer = AuthManagerLayerBuilder::new(
            Backend::new(pool),
            SessionManagerLayer::new(session_store).with_secure(false),
        )
        .build();
        let app = super::admin_router()
            .route_layer(from_fn(access::require_permission))
            .merge(super::me_router())
            .merge(auth::router())
            .layer(auth_layer);
        (dir, app)
    }

    async fn call(
        app: &Router,
        cookie: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Option<String>, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let cookie = response.headers().get(header::SET_COOKIE).map(|value| {
            value
                .to_str()
                .unwrap()
                .split(';')
                .next()
                .unwrap()
                .to_string()
        });
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, cookie, json)
    }

    async fn login(app: &Router, username: &str, password: &str) -> String {
        let (status, cookie, _) = call(
            app,
            None,
            "POST",
            "/v1/users/login",
            Some(json!({ "username": username, "password": password })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{username}");
        cookie.unwrap()
    }

    #[tokio::test]
    async fn admin_manages_users_end_to_end() {
        let (_dir, app) = app().await;
        let (status, admin, _) = call(
            &app,
            None,
            "POST",
            "/v1/users/register",
            Some(json!({ "username": "boss", "password": "admin-password" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let admin = admin.unwrap();

        let (status, _, me) = call(&app, Some(&admin), "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(me["username"], "boss");
        assert_eq!(me["role"], "admin");
        assert_eq!(me["auth_enabled"], true);
        assert!(
            me["permissions"]
                .as_array()
                .unwrap()
                .contains(&json!("user.manage"))
        );

        let (status, _, created) = call(
            &app,
            Some(&admin),
            "POST",
            "/v1/web-users",
            Some(
                json!({ "username": "helper", "password": "helper-password", "role": "operator" }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let helper_id = created["id"].as_i64().unwrap();
        assert!(created.get("password_hash").is_none());

        let (status, _, _) = call(
            &app,
            Some(&admin),
            "POST",
            "/v1/web-users",
            Some(json!({ "username": "HELPER", "password": "helper-password", "role": "viewer" })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);

        let helper = login(&app, "helper", "helper-password").await;
        let (_, _, me) = call(&app, Some(&helper), "GET", "/v1/me", None).await;
        assert_eq!(me["role"], "operator");
        assert!(
            !me["permissions"]
                .as_array()
                .unwrap()
                .contains(&json!("streamer.hooks"))
        );
        let (status, _, _) = call(&app, Some(&helper), "GET", "/v1/web-users", None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        // 重置密码让对方所有会话失效
        let (status, _, _) = call(
            &app,
            Some(&admin),
            "PUT",
            &format!("/v1/web-users/{helper_id}"),
            Some(json!({ "password": "reset-password" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _, _) = call(&app, Some(&helper), "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let helper = login(&app, "helper", "reset-password").await;

        // 强制下线
        let (status, _, _) = call(
            &app,
            Some(&admin),
            "POST",
            &format!("/v1/web-users/{helper_id}/logout-all"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, _, _) = call(&app, Some(&helper), "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _, list) = call(&app, Some(&admin), "GET", "/v1/web-users", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list.as_array().unwrap().len(), 2);

        let (status, _, _) = call(
            &app,
            Some(&admin),
            "DELETE",
            &format!("/v1/web-users/{helper_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn role_catalog_comes_from_the_policy() {
        let (_dir, app) = app().await;
        let (_, admin, _) = call(
            &app,
            None,
            "POST",
            "/v1/users/register",
            Some(json!({ "username": "biliup", "password": "admin-password" })),
        )
        .await;
        let admin = admin.unwrap();
        let (status, _, roles) = call(&app, Some(&admin), "GET", "/v1/web-users/roles", None).await;
        assert_eq!(status, StatusCode::OK);
        let roles = roles.as_array().unwrap();
        assert_eq!(roles.len(), Role::ALL.len());
        for (entry, role) in roles.iter().zip(Role::ALL) {
            assert_eq!(entry["role"], json!(role));
            assert_eq!(entry["permissions"], json!(role.permissions()));
        }

        let (status, _, _) = call(
            &app,
            Some(&admin),
            "POST",
            "/v1/web-users",
            Some(json!({ "username": "ro", "password": "viewer-password", "role": "viewer" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let viewer = login(&app, "ro", "viewer-password").await;
        let (status, _, _) = call(&app, Some(&viewer), "GET", "/v1/web-users/roles", None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn without_auth_there_are_no_users_to_manage() {
        let app = super::unrestricted_me_router();
        let (status, _, me) = call(&app, None, "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(me["auth_enabled"], false);
        assert_eq!(me["role"], "admin");
        let permissions = me["permissions"].as_array().unwrap();
        assert!(!permissions.contains(&json!("user.manage")));
        assert!(permissions.contains(&json!("streamer.hooks")));
        assert!(permissions.contains(&json!("config.edit")));
    }

    #[tokio::test]
    async fn admins_cannot_lock_themselves_out() {
        let (_dir, app) = app().await;
        let (_, admin, _) = call(
            &app,
            None,
            "POST",
            "/v1/users/register",
            Some(json!({ "username": "biliup", "password": "admin-password" })),
        )
        .await;
        let admin = admin.unwrap();
        let (_, _, me) = call(&app, Some(&admin), "GET", "/v1/me", None).await;
        let id = me["id"].as_i64().unwrap();
        for body in [json!({ "role": "viewer" }), json!({ "disabled": true })] {
            let (status, _, _) = call(
                &app,
                Some(&admin),
                "PUT",
                &format!("/v1/web-users/{id}"),
                Some(body),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }
        let (status, _, _) = call(
            &app,
            Some(&admin),
            "DELETE",
            &format!("/v1/web-users/{id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn changing_your_own_password_keeps_this_session_only() {
        let (_dir, app) = app().await;
        let (_, first, _) = call(
            &app,
            None,
            "POST",
            "/v1/users/register",
            Some(json!({ "username": "biliup", "password": "admin-password" })),
        )
        .await;
        let first = first.unwrap();
        let second = login(&app, "biliup", "admin-password").await;

        let (status, _, _) = call(
            &app,
            Some(&first),
            "PUT",
            "/v1/me/password",
            Some(json!({ "old_password": "wrong", "new_password": "new-password" })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, renewed, _) = call(
            &app,
            Some(&first),
            "PUT",
            "/v1/me/password",
            Some(json!({ "old_password": "admin-password", "new_password": "new-password" })),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let current = renewed.unwrap_or(first);
        let (status, _, _) = call(&app, Some(&current), "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::OK, "当前会话保持登录");
        let (status, _, _) = call(&app, Some(&second), "GET", "/v1/me", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "其它会话下线");
        login(&app, "biliup", "new-password").await;
    }
}
