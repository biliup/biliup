//! 节点上「托管」的本地行：控制面分派下来、落进本机 `livestreamers` / `uploadstreamers` 的那些。
//!
//! 它们的真身在控制面，本机改了下次对账就会被覆盖，所以本机的增删改一律拒绝（409），提示去控制面改（D7）。
//! 本机自己加的主播与模板不受影响。单机与控制面进程不挂这一层。

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
        })
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

pub async fn guard(State(handle): State<ManagedHandle>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    if !watched(&method, &path) {
        return next.run(request).await;
    }
    let Some(managed) = handle.read().unwrap().clone() else {
        return next.run(request).await;
    };
    let (parts, body) = request.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, BODY_LIMIT).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    if managed.blocks(&method, &path, &bytes) {
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
}
