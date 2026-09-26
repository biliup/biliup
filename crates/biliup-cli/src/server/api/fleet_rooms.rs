//! 控制面的房间、投稿模板与节点账号接口（路由在 `api/fleet.rs`）。
//! 列表归 `streamer.view`，增删改与分派归 `node.manage`（见 `permissions.rs`）。

use crate::server::api::access::Caller;
use crate::server::errors::{ApiError, report_to_response};
use crate::server::fleet::controller::{Controller, CreateRoom, DispatchError, UpdateRoom};
use crate::server::fleet::model::TemplateSpec;
use crate::server::fleet::now_ms;
use crate::server::infrastructure::policy::Field;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

fn error_response(error: DispatchError) -> Response {
    let (status, message) = match error {
        DispatchError::NotFound(message) => (StatusCode::NOT_FOUND, message.to_string()),
        DispatchError::Invalid(message) => (StatusCode::BAD_REQUEST, message),
        DispatchError::Conflict(message) => (StatusCode::CONFLICT, message),
        DispatchError::Internal(report) => return report_to_response(report),
    };
    (status, Json(ApiError::new(message))).into_response()
}

fn respond<T: serde::Serialize>(result: Result<T, DispatchError>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn list_rooms(State(controller): State<Arc<Controller>>, caller: Caller) -> Response {
    match controller
        .rooms(caller.can_access(Field::StreamerHooks))
        .await
    {
        Ok(rooms) => Json(json!({ "now": now_ms(), "rooms": rooms })).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn create_room(
    State(controller): State<Arc<Controller>>,
    caller: Caller,
    Json(mut request): Json<CreateRoom>,
) -> Response {
    if !caller.can_access(Field::StreamerHooks) {
        crate::server::fleet::controller::strip_hooks(&mut request.spec);
    }
    match controller.create_room(request).await {
        Ok(room) => (StatusCode::CREATED, Json(room)).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn update_room(
    State(controller): State<Arc<Controller>>,
    caller: Caller,
    Path(id): Path<i64>,
    Json(request): Json<UpdateRoom>,
) -> Response {
    let keep_hooks = !caller.can_access(Field::StreamerHooks);
    respond(controller.update_room(id, request, keep_hooks).await)
}

/// `DELETE /v1/fleet/nodes/{id}?reassign=auto`
pub async fn revoke_and_reassign(controller: &Controller, id: i64) -> Response {
    match controller.revoke_and_reassign(id).await {
        Ok(Some(outcome)) => Json(outcome).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "节点不存在或已被移除").into_response(),
        Err(error) => error_response(error),
    }
}

#[derive(Deserialize, Default)]
pub struct DeleteQuery {
    #[serde(default)]
    force: bool,
}

pub async fn delete_room(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    Query(query): Query<DeleteQuery>,
) -> Response {
    match controller.delete_room(id, query.force).await {
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        // 还在等节点释放
        Ok(Some(room)) => (StatusCode::ACCEPTED, Json(room)).into_response(),
        Err(error) => error_response(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assign {
    /// `null` 为取消分派
    node_id: Option<i64>,
    /// 不等上一台确认释放
    #[serde(default)]
    force: bool,
}

pub async fn assign_room(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    Json(request): Json<Assign>,
) -> Response {
    respond(controller.assign(id, request.node_id, request.force).await)
}

pub async fn force_release(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
) -> Response {
    match controller.force_release(id).await {
        Ok(Some(room)) => Json(room).into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pause {
    paused: bool,
}

pub async fn pause_room(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    Json(request): Json<Pause>,
) -> Response {
    respond(controller.pause_room(id, request.paused).await)
}

pub async fn list_templates(State(controller): State<Arc<Controller>>) -> Response {
    respond(controller.templates().await)
}

pub async fn create_template(
    State(controller): State<Arc<Controller>>,
    Json(spec): Json<TemplateSpec>,
) -> Response {
    match controller.create_template(spec).await {
        Ok(template) => (StatusCode::CREATED, Json(template)).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn update_template(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    Json(spec): Json<TemplateSpec>,
) -> Response {
    respond(controller.update_template(id, spec).await)
}

pub async fn delete_template(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
) -> Response {
    match controller.delete_template(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn list_accounts(State(controller): State<Arc<Controller>>) -> Response {
    respond(controller.accounts().await)
}
