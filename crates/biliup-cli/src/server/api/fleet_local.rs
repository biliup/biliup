//! `POST /v1/fleet/local-node`：在控制面上启用「本机」节点（见 `fleet::local`），归 `node.manage`。
//! 关闭走节点列表的移除（`DELETE /v1/fleet/nodes/{id}`）。

use crate::server::errors::{ApiError, report_to_response};
use crate::server::fleet::controller::Controller;
use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct EnableLocal {
    /// 接收带 `run` 命令的房间，与 `biliup node join --allow-hooks` 相同
    #[serde(default)]
    allow_hooks: bool,
}

pub async fn enable(State(controller): State<Arc<Controller>>, body: Bytes) -> Response {
    let request: EnableLocal = if body.iter().all(u8::is_ascii_whitespace) {
        EnableLocal::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError::new(format!("请求体无效：{e}"))),
                )
                    .into_response();
            }
        }
    };
    let Some(local) = controller.local() else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::new("这个控制面不支持「本机」节点".to_string())),
        )
            .into_response();
    };
    match local.enable(&controller, request.allow_hooks).await {
        Ok(Some(node_id)) => {
            (StatusCode::CREATED, Json(json!({ "node_id": node_id }))).into_response()
        }
        Ok(None) => (
            StatusCode::CONFLICT,
            Json(ApiError::new("「本机」节点已经启用".to_string())),
        )
            .into_response(),
        Err(e) => report_to_response(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::server::api::access;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::controller::{Controller, RelaySetup};
    use crate::server::fleet::guard::ManagedHandle;
    use crate::server::fleet::local::LocalNode;
    use crate::server::fleet::revoked::Revoked;
    use crate::server::fleet::{FLEET_MIGRATOR, now_ms, store};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::service_register::ServiceRegister;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::middleware::from_fn;
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};
    use tower::ServiceExt;
    use tracing_subscriber::{EnvFilter, reload};

    async fn send(app: &Router<()>, method: Method, uri: &str, body: &str) -> (StatusCode, Value) {
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// relay 连不上：节点代理一直在重连，「本机」在列表里是离线的
    async fn controller(dir: &std::path::Path) -> Arc<Controller> {
        let pool = ConnectionManager::new_pool_with(
            dir.join("fleet.sqlite3").to_str().unwrap(),
            &FLEET_MIGRATOR,
        )
        .await
        .unwrap();
        let secret = store::identity(&pool, now_ms()).await.unwrap();
        let relay: url::Url = "http://127.0.0.1:9/".parse().unwrap();
        let relays = RelaySetup {
            local: vec![relay.clone()],
            advertised: vec![relay],
            embedded_port: None,
        };
        Controller::start(pool, secret, relays, None).await.unwrap()
    }

    async fn local_node(dir: &std::path::Path) -> Arc<LocalNode> {
        let pool = ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        let services =
            ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await;
        let file = dir.join("data/local-node.json");
        let revoked = Arc::new(Revoked::load(
            crate::server::fleet::revoked::revoked_path(&file),
            services.clone(),
        ));
        Arc::new(LocalNode::new(
            file,
            services,
            ManagedHandle::default(),
            revoked,
        ))
    }

    #[tokio::test]
    async fn the_local_node_is_enabled_once_and_takes_no_fleet_config() {
        let dir = tempfile::tempdir().unwrap();
        let controller = controller(dir.path()).await;
        let app = crate::server::api::fleet::router(controller.clone())
            .route_layer(from_fn(access::unrestricted));

        let (_, nodes) = send(&app, Method::GET, "/v1/fleet/nodes", "").await;
        assert_eq!(nodes["local_node"], Value::Null);
        let (status, _) = send(&app, Method::POST, "/v1/fleet/local-node", "").await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let local = local_node(dir.path()).await;
        local.resume(&controller).await;
        controller.attach_local(local.clone());
        let (status, _) = send(&app, Method::POST, "/v1/fleet/local-node", r#"{"x":1}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let body = json!({ "allow_hooks": true }).to_string();
        let (status, created) = send(&app, Method::POST, "/v1/fleet/local-node", &body).await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        let id = created["node_id"].as_i64().unwrap();
        let (status, _) = send(&app, Method::POST, "/v1/fleet/local-node", &body).await;
        assert_eq!(status, StatusCode::CONFLICT);

        let (_, nodes) = send(&app, Method::GET, "/v1/fleet/nodes", "").await;
        assert_eq!(nodes["local_node"], id);
        let node = &nodes["nodes"][0];
        assert_eq!(
            (
                node["name"].as_str(),
                node["local"].as_bool(),
                node["allow_hooks"].as_bool()
            ),
            (Some("本机"), Some(true), Some(true))
        );

        let uri = format!("/v1/fleet/nodes/{id}/config");
        let (status, error) = send(&app, Method::PUT, &uri, r#"{"delay":30}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            error["message"].as_str().unwrap().contains("空间配置"),
            "{error}"
        );

        // 离线的「本机」：不勾改派也当场关掉，房间留在未分派
        let (status, removal) =
            send(&app, Method::DELETE, &format!("/v1/fleet/nodes/{id}"), "").await;
        assert_eq!(status, StatusCode::OK, "{removal}");
        assert_eq!(removal["state"], "done");
        let (_, nodes) = send(&app, Method::GET, "/v1/fleet/nodes", "").await;
        assert_eq!(nodes["local_node"], Value::Null);
        assert_eq!(nodes["nodes"], json!([]));
        assert!(!dir.path().join("data/local-node.json").exists());
        controller.shutdown().await;
    }
}
