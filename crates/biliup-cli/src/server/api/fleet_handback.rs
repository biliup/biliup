//! 解除配对后「待归还」的行上的两个手动动作（控制面，路由在 `fleet.rs` 里注册，归 `node.manage`）：
//! `POST /v1/fleet/ha/handback/{rooms|templates}/{id}/{abandon|force}`。
//! - `abandon`：放弃交还，这一行留在主机；备机那一行在它下次落地时撤掉（离线时等它回来）
//! - `force`：立即交还（只有房间）：备机挡住了开录、本机只在投没在录时，不等投完就交接
//!
//! 成功返回 `{"done": true}`，之后的进度看 `GET /v1/fleet/ha` 的 `handback`；做不了时 400 / 404 / 409 带原因。

use crate::server::api::fleet_ha::refused;
use crate::server::errors::ApiError;
use crate::server::fleet::controller::Controller;
use crate::server::fleet::ha::handback::{Action, Kind};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

fn invalid(message: String) -> Response {
    (StatusCode::BAD_REQUEST, Json(ApiError::new(message))).into_response()
}

pub async fn act(
    State(controller): State<Arc<Controller>>,
    Path((kind, id, action)): Path<(String, i64, String)>,
) -> Response {
    let Some(pairing) = controller.ha() else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::new("这个控制面不支持一主一备".to_string())),
        )
            .into_response();
    };
    let Some(kind) = Kind::parse(&kind) else {
        return invalid(format!("不认识的「{kind}」：只能是 rooms 或 templates"));
    };
    let Some(action) = Action::parse(&action) else {
        return invalid(format!(
            "不认识的处理「{action}」：只能是 abandon（放弃交还，留在主机）或 force（立即交还）"
        ));
    };
    match pairing.handback_action(&controller, kind, id, action).await {
        Ok(()) => Json(json!({ "done": true })).into_response(),
        Err(reason) => refused(reason),
    }
}
