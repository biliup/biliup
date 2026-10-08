use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Default)]
struct Fake {
    validations: AtomicUsize,
    refreshes: AtomicUsize,
    fail: Mutex<Option<DouyuRefreshError>>,
    fail_validation: Mutex<Option<DouyuRefreshError>>,
    entered: Notify,
    resume: Notify,
    block: std::sync::atomic::AtomicBool,
    invalid_probe: std::sync::atomic::AtomicBool,
    last_ticket: Mutex<Vec<String>>,
}
#[async_trait]
impl Backend for Fake {
    async fn validate(
        &self,
        cookie: &str,
        _: Option<&str>,
    ) -> Result<Option<DouyuLoginIdentity>, DouyuRefreshError> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = *self.fail_validation.lock().unwrap() {
            return Err(error);
        }
        if self.invalid_probe.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let parsed = DouyuCookieInput::parse(cookie)?;
        Ok(parsed
            .account_id
            .map(|account_id| DouyuLoginIdentity { account_id }))
    }
    async fn refresh(
        &self,
        _: &str,
        ticket: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<DouyuCookieRefresh, DouyuRefreshError> {
        self.refreshes.fetch_add(1, Ordering::SeqCst);
        self.last_ticket.lock().unwrap().push(ticket.into());
        self.entered.notify_one();
        if self.block.load(Ordering::SeqCst) {
            self.resume.notified().await;
        }
        if let Some(error) = *self.fail.lock().unwrap() {
            return Err(error);
        }
        Ok(DouyuCookieRefresh {
            cookie: "acf_auth=fresh; acf_uid=42".into(),
            ltp0: "rotated-ticket".into(),
            dy_did: "rotated-device".into(),
            account_id: "42".into(),
            updated_fields: 2,
        })
    }
}
fn config() -> Config {
    Config {
        douyu_cookie: Some("acf_uid=42; acf_auth=old".into()),
        douyu_ltp0: Some("initial-ticket".into()),
        douyu_refresh_device_id: Some("initial-device".into()),
        ..Config::default()
    }
}
async fn fixture() -> (
    ConnectionPool,
    Arc<RwLock<Config>>,
    Arc<Fake>,
    Arc<DouyuCookieKeeper>,
) {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/15_douyu_cookie_keeper.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TABLE livestreamers (id INTEGER PRIMARY KEY, \"override\" TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    let config = Arc::new(RwLock::new(config()));
    let fake = Arc::new(Fake::default());
    let keeper = DouyuCookieKeeper::with_backend(pool.clone(), config.clone(), fake.clone());
    (pool, config, fake, keeper)
}

#[test]
fn normalized_source_hash_and_credential_boundaries() {
    let a = Source::from_config(&config()).unwrap();
    let mut reorder = config();
    reorder.douyu_cookie = Some(" acf_auth=old ; acf_uid=42 ".into());
    assert_eq!(a.hash(), Source::from_config(&reorder).unwrap().hash());
    let mut room = config();
    let patch: ConfigPatch =
        serde_json::from_value(serde_json::json!({"douyu_cookie":"acf_uid=99; acf_auth=other"}))
            .unwrap();
    apply_douyu_override(&mut room, &patch);
    assert!(room.douyu_ltp0.is_none());
    assert!(room.douyu_refresh_device_id.is_none());
    let same: ConfigPatch =
        serde_json::from_value(serde_json::json!({"douyu_cookie":"acf_auth=old; acf_uid=42"}))
            .unwrap();
    let mut room = config();
    apply_douyu_override(&mut room, &same);
    assert_eq!(room.douyu_ltp0.as_deref(), Some("initial-ticket"));
    let imported: ConfigPatch = serde_json::from_value(serde_json::json!({"douyu_cookie":"acf_auth=old; acf_uid=42; LTP0=room-ticket; dy_did=room-device"})).unwrap();
    apply_douyu_override(&mut room, &imported);
    assert_eq!(room.douyu_ltp0.as_deref(), Some("room-ticket"));
    assert_eq!(room.douyu_refresh_device_id.as_deref(), Some("room-device"));
    let mut ticket_only = config();
    ticket_only.douyu_cookie = Some("acf_uid=42; acf_auth=old; dy_did=global-device".into());
    let patch: ConfigPatch =
        serde_json::from_value(serde_json::json!({"douyu_ltp0":"room-ticket"})).unwrap();
    apply_douyu_override(&mut ticket_only, &patch);
    assert!(Source::from_config(&ticket_only).unwrap().dy_did.is_empty());
}
#[test]
fn bounded_backoff_and_clock_recovery() {
    assert_eq!(backoff(1), 300);
    assert_eq!(backoff(2), 600);
    assert_eq!(backoff(1000), 21600);
    let source = Source::from_config(&config()).unwrap();
    let mut row = Row::new(&source, 1_000_000);
    row.next_refresh_at += SUCCESS_INTERVAL;
    assert!(row.due(900_000));
    row.next_refresh_at = 900_000 + INVALID_BACKOFF;
    assert!(!row.due(900_000));
}
#[tokio::test]
async fn renewal_persists_rotations_and_runtime_resolves_after_restart() {
    let (pool, config, fake, keeper) = fixture().await;
    keeper.initialize().await.unwrap();
    let status = keeper.refresh(None).await.unwrap();
    assert_eq!(status.login_state, LoginState::Valid);
    assert_eq!(status.refresh_state, RefreshState::Scheduled);
    assert!(status.next_refresh_at.unwrap() - unix_now() > SUCCESS_INTERVAL - 10);
    let mut runtime = config.read().unwrap().clone();
    keeper.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("fresh"));
    assert!(runtime.douyu_ltp0.is_none());
    let restarted = DouyuCookieKeeper::with_backend(pool, config, fake.clone());
    restarted.initialize().await.unwrap();
    let mut runtime = super::tests::config();
    restarted.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("fresh"));
    restarted.tick().await.unwrap();
    assert_eq!(fake.validations.load(Ordering::SeqCst), 2);
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    sqlx::query("UPDATE douyu_cookie_keeper SET last_attempt_at=?")
        .bind(unix_now() - MANUAL_COOLDOWN - 1)
        .execute(&restarted.pool)
        .await
        .unwrap();
    restarted.refresh(None).await.unwrap();
    assert_eq!(fake.last_ticket.lock().unwrap()[1], "rotated-ticket");
}
#[tokio::test]
async fn network_and_invalid_credentials_keep_distinct_durable_backoffs() {
    let (_, _, fake, keeper) = fixture().await;
    *fake.fail.lock().unwrap() = Some(DouyuRefreshError::Network);
    let status = keeper.refresh(None).await.unwrap();
    assert_eq!(status.refresh_state, RefreshState::Retry);
    assert_eq!(status.login_state, LoginState::Valid);
    assert!((status.next_refresh_at.unwrap() - unix_now() - MIN_BACKOFF).abs() < 3);
    *fake.fail.lock().unwrap() = Some(DouyuRefreshError::LoginInvalid);
    sqlx::query("UPDATE douyu_cookie_keeper SET last_attempt_at=?")
        .bind(unix_now() - MANUAL_COOLDOWN - 1)
        .execute(&keeper.pool)
        .await
        .unwrap();
    let status = keeper.refresh(None).await.unwrap();
    assert!(status.needs_login);
    assert_eq!(status.refresh_state, RefreshState::CredentialsInvalid);
    assert!((status.next_refresh_at.unwrap() - unix_now() - INVALID_BACKOFF).abs() < 3);
    keeper.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn manual_and_automatic_jobs_share_the_same_lock_and_durable_lease() {
    let (pool, config, fake, keeper) = fixture().await;
    fake.block.store(true, Ordering::SeqCst);
    let runner = keeper.clone();
    let task = tokio::spawn(async move { runner.refresh(None).await.unwrap() });
    fake.entered.notified().await;
    let other = DouyuCookieKeeper::with_backend(pool, config, fake.clone());
    other.initialize().await.unwrap();
    assert_eq!(
        keeper.refresh(None).await.unwrap().refresh_state,
        RefreshState::Refreshing
    );
    assert_eq!(
        other.refresh(None).await.unwrap().refresh_state,
        RefreshState::Refreshing
    );
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    fake.resume.notify_one();
    task.await.unwrap();
}
#[tokio::test]
async fn in_flight_account_switch_discards_old_result() {
    let (_, config, fake, keeper) = fixture().await;
    fake.block.store(true, Ordering::SeqCst);
    let runner = keeper.clone();
    let task = tokio::spawn(async move { runner.refresh(None).await.unwrap() });
    fake.entered.notified().await;
    config.write().unwrap().douyu_cookie = Some("acf_uid=99; acf_auth=other".into());
    fake.resume.notify_one();
    let status = task.await.unwrap();
    assert_eq!(status.account_id.as_deref(), Some("99"));
    assert!(status.last_success_at.is_none());
    let mut runtime = config.read().unwrap().clone();
    keeper.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("other"));
}
#[tokio::test]
async fn room_override_refresh_never_uses_global_ticket() {
    let (pool, _, fake, keeper) = fixture().await;
    sqlx::query("INSERT INTO livestreamers(id,\"override\") VALUES(1,?)")
        .bind(r#"{"douyu_cookie":"acf_uid=99; acf_auth=other"}"#)
        .execute(&pool)
        .await
        .unwrap();
    let status = keeper.refresh(Some(1)).await.unwrap();
    assert!(!status.has_ltp0);
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 0);
    assert_eq!(status.refresh_state, RefreshState::MissingCredentials);
}
#[tokio::test]
async fn status_does_not_serialize_any_credential() {
    let (_, _, _, keeper) = fixture().await;
    let status = serde_json::to_string(&keeper.status(None).await.unwrap()).unwrap();
    for value in [
        "initial-ticket",
        "initial-device",
        "acf_auth",
        "source_hash",
        "lease_token",
    ] {
        assert!(!status.contains(value));
    }
}

