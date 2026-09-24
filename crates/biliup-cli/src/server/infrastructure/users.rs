use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::permissions::Role;
use axum_login::{AuthUser, AuthnBackend, UserId};
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task;

const MAX_CONCURRENT_PASSWORD_TASKS: usize = 4;
static PASSWORD_TASKS: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub const MIN_PASSWORD_BYTES: usize = 8;
pub const MAX_PASSWORD_BYTES: usize = 1024;
pub const MAX_USERNAME_CHARS: usize = 32;

/// 同一用户名连续失败这么多次后锁定 [`LOGIN_LOCK`]。
const MAX_LOGIN_FAILURES: u32 = 5;
const LOGIN_LOCK: Duration = Duration::from_secs(30);
/// 限流表只在内存里；按用户名计数意味着攻击者可以用随机用户名撑大它，超过这个量就清掉未锁定的项。
const MAX_TRACKED_USERNAMES: usize = 4096;

async fn acquire_password_task_permit() -> OwnedSemaphorePermit {
    PASSWORD_TASKS
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_PASSWORD_TASKS)))
        .clone()
        .acquire_owned()
        .await
        .expect("the password-task semaphore is never closed")
}

/// Argon2 很慢，放到阻塞线程池里算，并受全局 4 路信号量约束。
pub async fn hash_password(password: String) -> Result<String, task::JoinError> {
    let permit = acquire_password_task_permit().await;
    task::spawn_blocking(move || {
        let _permit = permit;
        generate_hash(password)
    })
    .await
}

async fn password_matches(password: String, hash: String) -> Result<bool, task::JoinError> {
    let permit = acquire_password_task_permit().await;
    task::spawn_blocking(move || {
        let _permit = permit;
        verify_password(password, &hash).is_ok()
    })
    .await
}

/// 用户不存在时也跑一次同等代价的校验，免得靠响应时间枚举用户名。
fn dummy_hash() -> String {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY
        .get_or_init(|| generate_hash("biliup-timing-equalizer"))
        .clone()
}

pub fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.is_empty()
        || username.trim() != username
        || username.chars().count() > MAX_USERNAME_CHARS
        || username.chars().any(char::is_control)
    {
        return Err("用户名须为 1 至 32 个字符，且首尾不能有空白");
    }
    Ok(())
}

pub fn validate_new_password(password: &str) -> Result<(), &'static str> {
    if password.len() < MIN_PASSWORD_BYTES || password.len() > MAX_PASSWORD_BYTES {
        return Err("密码须为 8 至 1024 字节");
    }
    Ok(())
}

#[derive(FromRow)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    role: String,
    disabled: bool,
    session_version: i64,
    created_at: i64,
    last_login_at: Option<i64>,
}

const USER_COLUMNS: &str =
    "id, username, password_hash, role, disabled, session_version, created_at, last_login_at";

/// Web 用户（`web_users` 表）。
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    password_hash: String,
    pub role: Role,
    pub disabled: bool,
    session_version: i64,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    #[serde(skip)]
    auth_hash: Vec<u8>,
}

impl TryFrom<UserRow> for User {
    type Error = sqlx::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let role = Role::from_str(&row.role)
            .map_err(|_| sqlx::Error::Decode(format!("unknown role {:?}", row.role).into()))?;
        let auth_hash = session_auth_hash(&row.password_hash, row.session_version);
        Ok(User {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
            role,
            disabled: row.disabled,
            session_version: row.session_version,
            created_at: row.created_at,
            last_login_at: row.last_login_at,
            auth_hash,
        })
    }
}

/// 会话校验值：密码哈希 ‖ 会话版本。改密、重置、强制下线都会让它变化，从而让旧会话失效。
///
/// 版本为 0 时只用密码哈希 —— 与迁移 6 之前的单管理员一致，升级后已登录的浏览器不掉线。
fn session_auth_hash(password_hash: &str, session_version: i64) -> Vec<u8> {
    if session_version == 0 {
        password_hash.as_bytes().to_vec()
    } else {
        format!("{password_hash}:{session_version}").into_bytes()
    }
}

