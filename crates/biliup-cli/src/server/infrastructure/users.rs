use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use axum_login::{AuthUser, AuthnBackend, UserId};
use error_stack::{ResultExt, bail};
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::task;

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
    pub async fn create_user(&mut self, creds: Credentials) -> AppResult<User> {
        // 创建新用户账户
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
        let user: Option<Self::User> = sqlx::query_as("select * from configuration where key = ? ")
            .bind("biliup")
            .fetch_optional(&self.db)
            .await?;

        // 密码验证是阻塞且可能较慢的操作，所以通过spawn_blocking执行
        task::spawn_blocking(|| {
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
        let user = sqlx::query_as("select * from configuration where id = ?")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }
}

// 为了方便使用的类型别名
// 注意这里我们提供了具体的后端实现
pub type AuthSession = axum_login::AuthSession<Backend>;
