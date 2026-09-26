//! 进程内跑一整套：控制面 + 内嵌 relay（127.0.0.1 随机端口）+ 真实的节点代理。

use super::controller::{
    Controller, CreateRoom, DispatchError, NodeView, RelaySetup, Release, Removal, RemovalState,
    RoomStatus,
};
use super::guard::ManagedHandle;
use super::local::LocalNode;
use super::node::{self, NodeAgent};
use super::relay::{EmbeddedRelay, FleetAccess};
use super::revoked::{Revoked, RevokedHandle, revoked_path};
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

/// 节点进程的被移除清单：与 `fleet::start` 一样放在 `node.json` 旁边
fn revoked_for(node_file: &std::path::Path, services: &ServiceRegister) -> RevokedHandle {
    Arc::new(Revoked::load(revoked_path(node_file), services.clone()))
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

    let services = node_services(&dir.path().join("node")).await;
    let agent = NodeAgent::start(
        node_file.clone(),
        services.clone(),
        super::guard::ManagedHandle::default(),
        revoked_for(&node_file, &services),
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

/// 控制面 + 两台节点：分派、迁移（先释放后接手）、硬约束、移除后转本地并暂停、离开后转本地接着录。
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
        let revoked = revoked_for(&node_file, &services);
        let agent = NodeAgent::start(
            node_file.clone(),
            services.clone(),
            managed.clone(),
            revoked.clone(),
        )
        .await
        .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, managed, agent, node_file, revoked));
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

    // 移除 A：它的房间在控制面变成未分派，在 A 本机转成本地房间并暂停，记进被移除清单
    assert!(!revoked_path(&nodes[0].4).exists());
    assert!(controller.revoke(a).await.unwrap());
    eventually("A agent stops", Duration::from_secs(10), || {
        let finished = nodes[0].3.is_finished();
        async move { finished }
    })
    .await;
    assert!(nodes[0].2.read().unwrap().is_none());
    assert!(!super::reconcile::state_path(&nodes[0].4).exists());
    assert_eq!(local_urls(&services_a).await, ["https://stuck.example/1"]);
    let worker = services_a
        .managers
        .get_rooms()
        .await
        .into_iter()
        .find(|worker| worker.live_streamer.url == "https://stuck.example/1")
        .unwrap();
    assert!(matches!(
        *worker.downloader_status.read().unwrap(),
        crate::server::infrastructure::context::WorkerStatus::Pause
    ));
    assert_eq!(nodes[0].5.ids(), [worker.live_streamer.id]);
    assert!(revoked_path(&nodes[0].4).exists());
    let orphan = super::assignments::room(&pool, room.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(orphan.node_id, None);

    // B 的 node.json 已经没了（`biliup node leave` 正在进行）时收到吊销：是主动离开，转本地接着录
    std::fs::remove_file(&nodes[1].4).unwrap();
    assert!(controller.revoke(b).await.unwrap());
    eventually("B agent stops", Duration::from_secs(10), || {
        let finished = nodes[1].3.is_finished();
        async move { finished }
    })
    .await;
    assert_eq!(local_urls(&services_b).await, ["https://stuck.example/2"]);
    let worker = services_b
        .managers
        .get_rooms()
        .await
        .into_iter()
        .find(|worker| worker.live_streamer.url == "https://stuck.example/2")
        .unwrap();
    assert!(!matches!(
        *worker.downloader_status.read().unwrap(),
        crate::server::infrastructure::context::WorkerStatus::Pause
    ));
    assert!(nodes[1].5.is_empty());
    assert!(!revoked_path(&nodes[1].4).exists());

    for (_, _, _, agent, _, _) in nodes {
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
        let revoked = revoked_for(&node_file, &services);
        let agent = NodeAgent::start(
            node_file,
            services.clone(),
            ManagedHandle::default(),
            revoked,
        )
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

    // 标签：只有 B 带「海外」。不看标签时平局取 id 小的 A，要求「海外」就只能去 B
    controller
        .set_node_labels(b, &["海外".to_string()])
        .await
        .unwrap();
    let mut abroad = auto("https://stuck.example/abroad", None);
    abroad.required_labels = vec!["海外".into()];
    let abroad = controller.create_room(abroad).await.unwrap();
    assert_eq!(abroad.node_id, Some(b));
    let mut pinned = auto("https://stuck.example/pinned", None);
    pinned.auto_node = false;
    pinned.node_id = Some(a);
    pinned.required_labels = vec!["海外".into()];
    let rejected = controller.create_room(pinned).await.unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("缺少房间要求的标签「海外」")),
        "{rejected:?}"
    );
    let rejected = controller
        .assign(abroad.id, Some(a), false)
        .await
        .unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("缺少房间要求的标签「海外」")),
        "{rejected:?}"
    );
    let mut nowhere = auto("https://stuck.example/nowhere", Some(template.id));
    nowhere.required_labels = vec!["海外".into()];
    let rejected = controller.create_room(nowhere).await.unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m)
            if m.contains("」缺少标签「海外」") && m.contains("」没有登记 B 站账号 42")),
        "{rejected:?}"
    );
    // 摘掉 B 的标签：房间不挪，只标出来
    controller.set_node_labels(b, &[]).await.unwrap();
    let rooms = controller.rooms(true).await.unwrap();
    let view = rooms.iter().find(|room| room.room.id == abroad.id).unwrap();
    assert_eq!(view.room.node_id, Some(b));
    assert_eq!(view.labels_missing, ["海外"]);
    assert!(
        rooms
            .iter()
            .filter(|room| room.room.id != abroad.id)
            .all(|room| room.labels_missing.is_empty())
    );
    assert!(
        controller
            .delete_room(abroad.id, true)
            .await
            .unwrap()
            .is_none()
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
        let revoked = revoked_for(&node_file, &services);
        let agent = NodeAgent::start(node_file, services.clone(), managed.clone(), revoked)
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

/// 告警：节点上报的事件变成告警、同一个直播间合并计数；房间落地失败与配置应用失败随节点的 Ack 出现和恢复；离线告警出现、重连后恢复；「知道了」清掉。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn alerts_follow_node_events_and_connectivity() {
    use super::alerts::AlertKind;
    use super::controller::OFFLINE_ALERT_AFTER_MS;
    use super::events;
    use super::protocol::{EVENT_RECORDING_ERROR, EVENT_UPLOAD_FAILED, Event, RoomEvent};

    let _guard = events::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    controller.stop_alert_loop();
    let root = dir.path().join("a");
    let node_file = root.join("data/node.json");
    let joined = node::join(
        &ticket_for(&controller, &pool, &url).await,
        false,
        &node_file,
    )
    .await
    .unwrap();
    let id = joined.node_id;
    let services = node_services(&root).await;
    let start = || {
        NodeAgent::start(
            node_file.clone(),
            services.clone(),
            ManagedHandle::default(),
            revoked_for(&node_file, &services),
        )
    };
    let agent = start().await.unwrap();
    wait_for_node(&controller, id, true, Duration::from_secs(30)).await;

    let room_url = "https://stuck.example/alerts";
    let room = controller
        .create_room(
            serde_json::from_value(serde_json::json!({ "url": room_url, "remark": "房间" }))
                .unwrap(),
        )
        .await
        .unwrap();
    let event = |kind: &str, error: &str| Event {
        kind: kind.into(),
        at: now_ms(),
        detail: serde_json::to_value(RoomEvent {
            url: room_url.into(),
            remark: "房间".into(),
            error: error.into(),
        })
        .unwrap(),
    };
    events::inject(event(EVENT_RECORDING_ERROR, "mesio error: boom"));
    events::inject(event(EVENT_RECORDING_ERROR, "mesio error: again"));
    events::inject(event(EVENT_UPLOAD_FAILED, "open cookies file: x.json"));
    events::inject(event("from_the_future", "ignored"));
    eventually("events become alerts", Duration::from_secs(10), || {
        let controller = controller.clone();
        async move {
            let alerts = controller.alert_list().alerts;
            alerts.len() == 2 && alerts.iter().map(|alert| alert.count).sum::<u32>() == 3
        }
    })
    .await;
    let alerts = controller.alert_list().alerts;
    let recording = alerts
        .iter()
        .find(|alert| alert.kind == AlertKind::RecordingError)
        .unwrap();
    assert_eq!(recording.node_id, id);
    assert_eq!(recording.count, 2);
    assert_eq!(recording.message, "mesio error: again");
    assert_eq!(recording.room_id, Some(room.id));
    assert_eq!(recording.room.as_deref(), Some("房间"));
    assert_eq!(recording.url.as_deref(), Some(room_url));
    let upload = alerts
        .iter()
        .find(|alert| alert.kind == AlertKind::UploadFailed)
        .unwrap();
    assert_eq!(upload.count, 1);
    let summary = controller.summary().await.unwrap();
    assert_eq!(
        (
            summary.nodes_total,
            summary.nodes_online,
            summary.alerts,
            summary.alerts_open
        ),
        (1, 1, 2, 2)
    );
    let open_alert = |kind: AlertKind| {
        let controller = controller.clone();
        async move {
            controller.evaluate_alerts_at(now_ms()).await;
            controller
                .alert_list()
                .alerts
                .into_iter()
                .find(|alert| alert.kind == kind && alert.is_open())
        }
    };

    // 房间落地失败：节点上已有同一地址的本地主播，节点拒收并在 Ack 里说明；撤回分派后恢复
    let taken = "https://stuck.example/taken";
    crate::server::services::streamers::add_streamer(
        &services,
        serde_json::from_value(serde_json::json!({ "url": taken, "remark": "本地" })).unwrap(),
    )
    .await
    .unwrap();
    let clash = controller
        .create_room(
            serde_json::from_value(
                serde_json::json!({ "url": taken, "remark": "撞车", "node_id": id }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    eventually("room failure alert", Duration::from_secs(20), || async {
        open_alert(AlertKind::RoomFailed).await.is_some()
    })
    .await;
    let failed = open_alert(AlertKind::RoomFailed).await.unwrap();
    assert_eq!(failed.room_id, Some(clash.id));
    assert_eq!(failed.room.as_deref(), Some("撞车"));
    assert!(failed.message.contains("本地主播"), "{}", failed.message);
    controller.assign(clash.id, None, false).await.unwrap();
    eventually("room failure resolves", Duration::from_secs(20), || async {
        open_alert(AlertKind::RoomFailed).await.is_none()
    })
    .await;

    // 配置应用失败：节点写不进自己的库，保持原配置并在 Ack 里报原因；库恢复后再下发一次就好了
    for event in ["INSERT", "UPDATE"] {
        sqlx::query(&format!(
            "CREATE TRIGGER no_config_{event} BEFORE {event} ON configuration \
             WHEN NEW.key = 'config' BEGIN SELECT RAISE(ABORT, 'disk is full'); END"
        ))
        .execute(&services.pool)
        .await
        .unwrap();
    }
    let set_pool1 = |size: u32| {
        let controller = controller.clone();
        async move {
            let patch = serde_json::json!({ "pool1_size": size });
            super::config_store::set_node_override(
                controller.pool(),
                id,
                patch.as_object().unwrap(),
            )
            .await
            .unwrap();
            controller.push(id).await;
        }
    };
    set_pool1(3).await;
    eventually("config failure alert", Duration::from_secs(20), || async {
        open_alert(AlertKind::ConfigFailed).await.is_some()
    })
    .await;
    let failed = open_alert(AlertKind::ConfigFailed).await.unwrap();
    assert!(
        failed.message.contains("disk is full"),
        "{}",
        failed.message
    );
    assert_ne!(services.config.read().unwrap().pool1_size, 3);
    for event in ["INSERT", "UPDATE"] {
        sqlx::query(&format!("DROP TRIGGER no_config_{event}"))
            .execute(&services.pool)
            .await
            .unwrap();
    }
    set_pool1(4).await;
    eventually(
        "config failure resolves",
        Duration::from_secs(20),
        || async { open_alert(AlertKind::ConfigFailed).await.is_none() },
    )
    .await;
    assert_eq!(services.config.read().unwrap().pool1_size, 4);
    assert_eq!(
        controller
            .alert_list()
            .alerts
            .iter()
            .filter(|alert| alert.resolved_at.is_some())
            .count(),
        2
    );

    // 离线：60 s 之内不算，之后告警；重连后恢复
    agent.shutdown().await;
    wait_for_node(&controller, id, false, Duration::from_secs(10)).await;
    controller.evaluate_alerts_at(now_ms()).await;
    assert_eq!(controller.alert_list().alerts.len(), 4);
    let later = now_ms() + OFFLINE_ALERT_AFTER_MS + 1_000;
    controller.evaluate_alerts_at(later).await;
    let offline = controller
        .alert_list()
        .alerts
        .into_iter()
        .find(|alert| alert.kind == AlertKind::NodeOffline)
        .expect("offline alert");
    assert!(offline.is_open());
    assert!(offline.first_at <= now_ms());
    assert_eq!(controller.summary().await.unwrap().nodes_online, 0);

    let agent = start().await.unwrap();
    wait_for_node(&controller, id, true, Duration::from_secs(30)).await;
    controller.evaluate_alerts_at(now_ms()).await;
    let alerts = controller.alert_list().alerts;
    let offline = alerts
        .iter()
        .find(|alert| alert.kind == AlertKind::NodeOffline)
        .unwrap();
    assert!(offline.resolved_at.is_some());
    // 事件类不因重连恢复
    assert_eq!(alerts.iter().filter(|alert| alert.is_open()).count(), 2);

    assert!(controller.acknowledge_alert(upload.id));
    assert!(!controller.acknowledge_alert(upload.id));
    assert_eq!(controller.acknowledge_alerts(), 4);
    assert!(controller.alert_list().alerts.is_empty());

    agent.shutdown().await;
    controller.shutdown().await;
}

fn worker_paused(worker: &crate::server::infrastructure::context::Worker) -> bool {
    matches!(
        *worker.downloader_status.read().unwrap(),
        crate::server::infrastructure::context::WorkerStatus::Pause
    )
}

/// 控制面的「本机」节点：数据库与录制就是控制面自己的
struct LocalFixture {
    services: ServiceRegister,
    file: std::path::PathBuf,
    managed: ManagedHandle,
    revoked: RevokedHandle,
}

impl LocalFixture {
    async fn new(dir: &std::path::Path) -> Self {
        let root = dir.join("controller");
        let file = root.join("data/local-node.json");
        let services = node_services(&root).await;
        let revoked = revoked_for(&file, &services);
        LocalFixture {
            services,
            file,
            managed: ManagedHandle::default(),
            revoked,
        }
    }

    /// 与 `fleet::start` 一样：新建、按文件接着跑、挂到控制面上
    async fn attach(&self, controller: &Controller) -> Arc<LocalNode> {
        let local = Arc::new(LocalNode::new(
            self.file.clone(),
            self.services.clone(),
            self.managed.clone(),
            self.revoked.clone(),
        ));
        local.resume(controller).await;
        local
    }

    fn state(&self) -> std::path::PathBuf {
        super::reconcile::state_path(&self.file)
    }
}

/// 启用「本机」：不用票据加入自己，房间手动与自动都能分派给它，不收 Fleet 配置，
/// 控制面自己的主播不受影响；重启后按缓存接着录；关掉时先交出房间再吊销，不留暂停的本地行。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_controller_records_fleet_rooms_as_its_own_local_node() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, _url, _pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let own = "https://stuck.example/own";
    crate::server::services::streamers::add_streamer(
        &fx.services,
        serde_json::from_value(serde_json::json!({ "url": own, "remark": "控制面自己的" }))
            .unwrap(),
    )
    .await
    .unwrap();

    // 没启用：什么文件都不写
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    assert_eq!(controller.local_node_id(), None);
    assert!(!fx.file.exists() && !fx.state().exists());
    assert!(fx.managed.read().unwrap().is_none());

    let id = local.enable(&controller, false).await.unwrap().unwrap();
    assert_eq!(local.enable(&controller, true).await.unwrap(), None);
    let file = node::NodeFile::load(&fx.file).unwrap();
    assert!(file.local);
    assert_eq!(file.controller, controller.endpoint_id().to_string());
    assert_eq!(file.relays, controller.local_relays());
    let node = wait_for_node(&controller, id, true, Duration::from_secs(30)).await;
    assert!(node.local);
    assert_eq!(node.name, "本机");
    assert!(!node.allow_hooks);
    assert_eq!(controller.local_node_id(), Some(id));
    eventually("config shows as local", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move { controller.node_config_state(id, None).sync == Some("local") }
    })
    .await;

    // 手动分派
    let manual = controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": "https://stuck.example/1", "remark": "手动", "node_id": id,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(manual.node_id, Some(id));
    eventually(
        "room lands on the controller",
        Duration::from_secs(20),
        || {
            let services = fx.services.clone();
            async move { local_urls(&services).await.len() == 2 }
        },
    )
    .await;
    // 自动选节点：只有「本机」时选它
    let auto = controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": "https://stuck.example/2", "remark": "自动", "auto_node": true,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(auto.node_id, Some(id));
    eventually("both rooms are monitored", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move {
            let rooms = controller.rooms(true).await.unwrap();
            rooms.len() == 2
                && rooms
                    .iter()
                    .all(|room| room.status == RoomStatus::Monitoring)
        }
    })
    .await;
    let managed = fx.managed.read().unwrap().clone().unwrap();
    assert!(managed.local);
    assert_eq!(managed.controller, "本机");
    assert_eq!(managed.streamers.len(), 2);
    assert!(!managed.streamers.values().any(|url| url == own));
    assert!(managed.config.is_none());

    // 带 run 命令的房间：没勾 allow_hooks 不收
    let mut hooked: CreateRoom = serde_json::from_value(serde_json::json!({
        "url": "https://stuck.example/h", "remark": "钩子", "node_id": id,
    }))
    .unwrap();
    hooked.spec.postprocessor =
        serde_json::from_value(serde_json::json!([{ "run": "echo" }])).unwrap();
    let rejected = controller.create_room(hooked).await.unwrap_err();
    assert!(
        matches!(&rejected, DispatchError::Invalid(m) if m.contains("--allow-hooks")),
        "{rejected:?}"
    );

    // 与控制面自己的主播同地址的房间：节点拒收，不会两份一起录
    let clash = controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": own, "remark": "撞车", "node_id": id,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    eventually("clash is reported", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move {
            controller
                .rooms(true)
                .await
                .unwrap()
                .iter()
                .any(|room| room.room.id == clash.id && room.status == RoomStatus::Failed)
        }
    })
    .await;
    controller.delete_room(clash.id, true).await.unwrap();

    // Fleet 配置不下发给「本机」：即使库里有覆盖，控制面自己的配置也不变
    let pool1 = fx.services.config.read().unwrap().pool1_size;
    let patch = serde_json::json!({ "pool1_size": pool1 + 3 });
    super::config_store::set_node_override(controller.pool(), id, patch.as_object().unwrap())
        .await
        .unwrap();
    controller.push(id).await;
    eventually("push is acknowledged", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move {
            controller
                .nodes()
                .await
                .unwrap()
                .iter()
                .any(|node| node.id == id && node.synced == Some(true))
        }
    })
    .await;
    assert_eq!(fx.services.config.read().unwrap().pool1_size, pool1);
    assert!(
        fx.managed
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .config
            .is_none()
    );

    // 控制面重启（内嵌 relay 换了端口）：按缓存认回托管行，relay 改成新的回环地址，连上后对账
    local.shutdown().await;
    controller.shutdown().await;
    let (controller, _url, _pool) = start_controller(dir.path()).await;
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    assert_eq!(local.node_id(), Some(id));
    assert_eq!(
        fx.managed.read().unwrap().as_ref().unwrap().streamers.len(),
        2
    );
    assert_eq!(
        node::NodeFile::load(&fx.file).unwrap().relays,
        controller.local_relays()
    );
    wait_for_node(&controller, id, true, Duration::from_secs(30)).await;
    eventually("rooms are monitored again", Duration::from_secs(20), || {
        let controller = controller.clone();
        async move {
            let rooms = controller.rooms(true).await.unwrap();
            rooms.len() == 2
                && rooms
                    .iter()
                    .all(|room| room.status == RoomStatus::Monitoring)
        }
    })
    .await;
    assert_eq!(local_urls(&fx.services).await.len(), 3);

    // 关掉「本机」（不改派）：先取消分派、等它释放，再吊销；房间留在控制面未分派，本机只剩自己的主播
    let started = controller.remove_node(id, false).await.unwrap().unwrap();
    assert_eq!(started.state, RemovalState::Removing);
    let removed = wait_removed(&controller, id, Duration::from_secs(70)).await;
    assert!(removed.local && !removed.reassign);
    assert_eq!(removed.rooms.len(), 2);
    assert!(
        removed
            .rooms
            .iter()
            .all(|room| room.release == Release::Released && room.node_id.is_none()),
        "{removed:?}"
    );
    assert_eq!(local_urls(&fx.services).await, [own]);
    let worker = fx.services.managers.get_rooms().await;
    assert!(!worker_paused(&worker[0]));
    assert!(fx.managed.read().unwrap().is_none());
    assert!(fx.revoked.is_empty());
    assert!(!fx.file.exists() && !fx.state().exists());
    assert_eq!(controller.local_node_id(), None);
    assert!(controller.nodes().await.unwrap().is_empty());
    let rooms = controller.rooms(true).await.unwrap();
    assert!(
        rooms
            .iter()
            .all(|room| room.status == RoomStatus::Unassigned)
    );

    // 可以再启用：新的节点身份
    let again = local.enable(&controller, true).await.unwrap().unwrap();
    assert_ne!(again, id);
    wait_for_node(&controller, again, true, Duration::from_secs(30)).await;
    local.shutdown().await;
    controller.shutdown().await;
}

