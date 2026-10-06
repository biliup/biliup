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

/// 节点连上时记不下「最近在线」（库写失败）：连接照常建立，节点停下后从在线表里拿掉，不留一条没人收帧的连接
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_last_seen_write_does_not_strand_the_connection() {
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let node_file = dir.path().join("node/data/node.json");
    let joined = node::join(
        &ticket_for(&controller, &pool, &url).await,
        false,
        &node_file,
    )
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER no_last_seen BEFORE UPDATE OF last_seen_at ON fleet_nodes \
         BEGIN SELECT RAISE(ABORT, 'disk full'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let services = node_services(&dir.path().join("node")).await;
    let agent = NodeAgent::start(
        node_file.clone(),
        services.clone(),
        ManagedHandle::default(),
        revoked_for(&node_file, &services),
    )
    .await
    .unwrap();
    wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;

    agent.shutdown().await;
    let id = joined.node_id;
    eventually(
        "the stopped node leaves the live table",
        Duration::from_secs(10),
        || {
            let controller = controller.clone();
            async move { controller.node_link(id).is_none() }
        },
    )
    .await;
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
        matches!(&rejected, DispatchError::Invalid(m)
            if m.contains("「本机」") && m.contains("启用时没有勾选「允许钩子」") && !m.contains("--allow-hooks")),
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

/// 一主一备的控制面一侧：主机是「本机」，指定一台普通节点当备机。「本机」的房间与模板镜像给备机
/// （边录边传的不镜像），第三台节点不受影响，F2 的分派不变；备机连上先上报；改模式就地生效；
/// 解除后备机的镜像房间撤掉；备机被移除时配对跟着解除。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_local_node_is_mirrored_to_a_designated_standby() {
    use super::ha::agent::{load as load_ha_state, state_path as ha_state_path};
    use super::ha::pairing::{Designate, Pairing, Refused};
    use super::ha::params::HaMode;

    let _role = super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    // 与 `fleet::start` 同序：先挂配对，「本机」恢复之后再载入
    let pairing = Arc::new(Pairing::new(fx.services.clone(), dir.path()));
    controller.attach_ha(pairing.clone());
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    pairing.resume(&controller, local.node_id()).await;
    let designate = |standby: i64, mode: u8| -> Designate {
        serde_json::from_value(serde_json::json!({ "standby": standby, "mode": mode })).unwrap()
    };

    let refused = pairing
        .designate(&controller, designate(2, 1))
        .await
        .unwrap();
    assert!(
        matches!(&refused, Err(Refused::Conflict(m)) if m.contains("启用「本机」")),
        "{refused:?}"
    );

    let primary = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, primary, true, Duration::from_secs(30)).await;
    let mut nodes = Vec::new();
    for name in ["standby", "other"] {
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
        let agent = NodeAgent::start(
            node_file.clone(),
            services.clone(),
            ManagedHandle::default(),
            revoked_for(&node_file, &services),
        )
        .await
        .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, agent, node_file));
    }
    let (standby, other) = (nodes[0].0, nodes[1].0);
    let (standby_services, other_services) = (nodes[0].1.clone(), nodes[1].1.clone());
    let standby_state = ha_state_path(&nodes[0].3);

    let template = controller
        .create_template(
            serde_json::from_value(
                serde_json::json!({ "template_name": "ha", "title": "{title}" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let room = |url: &str, node: i64, downloader: Option<&str>| -> CreateRoom {
        serde_json::from_value(serde_json::json!({
            "url": url, "remark": "房间", "template_id": template.id, "node_id": node,
            "override": downloader.map(|downloader| serde_json::json!({ "downloader": downloader })),
        }))
        .unwrap()
    };
    let mirrored = controller
        .create_room(room("https://stuck.example/m", primary, None))
        .await
        .unwrap();
    let sync = controller
        .create_room(room(
            "https://stuck.example/s",
            primary,
            Some("sync-downloader"),
        ))
        .await
        .unwrap();
    controller
        .create_room(room("https://stuck.example/o", other, None))
        .await
        .unwrap();
    eventually("rooms land", Duration::from_secs(20), || {
        let (local, other) = (fx.services.clone(), other_services.clone());
        async move { local_urls(&local).await.len() == 2 && local_urls(&other).await.len() == 1 }
    })
    .await;

    // 备机不能是「本机」，也得是存在的节点
    let refused = pairing
        .designate(&controller, designate(primary, 1))
        .await
        .unwrap();
    assert!(matches!(refused, Err(Refused::Invalid(_))), "{refused:?}");
    let refused = pairing
        .designate(&controller, designate(999, 1))
        .await
        .unwrap();
    assert!(matches!(refused, Err(Refused::NotFound(_))), "{refused:?}");
    assert!(!standby_state.exists());

    let (pair, _) = pairing
        .designate(&controller, designate(standby, 1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (pair.primary_node_id, pair.standby_node_id),
        (primary, standby)
    );
    eventually(
        "the standby records the mirror",
        Duration::from_secs(20),
        || {
            let services = standby_services.clone();
            async move { local_urls(&services).await == ["https://stuck.example/m"] }
        },
    )
    .await;
    eventually("the standby reports first", Duration::from_secs(20), || {
        let (pairing, controller) = (pairing.clone(), controller.clone());
        async move {
            let view = pairing.view(&controller).await.unwrap();
            view["standby"]["linked"] == true && view["standby"]["reported"] == true
        }
    })
    .await;
    let state = load_ha_state(&standby_state).expect("备机建了 ha-state.json");
    let assignment = state.assignment.unwrap();
    assert_eq!(assignment.rooms, [mirrored.id]);
    assert_eq!(assignment.mode, HaMode::DualRecord);
    let view = pairing.view(&controller).await.unwrap();
    assert_eq!(view["active"], true);
    assert_eq!(view["local_node"], primary);
    assert_eq!(view["standby"]["proto"], super::protocol::PROTOCOL_MINOR);
    assert_eq!(view["excluded"][0]["id"], sync.id);
    assert_eq!(
        local_urls(&other_services).await,
        ["https://stuck.example/o"]
    );
    // 镜像不改分派：房间还在「本机」上，状态照常
    let rooms = controller.rooms(true).await.unwrap();
    let row = rooms.iter().find(|row| row.room.id == mirrored.id).unwrap();
    assert_eq!(row.room.node_id, Some(primary));
    assert_eq!(row.status, RoomStatus::Monitoring);

    // 后加到「本机」的房间也镜像过去
    controller
        .create_room(room("https://stuck.example/n", primary, None))
        .await
        .unwrap();
    eventually("new rooms are mirrored", Duration::from_secs(20), || {
        let services = standby_services.clone();
        async move { local_urls(&services).await.len() == 2 }
    })
    .await;

    // 只改模式：就地生效，备机收到新的配对
    pairing
        .designate(&controller, designate(standby, 2))
        .await
        .unwrap()
        .unwrap();
    eventually(
        "the standby switches to mode 2",
        Duration::from_secs(20),
        || {
            let path = standby_state.clone();
            async move {
                load_ha_state(&path)
                    .and_then(|state| state.assignment)
                    .is_some_and(|assignment| assignment.mode == HaMode::Takeover)
            }
        },
    )
    .await;
    assert!(
        pairing
            .manual("1:1", super::ha::wire::ManualAction::Drop)
            .is_ok()
    );

    // 解除：备机撤掉镜像房间，ha-state.json 留着但不再有配对
    assert!(pairing.dissolve(&controller).await.unwrap());
    eventually("the mirror is withdrawn", Duration::from_secs(20), || {
        let services = standby_services.clone();
        async move { local_urls(&services).await.is_empty() }
    })
    .await;
    eventually(
        "the standby drops the pair",
        Duration::from_secs(20),
        || {
            let path = standby_state.clone();
            async move { load_ha_state(&path).is_some_and(|state| state.assignment.is_none()) }
        },
    )
    .await;
    assert!(!pairing.dissolve(&controller).await.unwrap());
    assert_eq!(fx.services.managers.get_rooms().await.len(), 3);

    // 备机被移除：配对跟着解除
    pairing
        .designate(&controller, designate(standby, 1))
        .await
        .unwrap()
        .unwrap();
    assert!(controller.revoke(standby).await.unwrap());
    assert!(
        super::ha::store::pair(controller.pool())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(pairing.view(&controller).await.unwrap()["active"], false);

    // 备机自己离开（`biliup node leave`）：与被移除一样，配对跟着解除
    pairing
        .designate(&controller, designate(other, 1))
        .await
        .unwrap()
        .unwrap();
    let (_, _, other_agent, other_file) = nodes.pop().unwrap();
    other_agent.shutdown().await;
    assert!(node::leave(&other_file).await.unwrap());
    eventually(
        "the pair is dissolved after the standby left",
        Duration::from_secs(20),
        || {
            let (pairing, controller) = (pairing.clone(), controller.clone());
            async move {
                super::ha::store::pair(controller.pool())
                    .await
                    .unwrap()
                    .is_none()
                    && pairing.view(&controller).await.unwrap()["active"] == false
            }
        },
    )
    .await;

    for (_, _, agent, _) in nodes {
        agent.shutdown().await;
    }
    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

async fn row_by_url(services: &ServiceRegister, url: &str) -> Option<LiveStreamer> {
    LiveStreamer::select()
        .where_("url = ?")
        .bind(url)
        .fetch_optional(&services.pool)
        .await
        .unwrap()
}

async fn remark_on(services: &ServiceRegister, url: &str) -> Option<String> {
    row_by_url(services, url).await.map(|row| row.remark)
}

/// 在节点本机改一个主播的备注（与「直播管理」保存一样走 `update_streamer`）
async fn edit_remark(services: &ServiceRegister, url: &str, remark: &str) {
    let mut row = row_by_url(services, url).await.expect("本机有这一行");
    row.remark = remark.to_string();
    crate::server::services::streamers::update_streamer(services, row)
        .await
        .unwrap();
}

/// 一主一备的双向同步（H2）：备机镜像行可以改，改动经控制面仲裁回到 Fleet 房间、再到主机；主机的改动到备机；
/// 备机新建的主播加入配对（控制面按它建房间，备机认下自己那一行，不重复）；两边删除互相跟随。
/// 两台断开期间各改同一个房间，重连后按版本后写者赢（含「删了又被改」「改了又被删」）。
/// 备机的账号凭据到了主机，控制面库里没有凭据内容，两边读到的账号同步情况也没有；普通节点不参与同步、托管行照旧只读
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_paired_standby_edits_rooms_and_the_pair_converges() {
    use super::ha::member::member_for;
    use super::ha::pairing::{Designate, Pairing};
    use crate::server::services::streamers::{add_streamer, delete_streamer, toggle_pause};
    use axum::http::{Method, StatusCode};

    let _role = super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let pairing = Arc::new(Pairing::new(fx.services.clone(), dir.path()));
    controller.attach_ha(pairing.clone());
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    pairing.resume(&controller, local.node_id()).await;
    let primary = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, primary, true, Duration::from_secs(30)).await;

    let mut nodes = Vec::new();
    for name in ["standby", "other"] {
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
        let managed = ManagedHandle::default();
        let agent = NodeAgent::start(
            node_file.clone(),
            services.clone(),
            managed.clone(),
            revoked_for(&node_file, &services),
        )
        .await
        .unwrap();
        wait_for_node(&controller, joined.node_id, true, Duration::from_secs(30)).await;
        nodes.push((joined.node_id, services, Some(agent), node_file, managed));
    }
    let (standby, other) = (nodes[0].0, nodes[1].0);
    let s = nodes[0].1.clone();
    let standby_file = nodes[0].3.clone();
    let standby_managed = nodes[0].4.clone();
    let (other_services, other_managed) = (nodes[1].1.clone(), nodes[1].4.clone());
    let c = fx.services.clone();

    let template = controller
        .create_template(
            serde_json::from_value(
                serde_json::json!({ "template_name": "ha", "title": "{title}" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let m = "https://stuck.example/m";
    let room = |url: &str, node: i64| -> CreateRoom {
        serde_json::from_value(serde_json::json!({
            "url": url, "remark": "房间", "template_id": template.id, "node_id": node,
        }))
        .unwrap()
    };
    let mirrored = controller.create_room(room(m, primary)).await.unwrap();
    controller
        .create_room(room("https://stuck.example/o", other))
        .await
        .unwrap();
    let designate: Designate =
        serde_json::from_value(serde_json::json!({ "standby": standby, "mode": 1 })).unwrap();
    pairing
        .designate(&controller, designate)
        .await
        .unwrap()
        .unwrap();
    let paired_row = |managed: ManagedHandle, id: i64| {
        managed
            .read()
            .unwrap()
            .as_ref()
            .and_then(|managed| managed.pair.clone())
            .is_some_and(|pair| pair.streamers.contains(&id))
    };
    eventually(
        "the mirror is a paired row",
        Duration::from_secs(20),
        || {
            let (s, managed) = (s.clone(), standby_managed.clone());
            async move {
                let Some(row) = row_by_url(&s, m).await else {
                    return false;
                };
                let Some(member) = member_for(&s) else {
                    return false;
                };
                paired_row(managed, row.id)
                    && member
                        .book()
                        .await
                        .key_of_local(super::ha::sync::ROOM, row.id)
                        .is_some()
            }
        },
    )
    .await;
    let row = row_by_url(&s, m).await.unwrap();
    assert!(
        !standby_managed
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .streamers
            .contains_key(&row.id),
        "配对里的行不算托管，本机能改"
    );
    let member = || member_for(&s).expect("备机上有同步端");
    let fleet_room = |id: i64| {
        let controller = controller.clone();
        async move {
            super::assignments::room(controller.pool(), id)
                .await
                .unwrap()
                .filter(|room| room.deleted_at.is_none())
        }
    };
    let converged = |what: &'static str, remark: &'static str| {
        let (c, s, fleet_room) = (c.clone(), s.clone(), fleet_room);
        async move {
            eventually(what, Duration::from_secs(30), || {
                let (c, s) = (c.clone(), s.clone());
                async move {
                    fleet_room(mirrored.id)
                        .await
                        .is_some_and(|room| room.spec.remark == remark)
                        && remark_on(&c, m).await.as_deref() == Some(remark)
                        && remark_on(&s, m).await.as_deref() == Some(remark)
                }
            })
            .await;
        }
    };

    // 备机上改：控制面的房间与主机的行跟着变
    edit_remark(&s, m, "备机改的").await;
    member().scan().await;
    converged(
        "a standby edit reaches the controller and the primary",
        "备机改的",
    )
    .await;

    // 主机（控制面）上改：到备机
    let update = |remark: &str| -> super::controller::UpdateRoom {
        serde_json::from_value(serde_json::json!({
            "url": m, "remark": remark, "template_id": template.id,
        }))
        .unwrap()
    };
    controller
        .update_room(mirrored.id, update("控制面改的"), true)
        .await
        .unwrap();
    converged("a controller edit reaches the standby", "控制面改的").await;

    // 备机暂停：控制面的房间跟着暂停
    let local_id = row_by_url(&s, m).await.unwrap().id;
    // 行先落库、监控任务后重建；重建前的 toggle_pause 找不到任务，什么也不做
    eventually(
        "the standby rebuilds the monitor after the controller edit",
        Duration::from_secs(20),
        || {
            let s = s.clone();
            async move {
                s.managers
                    .get_room_by_id(local_id)
                    .await
                    .is_some_and(|worker| worker.live_streamer.remark == "控制面改的")
            }
        },
    )
    .await;
    landed(&member()).await;
    toggle_pause(&s.managers, local_id).await;
    member().scan().await;
    eventually(
        "a standby pause reaches the controller",
        Duration::from_secs(20),
        || async move {
            fleet_room(mirrored.id)
                .await
                .is_some_and(|room| room.paused)
        },
    )
    .await;
    controller.pause_room(mirrored.id, false).await.unwrap();
    eventually(
        "the controller resumes it on the standby",
        Duration::from_secs(20),
        || {
            let s = s.clone();
            async move {
                s.managers
                    .get_room_by_id(local_id)
                    .await
                    .is_some_and(|worker| !worker_paused(&worker))
            }
        },
    )
    .await;

    // 备机新建主播：加入配对，控制面建房间分派给主机，备机认下自己那一行
    let fresh = "https://stuck.example/fresh";
    let created = add_streamer(
        &s,
        serde_json::from_value(serde_json::json!({ "url": fresh, "remark": "备机新建" })).unwrap(),
    )
    .await
    .unwrap();
    member().join_room(created.id).await;
    eventually(
        "the new streamer becomes a fleet room",
        Duration::from_secs(30),
        || {
            let (c, controller, managed) = (c.clone(), controller.clone(), standby_managed.clone());
            async move {
                let rooms = super::assignments::list_rooms(controller.pool())
                    .await
                    .unwrap();
                let fleet = rooms.iter().any(|room| {
                    room.spec.url == fresh
                        && room.node_id == Some(primary)
                        && room.deleted_at.is_none()
                });
                fleet
                    && remark_on(&c, fresh).await.as_deref() == Some("备机新建")
                    && paired_row(managed, created.id)
            }
        },
    )
    .await;
    assert_eq!(
        local_urls(&s)
            .await
            .iter()
            .filter(|url| *url == fresh)
            .count(),
        1,
        "备机不按控制面 id 再建一行"
    );

    // 备机删掉：控制面的房间与主机的行跟着删
    landed(&member()).await;
    delete_streamer(&s.pool, &s.managers, created.id)
        .await
        .unwrap();
    member().scan().await;
    eventually(
        "a standby delete reaches the controller",
        Duration::from_secs(30),
        || {
            let (c, controller) = (c.clone(), controller.clone());
            async move {
                let rooms = super::assignments::list_rooms(controller.pool())
                    .await
                    .unwrap();
                !rooms.iter().any(|room| room.spec.url == fresh)
                    && remark_on(&c, fresh).await.is_none()
            }
        },
    )
    .await;

    // 两台断开期间各改同一个房间，重连后后写者赢
    let data = standby_file.parent().unwrap().to_path_buf();
    let mut agent = nodes[0].2.take().unwrap();
    let pause = || tokio::time::sleep(Duration::from_millis(30));

    // 控制面先改、备机后改：备机的赢
    let away = standby_offline(&controller, standby, agent, &s, &data).await;
    controller
        .update_room(mirrored.id, update("离线-控制面"), true)
        .await
        .unwrap();
    pause().await;
    edit_remark(&s, m, "离线-备机").await;
    away.scan().await;
    assert!(away.pending().await > 0, "断开期间的修改排进队列");
    away.stop();
    agent = start_standby(&standby_file, &s, &standby_managed).await;
    converged("the later standby edit wins", "离线-备机").await;
    // 重连后备机先重发队列、后应答期望状态：控制面的应答不能因为还没接上同步连接而丢掉
    eventually(
        "the controller acknowledges the replayed queue",
        Duration::from_secs(20),
        || {
            let s = s.clone();
            async move {
                match member_for(&s) {
                    Some(member) => member.pending().await == 0,
                    None => false,
                }
            }
        },
    )
    .await;

    // 备机先改、控制面后改：控制面的赢
    let away = standby_offline(&controller, standby, agent, &s, &data).await;
    edit_remark(&s, m, "离线-备机2").await;
    away.scan().await;
    away.stop();
    pause().await;
    controller
        .update_room(mirrored.id, update("离线-控制面2"), true)
        .await
        .unwrap();
    agent = start_standby(&standby_file, &s, &standby_managed).await;
    converged("the later controller edit wins", "离线-控制面2").await;

    // 备机先删、控制面后改：改的赢，备机按控制面重建这一行
    let away = standby_offline(&controller, standby, agent, &s, &data).await;
    let id = row_by_url(&s, m).await.unwrap().id;
    delete_streamer(&s.pool, &s.managers, id).await.unwrap();
    away.scan().await;
    away.stop();
    pause().await;
    controller
        .update_room(mirrored.id, update("删了又改"), true)
        .await
        .unwrap();
    agent = start_standby(&standby_file, &s, &standby_managed).await;
    converged("a later edit revives a deleted row", "删了又改").await;

    // 控制面先改、备机后删：删的赢，三处都没了
    let away = standby_offline(&controller, standby, agent, &s, &data).await;
    controller
        .update_room(mirrored.id, update("改了又删"), true)
        .await
        .unwrap();
    pause().await;
    let id = row_by_url(&s, m).await.unwrap().id;
    delete_streamer(&s.pool, &s.managers, id).await.unwrap();
    away.scan().await;
    away.stop();
    agent = start_standby(&standby_file, &s, &standby_managed).await;
    eventually("a later delete wins", Duration::from_secs(30), || {
        let (c, s, fleet_room) = (c.clone(), s.clone(), fleet_room);
        async move {
            fleet_room(mirrored.id).await.is_none()
                && remark_on(&c, m).await.is_none()
                && remark_on(&s, m).await.is_none()
        }
    })
    .await;

    // 备机登录（伪造的凭据）：主机写进本机并登记；控制面库里没有凭据内容
    let login = data.join("5151.json");
    std::fs::write(&login, super::ha::member::tests::credential(5151, "e2e")).unwrap();
    crate::server::infrastructure::repositories::register_bilibili_cookie(&s.pool, &login)
        .await
        .unwrap();
    member().scan().await;
    let received = dir.path().join("5151.json");
    eventually(
        "the credential reaches the primary",
        Duration::from_secs(20),
        || {
            let received = received.clone();
            async move {
                std::fs::read_to_string(&received)
                    .is_ok_and(|text| text.contains("PLACEHOLDER-ACCESS-e2e"))
            }
        },
    )
    .await;
    let registered: Vec<String> =
        sqlx::query_scalar("SELECT value FROM configuration WHERE key = 'bilibili-cookies'")
            .fetch_all(&c.pool)
            .await
            .unwrap();
    assert_eq!(registered, [received.to_string_lossy().into_owned()]);
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("fleet.sqlite3"))
        {
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                !bytes.windows(11).any(|window| window == b"PLACEHOLDER"),
                "控制面库 {} 里不能有凭据内容",
                path.display()
            );
        }
    }
    // 两边面板与账号页读到的账号同步情况：只有个数与时刻，没有凭据内容与路径
    eventually(
        "both sides count the account",
        Duration::from_secs(20),
        || async { member().pending().await == 0 },
    )
    .await;
    let (status, node_view) =
        node_ha_request(&s, Method::GET, "/v1/node/ha", serde_json::Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{node_view}");
    let controller_view = pairing.view(&controller).await.unwrap();
    for view in [&node_view, &controller_view] {
        let accounts = &view["sync"]["accounts"];
        assert_eq!(accounts["count"], 1, "{view}");
        assert_eq!(accounts["changed_on"], "node", "{view}");
        assert_eq!(accounts["pending"], 0, "{view}");
        let text = view.to_string();
        for secret in [
            "PLACEHOLDER",
            "5151.json",
            &*dir.path().to_string_lossy(),
            &*data.to_string_lossy(),
        ] {
            assert!(!text.contains(secret), "{secret} 出现在 {text}");
        }
    }

    // 普通节点：不参与同步，托管行照旧只读，也没收到凭据
    let other_view = other_managed.read().unwrap().clone().unwrap();
    assert!(other_view.pair.is_none());
    assert_eq!(other_view.streamers.len(), 1);
    let other_data = nodes[1].3.parent().unwrap().to_path_buf();
    assert!(!super::ha::outbox::path_in(&other_data).exists());
    assert!(!other_data.join("5151.json").exists());
    assert!(member_for(&other_services).is_none());

    agent.shutdown().await;
    if let Some(agent) = nodes[1].2.take() {
        agent.shutdown().await;
    }
    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

/// 等备机上正在落地的期望状态做完：落地从改行、重建监控、套暂停到记下各行落地的版本一直拿着同步端的锁。
/// 锁放开之前本机再动同一行，暂停会被这次落地按期望状态套回去，删掉的行落地记不下，之后的扫描都认不出来
async fn landed(member: &super::ha::member::Member) {
    member.pending().await;
}

/// 模拟备机进程还在、只是连不上控制面：停掉节点代理，单独起一个同步端记下断开期间本机的修改
async fn standby_offline(
    controller: &Controller,
    standby: i64,
    agent: NodeAgent,
    services: &ServiceRegister,
    data: &std::path::Path,
) -> Arc<super::ha::member::Member> {
    agent.shutdown().await;
    wait_for_node(controller, standby, false, Duration::from_secs(30)).await;
    super::ha::member::Member::start(
        super::ha::sync::Side::Node,
        data,
        &controller.endpoint_id().to_string(),
        services.clone(),
        super::ha::sync::Side::Controller,
    )
    .await
}

async fn start_standby(
    node_file: &std::path::Path,
    services: &ServiceRegister,
    managed: &ManagedHandle,
) -> NodeAgent {
    NodeAgent::start(
        node_file.to_path_buf(),
        services.clone(),
        managed.clone(),
        revoked_for(node_file, services),
    )
    .await
    .unwrap()
}

/// 经节点本地的 `/v1/node/ha*` 发一个请求
async fn node_ha_request(
    services: &ServiceRegister,
    method: axum::http::Method,
    uri: &str,
    body: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    use tower::ServiceExt;
    let app = crate::server::api::fleet_ha::node_router(services.clone()).route_layer(
        axum::middleware::from_fn(crate::server::api::access::unrestricted),
    );
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body.to_string()))
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
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// 上传主机两台都在线时能从任一台换：控制面换到节点（控制面改当备机、节点起主机并收到控制面的上报），
/// 节点上换回控制面、改模式；只剩一台在线时两边都拒绝，上传主机不变
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_upload_primary_switches_from_either_side_only_while_both_are_online() {
    use super::ha::agent::{load as load_ha_state, state_path as ha_state_path};
    use super::ha::pairing::{Designate, Pairing, Refused};
    use super::ha::params::HaMode;
    use super::ha::sync::Side;
    use axum::http::{Method, StatusCode};

    let _role = super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let pairing = Arc::new(Pairing::new(fx.services.clone(), dir.path()));
    controller.attach_ha(pairing.clone());
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    pairing.resume(&controller, local.node_id()).await;
    let primary = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, primary, true, Duration::from_secs(30)).await;

    let root = dir.path().join("standby");
    let node_file = root.join("data/node.json");
    let joined = node::join(
        &ticket_for(&controller, &pool, &url).await,
        false,
        &node_file,
    )
    .await
    .unwrap();
    let standby = joined.node_id;
    let s = node_services(&root).await;
    let managed = ManagedHandle::default();
    let agent = start_standby(&node_file, &s, &managed).await;
    wait_for_node(&controller, standby, true, Duration::from_secs(30)).await;
    let template = controller
        .create_template(
            serde_json::from_value(
                serde_json::json!({ "template_name": "ha", "title": "{title}" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    controller
        .create_room(
            serde_json::from_value(serde_json::json!({
                "url": "https://stuck.example/m", "remark": "房间", "template_id": template.id,
                "node_id": primary,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let designate: Designate =
        serde_json::from_value(serde_json::json!({ "standby": standby, "mode": 1 })).unwrap();
    pairing
        .designate(&controller, designate)
        .await
        .unwrap()
        .unwrap();
    let linked = |leader: &'static str| {
        let (pairing, controller) = (pairing.clone(), controller.clone());
        async move {
            eventually("both links are up", Duration::from_secs(30), || {
                let (pairing, controller) = (pairing.clone(), controller.clone());
                async move {
                    let view = pairing.view(&controller).await.unwrap();
                    view["leader"] == leader
                        && view["sync"]["linked"] == true
                        && view["standby"]["linked"] == true
                        && view["standby"]["reported"] == true
                }
            })
            .await;
        }
    };
    linked("controller").await;
    let state_file = ha_state_path(&node_file);
    let leader_on_node = || {
        load_ha_state(&state_file)
            .and_then(|state| state.assignment)
            .map(|assignment| assignment.leader)
    };

    // 控制面上换到节点
    let pair = pairing
        .switch(&controller, Side::Node)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pair.leader(), Side::Node);
    assert_eq!(pair.primary_node_id, primary, "机器身份不变");
    linked("node").await;
    let view = pairing.view(&controller).await.unwrap();
    assert_eq!(view["local_role"], "standby");
    assert_eq!(view["local_standby"]["role"], "standby");
    assert_eq!(leader_on_node(), Some(Side::Node));
    assert!(root.join("data/ha-primary.sqlite3").exists());
    eventually(
        "the node primary got the controller's report",
        Duration::from_secs(20),
        || async { super::ha::primary().is_some_and(|primary| primary.view()["reported"] == true) },
    )
    .await;
    let (status, body) =
        node_ha_request(&s, Method::GET, "/v1/node/ha", serde_json::Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "primary");
    assert_eq!(body["leader"], "node");
    // 同一台再换一次：什么都不做
    let again = pairing
        .switch(&controller, Side::Node)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.updated_at, pair.updated_at);

    // 节点上换回控制面，再改模式
    let (status, body) = node_ha_request(
        &s,
        Method::POST,
        "/v1/node/ha/role",
        serde_json::json!({ "primary": "controller" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    linked("controller").await;
    assert_eq!(
        pairing.view(&controller).await.unwrap()["local_role"],
        "primary"
    );
    assert_eq!(leader_on_node(), Some(Side::Controller));
    let (status, body) = node_ha_request(
        &s,
        Method::PUT,
        "/v1/node/ha",
        serde_json::json!({ "mode": 2, "params": { "offline_grace": 30 } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let saved = super::ha::store::pair(controller.pool())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.mode, HaMode::Takeover);
    assert_eq!(saved.params.offline_grace, 30);
    assert_eq!(saved.leader(), Side::Controller);
    eventually(
        "the node follows the new mode",
        Duration::from_secs(20),
        || async {
            load_ha_state(&state_file)
                .and_then(|state| state.assignment)
                .is_some_and(|assignment| assignment.mode == HaMode::Takeover)
        },
    )
    .await;
    let (status, body) = node_ha_request(
        &s,
        Method::PUT,
        "/v1/node/ha",
        serde_json::json!({ "mode": 1, "params": { "upload_start_timeout": 0 } }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // 节点离线：控制面上换被拒绝；节点那边（进程还在、连不上控制面）请求也被拒绝
    let data = node_file.parent().unwrap().to_path_buf();
    let offline = standby_offline(&controller, standby, agent, &s, &data).await;
    let refused = pairing
        .switch(&controller, Side::Node)
        .await
        .unwrap()
        .unwrap_err();
    assert!(
        matches!(&refused, Refused::Conflict(m) if m.contains("只剩一台在线")),
        "{refused:?}"
    );
    let refused = offline.ask(Some(Side::Node), None).await.unwrap_err();
    assert!(refused.contains("只剩一台在线"), "{refused}");
    let saved = super::ha::store::pair(controller.pool())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.leader(), Side::Controller);
    assert_eq!(
        pairing.view(&controller).await.unwrap()["local_role"],
        "primary"
    );
    offline.stop();

    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

async fn local_template(services: &ServiceRegister, name: &str) -> i64 {
    let insert: crate::server::infrastructure::models::upload_streamer::InsertUploadStreamer =
        serde_json::from_value(serde_json::json!({ "template_name": name, "tags": [] })).unwrap();
    ormlite::Insert::insert(insert, &services.pool)
        .await
        .unwrap()
        .id
}

async fn local_streamer(services: &ServiceRegister, fields: serde_json::Value) -> i64 {
    crate::server::services::streamers::add_streamer(
        services,
        serde_json::from_value(fields).unwrap(),
    )
    .await
    .unwrap()
    .id
}

async fn template_names(services: &ServiceRegister) -> Vec<String> {
    let mut names: Vec<String> =
        crate::server::infrastructure::models::upload_streamer::UploadStreamer::select()
            .fetch_all(&services.pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.template_name)
            .collect();
    names.sort();
    names
}

async fn sorted_urls(services: &ServiceRegister) -> Vec<String> {
    let mut urls = local_urls(services).await;
    urls.sort();
    urls
}

/// 清单里的一行（`streamers` 或 `templates`）
fn listed(view: &serde_json::Value, kind: &str, id: i64) -> serde_json::Value {
    view[kind]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == id))
        .cloned()
        .unwrap_or_else(|| panic!("{kind} 里没有 {id}：{view}"))
}

/// 指定备机时备机上已有的本地主播与模板缺省全部纳入，勾掉的留作备机的本地行，不能纳入的逐条带原因；
/// 配对之后按 id 把备机或主机上还没纳入的本地行加进来。加入都是原地认下（监控不重建、不重复建行），
/// 上一场还没投完的主播等它投完才加入；最后两台一致（不能纳入的那一行除外）
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn existing_local_rows_join_the_pair_by_default_or_later_without_a_rebuild() {
    use super::ha::member::member_for;
    use super::ha::pairing::{Designate, Join, Pairing, Refused};
    use crate::server::infrastructure::context::WorkerStatus;
    use axum::http::{Method, StatusCode};

    let _role = super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let pairing = Arc::new(Pairing::new(fx.services.clone(), dir.path()));
    controller.attach_ha(pairing.clone());
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    pairing.resume(&controller, local.node_id()).await;
    let primary = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, primary, true, Duration::from_secs(30)).await;

    let root = dir.path().join("standby");
    let node_file = root.join("data/node.json");
    let joined = node::join(
        &ticket_for(&controller, &pool, &url).await,
        false,
        &node_file,
    )
    .await
    .unwrap();
    let standby = joined.node_id;
    let s = node_services(&root).await;
    let managed = ManagedHandle::default();
    let agent = start_standby(&node_file, &s, &managed).await;
    wait_for_node(&controller, standby, true, Duration::from_secs(30)).await;
    let c = fx.services.clone();

    // 备机上的本地行：用模板的主播 a、要勾掉的 b、带 run 命令的 h、没有主播用的模板；主机「本机」上的 p
    let (a_url, b_url, h_url, p_url) = (
        "https://stuck.example/a",
        "https://stuck.example/b",
        "https://stuck.example/h",
        "https://stuck.example/p",
    );
    let t1 = local_template(&s, "备机模板A").await;
    let t2 = local_template(&s, "备机单独模板").await;
    let a = local_streamer(
        &s,
        serde_json::json!({ "url": a_url, "remark": "备机A", "upload_streamers_id": t1 }),
    )
    .await;
    let b = local_streamer(&s, serde_json::json!({ "url": b_url, "remark": "备机B" })).await;
    let h = local_streamer(
        &s,
        serde_json::json!({ "url": h_url, "remark": "备机钩子", "postprocessor": [{ "run": "echo" }] }),
    )
    .await;
    let p = local_streamer(&c, serde_json::json!({ "url": p_url, "remark": "本机P" })).await;
    let worker = |services: &ServiceRegister, id: i64| {
        let services = services.clone();
        async move { services.managers.get_room_by_id(id).await.unwrap() }
    };
    let (worker_a, worker_b, worker_p) = (
        worker(&s, a).await,
        worker(&s, b).await,
        worker(&c, p).await,
    );

    // 确认弹层的清单：能纳入的缺省都纳入，带 run 命令的不能纳入并说明原因
    let preview = pairing
        .candidates(&controller, Some(standby))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preview["paired"], false);
    let view = &preview["standby"];
    for id in [a, b] {
        assert_eq!(listed(view, "streamers", id)["included"], true, "{view}");
    }
    let hooks = listed(view, "streamers", h);
    assert_eq!(hooks["included"], false);
    assert!(
        hooks["reason"].as_str().unwrap().contains("run 命令"),
        "{hooks}"
    );
    for id in [t1, t2] {
        assert_eq!(listed(view, "templates", id)["included"], true, "{view}");
    }
    assert_eq!(
        listed(&preview["primary"], "streamers", p)["included"],
        true
    );
    let refused = pairing
        .candidates(&controller, Some(primary))
        .await
        .unwrap();
    assert!(matches!(refused, Err(Refused::Invalid(_))), "{refused:?}");

    // 指定备机，勾掉 b：a（连同它的模板）与单独的模板纳入，b 留作备机的本地行，h 带着原因不纳入
    let designate: Designate = serde_json::from_value(serde_json::json!({
        "standby": standby, "mode": 1, "adopt": { "streamers": [a, h] },
    }))
    .unwrap();
    let (_, adoption) = pairing
        .designate(&controller, designate)
        .await
        .unwrap()
        .unwrap();
    let adoption = adoption.expect("新的备机：应答里带纳入的清单");
    assert_eq!(listed(&adoption, "streamers", a)["included"], true);
    let unchecked = listed(&adoption, "streamers", b);
    assert_eq!(unchecked["included"], false);
    assert!(
        unchecked.get("reason").is_none(),
        "勾掉的不算不能纳入：{unchecked}"
    );
    let hooks = listed(&adoption, "streamers", h);
    assert_eq!(hooks["included"], false);
    assert!(
        hooks["reason"].as_str().unwrap().contains("run 命令"),
        "{hooks}"
    );
    for id in [t1, t2] {
        assert_eq!(listed(&adoption, "templates", id)["included"], true);
    }

    let fleet_room = |url: &'static str| {
        let controller = controller.clone();
        async move {
            super::assignments::list_rooms(controller.pool())
                .await
                .unwrap()
                .into_iter()
                .find(|room| room.spec.url == url && room.deleted_at.is_none())
        }
    };
    let paired = |id: i64| {
        managed
            .read()
            .unwrap()
            .as_ref()
            .and_then(|view| view.pair.clone())
            .is_some_and(|pair| pair.streamers.contains(&id))
    };
    eventually(
        "the included rows join the pair",
        Duration::from_secs(40),
        || {
            let c = c.clone();
            async move {
                fleet_room(a_url)
                    .await
                    .is_some_and(|room| room.node_id == Some(primary))
                    && paired(a)
                    && remark_on(&c, a_url).await.as_deref() == Some("备机A")
                    && template_names(&c).await == ["备机单独模板", "备机模板A"]
            }
        },
    )
    .await;
    assert!(
        Arc::ptr_eq(&worker_a, &worker(&s, a).await),
        "备机原地认下 a，监控不重建"
    );
    let primary_a = row_by_url(&c, a_url).await.unwrap();
    let primary_template =
        crate::server::infrastructure::models::upload_streamer::UploadStreamer::select()
            .where_("id = ?")
            .bind(primary_a.upload_streamers_id.unwrap())
            .fetch_one(&c.pool)
            .await
            .unwrap();
    assert_eq!(primary_template.template_name, "备机模板A");
    let standby_state = super::ha::adopt::fleet_state(node_file.parent().unwrap());
    let adopted: Vec<i64> = standby_state
        .templates
        .values()
        .map(|t| t.local_id)
        .collect();
    assert!(
        adopted.contains(&t1) && adopted.contains(&t2),
        "{adopted:?}"
    );
    assert_eq!(template_names(&s).await, ["备机单独模板", "备机模板A"]);
    assert_eq!(sorted_urls(&s).await, [a_url, b_url, h_url]);
    assert_eq!(sorted_urls(&c).await, [a_url, p_url]);
    assert!(fleet_room(b_url).await.is_none() && fleet_room(h_url).await.is_none());

    // 备机本机的清单：勾掉的 b 仍是本地行、缺省纳入，带 run 命令的 h 说明原因
    let (status, body) = node_ha_request(
        &s,
        Method::GET,
        "/v1/node/ha/candidates",
        serde_json::Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let unchecked = listed(&body, "streamers", b);
    assert_eq!(
        (unchecked["state"].clone(), unchecked["included"].clone()),
        ("local".into(), true.into())
    );
    assert!(
        listed(&body, "streamers", h)["reason"]
            .as_str()
            .unwrap()
            .contains("run 命令")
    );
    assert_eq!(body["streamers"].as_array().unwrap().len(), 2, "{body}");

    // 事后加入 b：上一场还没投完时不加入，投完了才加入，也是原地认下
    *worker_b.uploader_status.write().unwrap() = WorkerStatus::Pending;
    let join: Join =
        serde_json::from_value(serde_json::json!({ "side": "node", "streamers": [b] })).unwrap();
    let answer = pairing.join(&controller, join).await.unwrap().unwrap();
    let row = listed(&answer, "streamers", b);
    assert_eq!(
        (row["included"].clone(), row["busy"].clone()),
        (true.into(), true.into())
    );
    eventually("the standby queues b", Duration::from_secs(20), || {
        let s = s.clone();
        async move {
            let (status, body) = node_ha_request(
                &s,
                Method::GET,
                "/v1/node/ha/candidates",
                serde_json::Value::Null,
            )
            .await;
            status == StatusCode::OK && listed(&body, "streamers", b)["state"] == "waiting"
        }
    })
    .await;
    let member = member_for(&s).expect("备机上有同步端");
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        member.scan().await;
    }
    assert!(fleet_room(b_url).await.is_none(), "在投的主播不加入");
    assert!(!paired(b));
    assert!(matches!(
        *worker_b.uploader_status.read().unwrap(),
        WorkerStatus::Pending
    ));
    *worker_b.uploader_status.write().unwrap() = WorkerStatus::Idle;
    eventually("b joins once idle", Duration::from_secs(40), || {
        let c = c.clone();
        async move {
            fleet_room(b_url)
                .await
                .is_some_and(|room| room.node_id == Some(primary))
                && paired(b)
                && remark_on(&c, b_url).await.as_deref() == Some("备机B")
        }
    })
    .await;
    assert!(
        Arc::ptr_eq(&worker_b, &worker(&s, b).await),
        "备机原地认下 b，监控不重建"
    );

    // 事后加入主机「本机」上的 p：控制面按它建房间分派给「本机」，「本机」原地认下，备机收到镜像
    let join: Join =
        serde_json::from_value(serde_json::json!({ "side": "controller", "streamers": [p, 999] }))
            .unwrap();
    let answer = pairing.join(&controller, join).await.unwrap().unwrap();
    assert_eq!(listed(&answer, "streamers", p)["included"], true);
    let missing = listed(&answer, "streamers", 999);
    assert_eq!(missing["included"], false);
    assert!(missing["reason"].is_string(), "{missing}");
    eventually("p joins the pair", Duration::from_secs(40), || {
        let (s, fx_managed) = (s.clone(), fx.managed.clone());
        async move {
            fleet_room(p_url)
                .await
                .is_some_and(|room| room.node_id == Some(primary))
                && fx_managed
                    .read()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|view| view.streamers.contains_key(&p))
                && remark_on(&s, p_url).await.as_deref() == Some("本机P")
        }
    })
    .await;
    assert!(
        Arc::ptr_eq(&worker_p, &worker(&c, p).await),
        "「本机」原地认下 p，监控不重建"
    );

    // 备机本机的接口：带 run 命令的 h 仍然不能加入，原因逐条给出
    let (status, body) = node_ha_request(
        &s,
        Method::POST,
        "/v1/node/ha/join",
        serde_json::json!({ "streamers": [h] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let hooks = listed(&body, "streamers", h);
    assert_eq!(hooks["included"], false);
    assert!(
        hooks["reason"].as_str().unwrap().contains("run 命令"),
        "{hooks}"
    );
    let (status, _) =
        node_ha_request(&s, Method::POST, "/v1/node/ha/join", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 两台一致：除了不能纳入的 h，每个主播与模板两台各一行
    assert_eq!(sorted_urls(&c).await, [a_url, b_url, p_url]);
    assert_eq!(sorted_urls(&s).await, [a_url, b_url, h_url, p_url]);
    assert_eq!(template_names(&c).await, ["备机单独模板", "备机模板A"]);
    assert_eq!(template_names(&s).await, ["备机单独模板", "备机模板A"]);
    let view = pairing
        .candidates(&controller, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(view["paired"], true);
    assert_eq!(
        view["standby"]["streamers"].as_array().unwrap().len(),
        1,
        "{view}"
    );
    assert_eq!(
        view["primary"]["streamers"].as_array().unwrap().len(),
        0,
        "{view}"
    );

    agent.shutdown().await;
    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

/// 解除配对（做法 B）：从备机纳入的主播与模板回到备机、成为它的本地行（带着配对期间在主机上改的设置、
/// 原来那一行不重建），并从控制面移除；配对期间在备机上新建的主播留在主机、备机上撤掉。
/// 交接等主机「本机」上那一场投完：这之前备机那一行挡着开录、只有「本机」录，交接之后只有备机录
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rows_adopted_from_the_standby_go_back_to_it_when_the_pair_is_dissolved() {
    use super::ha::member::member_for;
    use super::ha::pairing::{Designate, Pairing};
    use crate::server::infrastructure::context::WorkerStatus;

    let _role = super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let (controller, url, pool) = start_controller(dir.path()).await;
    let fx = LocalFixture::new(dir.path()).await;
    let pairing = Arc::new(Pairing::new(fx.services.clone(), dir.path()));
    controller.attach_ha(pairing.clone());
    let local = fx.attach(&controller).await;
    controller.attach_local(local.clone());
    pairing.resume(&controller, local.node_id()).await;
    let primary = local.enable(&controller, false).await.unwrap().unwrap();
    wait_for_node(&controller, primary, true, Duration::from_secs(30)).await;

    let root = dir.path().join("standby");
    let node_file = root.join("data/node.json");
    let joined = node::join(
        &ticket_for(&controller, &pool, &url).await,
        false,
        &node_file,
    )
    .await
    .unwrap();
    let standby = joined.node_id;
    let s = node_services(&root).await;
    let managed = ManagedHandle::default();
    let agent = start_standby(&node_file, &s, &managed).await;
    wait_for_node(&controller, standby, true, Duration::from_secs(30)).await;
    let c = fx.services.clone();

    // 备机上已有的主播 a，用备机的模板
    let (a_url, n_url) = (
        "https://stuck.example/back-a",
        "https://stuck.example/back-n",
    );
    let t1 = local_template(&s, "备机模板").await;
    let a = local_streamer(
        &s,
        serde_json::json!({ "url": a_url, "remark": "备机A", "upload_streamers_id": t1 }),
    )
    .await;
    let designate: Designate =
        serde_json::from_value(serde_json::json!({ "standby": standby, "mode": 1 })).unwrap();
    pairing
        .designate(&controller, designate)
        .await
        .unwrap()
        .unwrap();

    let fleet_room = |url: &'static str| {
        let controller = controller.clone();
        async move {
            super::assignments::list_rooms(controller.pool())
                .await
                .unwrap()
                .into_iter()
                .find(|room| room.spec.url == url && room.deleted_at.is_none())
        }
    };
    let on_primary = move |room: Option<super::assignments::Room>| {
        room.is_some_and(|room| room.node_id == Some(primary))
    };
    let paired = |id: i64| {
        managed
            .read()
            .unwrap()
            .as_ref()
            .and_then(|view| view.pair.clone())
            .is_some_and(|pair| pair.streamers.contains(&id))
    };
    eventually("a joins the pair", Duration::from_secs(40), || {
        let c = c.clone();
        async move {
            on_primary(fleet_room(a_url).await)
                && paired(a)
                && remark_on(&c, a_url).await.as_deref() == Some("备机A")
        }
    })
    .await;
    let room_a = fleet_room(a_url).await.unwrap();

    // 配对期间：主机上改 a 的备注，备机上新建主播 n
    let update: super::controller::UpdateRoom = serde_json::from_value(serde_json::json!({
        "url": a_url, "remark": "主机改的", "template_id": room_a.template_id,
    }))
    .unwrap();
    controller
        .update_room(room_a.id, update, true)
        .await
        .unwrap();
    let n = local_streamer(
        &s,
        serde_json::json!({ "url": n_url, "remark": "备机新建" }),
    )
    .await;
    member_for(&s).expect("备机上有同步端").join_room(n).await;
    eventually(
        "the edit and the new streamer converge",
        Duration::from_secs(40),
        || {
            let (s, c) = (s.clone(), c.clone());
            async move {
                remark_on(&s, a_url).await.as_deref() == Some("主机改的")
                    && on_primary(fleet_room(n_url).await)
                    && paired(n)
                    && remark_on(&c, n_url).await.as_deref() == Some("备机新建")
            }
        },
    )
    .await;
    let worker_a = s.managers.get_room_by_id(a).await.unwrap();

    // 「本机」上 a 的上一场还没投完
    let local_a = row_by_url(&c, a_url).await.unwrap().id;
    let busy_a = c.managers.get_room_by_id(local_a).await.unwrap();
    *busy_a.uploader_status.write().unwrap() = WorkerStatus::Pending;

    // 解除之前面板就能列出会交还的：从备机纳入的 a 与它的模板；配对期间新建的 n 留在主机
    let view = pairing.view(&controller).await.unwrap();
    assert_eq!(
        view["returns"],
        serde_json::json!({ "rooms": [room_a.id], "templates": [room_a.template_id.unwrap()] })
    );

    assert!(pairing.dissolve(&controller).await.unwrap());
    let stage = || {
        let (pairing, controller) = (pairing.clone(), controller.clone());
        async move {
            let view = pairing.view(&controller).await.unwrap();
            view["handback"][standby.to_string().as_str()]["rooms"][room_a.id.to_string().as_str()]
                ["stage"]
                .clone()
        }
    };
    eventually(
        "the standby holds a and drops n",
        Duration::from_secs(40),
        || {
            let s = s.clone();
            async move { stage().await == "held" && !sorted_urls(&s).await.contains(&n_url.into()) }
        },
    )
    .await;
    assert!(
        super::ha::hold_recording(a_url).is_some(),
        "交还中备机不录 a"
    );
    assert!(
        Arc::ptr_eq(&worker_a, &s.managers.get_room_by_id(a).await.unwrap()),
        "交还中是原来那一行，监控不重建"
    );
    tokio::time::sleep(Duration::from_secs(12)).await;
    assert_eq!(stage().await, "held", "「本机」上那一场投完之前不交接");
    assert!(on_primary(fleet_room(a_url).await));
    assert_eq!(row_by_url(&c, a_url).await.map(|row| row.id), Some(local_a));

    // 投完了：交接
    *busy_a.uploader_status.write().unwrap() = WorkerStatus::Idle;
    eventually(
        "a goes back to the standby",
        Duration::from_secs(60),
        || {
            let (pairing, controller, c) = (pairing.clone(), controller.clone(), c.clone());
            async move {
                fleet_room(a_url).await.is_none()
                    && row_by_url(&c, a_url).await.is_none()
                    && pairing
                        .view(&controller)
                        .await
                        .unwrap()
                        .get("handback")
                        .is_none()
                    && template_names(&c).await.is_empty()
            }
        },
    )
    .await;
    let back = row_by_url(&s, a_url).await.unwrap();
    assert_eq!((back.id, back.remark.as_str()), (a, "主机改的"));
    assert_eq!(back.upload_streamers_id, Some(t1));
    assert!(
        Arc::ptr_eq(&worker_a, &s.managers.get_room_by_id(a).await.unwrap()),
        "交接不重建监控"
    );
    assert!(
        super::ha::hold_recording(a_url).is_none(),
        "交接完备机照常录"
    );
    let view = managed.read().unwrap().clone().unwrap();
    assert!(
        view.streamers.is_empty() && view.templates.is_empty() && view.pair.is_none(),
        "a 与它的模板是备机的本地行"
    );
    assert_eq!(template_names(&s).await, ["备机模板"]);
    assert!(controller.templates().await.unwrap().is_empty());

    // 配对期间新建的 n 留在主机
    assert!(on_primary(fleet_room(n_url).await));
    assert_eq!(sorted_urls(&c).await, [n_url]);
    assert_eq!(sorted_urls(&s).await, [a_url]);
    assert!(!root.join("data/pair-holds.json").exists());
    assert!(!dir.path().join("pair-handback.json").exists());

    agent.shutdown().await;
    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

mod handback_actions;
mod split_frame;
