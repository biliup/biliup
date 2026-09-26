//! 进程内跑一整套：控制面 + 内嵌 relay（127.0.0.1 随机端口）+ 真实的节点代理。

use super::controller::{
    Controller, CreateRoom, DispatchError, NodeView, RelaySetup, Release, Removal, RemovalState,
    RoomStatus,
};
use super::guard::ManagedHandle;
use super::node::{self, NodeAgent};
use super::relay::{EmbeddedRelay, FleetAccess};
use super::ticket::JoinTicket;
use super::{FLEET_MIGRATOR, net, now_ms, store};
use crate::server::config::Config;
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::service_register::ServiceRegister;
use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
use iroh_tickets::Ticket;
use ormlite::Model;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing_subscriber::{EnvFilter, reload};

/// 只认 `https://stuck.example/` 的平台；检测一直不返回，不会向任何真实平台发请求
struct StuckPlatform;

#[async_trait::async_trait]
impl LivePlugin for StuckPlatform {
    fn name(&self) -> &'static str {
        "stuck"
    }

    fn matches(&self, url: &str) -> bool {
        url.starts_with("https://stuck.example/")
    }

    async fn check_stream(&self, _request: LiveRequest) -> LiveResult<LiveStatus> {
        std::future::pending().await
    }
}

async fn node_services(dir: &std::path::Path) -> ServiceRegister {
    std::fs::create_dir_all(dir).unwrap();
    let pool = ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let config = Config::default();
    let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
    managers.add_plugin(Arc::new(StuckPlatform)).await;
    let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
    ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await
}

/// 伪造的凭据文件：只有 `token_info.mid`，登记进节点的库
async fn fake_account(services: &ServiceRegister, dir: &std::path::Path, mid: u64) {
    let path = dir.join(format!("cookies-{mid}.json"));
    std::fs::write(
        &path,
        serde_json::json!({ "token_info": { "mid": mid } }).to_string(),
    )
    .unwrap();
    sqlx::query("INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', ?)")
        .bind(path.to_string_lossy().into_owned())
        .execute(&services.pool)
        .await
        .unwrap();
}

