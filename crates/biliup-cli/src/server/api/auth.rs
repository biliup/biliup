use crate::server::infrastructure::users::{
    AuthSession, CreateUserError, Credentials, MAX_PASSWORD_BYTES,
};
use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use axum_login::AuthUser;
use serde::Deserialize;

pub fn router() -> Router<()> {
    Router::new()
        .route("/v1/users/login", post(post::login))
        .route("/v1/users/register", post(post::signup))
        .route("/v1/users/password", post(post::change_password))
        .route("/v1/users/biliup", get(get::get_user))
        // .route("/login", get(self::get::login))
        .route("/v1/logout", get(get::logout))
}

mod post {
    use super::*;
    use axum::Json;
    use tracing::log::info;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ChangePassword {
        pub current_password: String,
        pub new_password: String,
        pub confirm_password: String,
    }

    /// Handler for the "POST /signup" endpoint.
    pub async fn signup(
        mut auth_session: AuthSession,
        Json(creds): Json<Credentials>,
    ) -> impl IntoResponse {
        // TODO: we rely on `auth_session.user` and `auth_session.backend`, not sure
        // if this is a good sample impl of signing up?

        // Disallow signing up when currently logged in.
        if auth_session.user.is_some() {
            return StatusCode::BAD_REQUEST.into_response();
        }

        let user = match auth_session.backend.create_user(creds).await {
            Ok(user) => user,
            Err(CreateUserError::InvalidCredentials) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "message": "用户名必须为 biliup，且密码必须为 1 至 1024 字节" })),
                )
                    .into_response();
            }
            Err(CreateUserError::AlreadyExists) => {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "message": "管理员已初始化，不能重复注册" })),
                )
                    .into_response();
            }
            Err(CreateUserError::Database(error)) => {
                tracing::error!(error = ?error, "failed to initialize Web administrator");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            Err(CreateUserError::HashingTask(error)) => {
                tracing::error!(error = ?error, "Web administrator password hashing task failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        // Log the newly-created user in.
        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        StatusCode::OK.into_response()
    }

    pub async fn login(
        mut auth_session: AuthSession,
        Json(creds): Json<Credentials>,
    ) -> impl IntoResponse {
        info!("Web login attempt");
        let user = match auth_session.authenticate(creds.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                info!("Invalid credentials");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "message": "用户名或密码错误" })),
                )
                    .into_response();
            }
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        info!("Successfully logged in as {}", user.key);
        StatusCode::OK.into_response()
        // if let Some(ref next) = creds.next {
        //     Redirect::to(next)
        // } else {
        //     Redirect::to("/")
        // }
        //     .into_response()
    }

    /// Change the password for the currently authenticated administrator.
    /// A successful change logs the session out because the session auth hash
    /// intentionally contains the old password hash.
    pub async fn change_password(
        mut auth_session: AuthSession,
        Json(request): Json<ChangePassword>,
    ) -> impl IntoResponse {
        let Some(user) = auth_session.user.clone() else {
            return StatusCode::UNAUTHORIZED.into_response();
        };
        if request.current_password.is_empty()
            || request.current_password.len() > MAX_PASSWORD_BYTES
            || request.new_password.is_empty()
            || request.new_password.len() > MAX_PASSWORD_BYTES
            || request.confirm_password != request.new_password
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "message": "新密码必须为 1 至 1024 字节，且两次输入必须一致"
                })),
            )
                .into_response();
        }

        match auth_session
            .backend
            .change_password(user.id(), request.current_password, request.new_password)
            .await
        {
            Ok(true) => {
                if auth_session.logout().await.is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
                StatusCode::OK.into_response()
            }
            Ok(false) => (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "message": "当前密码错误" })),
            )
                .into_response(),
            Err(error) => {
                tracing::error!(error = ?error, "failed to change Web administrator password");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::router;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::users::Backend;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum_login::AuthManagerLayerBuilder;
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    #[tokio::test]
    async fn invalid_login_is_an_explicit_error_not_a_followed_redirect() {
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
        let app = router().layer(auth_layer);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"username":"biliup","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::LOCATION).is_none());
    }

    #[tokio::test]
    async fn password_change_requires_an_authenticated_session() {
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
        let app = router().layer(auth_layer);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/password")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"current_password":"old","new_password":"new","confirm_password":"new"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn authenticated_password_change_logs_out_the_old_session() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();

        let make_app = || {
            let auth_layer = AuthManagerLayerBuilder::new(
                Backend::new(pool.clone()),
                SessionManagerLayer::new(session_store.clone()).with_secure(false),
            )
            .build();
            router().layer(auth_layer)
        };

        let signup = make_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"biliup","password":"old password"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(signup.status(), StatusCode::OK);
        let cookie = signup
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();

        let changed = make_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/password")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(
                        r#"{"current_password":"old password","new_password":"new password","confirm_password":"new password"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(changed.status(), StatusCode::OK);
        assert!(changed.headers().get(header::SET_COOKIE).is_some());
    }
}

mod get {
    use super::*;

    use tracing::error;

    // pub async fn login(
    //     Query(NextUrl { next }): Query<NextUrl>,
    // ) -> impl IntoResponse {
    //     let mut login_url = "/login.html".to_string();
    //     if let Some(next) = next {
    //         login_url = format!("{login_url}?next={next}");
    //     };
    //     Redirect::permanent(&login_url).into_response()
    // }

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/login").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    pub async fn get_user(auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.backend.exists().await {
            Ok(true) => StatusCode::OK.into_response(),
            Ok(false) => StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                error!(error = ?e, "Error checking existing user");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