// 手动实现 Debug，避免把密码哈希打进日志
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("role", &self.role)
            .field("disabled", &self.disabled)
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
        &self.auth_hash
    }
}

/// 接口里返回给前端的用户信息（不含哈希）。
#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
}

impl From<&User> for UserSummary {
    fn from(user: &User) -> Self {
        UserSummary {
            id: user.id,
            username: user.username.clone(),
            role: user.role,
            disabled: user.disabled,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        }
    }
}

// 认证凭据结构，用于从表单中提取认证字段
#[derive(Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// 登录后跳转的URL（可选）
    pub next: Option<String>,
}

// 手动实现Debug trait以避免意外记录密码
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("next", &self.next)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

#[derive(Default)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
}

/// 按用户名的登录限流：连续失败 5 次锁 30 秒。锁定期间一律按密码错误处理，不暴露锁定状态。
#[derive(Default)]
struct LoginLimiter {
    attempts: Mutex<HashMap<String, Attempts>>,
}

impl LoginLimiter {
    fn key(username: &str) -> String {
        username.to_lowercase()
    }

    fn is_locked(&self, username: &str, now: Instant) -> bool {
        let mut attempts = self.attempts.lock().unwrap();
        let Some(entry) = attempts.get_mut(&Self::key(username)) else {
            return false;
        };
        match entry.locked_until {
            Some(until) if until > now => true,
            Some(_) => {
                *entry = Attempts::default();
                false
            }
            None => false,
        }
    }

    fn record_failure(&self, username: &str, now: Instant) {
        let mut attempts = self.attempts.lock().unwrap();
        if attempts.len() >= MAX_TRACKED_USERNAMES {
            attempts.retain(|_, entry| entry.locked_until.is_some_and(|until| until > now));
        }
        let entry = attempts.entry(Self::key(username)).or_default();
        entry.failures += 1;
        if entry.failures >= MAX_LOGIN_FAILURES {
            entry.locked_until = Some(now + LOGIN_LOCK);
        }
    }

    fn record_success(&self, username: &str) {
        self.attempts.lock().unwrap().remove(&Self::key(username));
    }
}

/// 认证后端
#[derive(Clone)]
pub struct Backend {
    db: ConnectionPool,
    limiter: Arc<LoginLimiter>,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateUserError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("the username is already taken or the administrator has already been initialized")]
    AlreadyExists,
    #[error("database error")]
    Database(#[source] sqlx::Error),
    #[error("password hashing task failed")]
    HashingTask(#[source] task::JoinError),
}

impl From<sqlx::Error> for CreateUserError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                CreateUserError::AlreadyExists
            }
            error => CreateUserError::Database(error),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateUserError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("user not found")]
    NotFound,
    #[error("the last enabled administrator cannot be removed, disabled or demoted")]
    LastAdmin,
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("password hashing task failed")]
    HashingTask(#[from] task::JoinError),
}

/// 认证相关的错误类型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),
}

/// `PUT /v1/web-users/{id}` 的可选改动；缺省字段保持原值。
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserChanges {
    pub role: Option<Role>,
    pub disabled: Option<bool>,
    pub password: Option<String>,
}

impl Backend {
    pub fn new(db: ConnectionPool) -> Self {
        Self {
            db,
            limiter: Arc::new(LoginLimiter::default()),
        }
    }

    pub fn pool(&self) -> &ConnectionPool {
        &self.db
    }

