//! 控制面的 Fleet 配置接口（F3），路由在 `fleet.rs` 里注册，只在 `--controller` 时存在。
//!
//! - `GET` / `PUT /v1/fleet/configuration`：全局配置，只含共享的白名单键；保存后重新给所有在线节点下发
//! - `GET /v1/fleet/configuration/history`：保留着的历史版本（只读）
//! - `GET` / `PUT /v1/fleet/nodes/{id}/config`：节点覆盖，保存后重新给这台节点下发
//!
//! 白名单外带值的键（Cookie、密码）整体拒绝，控制面的库里只有白名单字段。
//! 控制面自己的 `PUT /v1/configuration` 仍只管控制面这台机器，与这里无关。

use crate::server::api::access::Caller;
use crate::server::config::Config;
use crate::server::errors::{ApiError, report_to_response};
use crate::server::fleet::config_store::{self, ConfigVersion, KEEP_VERSIONS};
use crate::server::fleet::controller::Controller;
use crate::server::fleet::layers::{self, LayerError, Object, PER_NODE_KEYS};
use crate::server::fleet::{now_ms, store};
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

fn bad_request(message: String) -> Response {
    (StatusCode::BAD_REQUEST, Json(ApiError::new(message))).into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError::new("节点不存在或已被移除".into())),
    )
        .into_response()
}

fn parse_body(body: &[u8]) -> Result<Object, String> {
    serde_json::from_slice(body).map_err(|e| format!("请求体必须是 JSON 对象：{e}"))
}

fn layer_error(error: LayerError) -> Response {
    bad_request(error.to_string())
}

/// 还没保存过全局配置时界面显示的初始值：默认配置的共享键
fn default_shared() -> Object {
    layers::project(&Config::default())
        .into_iter()
        .filter(|(key, _)| layers::is_shared(key))
        .collect()
}

fn global_view(latest: Option<ConfigVersion>) -> Value {
    let saved = latest.is_some();
    let latest = latest.unwrap_or_else(|| ConfigVersion {
        config: default_shared(),
        ..ConfigVersion::default()
    });
    json!({
        "version": latest.version,
        "saved": saved,
        "updated_at": saved.then_some(latest.updated_at),
        "updated_by": latest.updated_by,
        "config": latest.config,
        "per_node_keys": PER_NODE_KEYS,
    })
}

pub async fn get_configuration(State(controller): State<Arc<Controller>>) -> Response {
    match config_store::latest_config(controller.pool()).await {
        Ok(latest) => Json(global_view(latest)).into_response(),
        Err(e) => report_to_response(e),
    }
}

pub async fn put_configuration(
    State(controller): State<Arc<Controller>>,
    caller: Caller,
    body: Bytes,
) -> Response {
    let body = match parse_body(&body) {
        Ok(body) => body,
        Err(message) => return bad_request(message),
    };
    let normalized = match layers::normalize_global(body) {
        Ok(normalized) => normalized,
        Err(e) => return layer_error(e),
    };
    let pool = controller.pool();
    let latest = match config_store::latest_config(pool).await {
        Ok(latest) => latest,
        Err(e) => return report_to_response(e),
    };
    // 内容没变就不记新版本、不重新下发
    let (saved, changed) = match latest {
        Some(latest) if latest.config == normalized.config => (latest, false),
        _ => {
            match config_store::save_config(
                pool,
                &normalized.config,
                caller.subject.user_id,
                now_ms(),
            )
            .await
            {
                Ok(saved) => (saved, true),
                Err(e) => return report_to_response(e),
            }
        }
    };
    if changed {
        controller.push_all().await;
    }
    let mut view = global_view(Some(saved));
    view["changed"] = json!(changed);
    view["ignored"] = json!(normalized.ignored);
    Json(view).into_response()
}

pub async fn configuration_history(State(controller): State<Arc<Controller>>) -> Response {
    match config_store::config_history(controller.pool()).await {
        Ok(versions) => {
            Json(json!({ "keep": KEEP_VERSIONS, "versions": versions })).into_response()
        }
        Err(e) => report_to_response(e),
    }
}

pub async fn get_node_config(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = controller.pool();
    let patch = match config_store::node_override(pool, id).await {
        Ok(Some(patch)) => patch,
        Ok(None) => return not_found(),
        Err(e) => return report_to_response(e),
    };
    let (global, row) = match (
        config_store::latest_config(pool).await,
        store::node(pool, id).await,
    ) {
        (Ok(global), Ok(row)) => (global, row),
        (Err(e), _) | (_, Err(e)) => return report_to_response(e),
    };
    let delivered = layers::delivered(global.as_ref().map(|global| &global.config), &patch);
    let version = row.and_then(|row| row.last_version);
    Json(json!({
        "node_id": id,
        "override": patch,
        "delivered": delivered,
        "global": global_view(global),
        "state": controller.node_config_state(id, version.as_deref()),
    }))
    .into_response()
}

pub async fn put_node_config(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    body: Bytes,
) -> Response {
    let body = match parse_body(&body) {
        Ok(body) => body,
        Err(message) => return bad_request(message),
    };
    let normalized = match layers::normalize_override(body) {
        Ok(normalized) => normalized,
        Err(e) => return layer_error(e),
    };
    match config_store::set_node_override(controller.pool(), id, &normalized.config).await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(e) => return report_to_response(e),
    }
    controller.push(id).await;
    Json(json!({
        "node_id": id,
        "override": normalized.config,
        "ignored": normalized.ignored,
    }))
    .into_response()
}
