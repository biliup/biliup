//! 一主一备（HA Pair）接口。
//!
//! 控制面（路由在 `fleet.rs` 里注册）：
//! - `GET /v1/fleet/ha`：配对、备机连接、不纳入配对的房间与最近的场次，归 `streamer.view`
//! - `PUT /v1/fleet/ha`：指定备机或改模式 / 参数（`{"standby": 节点 id, "mode": 1|2, "params": {...}}`），
//!   `DELETE /v1/fleet/ha`：解除配对，都归 `node.manage`
//! - `POST /v1/fleet/ha/sessions/{key}/{action}`：主机面板上的人工处理（`standby-upload` / `drop`），
//!   转给备机执行，归 `upload.submit`
//!
//! 备机节点（[`node_router`]）：`GET /v1/node/ha` 与 `POST /v1/node/ha/sessions/{key}/{action}`（备机本地的
//! 人工处理，§6 H），权限同上。本机不是备机时这两个地址落回页面，与没有这组路由时一样。

use crate::server::errors::{ApiError, report_to_response};
use crate::server::fleet::controller::Controller;
use crate::server::fleet::ha;
use crate::server::fleet::ha::pairing::{Designate, Refused};
use crate::server::fleet::ha::wire::ManualAction;
use axum::body::Bytes;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use std::sync::Arc;

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(ApiError::new(message))).into_response()
}

fn refused(refused: Refused) -> Response {
    match refused {
        Refused::Invalid(message) => error(StatusCode::BAD_REQUEST, message),
        Refused::NotFound(message) => error(StatusCode::NOT_FOUND, message),
        Refused::Conflict(message) => error(StatusCode::CONFLICT, message),
    }
}

fn not_supported() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "这个控制面不支持一主一备".to_string(),
    )
}

fn unknown_action(text: &str) -> Response {
    error(
        StatusCode::BAD_REQUEST,
        format!("不认识的处理「{text}」：只能是 standby-upload（备机直接投）或 drop（放弃）"),
    )
}

pub async fn get_pair(State(controller): State<Arc<Controller>>) -> Response {
    let Some(pairing) = controller.ha() else {
        return not_supported();
    };
    match pairing.view(&controller).await {
        Ok(view) => Json(view).into_response(),
        Err(e) => report_to_response(e),
    }
}

pub async fn put_pair(State(controller): State<Arc<Controller>>, body: Bytes) -> Response {
    let Some(pairing) = controller.ha() else {
        return not_supported();
    };
    let request: Designate = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(e) => return error(StatusCode::BAD_REQUEST, format!("请求体无效：{e}")),
    };
    match pairing.designate(&controller, request).await {
        Ok(Ok(pair)) => Json(json!({ "pair": pair })).into_response(),
        Ok(Err(reason)) => refused(reason),
        Err(e) => report_to_response(e),
    }
}

pub async fn delete_pair(State(controller): State<Arc<Controller>>) -> Response {
    let Some(pairing) = controller.ha() else {
        return not_supported();
    };
    match pairing.dissolve(&controller).await {
        Ok(dissolved) => Json(json!({ "dissolved": dissolved })).into_response(),
        Err(e) => report_to_response(e),
    }
}

/// 转给备机就返回 202；备机执行的结果看 `GET /v1/fleet/ha` 里这一场的状态
pub async fn manual(
    State(controller): State<Arc<Controller>>,
    Path((key, action_text)): Path<(String, String)>,
) -> Response {
    let Some(pairing) = controller.ha() else {
        return not_supported();
    };
    let Some(action) = ManualAction::parse(&action_text) else {
        return unknown_action(&action_text);
    };
    match pairing.manual(&key, action) {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "forwarded": true }))).into_response(),
        Err(reason) => refused(reason),
    }
}

/// 节点进程的 `/v1/node/ha*`
pub fn node_router() -> Router<()> {
    Router::new()
        .route("/v1/node/ha", get(node_view))
        .route("/v1/node/ha/sessions/{key}/{action}", post(node_manual))
        .route_layer(axum::middleware::from_fn(standby_only))
}