/// 「本机」节点代理停了（等价于远端节点离线）时关掉它：没法等它确认，控制面替它把托管行转本地并暂停，
/// 然后才改派，两边不会同时录。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disabling_a_stalled_local_node_pauses_its_rooms_before_reassigning() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, _url, _pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    let id = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, id, true, Duration::from_secs(30)).await;
    controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": "https://stuck.example/1", "remark": "房间", "node_id": id,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    eventually(
        "room lands on the controller",
        Duration::from_secs(20),
        || {
            let services = fx.services.clone();
            async move { local_urls(&services).await.len() == 1 }
        },
    )
    .await;

    local.shutdown().await;
    wait_for_node(&controller, id, false, Duration::from_secs(10)).await;
    let done = controller.remove_node(id, true).await.unwrap().unwrap();
    assert_eq!(done.state, RemovalState::Done);
    assert!(done.local && done.reassign);
    assert_eq!(done.rooms[0].release, Release::Offline);
    // 没有别的节点，留在未分派
    assert_eq!(done.rooms[0].node_id, None);
    let worker = fx.services.managers.get_rooms().await;
    assert_eq!(worker.len(), 1);
    assert!(worker_paused(&worker[0]));
    assert_eq!(fx.revoked.ids(), [worker[0].live_streamer.id]);
    assert!(fx.managed.read().unwrap().is_none());
    assert!(!fx.file.exists() && !fx.state().exists());
    assert_eq!(controller.local_node_id(), None);
    controller.shutdown().await;
}

