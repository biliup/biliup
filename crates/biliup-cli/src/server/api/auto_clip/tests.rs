use super::*;
use crate::server::api::access::require_permission;
use crate::server::auto_clip::fake::{FakeServer, Scenario};
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::permissions::{Permission, Role};
use crate::server::infrastructure::policy::{RouteRequirement, Subject};
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::infrastructure::users::{Backend, Credentials};
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, header};
use axum::middleware::from_fn;
use axum::response::Response;
use axum_login::AuthManagerLayerBuilder;
use serde_json::{Value, json};
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::{EnvFilter, reload};

mod session;

const KEY: &str = "sk-stored-0123456789abcdef";
const MASK: &str = "sk-…cdef";

struct Fixture {
    _dir: tempfile::TempDir,
    app: Router,
    config: Arc<RwLock<Config>>,
    pool: ConnectionPool,
    admin: String,
    operator: String,
    viewer: String,
}

async fn fixture(auto_clip: Option<AutoClipConfig>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let config = Arc::new(RwLock::new(Config {
        auto_clip,
        ..Config::default()
    }));
    let (pool1, pool2) = {
        let config = config.read().unwrap();
        (config.pool1_size, config.pool2_size)
    };
    let managers = DownloadManager::new(pool1, pool2, pool.clone());
    let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
    let register = ServiceRegister::new(pool.clone(), config.clone(), managers, log_handle).await;

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let backend = Backend::new(pool.clone());
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
    let auth_layer = AuthManagerLayerBuilder::new(
        backend,
        SessionManagerLayer::new(session_store).with_secure(false),
    )
    .build();
    let app = crate::server::router::router(register)
        .route_layer(from_fn(require_permission))
        .merge(crate::server::api::auth::router())
        .layer(auth_layer);
    let admin = login(&app, "biliup", "admin-password").await;
    let operator = login(&app, "op", "operator-password").await;
    let viewer = login(&app, "ro", "viewer-password").await;
    Fixture {
        _dir: dir,
        app,
        config,
        pool,
        admin,
        operator,
        viewer,
    }
}

fn stored(base_url: &str) -> AutoClipConfig {
    AutoClipConfig::builder()
        .enabled(true)
        .base_url(base_url.into())
        .api_key(KEY.into())
        .chat_model("chat-model".into())
        .asr_model("whisper-1".into())
        .chat_timeout_secs(2)
        .asr_timeout_secs(2)
        .build()
}

