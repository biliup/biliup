//! 控制面的告警与控制台汇总接口（路由在 `api/fleet.rs`）。
//! 查看归 `streamer.view`，「知道了」归 `node.manage`（见 `permissions.rs`）。

use crate::server::errors::report_to_response;
use crate::server::fleet::controller::Controller;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

pub async fn list_alerts(State(controller): State<Arc<Controller>>) -> Response {
    Json(controller.alert_list()).into_response()
}

/// 全部「知道了」
pub async fn clear_alerts(State(controller): State<Arc<Controller>>) -> Response {
    Json(json!({ "cleared": controller.acknowledge_alerts() })).into_response()
}

pub async fn clear_alert(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<u64>,
) -> Response {
    if controller.acknowledge_alert(id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (StatusCode::NOT_FOUND, "告警不存在或已被清除").into_response()
    }
}

pub async fn summary(State(controller): State<Arc<Controller>>) -> Response {
    match controller.summary().await {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => report_to_response(e),
    }
}
