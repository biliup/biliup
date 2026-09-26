//! 节点上「托管」的本地行：控制面分派下来、落进本机 `livestreamers` / `uploadstreamers` 的那些。
//!
//! 它们的真身在控制面，本机改了下次对账就会被覆盖，所以本机的增删改一律拒绝（409），提示去控制面改（D7）。
//! 本机自己加的主播与模板不受影响。单机与控制面进程不挂这一层。
//!
//! 控制面管配置时（F3），`PUT /v1/configuration` 改到白名单键同样 409；只改名单外的键
//! （Cookie、密码等本机密钥，控制面不下发）照常保存。

use super::layers::{self, Object};
use crate::server::config::Config;
use crate::server::errors::ApiError;
use axum::Json;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

/// 请求体上限：主播与模板的保存请求都很小
const BODY_LIMIT: usize = 2 * 1024 * 1024;

/// 此刻托管在本机的行
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Managed {
    /// 界面上显示的控制面名字：票据里 relay 地址的主机名
    pub controller: String,
    /// 托管主播的本地 id → 直播间地址
    pub streamers: BTreeMap<i64, String>,
    /// 托管模板的本地 id
    pub templates: BTreeSet<i64>,
    /// 控制面在管配置时，此刻生效配置的白名单投影
    pub config: Option<Object>,
}

/// 节点代理与 HTTP 层共享；`None` 表示没有被任何控制面托管（没加入、已离开或已被移除）
pub type ManagedHandle = Arc<RwLock<Option<Managed>>>;

impl Managed {
    /// `/v1/me` 的 `fleet_node`
    pub fn view(&self) -> Value {
        serde_json::json!({
            "controller": self.controller,
            "streamers": self.streamers.keys().collect::<Vec<_>>(),
            "templates": self.templates.iter().collect::<Vec<_>>(),
            "config": self.config.is_some(),
        })
    }

    /// 本机保存配置的请求改到了哪些控制面管的键；控制面不管配置或请求体解不开时为空（交给接口本身处理）
    fn config_conflicts(&self, body: &[u8]) -> Vec<String> {
        let Some(effective) = &self.config else {
            return Vec::new();
        };
        let Ok(mut requested) = serde_json::from_slice::<Config>(body) else {
            return Vec::new();
        };
        requested.normalize_segment_limits();
        layers::changed_keys(effective, &layers::project(&requested))
    }

    fn config_message(&self, keys: &[String]) -> String {
        format!(
            "配置由控制面 {} 管理，这些项请到控制面的「节点」页修改：{}。Cookie、密码等本机密钥不随控制面下发，仍可在本机保存",
            self.controller,
            keys.join("、")
        )
    }

    fn message(&self) -> String {
        format!(
            "由控制面 {} 管理，请到控制面修改；本机只能查看",
            self.controller
        )
    }

    fn manages_url(&self, url: &str) -> bool {
        let url = url.trim();
        self.streamers.values().any(|managed| managed == url)
    }

    /// 这个请求是否会改动托管行
    fn blocks(&self, method: &Method, path: &str, body: &[u8]) -> bool {
        let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
        let id = |text: &str| text.parse::<i64>().ok();
        let json = || serde_json::from_slice::<Value>(body).unwrap_or(Value::Null);
        let field = |value: &Value, key: &str| value.get(key).and_then(Value::as_i64);
        match (method.as_str(), segments.as_slice()) {
            ("PUT", ["v1", "streamers"]) => {
                let body = json();
                field(&body, "id").is_some_and(|id| self.streamers.contains_key(&id))
                    || body
                        .get("url")
                        .and_then(Value::as_str)
                        .is_some_and(|url| self.manages_url(url))
            }
            ("POST", ["v1", "streamers"]) => json()
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|url| self.manages_url(url)),
            ("DELETE", ["v1", "streamers", streamer])
            | ("PUT", ["v1", "streamers", streamer, "pause"]) => {
                id(streamer).is_some_and(|id| self.streamers.contains_key(&id))
            }
            ("POST", ["v1", "upload", "streamers"]) => {
                field(&json(), "id").is_some_and(|id| self.templates.contains(&id))
            }
            ("DELETE", ["v1", "upload", "streamers", template]) => {
                id(template).is_some_and(|id| self.templates.contains(&id))
            }
            _ => false,
        }
    }
}

