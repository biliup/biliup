use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use axum_login::{AuthUser, AuthnBackend, UserId};
use error_stack::ResultExt;
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::{Arc, OnceLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task;

const MAX_CONCURRENT_PASSWORD_TASKS: usize = 4;
static PASSWORD_TASKS: OnceLock<Arc<Semaphore>> = OnceLock::new();

async fn acquire_password_task_permit() -> OwnedSemaphorePermit {
    PASSWORD_TASKS
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_PASSWORD_TASKS)))
        .clone()
        .acquire_owned()
        .await
        .expect("the password-task semaphore is never closed")
}

/// 用户数据结构
/// 存储用户的基本信息，包括ID、用户名和密码哈希
#[derive(Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    /// 用户ID
    id: i64,
    /// 用户名
    pub key: String,
    /// 密码哈希值
    value: String,
}

// 手动实现Debug trait以避免意外记录密码哈希
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
        // 使用密码哈希作为认证哈希
        // 这意味着当用户更改密码时，认证会话将失效
        self.value.as_bytes()
    }
}

// 认证凭据结构，用于从表单中提取认证字段
// 用于与后端进行请求认证
#[derive(Clone, Deserialize)]
pub struct Credentials {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 登录后跳转的URL（可选）
    pub next: Option<String>,
}

// 手动实现Debug trait以避免意外记录密码哈希
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("next", &self.next)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

/// 认证后端
/// 负责处理用户认证相关的数据库操作
#[derive(Debug, Clone)]
pub struct Backend {
    /// 数据库连接池
    db: ConnectionPool,
}

impl Backend {
    /// 创建新的认证后端实例
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }

    /// 检查是否存在用户
    ///
    /// # 返回
    /// 如果存在用户返回true，否则返回false
    pub async fn exists(&self) -> AppResult<bool> {
        // 检查是否已存在对应的用户
        let user: Option<User> = sqlx::query_as("select * from configuration where key = ? ")
            .bind("biliup")
            .fetch_optional(&self.db)
            .await
            .change_context(AppError::Unknown)?;
        Ok(user.is_some())
    }

    /// 创建新用户
    ///
    /// # 参数
    /// * `creds` - 用户凭据
    ///
    /// # 返回
    /// 返回创建的用户信息
    pub async fn create_user(&self, creds: Credentials) -> Result<User, CreateUserError> {
        // 创建新用户账户
        // 验证输入
        const MAX_PASSWORD_BYTES: usize = 1024;
        if creds.username != "biliup"
            || creds.password.is_empty()
            || creds.password.len() > MAX_PASSWORD_BYTES
        {
            return Err(CreateUserError::InvalidCredentials);
        }

        // Avoid an unbounded Argon2 CPU sink on the public bootstrap route once
        // the one permitted administrator already exists. The unique index is
        // still the authoritative guard for concurrent first-time requests.
        let initialized: i64 =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM configuration WHERE key = 'biliup')")
                .fetch_one(&self.db)
                .await
                .map_err(CreateUserError::Database)?;
        if initialized != 0 {
            return Err(CreateUserError::AlreadyExists);
        }

        // Argon2 is deliberately expensive and must not block an async worker.
        let permit = acquire_password_task_permit().await;
        // Requests may have queued behind the bounded password workers while
        // another bootstrap completed. Re-check before spending CPU, and keep
        // the permit until the unique insert finishes so queued requests see
        // the newly-created identity instead of hashing needlessly.
        let initialized: i64 =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM configuration WHERE key = 'biliup')")
                .fetch_one(&self.db)
                .await
                .map_err(CreateUserError::Database)?;
        if initialized != 0 {
            return Err(CreateUserError::AlreadyExists);
        }
        let password = creds.password;
        let (password_hash, _permit) =
            task::spawn_blocking(move || (generate_hash(password), permit))
                .await
                .map_err(CreateUserError::HashingTask)?;
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
                CreateUserError::AlreadyExists
            }
            error => CreateUserError::Database(error),
        })?;

        Ok(user)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateUserError {
    #[error("username must be 'biliup' and password must contain 1 to 1024 bytes")]
    InvalidCredentials,
    #[error("the Web administrator has already been initialized")]
    AlreadyExists,
    #[error("database error")]
    Database(#[source] sqlx::Error),
    #[error("password hashing task failed")]
    HashingTask(#[source] task::JoinError),
}

/// 认证相关的错误类型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 数据库错误
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// 任务连接错误
    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    /// 认证用户
    ///
    /// # 参数
    /// * `creds` - 用户凭据
    ///
    /// # 返回
    /// 如果认证成功返回用户信息，否则返回None
    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        if creds.username != "biliup" || creds.password.is_empty() || creds.password.len() > 1024 {
            return Ok(None);
        }
        let user: Option<Self::User> = sqlx::query_as("select * from configuration where key = ? ")
            .bind("biliup")
            .fetch_optional(&self.db)
            .await?;

        // 密码验证是阻塞且可能较慢的操作，所以通过spawn_blocking执行
        let permit = acquire_password_task_permit().await;
        task::spawn_blocking(move || {
            let _permit = permit;
            // 使用基于密码的认证 - 通过比较表单输入与argon2密码哈希来工作
            Ok(user.filter(|user| verify_password(creds.password, &user.value).is_ok()))
        })
        .await?
    }

    /// 根据用户ID获取用户信息
    ///
    /// # 参数
    /// * `user_id` - 用户ID
    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as("select * from configuration where id = ? and key = ?")
            .bind(user_id)
            .bind("biliup")
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }
}

// 为了方便使用的类型别名
// 注意这里我们提供了具体的后端实现
pub type AuthSession = axum_login::AuthSession<Backend>;

#[cfg(test)]
mod tests {
    use super::{AuthnBackend, Backend, CreateUserError, Credentials};
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    fn credentials(password: &str) -> Credentials {
        Credentials {
            username: "biliup".into(),
            password: password.into(),
            next: None,
        }
    }

    #[tokio::test]
    async fn administrator_bootstrap_is_one_time_and_passwords_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let backend = Backend::new(pool);

        assert!(matches!(
            backend.create_user(credentials("")).await,
            Err(CreateUserError::InvalidCredentials)
        ));
        assert!(matches!(
            backend.create_user(credentials(&"x".repeat(1025))).await,
            Err(CreateUserError::InvalidCredentials)
        ));
        backend
            .create_user(credentials("correct horse"))
            .await
            .unwrap();
        assert!(matches!(
            backend.create_user(credentials("second password")).await,
            Err(CreateUserError::AlreadyExists)
        ));
        assert!(
            backend
                .authenticate(credentials("wrong password"))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            backend
                .authenticate(credentials("correct horse"))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            backend
                .authenticate(credentials(&"x".repeat(1025)))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn concurrent_bootstrap_creates_exactly_one_administrator() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let backend = Backend::new(pool.clone());

        let mut tasks = Vec::new();
        for index in 0..4 {
            let backend = backend.clone();
            tasks.push(tokio::spawn(async move {
                backend
                    .create_user(credentials(&format!("password-{index}")))
                    .await
            }));
        }

        let mut created = 0;
        let mut rejected = 0;
        for task in tasks {
            match task.await.unwrap() {
                Ok(_) => created += 1,
                Err(CreateUserError::AlreadyExists) => rejected += 1,
                Err(error) => panic!("unexpected bootstrap error: {error}"),
            }
        }
        assert_eq!(created, 1);
        assert_eq!(rejected, 3);

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'biliup'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }
}