#[tokio::test]
async fn restarted_invalid_web_cookie_observes_temporary_failure_backoff() {
    let (pool, config, fake, keeper) = fixture().await;
    *fake.fail.lock().unwrap() = Some(DouyuRefreshError::Network);
    let failure = keeper.refresh(None).await.unwrap();
    fake.invalid_probe.store(true, Ordering::SeqCst);
    let restarted = DouyuCookieKeeper::with_backend(pool, config, fake.clone());
    restarted.initialize().await.unwrap();
    restarted.tick().await.unwrap();
    let status = restarted.status(None).await.unwrap();
    assert_eq!(status.login_state, LoginState::Invalid);
    assert_eq!(status.next_refresh_at, failure.next_refresh_at);
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn repeated_manual_refreshes_observe_a_durable_short_cooldown() {
    let (pool, config, fake, keeper) = fixture().await;
    keeper.refresh(None).await.unwrap();
    keeper.refresh(None).await.unwrap();
    let other = DouyuCookieKeeper::with_backend(pool, config, fake.clone());
    other.initialize().await.unwrap();
    other.refresh(None).await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn renewal_failure_preserves_a_web_cookie_that_still_passed_login_probe() {
    let (_, config, fake, keeper) = fixture().await;
    *fake.fail.lock().unwrap() = Some(DouyuRefreshError::LoginInvalid);
    let status = keeper.refresh(None).await.unwrap();
    assert_eq!(status.login_state, LoginState::Valid);
    assert_eq!(status.refresh_state, RefreshState::CredentialsInvalid);
    let mut runtime = config.read().unwrap().clone();
    keeper.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("old"));
}

#[tokio::test]
async fn disabled_renewal_and_cookie_only_accounts_are_revalidated_periodically() {
    let (pool, config, fake, keeper) = fixture().await;
    config.write().unwrap().douyu_auto_refresh = Some(false);
    config.write().unwrap().douyu_ltp0 = None;
    keeper.tick().await.unwrap();
    assert_eq!(fake.validations.load(Ordering::SeqCst), 1);
    sqlx::query("UPDATE douyu_cookie_keeper SET last_checked_at=?")
        .bind(unix_now() - SUCCESS_INTERVAL - 1)
        .execute(&pool)
        .await
        .unwrap();
    keeper.tick().await.unwrap();
    assert_eq!(fake.validations.load(Ordering::SeqCst), 2);
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 0);
    assert_eq!(
        keeper.status(None).await.unwrap().refresh_state,
        RefreshState::Disabled
    );
}