async fn login(app: &Router, username: &str, password: &str) -> String {
    let response = call(
        app,
        None,
        "POST",
        "/v1/users/login",
        Some(json!({ "username": username, "password": password })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers().get(header::SET_COOKIE).unwrap();
    cookie
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn call(
    app: &Router,
    cookie: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn bytes_of(response: Response, status: StatusCode) -> Vec<u8> {
    assert_eq!(response.status(), status);
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

async fn json_of(response: Response, status: StatusCode) -> Value {
    serde_json::from_slice(&bytes_of(response, status).await).unwrap()
}

#[test]
fn routes_are_declared_in_the_policy() {
    for (method, route, permission) in [
        (Method::POST, "/v1/auto-clip/test", Permission::ConfigEdit),
        (Method::GET, "/v1/auto-clip/status", Permission::FileView),
    ] {
        let requirement = RouteRequirement::of(&method, route, route);
        assert_eq!(
            requirement,
            RouteRequirement::Permission(permission),
            "{method} {route}"
        );
        for role in Role::ALL {
            assert_eq!(
                Subject::user(1, role).satisfies(requirement),
                role.has(permission),
                "{method} {route} {role:?}"
            );
        }
    }
    for (method, route) in [
        (Method::GET, "/v1/auto-clip/test"),
        (Method::POST, "/v1/auto-clip/status"),
    ] {
        assert_eq!(
            RouteRequirement::of(&method, route, route),
            RouteRequirement::AdminOnly,
            "{method} {route}"
        );
    }
}

/// 没配置 `auto_clip` 时，`GET /v1/configuration` 就是整份配置原样序列化，没有多出任何键。
#[tokio::test]
async fn without_auto_clip_the_configuration_response_is_unchanged() {
    let f = fixture(None).await;
    let expected = serde_json::to_vec(&serde_json::to_value(Config::default()).unwrap()).unwrap();
    let body = bytes_of(
        call(&f.app, Some(&f.admin), "GET", "/v1/configuration", None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body, expected);
    assert!(!String::from_utf8_lossy(&body).contains("auto_clip"));

    let expected =
        serde_json::to_vec(&crate::server::api::redact::config(&Config::default())).unwrap();
    let body = bytes_of(
        call(&f.app, Some(&f.viewer), "GET", "/v1/configuration", None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body, expected);

    // 从界面保存一次（不碰自动切片）也不会多出这个键
    let config = json_of(
        call(&f.app, Some(&f.admin), "GET", "/v1/configuration", None).await,
        StatusCode::OK,
    )
    .await;
    let mut edited = config.clone();
    edited["auto_clip"] = json!({ "enabled": false, "base_url": "", "api_key": null });
    let saved = json_of(
        call(
            &f.app,
            Some(&f.admin),
            "PUT",
            "/v1/configuration",
            Some(edited),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved, config);
    assert_eq!(f.config.read().unwrap().auto_clip, None);
}

/// key 对所有角色都只给掩码（含有 `config.edit` 的超管）；操作员 / 只读看到的整块是 `null`。
#[tokio::test]
async fn keys_are_masked_in_every_response() {
    let asr_key = "gsk-asr-9876543210wxyz";
    let f = fixture(Some(AutoClipConfig {
        asr_base_url: Some("https://asr.example/v1".into()),
        asr_api_key: Some(asr_key.into()),
        ..stored("https://api.example.com/v1")
    }))
    .await;
    for uri in ["/v1/configuration", "/v1/status"] {
        let body = bytes_of(
            call(&f.app, Some(&f.admin), "GET", uri, None).await,
            StatusCode::OK,
        )
        .await;
        let text = String::from_utf8(body).unwrap();
        assert!(
            !text.contains(KEY) && !text.contains(asr_key),
            "{uri}: {text}"
        );
        let value: Value = serde_json::from_str(&text).unwrap();
        let section = if uri == "/v1/status" {
            &value["config"]["auto_clip"]
        } else {
            &value["auto_clip"]
        };
        assert_eq!(section["api_key"], MASK, "{uri}");
        assert_eq!(section["asr_api_key"], "gsk…wxyz", "{uri}");
        assert_eq!(section["base_url"], "https://api.example.com/v1", "{uri}");
        for cookie in [&f.operator, &f.viewer] {
            let body = bytes_of(
                call(&f.app, Some(cookie), "GET", uri, None).await,
                StatusCode::OK,
            )
            .await;
            let text = String::from_utf8(body).unwrap();
            assert!(
                !text.contains("sk-") && !text.contains("gsk"),
                "{uri}: {text}"
            );
        }
    }
}

/// 界面把掩码原样交回来时保留原 key；PUT 的响应同样只给掩码。
#[tokio::test]
async fn saving_a_masked_key_keeps_the_stored_one() {
    let f = fixture(Some(stored("https://api.example.com/v1"))).await;
    let mut config = json_of(
        call(&f.app, Some(&f.admin), "GET", "/v1/configuration", None).await,
        StatusCode::OK,
    )
    .await;
    config["auto_clip"]["chat_model"] = json!("chat-large");
    let saved = bytes_of(
        call(
            &f.app,
            Some(&f.admin),
            "PUT",
            "/v1/configuration",
            Some(config.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let text = String::from_utf8(saved).unwrap();
    assert!(!text.contains(KEY), "{text}");
    let current = f.config.read().unwrap().auto_clip.clone().unwrap();
    assert_eq!(current.api_key.as_deref(), Some(KEY));
    assert_eq!(current.chat_model.as_deref(), Some("chat-large"));
    let row: String = sqlx::query_scalar("SELECT value FROM configuration WHERE key = 'config'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert!(row.contains(KEY), "库里存的是原 key");

    // 改了地址却交回掩码：不保存，也不把原 key 发往新地址
    config["auto_clip"]["base_url"] = json!("https://elsewhere.example/v1");
    let response = call(
        &f.app,
        Some(&f.admin),
        "PUT",
        "/v1/configuration",
        Some(config.clone()),
    )
    .await;
    let error = json_of(response, StatusCode::BAD_REQUEST).await;
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("重新填写完整的 API key"),
        "{error}"
    );
    assert_eq!(
        f.config
            .read()
            .unwrap()
            .auto_clip
            .as_ref()
            .unwrap()
            .base_url
            .as_deref(),
        Some("https://api.example.com/v1")
    );

    // 填了新 key 就换成新的
    config["auto_clip"]["api_key"] = json!("sk-brand-new-000000000000");
    call(
        &f.app,
        Some(&f.admin),
        "PUT",
        "/v1/configuration",
        Some(config),
    )
    .await;
    assert_eq!(
        f.config
            .read()
            .unwrap()
            .auto_clip
            .as_ref()
            .unwrap()
            .api_key
            .as_deref(),
        Some("sk-brand-new-000000000000")
    );
}

/// 测试连接用表单里的值；key 是掩码时用已保存的 key，结果不含 key 并存下来供状态接口用。
#[tokio::test]
async fn the_connectivity_test_uses_the_stored_key_behind_a_mask() {
    let server = FakeServer::start(Scenario::NoVision).await;
    let f = fixture(Some(stored(server.base_url()))).await;
    let form = serde_json::to_value(stored(server.base_url()).masked()).unwrap();

    for cookie in [&f.operator, &f.viewer] {
        let response = call(
            &f.app,
            Some(cookie),
            "POST",
            "/v1/auto-clip/test",
            Some(form.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert!(server.requests().is_empty());

    let body = bytes_of(
        call(
            &f.app,
            Some(&f.admin),
            "POST",
            "/v1/auto-clip/test",
            Some(form),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let text = String::from_utf8(body).unwrap();
    assert!(!text.contains(KEY), "{text}");
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["chat"]["status"], "ok");
    assert_eq!(report["vision"]["status"], "warning");
    assert_eq!(report["vision"]["capable"], false);
    assert_eq!(report["asr"]["status"], "ok");
    let seen = server.requests();
    assert_eq!(seen.len(), 3, "chat、看图、转写各一次");
    assert!(
        seen.iter()
            .all(|r| r.authorization.as_deref() == Some(&*format!("Bearer {KEY}")))
    );

    let status = json_of(
        call(&f.app, Some(&f.viewer), "GET", "/v1/auto-clip/status", None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(status["enabled"], true);
    assert_eq!(status["vision"], false);
    assert_eq!(status["thumbnails"], "auto");
    assert_eq!(status["thumbnails_active"], false);
    assert_eq!(status["asr_segments"], true);
    let host = server.base_url().trim_start_matches("http://");
    assert_eq!(status["api_host"], host.trim_end_matches("/v1"));
    assert_eq!(status["key_source"], "config");
    assert_eq!(status["last_test"]["vision"], "warning");
    assert!(!status.to_string().contains("sk-"));
}

#[tokio::test]
async fn a_masked_key_is_not_tested_against_a_new_address() {
    let server = FakeServer::start(Scenario::Ok).await;
    let f = fixture(Some(stored("https://api.example.com/v1"))).await;
    let form = serde_json::to_value(AutoClipConfig {
        base_url: Some(server.base_url().to_string()),
        ..stored("https://api.example.com/v1").masked()
    })
    .unwrap();
    let response = call(
        &f.app,
        Some(&f.admin),
        "POST",
        "/v1/auto-clip/test",
        Some(form),
    )
    .await;
    let error = json_of(response, StatusCode::BAD_REQUEST).await;
    assert!(
        error["message"].as_str().unwrap().contains("重新填写"),
        "{error}"
    );
    assert!(server.requests().is_empty(), "原 key 不应发往新地址");
}

#[test]
fn status_without_configuration_is_all_off() {
    let view = status_view(&AutoClipConfig::default(), None, None);
    assert!(!view.enabled);
    assert!(!view.configured);
    assert_eq!(view.key_source, None);
    assert!(!view.thumbnails_active);
    assert_eq!(view.max_asr_minutes, 300);
    assert_eq!(view.max_chat_tokens, 300_000);
    assert_eq!(view.last_test, None);
    let env = status_view(&AutoClipConfig::default(), None, Some("sk-env".into()));
    assert_eq!(env.key_source, Some(KeySource::Env));
}
