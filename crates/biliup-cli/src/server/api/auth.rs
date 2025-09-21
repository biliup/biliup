use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::Deserialize;

use crate::server::infrastructure::users::{AuthSession, Credentials};

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub fn router() -> Router<()> {
    Router::new()
        .route("/v1/users/login", post(post::login))
        .route("/v1/users/register", post(post::signup))
        .route("/v1/users/biliup", get(get::get_user))
        // .route("/login", get(self::get::login))
        .route("/v1/logout", get(get::logout))
}

mod post {
    use super::*;
    use axum::Json;
    use tracing::log::info;

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
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
        info!("Login with credentials {:?}", creds);
        let user = match auth_session.authenticate(creds.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                info!("Invalid credentials");

                let mut login_url = "/login".to_string();
                if let Some(next) = creds.next {
                    login_url = format!("{login_url}?next={next}");
                };

                return Redirect::to(&login_url).into_response();
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
