//! 控制面的 `/v1/fleet/*` 接口，只在 `--controller` 时注册。
//! 节点列表归 `streamer.view`，生成 / 作废票据与移除节点归 `node.manage`（见 `permissions.rs`）。
//! 房间、投稿模板与节点账号的处理函数在 `fleet_rooms.rs`。

use crate::server::api::access::Caller;
use crate::server::api::fleet_rooms;
use crate::server::errors::report_to_response;
use crate::server::fleet::controller::Controller;
use crate::server::fleet::store;
use crate::server::fleet::ticket::JoinTicket;
use crate::server::fleet::{net, now_ms};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

const DEFAULT_TTL_SECS: i64 = 24 * 60 * 60;
const MIN_TTL_SECS: i64 = 60;
const MAX_TTL_SECS: i64 = 7 * 24 * 60 * 60;

pub fn router(controller: Arc<Controller>) -> Router<()> {
    Router::new()
        .route("/v1/fleet/nodes", get(list_nodes))
        .route("/v1/fleet/nodes/{id}", delete(revoke_node))
        .route("/v1/fleet/join-tokens", get(list_tokens).post(create_token))
        .route("/v1/fleet/join-tokens/{id}", delete(delete_token))
        .route(
            "/v1/fleet/rooms",
            get(fleet_rooms::list_rooms).post(fleet_rooms::create_room),
        )
        .route(
            "/v1/fleet/rooms/{id}",
            put(fleet_rooms::update_room).delete(fleet_rooms::delete_room),
        )
        .route(
            "/v1/fleet/rooms/{id}/assign",
            post(fleet_rooms::assign_room),
        )
        .route(
            "/v1/fleet/rooms/{id}/force",
            post(fleet_rooms::force_release),
        )
        .route("/v1/fleet/rooms/{id}/pause", post(fleet_rooms::pause_room))
        .route(
            "/v1/fleet/templates",
            get(fleet_rooms::list_templates).post(fleet_rooms::create_template),
        )
        .route(
            "/v1/fleet/templates/{id}",
            put(fleet_rooms::update_template).delete(fleet_rooms::delete_template),
        )
        .route("/v1/fleet/accounts", get(fleet_rooms::list_accounts))
        .with_state(controller)
}

async fn list_nodes(State(controller): State<Arc<Controller>>) -> Response {
    match controller.nodes().await {
        Ok(nodes) => Json(json!({
            "now": now_ms(),
            "controller": controller.endpoint_id().to_string(),
            "nodes": nodes,
        }))
        .into_response(),
        Err(e) => report_to_response(e),
    }
}

async fn revoke_node(State(controller): State<Arc<Controller>>, Path(id): Path<i64>) -> Response {
    match controller.revoke(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "节点不存在或已被移除").into_response(),
        Err(e) => report_to_response(e),
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreateToken {
    /// 有效期（秒），默认 24 小时，允许 1 分钟到 7 天
    ttl_secs: Option<i64>,
}

async fn create_token(
    State(controller): State<Arc<Controller>>,
    caller: Caller,
    body: Bytes,
) -> Response {
    let request: CreateToken = if body.iter().all(u8::is_ascii_whitespace) {
        CreateToken::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(e) => return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response(),
        }
    };
    let ttl = request.ttl_secs.unwrap_or(DEFAULT_TTL_SECS);
    if !(MIN_TTL_SECS..=MAX_TTL_SECS).contains(&ttl) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "有效期必须在 1 分钟到 7 天之间",
        )
            .into_response();
    }
    let relays = controller.advertised_relays();
    if relays.is_empty() {
        return (
            StatusCode::CONFLICT,
            "找不到可以写进票据的 relay 地址：本机没有非回环网卡地址，请用 --relay-url 指定",
        )
            .into_response();
    }
    let now = now_ms();
    let (token, secret) = match store::create_token(
        controller.pool(),
        caller.subject.user_id,
        now,
        now + ttl * 1000,
    )
    .await
    {
        Ok(created) => created,
        Err(e) => return report_to_response(e),
    };
    let relay_strings: Vec<String> = relays.iter().map(|url| url.to_string()).collect();
    let ticket = JoinTicket {
        controller: controller.endpoint_id(),
        token: token.id.clone(),
        secret,
        expires_at: token.expires_at,
        relays: relay_strings.clone(),
    }
    .encode_string();
    let command = format!("biliup node join {ticket}");
    (
        StatusCode::CREATED,
        Json(json!({
            "id": token.id,
            "created_at": token.created_at,
            "expires_at": token.expires_at,
            "ticket": ticket,
            "command": command,
            "docker_command": format!("docker exec <容器名> {command}"),
            "docker_env": format!("BILIUP_JOIN_TICKET={ticket}"),
            "relays": relay_strings,
            "private_only": net::only_private(&relays),
        })),
    )
        .into_response()
}