#[tokio::test]
async fn stale_cross_process_cache_cannot_start_a_second_automatic_exchange() {
    let (pool, config, fake, keeper) = fixture().await;
    keeper.initialize().await.unwrap();
    let other = DouyuCookieKeeper::with_backend(pool, config.clone(), fake.clone());
    other.initialize().await.unwrap();
    let source = Source::from_config(&config.read().unwrap()).unwrap();
    other.validated.lock().unwrap().insert(source.hash());
    keeper.refresh(None).await.unwrap();
    other.work(None, &source, false).await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    let mut runtime = config.read().unwrap().clone();
    other.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("fresh"));
}

#[tokio::test]
async fn failed_persistence_retries_exact_result_without_issuing_more_tickets() {
    let (pool, config, fake, keeper) = fixture().await;
    keeper.initialize().await.unwrap();
    sqlx::query("CREATE TRIGGER fail_finish BEFORE UPDATE OF cookie ON douyu_cookie_keeper BEGIN SELECT RAISE(FAIL, 'disk unavailable'); END").execute(&pool).await.unwrap();
    assert!(matches!(
        keeper.refresh(None).await,
        Err(KeeperError::Database(_))
    ));
    keeper.refresh(None).await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    let mut runtime = config.read().unwrap().clone();
    keeper.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("fresh"));
    sqlx::query("DROP TRIGGER fail_finish")
        .execute(&pool)
        .await
        .unwrap();
    keeper.tick().await.unwrap();
    assert!(keeper.pending.lock().unwrap().is_empty());
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    let restarted = DouyuCookieKeeper::with_backend(pool, config, fake);
    restarted.initialize().await.unwrap();
    let mut runtime = super::tests::config();
    restarted.resolve(&mut runtime);
    assert!(runtime.douyu_cookie.unwrap().contains("fresh"));
}