    /// 是否已有任何 Web 用户（`GET /v1/users/biliup` 据此决定登录页进不进注册模式）。
    pub async fn exists(&self) -> Result<bool, sqlx::Error> {
        let exists: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_users)")
            .fetch_one(&self.db)
            .await?;
        Ok(exists != 0)
    }

    /// 首次注册：只在没有任何 Web 用户时可用，创建的是超管。
    pub async fn bootstrap_admin(&self, creds: Credentials) -> Result<User, CreateUserError> {
        validate_username(&creds.username).map_err(CreateUserError::Invalid)?;
        validate_new_password(&creds.password).map_err(CreateUserError::Invalid)?;

        // 公开接口：已初始化时直接拒绝，不白白消耗 Argon2 的 CPU。
        if self.exists().await? {
            return Err(CreateUserError::AlreadyExists);
        }
        let permit = acquire_password_task_permit().await;
        // 排队等哈希线程期间别的请求可能已完成初始化；持有许可直到插入结束，
        // 让后面排队的请求看到新建的用户而不是再算一遍哈希。
        if self.exists().await? {
            return Err(CreateUserError::AlreadyExists);
        }
        let password = creds.password;
        let (password_hash, _permit) =
            task::spawn_blocking(move || (generate_hash(password), permit))
                .await
                .map_err(CreateUserError::HashingTask)?;
        // 条件插入是并发首次注册的最终裁决：同一时刻只有一条能插进去。
        let row: Option<UserRow> = sqlx::query_as(&format!(
            "INSERT INTO web_users (username, password_hash, role)
             SELECT ?, ?, 'admin' WHERE NOT EXISTS (SELECT 1 FROM web_users)
             RETURNING {USER_COLUMNS}"
        ))
        .bind(&creds.username)
        .bind(&password_hash)
        .fetch_optional(&self.db)
        .await?;
        row.ok_or(CreateUserError::AlreadyExists)?
            .try_into()
            .map_err(CreateUserError::Database)
    }

    pub async fn list_users(&self) -> Result<Vec<User>, sqlx::Error> {
        let rows: Vec<UserRow> =
            sqlx::query_as(&format!("SELECT {USER_COLUMNS} FROM web_users ORDER BY id"))
                .fetch_all(&self.db)
                .await?;
        rows.into_iter().map(User::try_from).collect()
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<User>, sqlx::Error> {
        let row: Option<UserRow> = sqlx::query_as(&format!(
            "SELECT {USER_COLUMNS} FROM web_users WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.db)
        .await?;
        row.map(User::try_from).transpose()
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        let row: Option<UserRow> = sqlx::query_as(&format!(
            "SELECT {USER_COLUMNS} FROM web_users WHERE username = ?"
        ))
        .bind(username)
        .fetch_optional(&self.db)
        .await?;
        row.map(User::try_from).transpose()
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: String,
        role: Role,
    ) -> Result<User, CreateUserError> {
        validate_username(username).map_err(CreateUserError::Invalid)?;
        validate_new_password(&password).map_err(CreateUserError::Invalid)?;
        let password_hash = hash_password(password)
            .await
            .map_err(CreateUserError::HashingTask)?;
        let row: UserRow = sqlx::query_as(&format!(
            "INSERT INTO web_users (username, password_hash, role) VALUES (?, ?, ?)
             RETURNING {USER_COLUMNS}"
        ))
        .bind(username)
        .bind(&password_hash)
        .bind(role.as_str())
        .fetch_one(&self.db)
        .await?;
        row.try_into().map_err(CreateUserError::Database)
    }

    /// 改角色 / 禁用 / 重置密码。重置密码会让该用户的所有会话失效。
    ///
    /// 「至少保留一个启用中的超管」写在同一条 UPDATE 的条件里，并发请求也绕不过去。
    pub async fn update_user(
        &self,
        id: i64,
        changes: UserChanges,
    ) -> Result<User, UpdateUserError> {
        let password_hash = match changes.password {
            Some(password) => {
                validate_new_password(&password).map_err(UpdateUserError::Invalid)?;
                Some(hash_password(password).await?)
            }
            None => None,
        };
        let result = sqlx::query(
            "UPDATE web_users SET
                role = COALESCE(?2, role),
                disabled = COALESCE(?3, disabled),
                password_hash = COALESCE(?4, password_hash),
                session_version = session_version + (?4 IS NOT NULL),
                updated_at = unixepoch()
             WHERE id = ?1
               AND (
                    (COALESCE(?2, role) = 'admin' AND COALESCE(?3, disabled) = 0)
                    OR role != 'admin' OR disabled = 1
                    OR EXISTS (SELECT 1 FROM web_users o
                               WHERE o.role = 'admin' AND o.disabled = 0 AND o.id != ?1)
               )",
        )
        .bind(id)
        .bind(changes.role.map(Role::as_str))
        .bind(changes.disabled)
        .bind(password_hash)
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            return Err(match self.find_by_id(id).await? {
                Some(_) => UpdateUserError::LastAdmin,
                None => UpdateUserError::NotFound,
            });
        }
        self.find_by_id(id).await?.ok_or(UpdateUserError::NotFound)
    }

    pub async fn delete_user(&self, id: i64) -> Result<(), UpdateUserError> {
        let result = sqlx::query(
            "DELETE FROM web_users
             WHERE id = ?1
               AND (role != 'admin' OR disabled = 1
                    OR EXISTS (SELECT 1 FROM web_users o
                               WHERE o.role = 'admin' AND o.disabled = 0 AND o.id != ?1))",
        )
        .bind(id)
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            return Err(match self.find_by_id(id).await? {
                Some(_) => UpdateUserError::LastAdmin,
                None => UpdateUserError::NotFound,
            });
        }
        Ok(())
    }

    /// 强制下线：递增会话版本，该用户所有已签发的会话在下一个请求时失效。
    pub async fn logout_everywhere(&self, id: i64) -> Result<(), UpdateUserError> {
        let result = sqlx::query(
            "UPDATE web_users SET session_version = session_version + 1, updated_at = unixepoch()
             WHERE id = ?",
        )
        .bind(id)
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            return Err(UpdateUserError::NotFound);
        }
        Ok(())
    }

    /// 用户自己改密：校验旧密码，成功后递增会话版本并返回新的用户（调用方据此重新登录当前会话）。
    pub async fn change_own_password(
        &self,
        id: i64,
        old_password: String,
        new_password: String,
    ) -> Result<Option<User>, UpdateUserError> {
        validate_new_password(&new_password).map_err(UpdateUserError::Invalid)?;
        let Some(user) = self.find_by_id(id).await? else {
            return Err(UpdateUserError::NotFound);
        };
        if old_password.len() > MAX_PASSWORD_BYTES
            || !password_matches(old_password, user.password_hash.clone()).await?
        {
            return Ok(None);
        }
        let changes = UserChanges {
            password: Some(new_password),
            ..Default::default()
        };
        self.update_user(id, changes).await.map(Some)
    }

    /// CLI 用：按用户名重置密码并让该用户所有会话失效。
    pub async fn reset_password(
        &self,
        username: &str,
        password: String,
    ) -> Result<User, UpdateUserError> {
        let user = self
            .find_by_username(username)
            .await?
            .ok_or(UpdateUserError::NotFound)?;
        let changes = UserChanges {
            password: Some(password),
            ..Default::default()
        };
        self.update_user(user.id, changes).await
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        if creds.username.is_empty()
            || creds.username.chars().count() > MAX_USERNAME_CHARS
            || creds.password.is_empty()
            || creds.password.len() > MAX_PASSWORD_BYTES
        {
            return Ok(None);
        }
        let now = Instant::now();
        if self.limiter.is_locked(&creds.username, now) {
            return Ok(None);
        }
        let user = self
            .find_by_username(&creds.username)
            .await?
            .filter(|user| !user.disabled);
        let hash = user
            .as_ref()
            .map(|user| user.password_hash.clone())
            .unwrap_or_else(dummy_hash);
        let matches = password_matches(creds.password, hash).await?;
        let Some(user) = user.filter(|_| matches) else {
            self.limiter.record_failure(&creds.username, now);
            return Ok(None);
        };
        self.limiter.record_success(&creds.username);
        sqlx::query("UPDATE web_users SET last_login_at = unixepoch() WHERE id = ?")
            .bind(user.id)
            .execute(&self.db)
            .await?;
        Ok(Some(user))
    }

    /// axum-login 每个请求都会调一次：禁用、删除、改角色都即时生效。
    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(self
            .find_by_id(*user_id)
            .await?
            .filter(|user| !user.disabled))
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    fn credentials(username: &str, password: &str) -> Credentials {
        Credentials {
            username: username.into(),
            password: password.into(),
            next: None,
        }
    }

    async fn backend() -> (tempfile::TempDir, Backend) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        (dir, Backend::new(pool))
    }

    /// 把一个全新库回退成迁移 6 之前的样子，并按旧版本的方式登记单管理员。
    async fn legacy_single_admin_database(password: &str) -> (tempfile::TempDir, String, i64) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 6")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE web_users")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO configuration (key, value) VALUES ('streamers', '{}')")
            .execute(&pool)
            .await
            .unwrap();
        let hash = hash_password(password.to_string()).await.unwrap();
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO configuration (key, value) VALUES ('biliup', ?) RETURNING id",
        )
        .bind(&hash)
        .fetch_one(&pool)
        .await
        .unwrap();
        pool.close().await;
        (dir, hash, id)
    }

    /// 老的单用户安装升级后无需任何操作：同一个 id、同一个密码、原来的会话照样有效。
    #[tokio::test]
    async fn upgrading_a_single_admin_install_keeps_login_and_sessions() {
        let (dir, legacy_hash, legacy_id) = legacy_single_admin_database("old-password").await;
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .expect("迁移 6 必须能在旧库上跑通");
        let backend = Backend::new(pool.clone());

        let users = backend.list_users().await.unwrap();
        assert_eq!(users.len(), 1);
        let admin = &users[0];
        assert_eq!(admin.id, legacy_id, "user id 不变，旧会话才能找回用户");
        assert_eq!(admin.username, "biliup");
        assert_eq!(admin.role, Role::Admin);
        assert!(!admin.disabled);
        assert_eq!(
            AuthUser::session_auth_hash(admin),
            legacy_hash.as_bytes(),
            "auth hash 与旧版本一致，已登录的浏览器不会掉线"
        );

        let logged_in = backend
            .authenticate(credentials("biliup", "old-password"))
            .await
            .unwrap()
            .expect("原密码必须还能登录");
        assert_eq!(logged_in.id, legacy_id);
        assert!(
            backend.exists().await.unwrap(),
            "升级后不能重新进入初始化流程"
        );

        let kept: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'biliup'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kept, 1, "旧行保留，回退旧版本时仍可登录");
    }

    /// 从未设置过密码的库升级后仍是空的，照常走首次注册。
    #[tokio::test]
    async fn upgrading_an_install_without_password_stays_uninitialized() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let backend = Backend::new(pool);
        assert!(!backend.exists().await.unwrap());
        assert!(backend.list_users().await.unwrap().is_empty());
        let admin = backend
            .bootstrap_admin(credentials("owner", "a-long-password"))
            .await
            .unwrap();
        assert_eq!(admin.role, Role::Admin);
    }

    #[tokio::test]
    async fn administrator_bootstrap_is_one_time_and_passwords_are_checked() {
        let (_dir, backend) = backend().await;

        assert!(matches!(
            backend
                .bootstrap_admin(credentials("biliup", "short"))
                .await,
            Err(CreateUserError::Invalid(_))
        ));
        assert!(matches!(
            backend
                .bootstrap_admin(credentials("biliup", &"x".repeat(1025)))
                .await,
            Err(CreateUserError::Invalid(_))
        ));
        assert!(matches!(
            backend
                .bootstrap_admin(credentials(" padded", "correct horse"))
                .await,
            Err(CreateUserError::Invalid(_))
        ));
        let admin = backend
            .bootstrap_admin(credentials("owner", "correct horse"))
            .await
            .unwrap();
        assert_eq!(admin.role, Role::Admin);
        assert!(matches!(
            backend
                .bootstrap_admin(credentials("second", "second password"))
                .await,
            Err(CreateUserError::AlreadyExists)
        ));
        assert!(
            backend
                .authenticate(credentials("owner", "wrong password"))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            backend
                .authenticate(credentials("OWNER", "correct horse"))
                .await
                .unwrap()
                .is_some(),
            "用户名大小写不敏感"
        );
        assert!(
            backend
                .authenticate(credentials("owner", &"x".repeat(1025)))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn concurrent_bootstrap_creates_exactly_one_administrator() {
        let (_dir, backend) = backend().await;

        let mut tasks = Vec::new();
        for index in 0..4 {
            let backend = backend.clone();
            tasks.push(tokio::spawn(async move {
                backend
                    .bootstrap_admin(credentials(&format!("admin{index}"), "password-123"))
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
        assert_eq!(backend.list_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn usernames_are_unique_case_insensitively() {
        let (_dir, backend) = backend().await;
        backend
            .bootstrap_admin(credentials("biliup", "password-1"))
            .await
            .unwrap();
        assert!(matches!(
            backend
                .create_user("BiliUp", "password-2".into(), Role::Viewer)
                .await,
            Err(CreateUserError::AlreadyExists)
        ));
    }

    #[tokio::test]
    async fn the_last_enabled_admin_cannot_be_removed() {
        let (_dir, backend) = backend().await;
        let admin = backend
            .bootstrap_admin(credentials("biliup", "password-1"))
            .await
            .unwrap();
        let demote = UserChanges {
            role: Some(Role::Operator),
            ..Default::default()
        };
        let disable = UserChanges {
            disabled: Some(true),
            ..Default::default()
        };
        assert!(matches!(
            backend.update_user(admin.id, demote.clone()).await,
            Err(UpdateUserError::LastAdmin)
        ));
        assert!(matches!(
            backend.update_user(admin.id, disable.clone()).await,
            Err(UpdateUserError::LastAdmin)
        ));
        assert!(matches!(
            backend.delete_user(admin.id).await,
            Err(UpdateUserError::LastAdmin)
        ));

        // 一个被禁用的超管不算数
        let spare = backend
            .create_user("spare", "password-2".into(), Role::Admin)
            .await
            .unwrap();
        backend
            .update_user(spare.id, disable.clone())
            .await
            .unwrap();
        assert!(matches!(
            backend.delete_user(admin.id).await,
            Err(UpdateUserError::LastAdmin)
        ));

        let enable = UserChanges {
            disabled: Some(false),
            ..Default::default()
        };
        backend.update_user(spare.id, enable).await.unwrap();
        backend.update_user(admin.id, demote).await.unwrap();
        assert!(matches!(
            backend.update_user(spare.id, disable).await,
            Err(UpdateUserError::LastAdmin)
        ));
        assert!(matches!(
            backend.delete_user(9999).await,
            Err(UpdateUserError::NotFound)
        ));
    }

    #[tokio::test]
    async fn concurrent_demotions_leave_one_admin() {
        let (_dir, backend) = backend().await;
        let a = backend
            .bootstrap_admin(credentials("a", "password-1"))
            .await
            .unwrap();
        let b = backend
            .create_user("b", "password-2".into(), Role::Admin)
            .await
            .unwrap();
        let demote = UserChanges {
            role: Some(Role::Viewer),
            ..Default::default()
        };
        let (ra, rb) = tokio::join!(
            backend.update_user(a.id, demote.clone()),
            backend.update_user(b.id, demote)
        );
        assert_eq!(ra.is_ok() as u8 + rb.is_ok() as u8, 1);
        let admins = backend
            .list_users()
            .await
            .unwrap()
            .into_iter()
            .filter(|user| user.role == Role::Admin)
            .count();
        assert_eq!(admins, 1);
    }

    #[tokio::test]
    async fn disabled_users_cannot_log_in_or_keep_sessions() {
        let (_dir, backend) = backend().await;
        backend
            .bootstrap_admin(credentials("biliup", "password-1"))
            .await
            .unwrap();
        let viewer = backend
            .create_user("viewer", "password-2".into(), Role::Viewer)
            .await
            .unwrap();
        assert!(backend.get_user(&viewer.id).await.unwrap().is_some());
        let disable = UserChanges {
            disabled: Some(true),
            ..Default::default()
        };
        backend.update_user(viewer.id, disable).await.unwrap();
        assert!(backend.get_user(&viewer.id).await.unwrap().is_none());
        assert!(
            backend
                .authenticate(credentials("viewer", "password-2"))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn password_resets_and_forced_logout_change_the_session_hash() {
        let (_dir, backend) = backend().await;
        let admin = backend
            .bootstrap_admin(credentials("biliup", "password-1"))
            .await
            .unwrap();
        let original = admin.session_auth_hash().to_vec();
        assert_eq!(
            original,
            admin.password_hash.as_bytes(),
            "版本 0 与旧版会话兼容"
        );

        backend.logout_everywhere(admin.id).await.unwrap();
        let after_logout = backend.find_by_id(admin.id).await.unwrap().unwrap();
        assert_ne!(after_logout.session_auth_hash(), original.as_slice());

        let changed = backend
            .change_own_password(admin.id, "wrong-old".into(), "password-new".into())
            .await
            .unwrap();
        assert!(changed.is_none(), "旧密码不对不能改");
        let changed = backend
            .change_own_password(admin.id, "password-1".into(), "password-new".into())
            .await
            .unwrap()
            .unwrap();
        assert_ne!(
            changed.session_auth_hash(),
            after_logout.session_auth_hash()
        );
        assert!(
            backend
                .authenticate(credentials("biliup", "password-new"))
                .await
                .unwrap()
                .is_some()
        );

        let reset = backend
            .reset_password("biliup", "password-cli".into())
            .await
            .unwrap();
        assert_ne!(reset.session_auth_hash(), changed.session_auth_hash());
        assert!(matches!(
            backend
                .reset_password("nobody", "password-cli".into())
                .await,
            Err(UpdateUserError::NotFound)
        ));
    }

    #[tokio::test]
    async fn repeated_failures_lock_the_username_for_a_while() {
        let (_dir, backend) = backend().await;
        backend
            .bootstrap_admin(credentials("biliup", "password-1"))
            .await
            .unwrap();
        for _ in 0..MAX_LOGIN_FAILURES {
            assert!(
                backend
                    .authenticate(credentials("biliup", "nope-nope"))
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            backend
                .authenticate(credentials("biliup", "password-1"))
                .await
                .unwrap()
                .is_none(),
            "锁定期间正确密码也按失败处理"
        );

        // 锁到期后恢复
        let limiter = LoginLimiter::default();
        let start = Instant::now();
        for _ in 0..MAX_LOGIN_FAILURES {
            limiter.record_failure("x", start);
        }
        assert!(limiter.is_locked("X", start));
        assert!(!limiter.is_locked("x", start + LOGIN_LOCK + Duration::from_secs(1)));
        limiter.record_failure("x", start + LOGIN_LOCK + Duration::from_secs(1));
        assert!(!limiter.is_locked("x", start + LOGIN_LOCK + Duration::from_secs(1)));
    }

    #[test]
    fn the_limiter_table_is_bounded() {
        let limiter = LoginLimiter::default();
        let now = Instant::now();
        for index in 0..(MAX_TRACKED_USERNAMES * 2) {
            limiter.record_failure(&format!("user-{index}"), now);
        }
        assert!(limiter.attempts.lock().unwrap().len() <= MAX_TRACKED_USERNAMES);
    }
}
