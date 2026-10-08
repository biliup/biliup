//! Durable Douyu Web login maintenance. Refreshed credentials live separately
//! from config snapshots and are resolved only against their exact source hash.

use crate::server::config::{Config, ConfigPatch};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use async_trait::async_trait;
use biliup::downloader::live::{
    DouyuCookieInput, DouyuCookieRefresh, DouyuLoginIdentity, DouyuRefreshClient, DouyuRefreshError,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use struct_patch::Patch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const SUCCESS_INTERVAL: i64 = 3 * 24 * 60 * 60;
const MIN_BACKOFF: i64 = 5 * 60;
const MANUAL_COOLDOWN: i64 = 60;
const PRUNE_INTERVAL: i64 = 5 * 60;
const MAX_BACKOFF: i64 = 6 * 60 * 60;
const INVALID_BACKOFF: i64 = 24 * 60 * 60;
// The complete protocol is bounded by an outer timeout shorter than the lease.
const LEASE_SECONDS: i64 = 5 * 60;
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(240);
const TICK: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum KeeperError {
    #[error("斗鱼续期状态暂时无法保存，请检查数据库后稍后重试")]
    Database(#[source] sqlx::Error),
    #[error("斗鱼 Cookie 格式无效，请重新导出同一账号的 Cookie")]
    InvalidCookie,
    #[error("未找到该主播")]
    NotFound,
}
impl From<sqlx::Error> for KeeperError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoginState {
    Unknown,
    Valid,
    Invalid,
    Anonymous,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshState {
    Disabled,
    MissingCredentials,
    Scheduled,
    Refreshing,
    Retry,
    CredentialsInvalid,
}

/// No cookies, ticket hashes, or device identifiers are serialized to the GUI.
/// All times are Unix seconds; account_id is the Web cookie's public numeric UID.
#[derive(Debug, Clone, Serialize)]
pub struct DouyuKeeperStatus {
    pub streamer_id: Option<i64>,
    pub enabled: bool,
    pub has_cookie: bool,
    pub has_ltp0: bool,
    pub has_device_id: bool,
    pub login_state: LoginState,
    pub refresh_state: RefreshState,
    pub account_id: Option<String>,
    pub last_success_at: Option<i64>,
    pub last_checked_at: Option<i64>,
    pub next_refresh_at: Option<i64>,
    pub failure_count: u32,
    pub last_error: Option<String>,
    pub needs_login: bool,
}

#[derive(Clone)]
struct Source {
    cookie: String,
    ltp0: String,
    dy_did: String,
    account_id: Option<String>,
    enabled: bool,
}
impl Source {
    fn from_config(config: &Config) -> Result<Self, KeeperError> {
        let parsed = DouyuCookieInput::parse(config.douyu_cookie.as_deref().unwrap_or_default())
            .map_err(|_| KeeperError::InvalidCookie)?;
        let ltp0 = nonempty(config.douyu_ltp0.as_deref())
            .map(str::to_owned)
            .or(parsed.ltp0)
            .unwrap_or_default();
        let dy_did = nonempty(config.douyu_refresh_device_id.as_deref())
            .map(str::to_owned)
            .or(parsed.dy_did)
            .unwrap_or_default();
        // Validate explicit passport fields through the same strict cookie parser.
        if !ltp0.is_empty() || !dy_did.is_empty() {
            let passport = DouyuCookieInput::parse(&format!("LTP0={ltp0}; dy_did={dy_did}"))
                .map_err(|_| KeeperError::InvalidCookie)?;
            if passport.ltp0.as_deref().unwrap_or_default() != ltp0
                || passport.dy_did.as_deref().unwrap_or_default() != dy_did
            {
                return Err(KeeperError::InvalidCookie);
            }
        }
        Ok(Self {
            cookie: parsed.cookie,
            ltp0,
            dy_did,
            account_id: parsed.account_id,
            enabled: config.douyu_auto_refresh.unwrap_or(true),
        })
    }
    fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        for field in [&self.cookie, &self.ltp0, &self.dy_did] {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    fn renewable(&self) -> bool {
        !self.ltp0.is_empty() && !self.dy_did.is_empty()
    }
    fn anonymous(&self) -> bool {
        self.cookie.is_empty() && self.ltp0.is_empty()
    }
}

/// Apply all ordinary overrides while isolating passport credentials whenever a
/// room supplies a different cookie/ticket or explicitly clears its cookie.
/// ConfigPatch treats null like absence; blank string is an anonymous-room request.
pub fn apply_douyu_override(config: &mut Config, patch: &ConfigPatch) {
    let old = Source::from_config(config).ok();
    let room_cookie_value = patch.douyu_cookie.as_ref().and_then(|v| v.as_deref());
    let cookie_cleared = room_cookie_value.is_some_and(|value| value.trim().is_empty());
    let room_cookie = nonempty(room_cookie_value);
    let imported = room_cookie.and_then(|value| DouyuCookieInput::parse(value).ok());
    let room_ticket = nonempty(patch.douyu_ltp0.as_ref().and_then(|v| v.as_deref()))
        .map(str::to_owned)
        .or_else(|| imported.as_ref().and_then(|p| p.ltp0.clone()));
    let room_did = nonempty(
        patch
            .douyu_refresh_device_id
            .as_ref()
            .and_then(|v| v.as_deref()),
    )
    .map(str::to_owned)
    .or_else(|| imported.as_ref().and_then(|p| p.dy_did.clone()));
    let cookie_changed = room_cookie.is_some_and(|value| {
        DouyuCookieInput::parse(value).ok().map(|p| p.cookie)
            != old.as_ref().map(|s| s.cookie.clone())
    });
    let ticket_changed = room_ticket
        .as_deref()
        .is_some_and(|value| old.as_ref().map(|s| s.ltp0.as_str()) != Some(value));
    let imported_pair = imported.as_ref().is_some_and(|p| p.ltp0.is_some());
    config.apply(patch.clone());
    if cookie_cleared || cookie_changed || ticket_changed || imported_pair {
        config.douyu_ltp0 = room_ticket;
        config.douyu_refresh_device_id = room_did;
        // A different ticket supplied alone represents its own credential
        // source. Clear the inherited Web cookie as well as its embedded DID;
        // the new account is established only after a validated exchange.
        if ticket_changed && room_cookie.is_none() {
            config.douyu_cookie = None;
        }
    }
}

#[derive(Clone, sqlx::FromRow)]
struct Row {
    source_hash: String,
    cookie: String,
    ltp0: String,
    dy_did: String,
    account_id: Option<String>,
    login_state: String,
    refresh_state: String,
    last_success_at: Option<i64>,
    last_checked_at: Option<i64>,
    last_attempt_at: Option<i64>,
    next_refresh_at: i64,
    failure_count: i64,
    last_error: Option<String>,
    lease_token: Option<String>,
    lease_until: Option<i64>,
    updated_at: i64,
}
impl Row {
    fn new(source: &Source, now: i64) -> Self {
        Self {
            source_hash: source.hash(),
            cookie: source.cookie.clone(),
            ltp0: source.ltp0.clone(),
            dy_did: source.dy_did.clone(),
            account_id: source.account_id.clone(),
            login_state: if source.cookie.is_empty() {
                "anonymous"
            } else {
                "unknown"
            }
            .into(),
            refresh_state: "scheduled".into(),
            last_success_at: None,
            last_checked_at: None,
            last_attempt_at: None,
            next_refresh_at: now,
            failure_count: 0,
            last_error: None,
            lease_token: None,
            lease_until: None,
            updated_at: now,
        }
    }
    fn due(&self, now: i64) -> bool {
        now >= self.next_refresh_at
            || (self.failure_count == 0
                && self.updated_at > now
                && self.next_refresh_at - now > INVALID_BACKOFF)
    }
    fn periodic_validation_due(&self, now: i64) -> bool {
        // An old successful probe is not permission to ignore a newer failed
        // attempt's retry schedule. Restart gets one separate validation pass.
        if self.failure_count > 0 && !self.due(now) {
            return false;
        }
        match self.last_checked_at {
            Some(at) => now.saturating_sub(at) >= SUCCESS_INTERVAL || at > now,
            None => self.failure_count == 0 || self.due(now),
        }
    }
}

#[async_trait]
trait Backend: Send + Sync {
    async fn validate(
        &self,
        cookie: &str,
        expected: Option<&str>,
    ) -> Result<Option<DouyuLoginIdentity>, DouyuRefreshError>;
    async fn refresh(
        &self,
        cookie: &str,
        ltp0: &str,
        dy_did: &str,
        expected: Option<&str>,
    ) -> Result<DouyuCookieRefresh, DouyuRefreshError>;
}
struct HttpBackend(Result<DouyuRefreshClient, DouyuRefreshError>);
#[async_trait]
impl Backend for HttpBackend {
    async fn validate(
        &self,
        cookie: &str,
        expected: Option<&str>,
    ) -> Result<Option<DouyuLoginIdentity>, DouyuRefreshError> {
        self.0
            .as_ref()
            .map_err(|error| *error)?
            .validate(cookie, expected)
            .await
    }
    async fn refresh(
        &self,
        cookie: &str,
        ltp0: &str,
        dy_did: &str,
        expected: Option<&str>,
    ) -> Result<DouyuCookieRefresh, DouyuRefreshError> {
        self.0
            .as_ref()
            .map_err(|error| *error)?
            .refresh(cookie, ltp0, dy_did, expected)
            .await
    }
}

pub struct DouyuCookieKeeper {
    pool: ConnectionPool,
    config: Arc<RwLock<Config>>,
    backend: Arc<dyn Backend>,
    cache: RwLock<HashMap<String, Row>>,
    retired: RwLock<HashSet<String>>,
    last_prune: Mutex<i64>,
    validated: Mutex<HashSet<String>>,
    in_flight: Mutex<HashSet<String>>,
    // A successful exchange can outlive a disk failure. Retry saving that exact
    // result before issuing more tickets, and keep a process-local network delay.
    pending: Mutex<HashMap<String, Row>>,
    memory_backoff: Mutex<HashMap<String, i64>>,
    lifecycle: Mutex<Option<(CancellationToken, JoinHandle<()>)>>,
}
impl fmt::Debug for DouyuCookieKeeper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DouyuCookieKeeper").finish_non_exhaustive()
    }
}
impl DouyuCookieKeeper {
    pub fn new(pool: ConnectionPool, config: Arc<RwLock<Config>>) -> Arc<Self> {
        Self::with_backend(
            pool,
            config,
            Arc::new(HttpBackend(DouyuRefreshClient::new())),
        )
    }
    fn with_backend(
        pool: ConnectionPool,
        config: Arc<RwLock<Config>>,
        backend: Arc<dyn Backend>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            config,
            backend,
            cache: RwLock::new(HashMap::new()),
            retired: RwLock::new(HashSet::new()),
            last_prune: Mutex::new(0),
            validated: Mutex::new(HashSet::new()),
            in_flight: Mutex::new(HashSet::new()),
            pending: Mutex::new(HashMap::new()),
            memory_backoff: Mutex::new(HashMap::new()),
            lifecycle: Mutex::new(None),
        })
    }
    /// Hydrate durable cookies before creating recording workers. No HTTP occurs.
    pub async fn initialize(&self) -> Result<(), KeeperError> {
        let sources = self.sources().await?;
        self.reconcile_sources(&sources).await?;
        for (_, source) in sources {
            if !source.anonymous() {
                self.ensure(&source).await?;
            }
        }
        Ok(())
    }
    pub fn start(self: &Arc<Self>) {
        let mut slot = self.lifecycle.lock().unwrap();
        if slot.as_ref().is_some_and(|(_, task)| !task.is_finished()) {
            return;
        }
        let cancel = CancellationToken::new();
        let runner = Arc::clone(self);
        let cancellation = cancel.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(TICK);
            loop {
                tokio::select! {
                    _ = cancellation.cancelled() => break,
                    _ = interval.tick() => { if runner.tick().await.is_err() { tracing::warn!("斗鱼续期状态暂时无法保存"); } }
                }
            }
        });
        *slot = Some((cancel, task));
    }
    /// Abort promptly, including in-flight HTTP, so cloned services do not keep
    /// the keeper alive after shutdown. The durable lease expires on recovery.
    pub fn stop(&self) {
        if let Some((cancel, task)) = self.lifecycle.lock().unwrap().take() {
            cancel.cancel();
            task.abort();
        }
    }
    pub async fn status(&self, streamer_id: Option<i64>) -> Result<DouyuKeeperStatus, KeeperError> {
        let source = self.source_for_scope(streamer_id).await?;
        let row = if source.anonymous() {
            None
        } else {
            Some(self.ensure(&source).await?)
        };
        Ok(self.make_status(streamer_id, &source, row.as_ref()))
    }
    pub async fn refresh(
        &self,
        streamer_id: Option<i64>,
    ) -> Result<DouyuKeeperStatus, KeeperError> {
        let source = self.source_for_scope(streamer_id).await?;
        if !source.anonymous() {
            self.ensure(&source).await?;
            // Manual refresh is allowed while automatic renewal is disabled.
            self.work(streamer_id, &source, true).await?;
        }
        self.status(streamer_id).await
    }
    /// Called on a config clone after room overrides. Source matching includes
    /// the original Web cookie and passport pair, not a global account fallback.
    pub fn resolve(&self, config: &mut Config) {
        let parsed = Source::from_config(config);
        // Strip passport secrets even when a new source has not been hydrated.
        config.douyu_ltp0 = None;
        config.douyu_refresh_device_id = None;
        let Ok(source) = parsed else {
            return;
        };
        config.douyu_cookie = (!source.cookie.is_empty()).then(|| source.cookie.clone());
        let hash = source.hash();
        if self.retired.read().unwrap().contains(&hash) {
            config.douyu_cookie = None;
            config.douyu_ltp0 = None;
            config.douyu_refresh_device_id = None;
            return;
        }
        let cache = self.cache.read().unwrap();
        let Some(row) = cache.get(&hash) else {
            return;
        };
        // Explicit invalid-login evidence means never sending this known-dead
        // cookie to recording APIs. A temporary failure retains the last cookie.
        config.douyu_cookie = Some(if row.login_state == "invalid" {
            String::new()
        } else {
            row.cookie.clone()
        });
        // Long-lived credentials are never needed by recording requests.
        config.douyu_ltp0 = None;
        config.douyu_refresh_device_id = None;
    }
    async fn tick(&self) -> Result<(), KeeperError> {
        let sources = self.sources().await?;
        self.reconcile_sources(&sources).await?;
        self.flush_pending().await;
        // The scheduling flag is scope-local and deliberately excluded from
        // credential identity. Enable a shared group when ANY scope enables it,
        // choosing that scope for the post-request source guard.
        let mut groups: HashMap<String, (Option<i64>, Source)> = HashMap::new();
        for (scope, source) in sources {
            if source.anonymous() {
                continue;
            }
            let hash = source.hash();
            if groups
                .get(&hash)
                .is_none_or(|(_, previous)| !previous.enabled && source.enabled)
            {
                groups.insert(hash, (scope, source));
            }
        }
        for (_, (scope, source)) in groups {
            let row = self.ensure(&source).await?;
            let validation_needed = !self.validated.lock().unwrap().contains(&source.hash())
                || row.periodic_validation_due(unix_now());
            if validation_needed || (source.enabled && source.renewable() && row.due(unix_now())) {
                self.work(scope, &source, false).await?;
            }
        }
        Ok(())
    }
    async fn work(
        &self,
        scope: Option<i64>,
        source: &Source,
        manual: bool,
    ) -> Result<(), KeeperError> {
        let hash = source.hash();
        let now = unix_now();
        // Even a manual request observes an in-memory delay after disk failure;
        // repeatedly clicking must not bypass the anti-ticket-storm safeguard.
        if self
            .memory_backoff
            .lock()
            .unwrap()
            .get(&hash)
            .is_some_and(|at| *at > now)
        {
            return Ok(());
        }
        let Some(_guard) = InFlightGuard::acquire(&self.in_flight, &hash) else {
            return Ok(());
        };
        let observed = self.ensure(source).await?;
        let startup_validation = !self.validated.lock().unwrap().contains(&hash);
        let validation_requested = startup_validation || observed.periodic_validation_due(now);
        let Some(mut row) = self
            .claim(
                &hash,
                now,
                manual,
                startup_validation,
                validation_requested,
                source.enabled && source.renewable(),
            )
            .await?
        else {
            return Ok(());
        };
        self.cache
            .write()
            .unwrap()
            .insert(hash.clone(), row.clone());
        let validate_needed =
            !self.validated.lock().unwrap().contains(&hash) || row.periodic_validation_due(now);
        let outcome = tokio::time::timeout(
            EXCHANGE_TIMEOUT,
            self.exchange(&mut row, source, manual, validate_needed, now),
        )
        .await;
        if outcome.is_err() {
            record_failure(&mut row, DouyuRefreshError::Network, now);
        }
        // Check the same scope again after network completion. Source hashes on
        // persisted rows ensure an edit racing this final check is harmless too.
        let current = self.source_for_scope(scope).await;
        if current.as_ref().is_ok_and(|s| s.hash() == hash) {
            row.updated_at = unix_now();
            match self.finish(&row).await {
                Ok(true) => {
                    row.lease_token = None;
                    row.lease_until = None;
                    self.cache.write().unwrap().insert(hash.clone(), row);
                    self.validated.lock().unwrap().insert(hash);
                }
                Ok(false) => {
                    self.cache.write().unwrap().remove(&hash);
                }
                Err(error) => {
                    self.memory_backoff
                        .lock()
                        .unwrap()
                        .insert(hash.clone(), now + MAX_BACKOFF);
                    self.pending
                        .lock()
                        .unwrap()
                        .insert(hash.clone(), row.clone());
                    let mut visible = row;
                    visible.refresh_state = "retry".into();
                    visible.last_error = Some("续期状态尚未保存，正在重试保存".into());
                    visible.lease_token = None;
                    visible.lease_until = None;
                    self.cache.write().unwrap().insert(hash, visible);
                    return Err(error);
                }
            }
        } else {
            self.release(&hash, row.lease_token.as_deref().unwrap_or_default())
                .await?;
            self.cache.write().unwrap().remove(&hash);
        }
        Ok(())
    }
    async fn exchange(
        &self,
        row: &mut Row,
        source: &Source,
        manual: bool,
        validate_needed: bool,
        now: i64,
    ) {
        let mut invalid_current = false;
        if validate_needed {
            match self
                .backend
                .validate(&row.cookie, row.account_id.as_deref())
                .await
            {
                Ok(identity) => {
                    row.last_checked_at = Some(now);
                    row.login_state = if identity.is_some() {
                        "valid"
                    } else if row.cookie.is_empty() {
                        "anonymous"
                    } else {
                        "invalid"
                    }
                    .into();
                    invalid_current = identity.is_none();
                }
                Err(error) => {
                    record_failure(row, error, now);
                    return;
                }
            }
        }
        let can_auto = source.enabled && source.renewable();
        // A previously failed long-term credential keeps its persisted backoff
        // on restart even when its Web cookie is now known invalid.
        let recover_invalid = invalid_current && row.failure_count == 0;
        let do_refresh =
            source.renewable() && (manual || (can_auto && (row.due(now) || recover_invalid)));
        if !do_refresh {
            if row.refresh_state == "refreshing" {
                row.refresh_state = "scheduled".into();
            }
            // Validation has its own last_checked_at schedule. Preserve the
            // renewal deadline while disabled, so re-enabling catches up rather
            // than extending an old cookie beyond its actual login lifetime.
            return;
        }
        match self
            .backend
            .refresh(
                &row.cookie,
                &row.ltp0,
                &row.dy_did,
                row.account_id.as_deref(),
            )
            .await
        {
            Ok(fresh) => {
                if row
                    .account_id
                    .as_ref()
                    .is_some_and(|uid| uid != &fresh.account_id)
                {
                    record_failure(row, DouyuRefreshError::AccountMismatch, now);
                    return;
                }
                row.cookie = fresh.cookie;
                row.ltp0 = fresh.ltp0;
                row.dy_did = fresh.dy_did;
                row.account_id = Some(fresh.account_id);
                row.login_state = "valid".into();
                row.refresh_state = "scheduled".into();
                row.last_success_at = Some(now);
                row.last_checked_at = Some(now);
                row.next_refresh_at = now + SUCCESS_INTERVAL;
                row.failure_count = 0;
                row.last_error = None;
            }
            Err(error) => record_failure(row, error, now),
        }
    }
    async fn ensure(&self, source: &Source) -> Result<Row, KeeperError> {
        let hash = source.hash();
        if self.pending.lock().unwrap().contains_key(&hash) {
            if let Some(row) = self.cache.read().unwrap().get(&hash).cloned() {
                return Ok(row);
            }
        }
        if let Some(row) = self.read(&hash).await? {
            self.cache.write().unwrap().insert(hash, row.clone());
            return Ok(row);
        }
        let initial = Row::new(source, unix_now());
        sqlx::query("INSERT OR IGNORE INTO douyu_cookie_keeper (source_hash,cookie,ltp0,dy_did,account_id,login_state,refresh_state,next_refresh_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(&initial.source_hash).bind(&initial.cookie).bind(&initial.ltp0).bind(&initial.dy_did).bind(&initial.account_id)
            .bind(&initial.login_state).bind(&initial.refresh_state).bind(initial.next_refresh_at).bind(initial.updated_at)
            .execute(&self.pool).await?;
        let row = self.read(&hash).await?.ok_or(KeeperError::NotFound)?;
        self.cache.write().unwrap().insert(hash, row.clone());
        Ok(row)
    }
    async fn read(&self, hash: &str) -> Result<Option<Row>, KeeperError> {
        Ok(
            sqlx::query_as::<_, Row>("SELECT * FROM douyu_cookie_keeper WHERE source_hash=?")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
    async fn claim(
        &self,
        hash: &str,
        now: i64,
        manual: bool,
        startup_validation: bool,
        validation: bool,
        auto: bool,
    ) -> Result<Option<Row>, KeeperError> {
        let token = format!("{:032x}", rand::random::<u128>());
        let result = sqlx::query("UPDATE douyu_cookie_keeper SET lease_token=?,lease_until=?,last_attempt_at=? WHERE source_hash=? AND (lease_token IS NULL OR lease_until<=?) AND ((? AND (last_attempt_at IS NULL OR last_attempt_at<=?)) OR (NOT ? AND (? OR (? AND (failure_count=0 OR next_refresh_at<=?) AND (last_checked_at IS NULL OR last_checked_at<=? OR last_checked_at>?)) OR (? AND (next_refresh_at<=? OR (failure_count=0 AND updated_at>? AND next_refresh_at-?>?))))))")
            .bind(&token).bind(now+LEASE_SECONDS).bind(now).bind(hash).bind(now)
            .bind(manual).bind(now-MANUAL_COOLDOWN).bind(manual).bind(startup_validation).bind(validation)
            .bind(now).bind(now-SUCCESS_INTERVAL).bind(now).bind(auto)
            .bind(now).bind(now).bind(now).bind(INVALID_BACKOFF).execute(&self.pool).await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.memory_backoff
                    .lock()
                    .unwrap()
                    .insert(hash.into(), now + MIN_BACKOFF);
                return Err(error.into());
            }
        };
        if result.rows_affected() == 0 {
            if let Some(row) = self.read(hash).await? {
                self.cache.write().unwrap().insert(hash.into(), row);
            }
            return Ok(None);
        }
        self.read(hash).await
    }
    async fn finish(&self, row: &Row) -> Result<bool, KeeperError> {
        let result = sqlx::query("UPDATE douyu_cookie_keeper SET cookie=?,ltp0=?,dy_did=?,account_id=?,login_state=?,refresh_state=?,last_success_at=?,last_checked_at=?,last_attempt_at=?,next_refresh_at=?,failure_count=?,last_error=?,updated_at=?,lease_token=NULL,lease_until=NULL WHERE source_hash=? AND lease_token=?")
            .bind(&row.cookie).bind(&row.ltp0).bind(&row.dy_did).bind(&row.account_id).bind(&row.login_state).bind(&row.refresh_state)
            .bind(row.last_success_at).bind(row.last_checked_at).bind(row.last_attempt_at).bind(row.next_refresh_at).bind(row.failure_count).bind(&row.last_error)
            .bind(row.updated_at).bind(&row.source_hash).bind(&row.lease_token).execute(&self.pool).await?;
        Ok(result.rows_affected() == 1)
    }
    async fn release(&self, hash: &str, token: &str) -> Result<(), KeeperError> {
        sqlx::query("UPDATE douyu_cookie_keeper SET lease_token=NULL,lease_until=NULL WHERE source_hash=? AND lease_token=?")
            .bind(hash).bind(token).execute(&self.pool).await?;
        Ok(())
    }
    async fn reconcile_sources(
        &self,
        sources: &[(Option<i64>, Source)],
    ) -> Result<(), KeeperError> {
        let active: HashSet<String> = sources.iter().map(|(_, source)| source.hash()).collect();
        {
            let mut cache = self.cache.write().unwrap();
            let mut retired = self.retired.write().unwrap();
            for hash in cache.keys().filter(|hash| !active.contains(*hash)) {
                retired.insert(hash.clone());
            }
            retired.retain(|hash| !active.contains(hash));
            cache.retain(|hash, _| active.contains(hash));
        }
        // Unsaved results for removed sources are obsolete too; don't write them
        // back after a user clears or replaces the account.
        self.pending
            .lock()
            .unwrap()
            .retain(|hash, _| active.contains(hash));
        self.memory_backoff
            .lock()
            .unwrap()
            .retain(|hash, _| active.contains(hash));
        self.validated
            .lock()
            .unwrap()
            .retain(|hash| active.contains(hash));
        let now = unix_now();
        {
            let mut last = self.last_prune.lock().unwrap();
            if *last <= now && now - *last < PRUNE_INTERVAL {
                return Ok(());
            }
            *last = now;
        }
        let hashes: Vec<String> = sqlx::query_scalar("SELECT source_hash FROM douyu_cookie_keeper")
            .fetch_all(&self.pool)
            .await?;
        let inactive: Vec<String> = hashes
            .into_iter()
            .filter(|hash| !active.contains(hash))
            .collect();
        self.retired
            .write()
            .unwrap()
            .extend(inactive.iter().cloned());
        // Delete at most 100 obsolete rows per pass. Live leases remain until
        // their owner discards its stale response or the bounded lease expires.
        for hash in inactive.into_iter().take(100) {
            sqlx::query("DELETE FROM douyu_cookie_keeper WHERE source_hash=? AND (lease_token IS NULL OR lease_until<=?)")
                .bind(hash).bind(now).execute(&self.pool).await?;
        }
        Ok(())
    }
    async fn flush_pending(&self) {
        let pending: Vec<(String, Row)> = self
            .pending
            .lock()
            .unwrap()
            .iter()
            .map(|(hash, row)| (hash.clone(), row.clone()))
            .collect();
        for (hash, mut row) in pending {
            match self.finish(&row).await {
                Ok(saved) => {
                    self.pending.lock().unwrap().remove(&hash);
                    self.memory_backoff.lock().unwrap().remove(&hash);
                    if saved {
                        row.lease_token = None;
                        row.lease_until = None;
                        self.cache.write().unwrap().insert(hash.clone(), row);
                        self.validated.lock().unwrap().insert(hash);
                    } else {
                        self.cache.write().unwrap().remove(&hash);
                    }
                }
                Err(_) => {}
            }
        }
    }
    async fn source_for_scope(&self, scope: Option<i64>) -> Result<Source, KeeperError> {
        let mut config = self.config.read().unwrap().clone();
        if let Some(id) = scope {
            let patch = sqlx::query_scalar::<_, Option<String>>(
                "SELECT \"override\" FROM livestreamers WHERE id=?",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(KeeperError::NotFound)?;
            if let Some(patch) = patch {
                let patch: ConfigPatch =
                    serde_json::from_str(&patch).map_err(|_| KeeperError::InvalidCookie)?;
                apply_douyu_override(&mut config, &patch);
            }
        }
        Source::from_config(&config)
    }
    async fn sources(&self) -> Result<Vec<(Option<i64>, Source)>, KeeperError> {
        let config = self.config.read().unwrap().clone();
        let mut sources = Vec::new();
        if let Ok(source) = Source::from_config(&config) {
            sources.push((None, source));
        }
        let rows = sqlx::query_as::<_, (i64, Option<String>)>(
            "SELECT id, \"override\" FROM livestreamers",
        )
        .fetch_all(&self.pool)
        .await?;
        for (id, patch) in rows {
            if let Some(patch) = patch {
                let patch: ConfigPatch = match serde_json::from_str(&patch) {
                    Ok(patch) => patch,
                    Err(_) => continue,
                };
                let mut room = config.clone();
                apply_douyu_override(&mut room, &patch);
                if let Ok(source) = Source::from_config(&room) {
                    sources.push((Some(id), source));
                }
            }
        }
        Ok(sources)
    }
    fn make_status(
        &self,
        scope: Option<i64>,
        source: &Source,
        row: Option<&Row>,
    ) -> DouyuKeeperStatus {
        let login_state = match row.map(|r| r.login_state.as_str()) {
            Some("valid") => LoginState::Valid,
            Some("invalid") => LoginState::Invalid,
            Some("anonymous") => LoginState::Anonymous,
            _ if source.cookie.is_empty() => LoginState::Anonymous,
            _ => LoginState::Unknown,
        };
        let refresh_state = if !source.enabled {
            RefreshState::Disabled
        } else if !source.renewable() {
            RefreshState::MissingCredentials
        } else if self.in_flight.lock().unwrap().contains(&source.hash())
            || row.is_some_and(|r| r.lease_until.is_some_and(|at| at > unix_now()))
        {
            RefreshState::Refreshing
        } else {
            match row.map(|r| r.refresh_state.as_str()) {
                Some("credentials_invalid") => RefreshState::CredentialsInvalid,
                Some("retry") => RefreshState::Retry,
                _ => RefreshState::Scheduled,
            }
        };
        DouyuKeeperStatus {
            streamer_id: scope,
            enabled: source.enabled,
            has_cookie: !source.cookie.is_empty(),
            has_ltp0: !source.ltp0.is_empty(),
            has_device_id: !source.dy_did.is_empty(),
            login_state,
            refresh_state,
            account_id: row
                .and_then(|r| r.account_id.clone())
                .or_else(|| source.account_id.clone()),
            last_success_at: row.and_then(|r| r.last_success_at),
            last_checked_at: row.and_then(|r| r.last_checked_at),
            next_refresh_at: (source.enabled && source.renewable())
                .then(|| row.map(|r| r.next_refresh_at))
                .flatten(),
            failure_count: row
                .map(|r| r.failure_count.max(0).min(u32::MAX as i64) as u32)
                .unwrap_or(0),
            last_error: row.and_then(|r| r.last_error.clone()),
            needs_login: matches!(login_state, LoginState::Invalid)
                || row.is_some_and(|r| r.refresh_state == "credentials_invalid"),
        }
    }
}

struct InFlightGuard<'a> {
    set: &'a Mutex<HashSet<String>>,
    hash: String,
}
impl<'a> InFlightGuard<'a> {
    fn acquire(set: &'a Mutex<HashSet<String>>, hash: &str) -> Option<Self> {
        set.lock().unwrap().insert(hash.into()).then(|| Self {
            set,
            hash: hash.into(),
        })
    }
}
impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.set.lock().unwrap().remove(&self.hash);
    }
}
fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}
fn backoff(failures: i64) -> i64 {
    MIN_BACKOFF
        .saturating_mul(1_i64 << failures.saturating_sub(1).clamp(0, 10))
        .min(MAX_BACKOFF)
}
fn record_failure(row: &mut Row, error: DouyuRefreshError, now: i64) {
    row.failure_count = row.failure_count.saturating_add(1);
    row.last_error = Some(error.to_string());
    row.refresh_state = if error.is_login_invalid() {
        "credentials_invalid"
    } else {
        "retry"
    }
    .into();
    row.next_refresh_at = now
        + if error.is_login_invalid() {
            INVALID_BACKOFF
        } else {
            backoff(row.failure_count)
        };
}

#[cfg(test)]
mod tests;