#[tokio::test]
async fn abandoned_lease_recovers_after_its_deadline() {
    let (pool, config, fake, keeper) = fixture().await;
    let source = Source::from_config(&config.read().unwrap()).unwrap();
    keeper.ensure(&source).await.unwrap();
    keeper
        .claim(&source.hash(), unix_now(), true, true, true, true)
        .await
        .unwrap()
        .unwrap();
    let restarted = DouyuCookieKeeper::with_backend(pool.clone(), config, fake.clone());
    restarted.initialize().await.unwrap();
    restarted.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 0);
    sqlx::query("UPDATE douyu_cookie_keeper SET lease_until=?")
        .bind(unix_now() - 1)
        .execute(&pool)
        .await
        .unwrap();
    restarted.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn obsolete_credentials_are_pruned_and_stale_worker_configs_become_anonymous() {
    let (pool, config, _, keeper) = fixture().await;
    keeper.initialize().await.unwrap();
    let mut stale = config.read().unwrap().clone();
    config.write().unwrap().douyu_cookie = Some("acf_uid=99; acf_auth=other".into());
    *keeper.last_prune.lock().unwrap() = 0;
    let active = keeper.sources().await.unwrap();
    keeper.reconcile_sources(&active).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM douyu_cookie_keeper")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    keeper.resolve(&mut stale);
    assert!(stale.douyu_cookie.is_none());
    assert!(stale.douyu_ltp0.is_none());
}

#[tokio::test]
async fn pruning_retains_inactive_live_lease_until_the_stale_response_is_discarded() {
    let (pool, config, fake, keeper) = fixture().await;
    fake.block.store(true, Ordering::SeqCst);
    let mut stale = config.read().unwrap().clone();
    let runner = keeper.clone();
    let task = tokio::spawn(async move { runner.refresh(None).await.unwrap() });
    fake.entered.notified().await;
    config.write().unwrap().douyu_cookie = Some("acf_uid=99; acf_auth=other".into());
    let sources = keeper.sources().await.unwrap();
    keeper.reconcile_sources(&sources).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM douyu_cookie_keeper")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    keeper.resolve(&mut stale);
    assert!(stale.douyu_cookie.is_none());
    fake.resume.notify_one();
    task.await.unwrap();
    *keeper.last_prune.lock().unwrap() = 0;
    keeper.reconcile_sources(&sources).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM douyu_cookie_keeper WHERE account_id='42'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn failed_periodic_validation_observes_backoff_instead_of_retrying_each_tick() {
    let (pool, config, fake, keeper) = fixture().await;
    config.write().unwrap().douyu_auto_refresh = Some(false);
    keeper.tick().await.unwrap();
    sqlx::query("UPDATE douyu_cookie_keeper SET last_checked_at=?")
        .bind(unix_now() - SUCCESS_INTERVAL - 1)
        .execute(&pool)
        .await
        .unwrap();
    *fake.fail_validation.lock().unwrap() = Some(DouyuRefreshError::Network);
    keeper.tick().await.unwrap();
    keeper.tick().await.unwrap();
    assert_eq!(fake.validations.load(Ordering::SeqCst), 2);
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 0);
    let source = Source::from_config(&config.read().unwrap()).unwrap();
    // Even a stale caller claiming that periodic validation is needed cannot
    // bypass the persisted failure backoff after another process has failed.
    assert!(
        keeper
            .claim(&source.hash(), unix_now(), false, false, true, false)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn disabled_validation_does_not_postpone_an_overdue_renewal_when_reenabled() {
    let (pool, config, fake, keeper) = fixture().await;
    config.write().unwrap().douyu_auto_refresh = Some(false);
    keeper.initialize().await.unwrap();
    sqlx::query(
        "UPDATE douyu_cookie_keeper SET last_success_at=?,next_refresh_at=?,last_checked_at=?",
    )
    .bind(unix_now() - 4 * 86400)
    .bind(unix_now() - 86400)
    .bind(unix_now() - SUCCESS_INTERVAL - 1)
    .execute(&pool)
    .await
    .unwrap();
    keeper.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 0);
    let source = Source::from_config(&config.read().unwrap()).unwrap();
    assert!(
        keeper
            .read(&source.hash())
            .await
            .unwrap()
            .unwrap()
            .next_refresh_at
            < unix_now()
    );
    config.write().unwrap().douyu_auto_refresh = Some(true);
    keeper.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
}

#[test]
fn empty_global_device_field_uses_the_complete_exported_passport_pair() {
    let mut config = super::tests::config();
    config.douyu_cookie = Some(
        r#"[
        {"name":"acf_uid","value":"42","domain":"www.douyu.com"},
        {"name":"acf_auth","value":"old","domain":"www.douyu.com"},
        {"name":"LTP0","value":"passport-ticket","domain":"passport.douyu.com"},
        {"name":"dy_did","value":"passport-device","domain":"passport.douyu.com"}
    ]"#
        .into(),
    );
    config.douyu_ltp0 = None;
    config.douyu_refresh_device_id = Some(" ".into());
    let source = Source::from_config(&config).unwrap();
    assert_eq!(source.ltp0, "passport-ticket");
    assert_eq!(source.dy_did, "passport-device");
    assert!(source.renewable());
    assert!(!source.cookie.contains("LTP0"));
}