fn watched(method: &Method, path: &str) -> bool {
    matches!(*method, Method::PUT | Method::POST | Method::DELETE)
        && (path.starts_with("/v1/streamers") || path.starts_with("/v1/upload/streamers"))
}

fn is_config_save(method: &Method, path: &str) -> bool {
    *method == Method::PUT && path == "/v1/configuration"
}

pub async fn guard(State(handle): State<ManagedHandle>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let config_save = is_config_save(&method, &path);
    if !config_save && !watched(&method, &path) {
        return next.run(request).await;
    }
    let Some(managed) = handle.read().unwrap().clone() else {
        return next.run(request).await;
    };
    let (parts, body) = request.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, BODY_LIMIT).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    if config_save {
        let conflicts = managed.config_conflicts(&bytes);
        if !conflicts.is_empty() {
            let message = managed.config_message(&conflicts);
            return (StatusCode::CONFLICT, Json(ApiError::new(message))).into_response();
        }
    } else if managed.blocks(&method, &path, &bytes) {
        return (StatusCode::CONFLICT, Json(ApiError::new(managed.message()))).into_response();
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::{any, put};
    use tower::ServiceExt;

    fn managed() -> Managed {
        Managed {
            controller: "192.168.1.2".into(),
            streamers: BTreeMap::from([(3, "https://live.example/3".to_string())]),
            templates: BTreeSet::from([5]),
            ..Managed::default()
        }
    }

    #[test]
    fn only_changes_to_managed_rows_are_blocked() {
        let m = managed();
        let blocks = |method: Method, path: &str, body: Value| {
            m.blocks(&method, path, body.to_string().as_bytes())
        };
        assert!(blocks(
            Method::PUT,
            "/v1/streamers",
            serde_json::json!({ "id": 3 })
        ));
        assert!(!blocks(
            Method::PUT,
            "/v1/streamers",
            serde_json::json!({ "id": 4, "url": "x" })
        ));
        // 本地主播改成托管房间的地址也不行
        assert!(blocks(
            Method::PUT,
            "/v1/streamers",
            serde_json::json!({ "id": 4, "url": " https://live.example/3 " })
        ));
        assert!(blocks(
            Method::POST,
            "/v1/streamers",
            serde_json::json!({ "url": "https://live.example/3" })
        ));
        assert!(!blocks(
            Method::POST,
            "/v1/streamers",
            serde_json::json!({ "url": "https://live.example/4", "upload_streamers_id": 5 })
        ));
        assert!(blocks(Method::DELETE, "/v1/streamers/3", Value::Null));
        assert!(!blocks(Method::DELETE, "/v1/streamers/4", Value::Null));
        assert!(blocks(Method::PUT, "/v1/streamers/3/pause", Value::Null));
        assert!(!blocks(Method::PUT, "/v1/streamers/4/pause", Value::Null));
        assert!(blocks(
            Method::POST,
            "/v1/upload/streamers",
            serde_json::json!({ "id": 5 })
        ));
        assert!(!blocks(
            Method::POST,
            "/v1/upload/streamers",
            serde_json::json!({ "template_name": "t" })
        ));
        assert!(blocks(
            Method::DELETE,
            "/v1/upload/streamers/5",
            Value::Null
        ));
        assert!(!blocks(
            Method::DELETE,
            "/v1/upload/streamers/6",
            Value::Null
        ));
        // 查看不拦
        assert!(!blocks(Method::GET, "/v1/streamers", Value::Null));
    }

    async fn status(app: &Router, method: Method, uri: &str, body: &str) -> (StatusCode, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn the_middleware_answers_409_and_passes_the_body_through_otherwise() {
        let handle: ManagedHandle = Arc::new(RwLock::new(Some(managed())));
        let app = Router::new()
            .route("/v1/streamers", any(|body: String| async move { body }))
            .route("/v1/streamers/{id}/pause", put(|| async { "paused" }))
            .layer(axum::middleware::from_fn_with_state(handle.clone(), guard));

        let (code, body) = status(&app, Method::PUT, "/v1/streamers", r#"{"id":3}"#).await;
        assert_eq!(code, StatusCode::CONFLICT);
        assert!(body.contains("由控制面 192.168.1.2 管理"));
        let (code, body) = status(&app, Method::PUT, "/v1/streamers", r#"{"id":4}"#).await;
        assert_eq!((code, body.as_str()), (StatusCode::OK, r#"{"id":4}"#));
        let (code, _) = status(&app, Method::PUT, "/v1/streamers/3/pause", "").await;
        assert_eq!(code, StatusCode::CONFLICT);

        // 离开 / 被移除后不再拦
        *handle.write().unwrap() = None;
        let (code, _) = status(&app, Method::PUT, "/v1/streamers", r#"{"id":3}"#).await;
        assert_eq!(code, StatusCode::OK);
        let (code, _) = status(&app, Method::PUT, "/v1/streamers/3/pause", "").await;
        assert_eq!(code, StatusCode::OK);
    }

    fn config_body(config: &Config) -> String {
        serde_json::to_string(config).unwrap()
    }

    #[test]
    fn config_saves_are_checked_only_while_the_controller_manages_config() {
        let live = Config {
            kuaishou_cookie: Some("old".into()),
            ..Config::default()
        };
        let mut m = managed();
        assert!(m.config_conflicts(config_body(&live).as_bytes()).is_empty());
        assert_eq!(m.view()["config"], false);

        m.config = Some(layers::project(&live));
        assert_eq!(m.view()["config"], true);
        // 只改本机密钥：放行
        let secrets = Config {
            kuaishou_cookie: Some("new".into()),
            user: Some(crate::server::config::UserConfig {
                bili_cookie: Some("SESSDATA=x".into()),
                ..Default::default()
            }),
            ..live.clone()
        };
        assert!(
            m.config_conflicts(config_body(&secrets).as_bytes())
                .is_empty()
        );
        // 改到控制面管的键：列出来
        let changed = Config {
            pool1_size: 9,
            segment_time: Some("01:00:00".into()),
            ..secrets
        };
        assert_eq!(
            m.config_conflicts(config_body(&changed).as_bytes()),
            ["segment_time", "pool1_size"]
        );
        // 空白分段时长按未设置比较，与保存时的整理一致
        let blank = Config {
            segment_time: Some(" ".into()),
            ..live
        };
        assert!(
            m.config_conflicts(config_body(&blank).as_bytes())
                .is_empty()
        );
        // 解不开的请求体交给接口自己报错
        assert!(m.config_conflicts(b"not json").is_empty());
    }

    #[tokio::test]
    async fn local_config_saves_get_409_only_for_controller_managed_keys() {
        let live = Config::default();
        let handle: ManagedHandle = Arc::new(RwLock::new(Some(Managed {
            config: Some(layers::project(&live)),
            ..managed()
        })));
        let app = Router::new()
            .route("/v1/configuration", any(|body: String| async move { body }))
            .layer(axum::middleware::from_fn_with_state(handle.clone(), guard));

        let changed = Config {
            filename_prefix: Some("{title}".into()),
            ..live.clone()
        };
        let (code, body) = status(
            &app,
            Method::PUT,
            "/v1/configuration",
            &config_body(&changed),
        )
        .await;
        assert_eq!(code, StatusCode::CONFLICT);
        assert!(body.contains("控制面 192.168.1.2"), "{body}");
        assert!(body.contains("filename_prefix"), "{body}");

        let secrets = Config {
            twitcasting_password: Some("pw".into()),
            ..live
        };
        let payload = config_body(&secrets);
        let (code, body) = status(&app, Method::PUT, "/v1/configuration", &payload).await;
        assert_eq!((code, body), (StatusCode::OK, payload.clone()));
        let (code, _) = status(&app, Method::GET, "/v1/configuration", "").await;
        assert_eq!(code, StatusCode::OK);

        // 控制面不管配置（F2 控制面）或离开之后：随便改
        handle.write().unwrap().as_mut().unwrap().config = None;
        let body = config_body(&changed);
        let (code, _) = status(&app, Method::PUT, "/v1/configuration", &body).await;
        assert_eq!(code, StatusCode::OK);
        *handle.write().unwrap() = None;
        let (code, _) = status(&app, Method::PUT, "/v1/configuration", &body).await;
        assert_eq!(code, StatusCode::OK);
    }
}