/// 运行中启用、卡住后关闭的「本机」：暂停的主播不用重启就出现在 `/v1/me` 里，「全部恢复」能用；
/// 清单空时 `/v1/node/revoked*` 与没挂一样
#[tokio::test]
async fn a_local_node_closed_at_runtime_offers_its_paused_rooms_without_a_restart() {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    let dir = tempfile::tempdir().unwrap();
    let (controller, _url, _pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let local = fx.attach(&controller).await;
    let fleet = super::Fleet::controller(
        controller.clone(),
        local.clone(),
        fx.managed.clone(),
        fx.revoked.clone(),
    );
    let app = fleet.router().unwrap();
    let send = |method: Method, uri: &'static str| {
        let app = app.clone();
        async move {
            let response = app
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
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
                serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
            )
        }
    };
    assert_eq!(fleet.capability().revoked_view(), None);
    let (status, _) = send(Method::POST, "/v1/node/revoked/resume").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(Method::DELETE, "/v1/node/revoked").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let id = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, id, true, Duration::from_secs(30)).await;
    controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": "https://stuck.example/1", "remark": "房间", "node_id": id,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    eventually(
        "room lands on the controller",
        Duration::from_secs(20),
        || {
            let services = fx.services.clone();
            async move { local_urls(&services).await.len() == 1 }
        },
    )
    .await;

    local.shutdown().await;
    wait_for_node(&controller, id, false, Duration::from_secs(10)).await;
    let done = controller.remove_node(id, false).await.unwrap().unwrap();
    assert_eq!(done.rooms[0].release, Release::Offline);
    let worker = fx.services.managers.get_rooms().await;
    assert!(worker_paused(&worker[0]));
    let streamer = worker[0].live_streamer.id;
    let view = fleet
        .capability()
        .revoked_view()
        .expect("fleet_revoked right away");
    assert_eq!(view["controller"], "本机");
    assert_eq!(view["local"], true);
    assert_eq!(view["streamers"], serde_json::json!([streamer]));

    let (status, body) = send(Method::POST, "/v1/node/revoked/resume").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["resumed"], serde_json::json!([streamer]));
    let worker = fx.services.managers.get_rooms().await;
    assert!(!worker_paused(&worker[0]));
    assert_eq!(fleet.capability().revoked_view(), None);
    assert!(!revoked_path(&fx.file).exists());
    let (status, _) = send(Method::POST, "/v1/node/revoked/resume").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    controller.shutdown().await;
}

/// `local-node.json` 与控制面对不上（控制面的数据被重置过）：不启动，托管行留作本地行
#[tokio::test]
async fn a_local_node_file_from_another_controller_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, _url, _pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let stale = node::NodeFile {
        local: true,
        ..node::NodeFile::new(
            iroh::SecretKey::generate().public(),
            controller.local_relays(),
            1,
            &iroh::SecretKey::generate(),
            false,
        )
    };
    stale.save(&fx.file).unwrap();
    std::fs::write(fx.state(), "{}").unwrap();
    let local = fx.attach(&controller).await;
    assert_eq!(local.node_id(), None);
    assert!(!local.agent_running().await);
    assert!(!fx.file.exists() && !fx.state().exists());
    controller.shutdown().await;
}