#[test]
fn explicitly_empty_room_cookie_stays_anonymous_and_cannot_relogin_with_global_ticket() {
    let mut room = config();
    let patch: ConfigPatch =
        serde_json::from_value(serde_json::json!({"douyu_cookie":""})).unwrap();
    apply_douyu_override(&mut room, &patch);
    let source = Source::from_config(&room).unwrap();
    assert!(source.anonymous());
    assert!(!source.renewable());
    assert!(room.douyu_ltp0.is_none());
    assert!(room.douyu_refresh_device_id.is_none());
}

#[tokio::test]
async fn existing_worker_web_requests_use_refreshed_cookie_without_restarting() {
    use crate::server::core::live::live_request;
    use crate::server::infrastructure::context::Worker;
    use crate::server::infrastructure::models::live_streamer::LiveStreamer;
    use biliup::client::StatelessClient;
    let (_, config, _, keeper) = fixture().await;
    keeper.initialize().await.unwrap();
    let streamer: LiveStreamer = serde_json::from_value(
        serde_json::json!({"id":1,"url":"https://www.douyu.com/1","remark":"hot-cookie-test"}),
    )
    .unwrap();
    let worker = Worker::new(streamer, None, config.clone(), StatelessClient::default())
        .with_douyu_keeper(keeper.clone());
    let initial = live_request(&worker);
    assert!(initial.credentials.douyu_cookie.unwrap().contains("old"));
    keeper.refresh(None).await.unwrap();
    let refreshed = live_request(&worker);
    assert!(
        refreshed
            .credentials
            .douyu_cookie
            .unwrap()
            .contains("fresh")
    );
    // The config form's original source is unchanged; hot application comes
    // from the exact-source keeper row, and an old form cannot roll it back.
    assert!(
        config
            .read()
            .unwrap()
            .douyu_cookie
            .as_ref()
            .unwrap()
            .contains("old")
    );
}

#[tokio::test]
async fn enabled_room_scope_renews_shared_global_credentials_once_when_global_is_disabled() {
    let (pool, config, fake, keeper) = fixture().await;
    config.write().unwrap().douyu_auto_refresh = Some(false);
    // Both rooms reuse the global account, but enable its maintenance locally.
    for id in [1_i64, 2] {
        sqlx::query("INSERT INTO livestreamers(id,\"override\") VALUES(?,?)")
            .bind(id)
            .bind(r#"{"douyu_auto_refresh":true}"#)
            .execute(&pool)
            .await
            .unwrap();
    }
    keeper.tick().await.unwrap();
    keeper.tick().await.unwrap();
    assert_eq!(fake.refreshes.load(Ordering::SeqCst), 1);
    assert_eq!(fake.validations.load(Ordering::SeqCst), 1);
    let global = keeper.status(None).await.unwrap();
    let room = keeper.status(Some(1)).await.unwrap();
    assert_eq!(global.refresh_state, RefreshState::Disabled);
    assert_eq!(room.refresh_state, RefreshState::Scheduled);
    assert_eq!(global.last_success_at, room.last_success_at);
    assert!(room.last_success_at.is_some());
}