async fn standby_only(request: Request, next: Next) -> Response {
    if ha::standby().is_none() {
        return crate::server::api::spa::static_handler(request.uri().clone())
            .await
            .into_response();
    }
    next.run(request).await
}

async fn node_view() -> Response {
    match ha::standby() {
        Some(standby) => Json(standby.view()).into_response(),
        None => error(StatusCode::NOT_FOUND, "本机不是备机".to_string()),
    }
}

async fn node_manual(Path((key, action_text)): Path<(String, String)>) -> Response {
    let Some(standby) = ha::standby() else {
        return error(StatusCode::NOT_FOUND, "本机不是备机".to_string());
    };
    let Some(action) = ManualAction::parse(&action_text) else {
        return unknown_action(&action_text);
    };
    match standby.manual(&key, action) {
        Ok(()) => Json(json!({ "done": true })).into_response(),
        Err(message) => error(StatusCode::CONFLICT, message),
    }
}

#[cfg(test)]
mod tests {
    use crate::server::api::access;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::controller::{Controller, RelaySetup};
    use crate::server::fleet::ha::pairing::Pairing;
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

    /// relay 连不上的控制面，没有「本机」节点
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

    async fn services(dir: &std::path::Path) -> ServiceRegister {
        let pool = ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await
    }

    #[tokio::test]
    async fn the_pair_api_explains_every_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let controller = controller(dir.path()).await;
        let app = crate::server::api::fleet::router(controller.clone())
            .route_layer(from_fn(access::unrestricted));

        let (status, _) = send(&app, Method::GET, "/v1/fleet/ha", "").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "没挂配对的控制面");

        let pairing = Arc::new(Pairing::new(services(dir.path()).await));
        controller.attach_ha(pairing.clone());
        pairing.resume(&controller, None).await;

        let (status, view) = send(&app, Method::GET, "/v1/fleet/ha", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["pair"], Value::Null);
        assert_eq!(view["active"], false);
        assert_eq!(view["min_proto"], 4);
        assert_eq!(view["sessions"], json!([]));

        let (status, error) = send(&app, Method::PUT, "/v1/fleet/ha", r#"{"standby":2}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        let body = json!({ "standby": 2, "mode": 1, "extra": true }).to_string();
        let (status, _) = send(&app, Method::PUT, "/v1/fleet/ha", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let body = json!({ "standby": 2, "mode": 1, "params": { "upload_start_timeout": 0 } });
        let (status, error) = send(&app, Method::PUT, "/v1/fleet/ha", &body.to_string()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");

        let body = json!({ "standby": 2, "mode": 2 }).to_string();
        let (status, error) = send(&app, Method::PUT, "/v1/fleet/ha", &body).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(
            error["message"].as_str().unwrap().contains("「本机」"),
            "{error}"
        );

        let (status, error) = send(
            &app,
            Method::POST,
            "/v1/fleet/ha/sessions/7:1000/upload",
            "",
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("standby-upload"),
            "{error}"
        );
        let (status, error) =
            send(&app, Method::POST, "/v1/fleet/ha/sessions/7:1000/drop", "").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("没有生效中的配对"),
            "{error}"
        );

        let (status, dissolved) = send(&app, Method::DELETE, "/v1/fleet/ha", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(dissolved["dissolved"], false);
        pairing.shutdown();
        controller.shutdown().await;
    }

    /// 不是备机的节点：`/v1/node/ha*` 落回页面，与没有这组路由时一样
    #[tokio::test]
    async fn the_node_ha_routes_fall_back_to_the_page_when_not_a_standby() {
        let _role = crate::server::fleet::ha::test_guard().await;
        let app = super::node_router().route_layer(from_fn(access::unrestricted));
        for (method, uri) in [
            (Method::GET, "/v1/node/ha"),
            (Method::POST, "/v1/node/ha/sessions/7:1000/drop"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let expected = crate::server::api::spa::static_handler(uri.parse().unwrap()).await;
            let expected = axum::response::IntoResponse::into_response(expected);
            assert_eq!(response.status(), expected.status(), "{uri}");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let expected = axum::body::to_bytes(expected.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body, expected, "{uri}");
        }
    }
}
