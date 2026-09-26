//! 控制面的 `/v1/fleet/*` 接口，只在 `--controller` 时注册。
//! 节点列表归 `streamer.view`，生成 / 作废票据与移除节点归 `node.manage`（见 `permissions.rs`）。
//! 房间、投稿模板与节点账号的处理函数在 `fleet_rooms.rs`，Fleet 配置的在 `fleet_config.rs`。

use crate::server::api::access::Caller;
use crate::server::api::fleet_config;
use crate::server::api::fleet_rooms;
use crate::server::errors::{ApiError, report_to_response};
use crate::server::fleet::controller::{CONTROLLER_VERSION, Controller};
use crate::server::fleet::protocol::PROTOCOL_MINOR;
use crate::server::fleet::store;
use crate::server::fleet::ticket::JoinTicket;
use crate::server::fleet::{net, now_ms};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
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
        .route(
            "/v1/fleet/configuration",
            get(fleet_config::get_configuration).put(fleet_config::put_configuration),
        )
        .route(
            "/v1/fleet/configuration/history",
            get(fleet_config::configuration_history),
        )
        .route(
            "/v1/fleet/nodes/{id}/config",
            get(fleet_config::get_node_config).put(fleet_config::put_node_config),
        )
        .with_state(controller)
}

async fn list_nodes(State(controller): State<Arc<Controller>>) -> Response {
    match controller.nodes().await {
        Ok(nodes) => Json(json!({
            "now": now_ms(),
            "controller": controller.endpoint_id().to_string(),
            "controller_version": CONTROLLER_VERSION,
            "controller_proto": PROTOCOL_MINOR,
            "nodes": nodes,
            "removals": controller.removals(),
        }))
        .into_response(),
        Err(e) => report_to_response(e),
    }
}

#[derive(Deserialize, Default)]
struct RevokeQuery {
    /// `auto`：把它的房间按负载改派到其他节点
    reassign: Option<String>,
}

async fn revoke_node(
    State(controller): State<Arc<Controller>>,
    Path(id): Path<i64>,
    Query(query): Query<RevokeQuery>,
) -> Response {
    match query.reassign.as_deref() {
        None => {}
        Some("auto") => return fleet_rooms::revoke_and_reassign(&controller, id).await,
        Some(_) => return (StatusCode::BAD_REQUEST, "reassign 只能是 auto").into_response(),
    }
    if controller.is_removing(id) {
        return (
            StatusCode::CONFLICT,
            Json(ApiError::new(
                "这台节点正在移除：等它确认释放房间后会自动移除".to_string(),
            )),
        )
            .into_response();
    }
    match controller.revoke(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "节点不存在或已被移除").into_response(),
        Err(e) => report_to_response(e),
    }
}

/// 界面一次最多补几个 relay 地址
const MAX_EXTRA_RELAYS: usize = 4;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreateToken {
    /// 有效期（秒），默认 24 小时，允许 1 分钟到 7 天
    ttl_secs: Option<i64>,
    /// 额外写进票据的 relay 地址（「添加节点」弹层里的候选地址）；`--relay-url` 仍排在它们前面
    #[serde(default)]
    extra_relays: Vec<String>,
}