async fn eventually<F, Fut>(what: &str, within: Duration, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + within;
    while !check().await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{what} did not happen within {within:?}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn local_urls(services: &ServiceRegister) -> Vec<String> {
    LiveStreamer::select()
        .fetch_all(&services.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|streamer| streamer.url)
        .collect()
}

async fn start_controller(
    dir: &std::path::Path,
) -> (
    Arc<Controller>,
    url::Url,
    crate::server::infrastructure::connection_pool::ConnectionPool,
) {
    let pool = ConnectionManager::new_pool_with(
        dir.join("fleet.sqlite3").to_str().unwrap(),
        &FLEET_MIGRATOR,
    )
    .await
    .unwrap();
    let secret = store::identity(&pool, now_ms()).await.unwrap();
    let relay = EmbeddedRelay::spawn(
        "127.0.0.1:0".parse().unwrap(),
        FleetAccess::new(secret.public(), pool.clone()),
    )
    .await
    .unwrap();
    let url = net::local_relay_url(relay.addr());
    let setup = RelaySetup {
        local: vec![url.clone()],
        advertised: vec![url.clone()],
        embedded_port: Some(relay.addr().port()),
    };
    let controller = Controller::start(pool.clone(), secret, setup, Some(relay))
        .await
        .unwrap();
    (controller, url, pool)
}

async fn ticket_for(
    controller: &Controller,
    pool: &crate::server::infrastructure::connection_pool::ConnectionPool,
    url: &url::Url,
) -> String {
    let now = now_ms();
    let (token, join_secret) = store::create_token(pool, None, now, now + 60_000)
        .await
        .unwrap();
    JoinTicket {
        controller: controller.endpoint_id(),
        token: token.id.clone(),
        secret: join_secret,
        expires_at: now + 60_000,
        relays: vec![url.to_string()],
    }
    .encode_string()
}

async fn wait_for_node(
    controller: &Controller,
    id: i64,
    online: bool,
    within: Duration,
) -> NodeView {
    let deadline = tokio::time::Instant::now() + within;
    loop {
        let found = controller
            .nodes()
            .await
            .unwrap()
            .into_iter()
            .find(|node| node.id == id);
        if let Some(node) = found
            && node.online == online
            && (!online || node.summary.is_some())
        {
            return node;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "node {id} did not become online={online} within {within:?}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// 等「移除并自动改派」结束，返回结果
async fn wait_removed(controller: &Controller, id: i64, within: Duration) -> Removal {
    let deadline = tokio::time::Instant::now() + within;
    loop {
        let done = controller
            .removals()
            .into_iter()
            .find(|removal| removal.node_id == id && removal.state == RemovalState::Done);
        if let Some(removal) = done {
            return removal;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "removal of node {id} did not finish within {within:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_node_joins_reports_and_is_revoked_through_the_embedded_relay() {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool_with(
        dir.path().join("fleet.sqlite3").to_str().unwrap(),
        &FLEET_MIGRATOR,
    )
    .await
    .unwrap();
    let secret = store::identity(&pool, now_ms()).await.unwrap();
    let relay = EmbeddedRelay::spawn(
        "127.0.0.1:0".parse().unwrap(),
        FleetAccess::new(secret.public(), pool.clone()),
    )
    .await
    .unwrap();
    let url = net::local_relay_url(relay.addr());
    let setup = RelaySetup {
        local: vec![url.clone()],
        advertised: vec![url.clone()],
        embedded_port: Some(relay.addr().port()),
    };
    let controller = Controller::start(pool.clone(), secret, setup, Some(relay))
        .await
        .unwrap();

    let now = now_ms();
    let (token, join_secret) = store::create_token(&pool, None, now, now + 60_000)
        .await
        .unwrap();
    let ticket = JoinTicket {
        controller: controller.endpoint_id(),
        token: token.id.clone(),
        secret: join_secret,
        expires_at: now + 60_000,
        relays: vec![url.to_string()],
    }
    .encode_string();

    let node_file = dir.path().join("node/data/node.json");
    let joined = node::join(&ticket, true, &node_file).await.unwrap();
    assert!(node_file.exists());
    assert_eq!(joined.controller, controller.endpoint_id().to_string());
    assert_eq!(joined.relays, [url.to_string()]);

    // 同一张票据只能用一次：relay 按 token id 就把第二把钥匙挡在外面
    let reused = node::join(&ticket, false, &dir.path().join("other/node.json"))
        .await
        .unwrap_err();
    assert!(format!("{reused:?}").contains("已被使用"), "{reused:?}");
    assert_eq!(store::list_nodes(&pool).await.unwrap().len(), 1);

    let agent = NodeAgent::start(
        node_file.clone(),
        node_services(&dir.path().join("node")).await,
        super::guard::ManagedHandle::default(),
    )
    .await
    .unwrap();
    let online = wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
    assert!(online.allow_hooks);
    let summary = online.summary.unwrap();
    assert_eq!(
        summary.pools.download.capacity,
        Config::default().pool1_size as usize
    );
    assert_eq!(summary.rooms, 0);
    assert_eq!(online.version.as_deref(), Some(env!("CARGO_PKG_VERSION")));

    // 移除：连接当场关闭，节点代理不再重连
    assert!(controller.revoke(joined.node_id).await.unwrap());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !agent.is_finished() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "agent kept running after revoke"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(controller.nodes().await.unwrap().is_empty());
    assert!(!controller.revoke(joined.node_id).await.unwrap());

    agent.shutdown().await;
    controller.shutdown().await;
}

/// 控制面 + 两台节点：分派、迁移（先释放后接手）、硬约束、移除后转本地。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rooms_follow_assignments_across_two_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;

    let mut nodes = Vec::new();
    for (name, hooks) in [("a", false), ("b", false)] {
        let root = dir.path().join(name);
        let node_file = root.join("data/node.json");
        let joined = node::join(
            &ticket_for(&controller, &pool, &url).await,
            hooks,
            &node_file,
        )
        .await
        .unwrap();
        let services = node_services(&root).await;
        if name == "a" {
            fake_account(&services, &root, 42).await;
        }
        let managed = ManagedHandle::default();
        let agent = NodeAgent::start(node_file.clone(), services.clone(), managed.clone())
            .await
            .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, managed, agent, node_file));
    }
    let (a, b) = (nodes[0].0, nodes[1].0);
    // A 上报了账号 42，B 没有
    let accounts = controller.accounts().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!((accounts[0].node_id, accounts[0].mid), (a, 42));

    let template = controller
        .create_template(
            serde_json::from_value(serde_json::json!({
                "template_name": "fleet",
                "account_mid": 42,
                "uploader": "Noop",
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let request = |url: &str, node: i64, template: Option<i64>| -> CreateRoom {
        serde_json::from_value(serde_json::json!({
            "url": url,
            "remark": "房间",
            "template_id": template,
            "node_id": node,
        }))
        .unwrap()
    };
    // 硬约束：B 没有模板的账号；B 没有 --allow-hooks
    let rejected = controller
        .create_room(request("https://stuck.example/1", b, Some(template.id)))
        .await
        .unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("42")),
        "{rejected:?}"
    );
    let mut hooked = request("https://stuck.example/h", a, None);
    hooked.spec.postprocessor =
        serde_json::from_value(serde_json::json!([{ "run": "echo" }])).unwrap();
    let rejected = controller.create_room(hooked).await.unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("--allow-hooks")),
        "{rejected:?}"
    );

    let room = controller
        .create_room(request("https://stuck.example/1", a, Some(template.id)))
        .await
        .unwrap();
    let services_a = nodes[0].1.clone();
    let services_b = nodes[1].1.clone();
    eventually("room lands on A", Duration::from_secs(20), || {
        let services = services_a.clone();
        async move { local_urls(&services).await == ["https://stuck.example/1"] }
    })
    .await;
    eventually("A acks the room", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move {
            let rooms = controller.rooms(true).await.unwrap();
            rooms[0].status == RoomStatus::Monitoring
        }
    })
    .await;
    assert!(nodes[0].2.read().unwrap().as_ref().unwrap().streamers.len() == 1);

    // 迁移不能去没有账号的 B
    assert!(controller.assign(room.id, Some(b), false).await.is_err());
    // 换一个不投稿的房间做迁移。它的后处理只有 rm / mv 这类文件操作，不算钩子，
    // 两台都没带 --allow-hooks 也能收
    let mut plain = request("https://stuck.example/2", a, None);
    plain.spec.postprocessor =
        serde_json::from_value(serde_json::json!(["rm", { "mv": "backup/" }])).unwrap();
    let plain = controller.create_room(plain).await.unwrap();
    eventually("plain room lands on A", Duration::from_secs(20), || {
        let services = services_a.clone();
        async move { local_urls(&services).await.len() == 2 }
    })
    .await;
    let moved = controller.assign(plain.id, Some(b), false).await.unwrap();
    assert_eq!((moved.node_id, moved.releasing_node_id), (Some(b), Some(a)));
    eventually(
        "A releases and B takes over",
        Duration::from_secs(20),
        || {
            let (services_a, services_b) = (services_a.clone(), services_b.clone());
            async move {
                local_urls(&services_a).await == ["https://stuck.example/1"]
                    && local_urls(&services_b).await == ["https://stuck.example/2"]
            }
        },
    )
    .await;
    let moved = super::assignments::room(&pool, plain.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (moved.node_id, moved.releasing_node_id, moved.epoch),
        (Some(b), None, 2)
    );

    // 移除 A：它的房间在控制面变成未分派，在 A 本机转成本地房间接着录
    assert!(controller.revoke(a).await.unwrap());
    eventually("A agent stops", Duration::from_secs(10), || {
        let finished = nodes[0].3.is_finished();
        async move { finished }
    })
    .await;
    assert!(nodes[0].2.read().unwrap().is_none());
    assert!(!super::reconcile::state_path(&nodes[0].4).exists());
    assert_eq!(local_urls(&services_a).await, ["https://stuck.example/1"]);
    assert!(
        services_a
            .managers
            .get_rooms()
            .await
            .iter()
            .any(|worker| worker.live_streamer.url == "https://stuck.example/1")
    );
    let orphan = super::assignments::room(&pool, room.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(orphan.node_id, None);

    for (_, _, _, agent, _) in nodes {
        agent.shutdown().await;
    }
    controller.shutdown().await;
}

/// 按负载自动选节点：硬约束先筛，平局看分到的房间数；移除节点时自动改派。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn automatic_placement_respects_accounts_and_spreads_rooms() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;

    let mut nodes = Vec::new();
    for name in ["a", "b"] {
        let root = dir.path().join(name);
        let node_file = root.join("data/node.json");
        let joined = node::join(
            &ticket_for(&controller, &pool, &url).await,
            false,
            &node_file,
        )
        .await
        .unwrap();
        let services = node_services(&root).await;
        if name == "a" {
            fake_account(&services, &root, 42).await;
        }
        let agent = NodeAgent::start(node_file, services.clone(), ManagedHandle::default())
            .await
            .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, agent));
    }
    let (a, b) = (nodes[0].0, nodes[1].0);
    eventually("A reports its account", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move { controller.accounts().await.unwrap().len() == 1 }
    })
    .await;

    let template = controller
        .create_template(
            serde_json::from_value(serde_json::json!({
                "template_name": "fleet",
                "account_mid": 42,
                "uploader": "Noop",
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let auto = |url: &str, template: Option<i64>| -> CreateRoom {
        serde_json::from_value(serde_json::json!({
            "url": url,
            "remark": "房间",
            "template_id": template,
            "auto_node": true,
        }))
        .unwrap()
    };

    let mut both = auto("https://stuck.example/x", None);
    both.node_id = Some(a);
    assert!(matches!(
        controller.create_room(both).await,
        Err(DispatchError::Invalid(_))
    ));
    // 只有 A 登记了模板账号
    let first = controller
        .create_room(auto("https://stuck.example/1", Some(template.id)))
        .await
        .unwrap();
    assert_eq!(first.node_id, Some(a));
    // 其余都一样时去分到房间少的 B
    let second = controller
        .create_room(auto("https://stuck.example/2", None))
        .await
        .unwrap();
    assert_eq!(second.node_id, Some(b));
    let mut hooked = auto("https://stuck.example/h", None);
    hooked.spec.postprocessor =
        serde_json::from_value(serde_json::json!([{ "run": "echo" }])).unwrap();
    let rejected = controller.create_room(hooked).await.unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("不允许钩子")),
        "{rejected:?}"
    );

    // 移除在线的 B 并自动改派：先迁移，B 确认释放后 A 才接手，然后才吊销 B
    let started = controller.revoke_and_reassign(b).await.unwrap().unwrap();
    assert_eq!(started.state, RemovalState::Removing);
    assert_eq!(
        (started.rooms[0].room_id, started.rooms[0].node_id),
        (second.id, Some(a))
    );
    let done = wait_removed(&controller, b, Duration::from_secs(20)).await;
    assert_eq!(done.rooms.len(), 1);
    assert_eq!(done.rooms[0].release, Release::Released);
    assert!(done.finished_at.unwrap() <= started.deadline);
    let services_a = nodes[0].1.clone();
    let services_b = nodes[1].1.clone();
    eventually("A records both rooms", Duration::from_secs(20), || {
        let services = services_a.clone();
        async move { local_urls(&services).await.len() == 2 }
    })
    .await;
    // B 交出了房间，吊销后本机也没留下它
    assert!(local_urls(&services_b).await.is_empty());
    eventually("B agent stops", Duration::from_secs(10), || {
        let finished = nodes[1].2.is_finished();
        async move { finished }
    })
    .await;
    assert!(controller.revoke_and_reassign(b).await.unwrap().is_none());

    // A 离线时移除 A：当场吊销；没有节点可去，房间留在未分派
    let (_, _, agent_a) = nodes.remove(0);
    agent_a.shutdown().await;
    wait_for_node(&controller, a, false, Duration::from_secs(10)).await;
    let done = controller.revoke_and_reassign(a).await.unwrap().unwrap();
    assert_eq!(done.state, RemovalState::Done);
    assert_eq!(done.rooms.len(), 2);
    for room in &done.rooms {
        assert_eq!(room.node_id, None);
        assert!(
            room.unplaced.as_deref().unwrap().contains("没有节点"),
            "{room:?}"
        );
        assert_eq!(room.release, Release::Offline);
    }
    assert!(controller.revoke_and_reassign(a).await.unwrap().is_none());
    assert!(controller.nodes().await.unwrap().is_empty());

    for (_, _, agent) in nodes {
        agent.shutdown().await;
    }
    controller.shutdown().await;
}

/// 控制面 + 两台节点：全局配置两台都生效，覆盖只动一台，本机密钥不出节点、不进控制面的库。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn layered_config_reaches_nodes_without_their_secrets() {
    use crate::server::api::access;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let app = crate::server::api::fleet::router(controller.clone())
        .route_layer(axum::middleware::from_fn(access::unrestricted));
    let send = |method: Method, uri: String, body: serde_json::Value| {
        let app = app.clone();
        async move {
            let response = app
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
                serde_json::from_slice::<serde_json::Value>(&bytes)
                    .unwrap_or(serde_json::Value::Null),
            )
        }
    };

    let mut nodes = Vec::new();
    for name in ["a", "b"] {
        let root = dir.path().join(name);
        let node_file = root.join("data/node.json");
        let joined = node::join(
            &ticket_for(&controller, &pool, &url).await,
            false,
            &node_file,
        )
        .await
        .unwrap();
        let services = node_services(&root).await;
        {
            let mut config = services.config.write().unwrap();
            config.kuaishou_cookie = Some(format!("ks-secret-{name}"));
            config.pool1_size = 2;
        }
        let managed = ManagedHandle::default();
        let agent = NodeAgent::start(node_file, services.clone(), managed.clone())
            .await
            .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, managed, agent));
    }
    let (a, b) = (nodes[0].0, nodes[1].0);
    let config_of = |index: usize| nodes[index].1.config.read().unwrap().clone();
    let sync_of = |id: i64| {
        let controller = controller.clone();
        async move {
            controller
                .nodes()
                .await
                .unwrap()
                .into_iter()
                .find(|node| node.id == id)
                .unwrap()
                .config
        }
    };

    // 连上就托管配置；控制面还没存全局配置，本机配置不变
    eventually(
        "both nodes report config applied",
        Duration::from_secs(20),
        || async {
            sync_of(a).await.sync == Some("applied") && sync_of(b).await.sync == Some("applied")
        },
    )
    .await;
    assert!(
        nodes[0]
            .2
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .config
            .is_some()
    );
    assert_eq!(config_of(0).pool1_size, 2);

    let (status, saved) = send(
        Method::PUT,
        "/v1/fleet/configuration".into(),
        serde_json::json!({
            "segment_time": "01:00:00",
            "filename_prefix": "{streamer}%Y-%m-%d",
            "pool1_size": 9,
            "kuaishou_cookie": null,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        saved["ignored"],
        serde_json::json!(["kuaishou_cookie", "pool1_size"])
    );
    eventually(
        "global config applied on both nodes",
        Duration::from_secs(20),
        || async {
            [0, 1].iter().all(|index| {
                let config = config_of(*index);
                config.segment_time.as_deref() == Some("01:00:00")
                    && config.filename_prefix.as_deref() == Some("{streamer}%Y-%m-%d")
            })
        },
    )
    .await;
    // 按节点的键全局不管
    assert_eq!(config_of(0).pool1_size, 2);

    let (status, error) = send(
        Method::PUT,
        format!("/v1/fleet/nodes/{a}/config"),
        serde_json::json!({ "pool1_size": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(error["message"].as_str().unwrap().contains("pool1_size"));
    let (status, _) = send(
        Method::PUT,
        format!("/v1/fleet/nodes/{a}/config"),
        serde_json::json!({ "pool1_size": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    eventually(
        "override applied on node a",
        Duration::from_secs(20),
        || async { nodes[0].1.managers.download_pool_size() == 1 },
    )
    .await;
    eventually(
        "node a acknowledges the override",
        Duration::from_secs(20),
        || async { sync_of(a).await.sync == Some("applied") },
    )
    .await;
    assert_eq!(nodes[1].1.managers.download_pool_size(), 2);
    assert_eq!(sync_of(a).await.override_keys, ["pool1_size"]);
    assert!(sync_of(b).await.override_keys.is_empty());

    // 密钥留在各自节点上，控制面的库里没有
    assert_eq!(config_of(0).kuaishou_cookie.as_deref(), Some("ks-secret-a"));
    assert_eq!(config_of(1).kuaishou_cookie.as_deref(), Some("ks-secret-b"));
    let (status, rejected) = send(
        Method::PUT,
        "/v1/fleet/configuration".into(),
        serde_json::json!({ "kuaishou_cookie": "ks-secret-controller" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{rejected}");
    let (_, node_view) = send(
        Method::GET,
        format!("/v1/fleet/nodes/{a}/config"),
        serde_json::Value::Null,
    )
    .await;
    assert_eq!(node_view["delivered"]["pool1_size"], 1);
    assert_eq!(node_view["delivered"]["segment_time"], "01:00:00");
    for file in ["fleet.sqlite3", "fleet.sqlite3-wal"] {
        let path = dir.path().join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("ks-secret"), "{file} contains a node secret");
        }
    }

    for (_, _, _, agent) in nodes {
        agent.shutdown().await;
    }
    controller.shutdown().await;
}
