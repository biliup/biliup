use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::Configuration;
use axum_login::{AuthUser, AuthnBackend, UserId};
use error_stack::FutureExt;
use error_stack::{Report, ResultExt, bail};
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tokio::task;

#[derive(Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    id: i64,
    pub key: String,
    value: String,
}

// Here we've implemented `Debug` manually to avoid accidentally logging the
// password hash.
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.key)
            .field("password", &"[redacted]")
            .finish()
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.value.as_bytes() // We use the password hash as the auth
        // hash--what this means
        // is when the user changes their password the
        // auth session becomes invalid.
    }
}

// This allows us to extract the authentication fields from forms. We use this
// to authenticate requests with the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: ConnectionPool,
}

impl Backend {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }

    pub async fn exists(&self) -> AppResult<bool> {
        // check if a user corresponding to the given creds already exists...
        let user: Option<User> = sqlx::query_as("select * from configuration where key = ? ")
            // .bind(creds.username)
            .bind("biliup")
            .fetch_optional(&self.db)
            .await
            .change_context(AppError::Unknown)?;
        Ok(user.is_some())
    }

    pub(crate) async fn create_user(&mut self, creds: Credentials) -> AppResult<User> {
        // create the new user account...
        // 验证输入
        if creds.username.is_empty() || creds.password.is_empty() {
            bail!(AppError::Custom("用户名和密码不能为空".into()))
        }

        // 生成密码哈希
        let password_hash = generate_hash(&creds.password);
        // 插入用户并返回
        let user = sqlx::query_as(
            r#"
        INSERT INTO configuration (key, value)
        VALUES ($1, $2)
        RETURNING *
        "#,
        )
        .bind("biliup")
        .bind(&password_hash)
        .fetch_one(&self.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Custom("用户名已存在".into())
            }
            _ => AppError::Custom(e.to_string()),
        })?;

        Ok(user)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<Self::User> = sqlx::query_as("select * from configuration where key = ? ")
            // .bind(creds.username)
            .bind("biliup")
            .fetch_optional(&self.db)
            .await?;

        // Verifying the password is blocking and potentially slow, so we'll do so via
        // `spawn_blocking`.
        task::spawn_blocking(|| {
            // We're using password-based authentication--this works by comparing our form
            // input with an argon2 password hash.
            Ok(user.filter(|user| verify_password(creds.password, &user.value).is_ok()))
        })
        .await?
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as("select * from configuration where id = ?")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;