/// 校验界面补充的 relay 地址：只收 http / https、带主机名、没有账号密码 / 查询串 / 片段 / 路径
fn parse_extra_relays(extra: &[String]) -> Result<Vec<url::Url>, String> {
    if extra.len() > MAX_EXTRA_RELAYS {
        return Err(format!("额外的 relay 地址最多 {MAX_EXTRA_RELAYS} 个"));
    }
    extra
        .iter()
        .map(|text| {
            let text = text.trim();
            let url = url::Url::parse(text).map_err(|_| format!("不是有效的地址：{text}"))?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(format!("relay 地址只能是 http:// 或 https://：{text}"));
            }
            if url.host_str().is_none_or(str::is_empty) {
                return Err(format!("relay 地址缺少主机名：{text}"));
            }
            if !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                return Err(format!(
                    "relay 地址只写 协议://主机:端口，不要带路径、参数或账号：{text}"
                ));
            }
            Ok(url)
        })
        .collect()
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
    let extra = match parse_extra_relays(&request.extra_relays) {
        Ok(extra) => extra,
        Err(message) => return (StatusCode::UNPROCESSABLE_ENTITY, message).into_response(),
    };
    let relays = controller.ticket_relays(&extra);
    if relays.is_empty() {
        return (
            StatusCode::CONFLICT,
            "找不到可以写进票据的 relay 地址：本机没有非回环网卡地址，请用 --relay-url 指定，或在「添加节点」里填候选地址",
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
            "relay_port": controller.relay_port(),
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
            (
                Method::GET,
                "/v1/fleet/configuration",
                Permission::ConfigView,
            ),
            (
                Method::GET,
                "/v1/fleet/configuration/history",
                Permission::ConfigView,
            ),
            (
                Method::GET,
                "/v1/fleet/nodes/{id}/config",
                Permission::ConfigView,
            ),
            (
                Method::PUT,
                "/v1/fleet/configuration",
                Permission::NodeManage,
            ),
            (
                Method::PUT,
                "/v1/fleet/nodes/{id}/config",
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
        assert!(Role::Viewer.has(Permission::ConfigView));
    }

    #[tokio::test]
    async fn fleet_config_is_versioned_and_keeps_secrets_out() {
        let dir = tempfile::tempdir().unwrap();
        let relay: url::Url = "http://192.168.7.2:19160/".parse().unwrap();
        let controller = controller_with(
            dir.path(),
            RelaySetup {
                local: vec![relay.clone()],
                advertised: vec![relay],
                embedded_port: None,
            },
        )
        .await;
        let app = router(controller.clone()).route_layer(from_fn(access::unrestricted));

        let (status, initial) = send(&app, Method::GET, "/v1/fleet/configuration").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(initial["version"], 0);
        assert_eq!(initial["saved"], false);
        assert_eq!(initial["config"]["delay"], 300);
        assert!(initial["config"].get("pool1_size").is_none());
        assert!(initial["config"].get("user").is_none());
        assert!(
            initial["per_node_keys"]
                .as_array()
                .unwrap()
                .contains(&json!("ffmpeg_path"))
        );

        // 白名单外带值：整体拒绝，什么都不存
        let (status, error) = send_json(
            &app,
            Method::PUT,
            "/v1/fleet/configuration",
            Some(json!({ "segment_time": "01:00:00", "user": { "bili_cookie": "SESSDATA=x" } })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            error["message"].as_str().unwrap().contains("user"),
            "{error}"
        );
        let (status, error) = send_json(
            &app,
            Method::PUT,
            "/v1/fleet/configuration",
            Some(json!([1])),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");

        // 回传整份（含按节点键与脱敏成 null 的密钥）：按节点键与 null 密钥丢掉并列出
        let mut body = initial["config"].clone();
        body["segment_time"] = json!("01:00:00");
        body["pool1_size"] = json!(9);
        body["kuaishou_cookie"] = serde_json::Value::Null;
        let (status, saved) = send_json(
            &app,
            Method::PUT,
            "/v1/fleet/configuration",
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        assert_eq!(saved["version"], 1);
        assert_eq!(saved["changed"], true);
        assert_eq!(saved["ignored"], json!(["kuaishou_cookie", "pool1_size"]));
        assert_eq!(saved["config"]["segment_time"], "01:00:00");
        assert!(saved["updated_at"].is_i64());

        // 原样再存：不记新版本
        let (status, again) =
            send_json(&app, Method::PUT, "/v1/fleet/configuration", Some(body)).await;
        assert_eq!(
            (status, again["version"].clone()),
            (StatusCode::OK, json!(1))
        );
        assert_eq!(again["changed"], false);

        let (status, history) = send(&app, Method::GET, "/v1/fleet/configuration/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(history["versions"].as_array().unwrap().len(), 1);
        assert_eq!(history["versions"][0]["config"]["segment_time"], "01:00:00");

        // 节点覆盖：不存在的节点 404
        let (status, _) = send(&app, Method::GET, "/v1/fleet/nodes/9/config").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = send_json(
            &app,
            Method::PUT,
            "/v1/fleet/nodes/9/config",
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (token, secret) = store::create_token(controller.pool(), None, 1, i64::MAX)
            .await
            .unwrap();
        let store::Redeem::Joined(node) =
            store::redeem_token(controller.pool(), &token.id, &secret, "aa", "n", false, 2)
                .await
                .unwrap()
        else {
            panic!()
        };
        let uri = format!("/v1/fleet/nodes/{}/config", node.id);
        let (status, error) =
            send_json(&app, Method::PUT, &uri, Some(json!({ "pool1_size": 0 }))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error["message"].as_str().unwrap().contains("pool1_size"));
        let (status, error) = send_json(
            &app,
            Method::PUT,
            &uri,
            Some(json!({ "twitcasting_password": "pw" })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        let (status, saved) = send_json(
            &app,
            Method::PUT,
            &uri,
            Some(json!({ "pool1_size": 2, "segment_time": "", "delay": 30, "user": null })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        assert_eq!(saved["override"], json!({ "pool1_size": 2, "delay": 30 }));
        assert_eq!(saved["ignored"], json!(["user"]));

        let (status, view) = send(&app, Method::GET, &uri).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["override"], json!({ "pool1_size": 2, "delay": 30 }));
        assert_eq!(view["delivered"]["delay"], 30);
        assert_eq!(view["delivered"]["pool1_size"], 2);
        assert_eq!(view["delivered"]["segment_time"], "01:00:00");
        assert_eq!(view["global"]["version"], 1);
        assert_eq!(view["state"]["sync"], serde_json::Value::Null);

        let (_, nodes) = send(&app, Method::GET, "/v1/fleet/nodes").await;
        assert_eq!(nodes["controller_version"], CONTROLLER_VERSION);
        assert_eq!(nodes["controller_proto"], PROTOCOL_MINOR);
        assert_eq!(
            nodes["nodes"][0]["config"]["override_keys"],
            json!(["delay", "pool1_size"])
        );
        controller.shutdown().await;
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

    async fn controller_with(dir: &std::path::Path, relays: RelaySetup) -> Arc<Controller> {
        let pool = ConnectionManager::new_pool_with(
            dir.join("fleet.sqlite3").to_str().unwrap(),
            &FLEET_MIGRATOR,
        )
        .await
        .unwrap();
        let secret = store::identity(&pool, now_ms()).await.unwrap();
        Controller::start(pool, secret, relays, None).await.unwrap()
    }

    fn ticket_relays(created: &serde_json::Value) -> Vec<String> {
        JoinTicket::decode_string(created["ticket"].as_str().unwrap())
            .unwrap()
            .relays
    }

    #[tokio::test]
    async fn extra_relays_are_validated_and_come_after_relay_url() {
        let dir = tempfile::tempdir().unwrap();
        let relay: url::Url = "http://192.168.7.2:19160/".parse().unwrap();
        let controller = controller_with(
            dir.path(),
            RelaySetup {
                local: vec![relay.clone()],
                advertised: vec![relay.clone()],
                embedded_port: None,
            },
        )
        .await;
        let app = router(controller.clone()).route_layer(from_fn(access::unrestricted));
        let issue = |extra: serde_json::Value| {
            send_json(
                &app,
                Method::POST,
                "/v1/fleet/join-tokens",
                Some(json!({ "extra_relays": extra })),
            )
        };

        let (status, created) = issue(json!([
            " http://nas.example:19160 ",
            "https://relay.example",
            "http://192.168.7.2:19160"
        ]))
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let expected = [
            "http://192.168.7.2:19160/",
            "http://nas.example:19160/",
            "https://relay.example/",
        ];
        assert_eq!(ticket_relays(&created), expected);
        assert_eq!(created["relays"], json!(expected));
        assert_eq!(created["private_only"], false);
        assert_eq!(created["relay_port"], serde_json::Value::Null);

        for bad in [
            json!(["ftp://nas.example:19160"]),
            json!(["nas.example:19160"]),
            json!(["http://nas.example:19160/relay"]),
            json!(["http://user:pass@nas.example:19160"]),
            json!(["http://nas.example:19160/?a=1"]),
            json!([
                "http://a/",
                "http://b/",
                "http://c/",
                "http://d/",
                "http://e/"
            ]),
        ] {
            let (status, _) = issue(bad.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        }
        let (_, listed) = send(&app, Method::GET, "/v1/fleet/join-tokens").await;
        assert_eq!(listed["tokens"].as_array().unwrap().len(), 1);
        controller.shutdown().await;
    }

    #[tokio::test]
    async fn without_relay_url_extra_relays_come_first() {
        let dir = tempfile::tempdir().unwrap();
        let local: url::Url = "http://10.0.0.5:19160/".parse().unwrap();
        let controller = controller_with(
            dir.path(),
            RelaySetup {
                local: vec![local.clone()],
                advertised: Vec::new(),
                embedded_port: None,
            },
        )
        .await;
        let app = router(controller.clone()).route_layer(from_fn(access::unrestricted));
        let (status, created) = send_json(
            &app,
            Method::POST,
            "/v1/fleet/join-tokens",
            Some(json!({ "extra_relays": ["http://nas.example.com:19160"] })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            ticket_relays(&created),
            ["http://nas.example.com:19160/", "http://10.0.0.5:19160/"]
        );
        assert_eq!(created["private_only"], false);
        let (_, plain) = send(&app, Method::POST, "/v1/fleet/join-tokens").await;
        assert_eq!(ticket_relays(&plain), ["http://10.0.0.5:19160/"]);
        assert_eq!(plain["private_only"], true);
        controller.shutdown().await;
    }
}