async fn list_tokens(State(controller): State<Arc<Controller>>) -> Response {
    let now = now_ms();
    match store::list_tokens(controller.pool()).await {
        Ok(tokens) => {
            let tokens: Vec<_> = tokens
                .into_iter()
                .map(|token| {
                    let state = if token.used_at.is_some() {
                        "used"
                    } else if token.expires_at <= now {
                        "expired"
                    } else {
                        "active"
                    };
                    json!({
                        "id": token.id,
                        "created_by": token.created_by,
                        "created_at": token.created_at,
                        "expires_at": token.expires_at,
                        "used_at": token.used_at,
                        "used_by_node": token.used_by_node,
                        "state": state,
                    })
                })
                .collect();
            Json(json!({ "now": now, "tokens": tokens })).into_response()
        }
        Err(e) => report_to_response(e),
    }
}

async fn delete_token(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<String>,
) -> Response {
    match store::delete_token(controller.pool(), &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "票据不存在").into_response(),
        Err(e) => report_to_response(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::access;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::fleet::controller::RelaySetup;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::permissions::{Permission, Role, required_permission};
    use axum::body::Body;
    use axum::http::{Method, Request};
    use axum::middleware::from_fn;
    use tower::ServiceExt;

    #[test]
    fn listing_is_for_everyone_and_management_is_admin_only() {
        let cases = [
            (Method::GET, "/v1/fleet/nodes", Permission::StreamerView),
            (
                Method::DELETE,
                "/v1/fleet/nodes/{id}",
                Permission::NodeManage,
            ),
            (Method::GET, "/v1/fleet/join-tokens", Permission::NodeManage),
            (
                Method::POST,
                "/v1/fleet/join-tokens",
                Permission::NodeManage,
            ),
            (
                Method::DELETE,
                "/v1/fleet/join-tokens/{id}",
                Permission::NodeManage,
            ),
            (Method::GET, "/v1/fleet/rooms", Permission::StreamerView),
            (Method::GET, "/v1/fleet/templates", Permission::StreamerView),
            (Method::GET, "/v1/fleet/accounts", Permission::StreamerView),
            (Method::POST, "/v1/fleet/rooms", Permission::NodeManage),
            (Method::PUT, "/v1/fleet/rooms/{id}", Permission::NodeManage),
            (
                Method::DELETE,
                "/v1/fleet/rooms/{id}",
                Permission::NodeManage,
            ),
            (
                Method::POST,
                "/v1/fleet/rooms/{id}/assign",
                Permission::NodeManage,
            ),
            (
                Method::POST,
                "/v1/fleet/rooms/{id}/force",
                Permission::NodeManage,
            ),
            (
                Method::POST,
                "/v1/fleet/rooms/{id}/pause",
                Permission::NodeManage,
            ),
            (Method::POST, "/v1/fleet/templates", Permission::NodeManage),
            (
                Method::PUT,
                "/v1/fleet/templates/{id}",
                Permission::NodeManage,
            ),
            (
                Method::DELETE,
                "/v1/fleet/templates/{id}",
                Permission::NodeManage,
            ),
        ];
        for (method, route, permission) in cases {
            assert_eq!(
                required_permission(&method, route, route),
                Some(permission),
                "{method} {route}"
            );
        }
        assert!(Role::Admin.has(Permission::NodeManage));
        assert!(!Role::Operator.has(Permission::NodeManage));
        assert!(!Role::Viewer.has(Permission::NodeManage));
    }

    async fn send(app: &Router<()>, method: Method, uri: &str) -> (StatusCode, serde_json::Value) {
        send_json(app, method, uri, None).await
    }

    async fn send_json(
        app: &Router<()>,
        method: Method,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn tickets_are_issued_listed_without_secrets_and_voided() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool_with(
            dir.path().join("fleet.sqlite3").to_str().unwrap(),
            &FLEET_MIGRATOR,
        )
        .await
        .unwrap();
        let secret = store::identity(&pool, now_ms()).await.unwrap();
        let relay: url::Url = "http://192.168.7.2:19160/".parse().unwrap();
        let controller = Controller::start(
            pool,
            secret,
            RelaySetup {
                local: vec![relay.clone()],
                advertised: vec![relay.clone()],
                embedded_port: None,
            },
            None,
        )
        .await
        .unwrap();
        let app = router(controller.clone()).route_layer(from_fn(access::unrestricted));

        let (status, created) = send(&app, Method::POST, "/v1/fleet/join-tokens").await;
        assert_eq!(status, StatusCode::CREATED);
        let ticket = JoinTicket::decode_string(created["ticket"].as_str().unwrap()).unwrap();
        assert_eq!(ticket.controller, controller.endpoint_id());
        assert_eq!(ticket.token, created["id"].as_str().unwrap());
        assert_eq!(ticket.relays, [relay.to_string()]);
        assert_eq!(created["private_only"], true);
        let ttl = created["expires_at"].as_i64().unwrap() - created["created_at"].as_i64().unwrap();
        assert_eq!(ttl, DEFAULT_TTL_SECS * 1000);
        assert!(
            created["command"]
                .as_str()
                .unwrap()
                .starts_with("biliup node join bfleet")
        );

        let (status, listed) = send(&app, Method::GET, "/v1/fleet/join-tokens").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(listed["tokens"][0]["state"], "active");
        let listed_text = listed.to_string();
        assert!(!listed_text.contains("secret"));
        assert!(!listed_text.contains(created["ticket"].as_str().unwrap()));

        let id = created["id"].as_str().unwrap();
        let uri = format!("/v1/fleet/join-tokens/{id}");
        assert_eq!(
            send(&app, Method::DELETE, &uri).await.0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            send(&app, Method::DELETE, &uri).await.0,
            StatusCode::NOT_FOUND
        );

        let (status, nodes) = send(&app, Method::GET, "/v1/fleet/nodes").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(nodes["nodes"], serde_json::json!([]));
        assert_eq!(
            send(&app, Method::DELETE, "/v1/fleet/nodes/1").await.0,
            StatusCode::NOT_FOUND
        );
        controller.shutdown().await;
    }

    #[tokio::test]
    async fn rooms_and_templates_are_managed_over_http() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool_with(
            dir.path().join("fleet.sqlite3").to_str().unwrap(),
            &FLEET_MIGRATOR,
        )
        .await
        .unwrap();
        let secret = store::identity(&pool, now_ms()).await.unwrap();
        let relay: url::Url = "http://192.168.7.2:19160/".parse().unwrap();
        let controller = Controller::start(
            pool,
            secret,
            RelaySetup {
                local: vec![relay.clone()],
                advertised: vec![relay],
                embedded_port: None,
            },
            None,
        )
        .await
        .unwrap();
        let app = router(controller.clone()).route_layer(from_fn(access::unrestricted));

        let (status, template) = send_json(
            &app,
            Method::POST,
            "/v1/fleet/templates",
            Some(json!({ "template_name": "t", "account_mid": 42, "user_cookie": "x.json" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(template.get("user_cookie").is_none());
        let template_id = template["id"].as_i64().unwrap();

        let room =
            json!({ "url": "https://live.example/1", "remark": "r", "template_id": template_id });
        let (status, created) =
            send_json(&app, Method::POST, "/v1/fleet/rooms", Some(room.clone())).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created["node_id"], serde_json::Value::Null);
        let (status, error) = send_json(&app, Method::POST, "/v1/fleet/rooms", Some(room)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{error}");
        let (status, error) = send_json(
            &app,
            Method::POST,
            "/v1/fleet/rooms",
            Some(json!({ "url": "https://live.example/2", "remark": "r", "node_id": 9 })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error["message"].as_str().unwrap().contains("节点 9"));

        let (status, listed) = send(&app, Method::GET, "/v1/fleet/rooms").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(listed["rooms"][0]["status"], "unassigned");
        let id = created["id"].as_i64().unwrap();
        let (status, paused) = send_json(
            &app,
            Method::POST,
            &format!("/v1/fleet/rooms/{id}/pause"),
            Some(json!({ "paused": true })),
        )
        .await;
        assert_eq!(
            (status, paused["paused"].clone()),
            (StatusCode::OK, json!(true))
        );
        let (status, _) = send(
            &app,
            Method::DELETE,
            &format!("/v1/fleet/templates/{template_id}"),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            send(&app, Method::DELETE, &format!("/v1/fleet/rooms/{id}"))
                .await
                .0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            send(
                &app,
                Method::DELETE,
                &format!("/v1/fleet/templates/{template_id}")
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
        let (status, accounts) = send(&app, Method::GET, "/v1/fleet/accounts").await;
        assert_eq!((status, accounts), (StatusCode::OK, json!([])));
        controller.shutdown().await;
    }
}
