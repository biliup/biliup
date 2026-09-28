//! 解除配对后「待归还」的两个手动动作（H3）：放弃交还、立即交还。
//! 全程采样两台上每个交还房间「有这一行、没被挡开录」，断言任何时刻最多一台会开录；
//! 立即交还的前提（F2 释放那一行不打断在投的一场）单独用投稿替身跑一遍，断言只提交一次、不重传。

use super::*;
use crate::server::api::access;
use crate::server::fleet::ha::handback::blocked_on;
use crate::server::fleet::ha::member::member_for;
use crate::server::fleet::ha::pairing::{Designate, Pairing};
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::from_fn;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use tower::ServiceExt;

async fn send(app: &axum::Router<()>, method: Method, uri: &str) -> (StatusCode, Value) {
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
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn live_stream(url: &str) -> biliup::downloader::live::LiveStream {
    serde_json::from_value(json!({
        "name": "n", "url": url, "title": "直播标题", "date": "2026-01-01T00:00:00Z",
        "live_cover_url": "", "raw_stream_url": "http://127.0.0.1:9/x.flv", "platform": "stuck",
        "stream_headers": {}, "suffix": "flv", "danmaku": null, "downloader_hint": "StreamGears",
        "runtime_options": null,
    }))
    .unwrap()
}

/// 把这个房间标成正在录（不真的拉流）
fn mark_recording(worker: &Worker, url: &str) {
    use crate::server::common::download::DownloadTask;
    use crate::server::core::downloader::{DownloaderRuntime, DownloaderType};
    let task = DownloadTask::new(
        DownloaderRuntime::from_type(DownloaderType::StreamGears),
        &live_stream(url),
    );
    *worker.downloader_status.write().unwrap() = WorkerStatus::Working(Arc::new(task));
}

/// 这台会不会开录这个地址：有这一行、没被挡
async fn recordable(services: &ServiceRegister, url: &str) -> bool {
    row_by_url(services, url).await.is_some() && !blocked_on(services, url)
}

/// 后台一直看两台，记下同一个地址两台都会开录的时刻
fn watch_single_recorder(
    c: ServiceRegister,
    s: ServiceRegister,
    urls: Vec<&'static str>,
    stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<Vec<String>> {
    tokio::spawn(async move {
        let mut both = Vec::new();
        let mut samples = 0u64;
        while !stop.load(Ordering::Acquire) {
            for url in &urls {
                if recordable(&c, url).await && recordable(&s, url).await {
                    both.push(format!("{url} @ {}", now_ms()));
                }
            }
            samples += 1;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(samples > 50, "采样太少：{samples}");
        both
    })
}

/// 放弃交还（备机在线 / 离线）、立即交还（本机只在投）与各种做不了的情况；全程任何时刻一个地址最多一台会开录
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pending_handbacks_can_be_abandoned_or_handed_over_at_once() {
    let _role = super::super::ha::test_guard().await;
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
    let app = crate::server::api::fleet::router(controller.clone())
        .route_layer(from_fn(access::unrestricted));

    // 备机上已有的四个主播，a、b、d 用备机的模板
    let (a_url, b_url, c_url, d_url) = (
        "https://stuck.example/act-a",
        "https://stuck.example/act-b",
        "https://stuck.example/act-c",
        "https://stuck.example/act-d",
    );
    let t1 = local_template(&s, "备机模板").await;
    let mut ids = Vec::new();
    for (url, template) in [
        (a_url, Some(t1)),
        (b_url, Some(t1)),
        (c_url, None),
        (d_url, Some(t1)),
    ] {
        let fields = json!({ "url": url, "remark": url, "upload_streamers_id": template });
        ids.push(local_streamer(&s, fields).await);
    }
    let designate: Designate =
        serde_json::from_value(json!({ "standby": standby, "mode": 2 })).unwrap();
    pairing
        .designate(&controller, designate)
        .await
        .unwrap()
        .unwrap();
    let fleet_room = |url: &'static str| {
        let controller = controller.clone();
        async move {
            super::super::assignments::list_rooms(controller.pool())
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
    eventually("all four join the pair", Duration::from_secs(60), || {
        let ids = ids.clone();
        async move {
            for url in [a_url, b_url, c_url, d_url] {
                if !fleet_room(url)
                    .await
                    .is_some_and(|room| room.node_id == Some(primary))
                {
                    return false;
                }
            }
            ids.iter().all(|id| paired(*id))
        }
    })
    .await;
    let mut rooms = Vec::new();
    for url in [a_url, b_url, c_url, d_url] {
        rooms.push(fleet_room(url).await.unwrap().id);
    }
    let (room_a, room_b, room_c, room_d) = (rooms[0], rooms[1], rooms[2], rooms[3]);
    let fleet_t1 = fleet_room(a_url).await.unwrap().template_id.unwrap();

    // 「本机」上 a、b、d 还在投上一场，c 正在录
    let mut local_workers = Vec::new();
    for url in [a_url, b_url, c_url, d_url] {
        let id = row_by_url(&c, url).await.unwrap().id;
        local_workers.push(c.managers.get_room_by_id(id).await.unwrap());
    }
    for worker in [&local_workers[0], &local_workers[1], &local_workers[3]] {
        *worker.uploader_status.write().unwrap() = WorkerStatus::Pending;
    }
    mark_recording(&local_workers[2], c_url);

    assert!(pairing.dissolve(&controller).await.unwrap());
    let handback = || {
        let (pairing, controller) = (pairing.clone(), controller.clone());
        async move {
            pairing.view(&controller).await.unwrap()["handback"][standby.to_string().as_str()]
                .clone()
        }
    };
    let stage = |view: &Value, room: i64| view["rooms"][room.to_string().as_str()]["stage"].clone();
    eventually(
        "the standby holds all four",
        Duration::from_secs(40),
        || async move {
            let view = handback().await;
            [room_a, room_b, room_c, room_d]
                .iter()
                .all(|room| stage(&view, *room) == "held")
        },
    )
    .await;
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = watch_single_recorder(
        c.clone(),
        s.clone(),
        vec![a_url, b_url, c_url, d_url],
        stop.clone(),
    );
    let view = handback().await;
    assert_eq!(view["online"], true, "{view}");
    let local_of = |room: i64| view["rooms"][room.to_string().as_str()]["local"].clone();
    assert_eq!(local_of(room_b), "uploading");
    assert_eq!(local_of(room_c), "recording");

    let path =
        |kind: &str, id: i64, action: &str| format!("/v1/fleet/ha/handback/{kind}/{id}/{action}");
    let message = |body: &Value| body["message"].as_str().unwrap_or_default().to_string();
    let (status, body) = send(&app, Method::POST, &path("rooms", room_c, "force")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(message(&body).contains("正在录"), "{body}");
    let (status, body) = send(&app, Method::POST, &path("templates", fleet_t1, "abandon")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(message(&body).contains("还在用这个模板"), "{body}");
    let (status, _) = send(&app, Method::POST, &path("templates", fleet_t1, "force")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(&app, Method::POST, &path("room", room_a, "abandon")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(&app, Method::POST, &path("rooms", room_a, "drop")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(&app, Method::POST, &path("rooms", 999_999, "abandon")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&app, Method::GET, &path("rooms", room_a, "abandon")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    // 立即交还 b：「本机」只在投、没在录，不等投完
    let (status, body) = send(&app, Method::POST, &path("rooms", room_b, "force")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    eventually(
        "b goes back while the upload is still running",
        Duration::from_secs(40),
        || {
            let (c, s) = (c.clone(), s.clone());
            async move {
                fleet_room(b_url).await.is_none()
                    && row_by_url(&c, b_url).await.is_none()
                    && recordable(&s, b_url).await
                    && handback().await["rooms"].get(room_b.to_string()).is_none()
            }
        },
    )
    .await;
    assert!(
        matches!(
            *local_workers[1].uploader_status.read().unwrap(),
            WorkerStatus::Pending
        ),
        "交接时「本机」那一场还在投"
    );
    assert_eq!(
        row_by_url(&s, b_url).await.unwrap().id,
        ids[1],
        "原来那一行"
    );

    // 放弃交还 a（备机在线）：a 留在主机，备机那一行撤掉
    let (status, body) = send(&app, Method::POST, &path("rooms", room_a, "abandon")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    eventually("a stays on the primary", Duration::from_secs(40), || {
        let (c, s) = (c.clone(), s.clone());
        async move {
            row_by_url(&s, a_url).await.is_none()
                && !blocked_on(&s, a_url)
                && recordable(&c, a_url).await
                && fleet_room(a_url)
                    .await
                    .is_some_and(|room| room.node_id == Some(primary))
        }
    })
    .await;
    let (status, _) = send(&app, Method::POST, &path("rooms", room_a, "abandon")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "放弃过的不在交还中");

    // 备机离线：不能立即交还，可以放弃；放弃的 c 在它回来之前那一行仍挡着
    agent.shutdown().await;
    wait_for_node(&controller, standby, false, Duration::from_secs(30)).await;
    let (status, body) = send(&app, Method::POST, &path("rooms", room_d, "force")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(message(&body).contains("离线"), "{body}");
    assert_eq!(handback().await["online"], false);
    let (status, body) = send(&app, Method::POST, &path("rooms", room_c, "abandon")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(blocked_on(&s, c_url), "离线的备机接着挡");
    *local_workers[2].downloader_status.write().unwrap() = WorkerStatus::Idle;
    *local_workers[3].uploader_status.write().unwrap() = WorkerStatus::Idle;
    tokio::time::sleep(Duration::from_secs(7)).await;
    assert_eq!(stage(&handback().await, room_d), "held", "离线时不交接");
    assert!(fleet_room(d_url).await.is_some());

    // 备机回来：c 那一行撤掉，d 照常交接，模板交还（主机上的 a 还在用，主机留一份）
    let agent = start_standby(&node_file, &s, &managed).await;
    wait_for_node(&controller, standby, true, Duration::from_secs(30)).await;
    eventually("the rest settles", Duration::from_secs(60), || {
        let (c, s, pairing, controller) =
            (c.clone(), s.clone(), pairing.clone(), controller.clone());
        async move {
            row_by_url(&s, c_url).await.is_none()
                && !blocked_on(&s, c_url)
                && fleet_room(d_url).await.is_none()
                && row_by_url(&c, d_url).await.is_none()
                && recordable(&s, d_url).await
                && pairing
                    .view(&controller)
                    .await
                    .unwrap()
                    .get("handback")
                    .is_none()
        }
    })
    .await;
    stop.store(true, Ordering::Release);
    let both = watcher.await.unwrap();
    assert!(both.is_empty(), "同一个地址两台都会开录：{both:?}");

    assert_eq!(sorted_urls(&s).await, [b_url, d_url]);
    assert_eq!(sorted_urls(&c).await, [a_url, c_url]);
    assert_eq!(template_names(&s).await, ["备机模板"]);
    assert!(
        controller
            .templates()
            .await
            .unwrap()
            .iter()
            .any(|t| t.id == fleet_t1),
        "主机上的 a 还在用这个模板"
    );
    let view = managed.read().unwrap().clone().unwrap();
    assert!(view.streamers.is_empty() && view.pair.is_none(), "{view:?}");
    assert!(member_for(&s).is_none());
    assert!(!root.join("data/pair-holds.json").exists());
    assert!(!dir.path().join("pair-handback.json").exists());

    agent.shutdown().await;
    pairing.shutdown();
    local.shutdown().await;
    controller.shutdown().await;
}

/// 立即交还的前提：「本机」按 F2 释放撤掉一个房间（删监控、删那一行）时，这个房间在投的一场接着投完，
/// 不中断、不重传，只提交一次
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_upload_in_flight_finishes_once_when_its_row_is_released() {
    use crate::server::common::upload::process_with_upload;
    use crate::server::fleet::ha::params::{HaMode, HaParams};
    use crate::server::fleet::ha::primary::Primary;
    use crate::server::fleet::ha::upload::double::{self, Double};
    use crate::server::fleet::ha::wire::HaMessage;
    use crate::server::fleet::ha::{Link, Role, UnitOutput, set_role};
    use crate::server::infrastructure::context::Context;
    use futures::StreamExt;

    let _role = super::super::ha::test_guard().await;
    let dir = tempfile::tempdir().unwrap();
    let control = dir.path().join("control");
    std::fs::create_dir_all(&control).unwrap();
    std::fs::write(control.join("stall"), "").unwrap();
    let double = Arc::new(Double::new(
        dir.path().join("double.jsonl"),
        control.clone(),
        0,
        "P",
    ));
    double::install(Some(double.clone()));

    let services = node_services(&dir.path().join("local")).await;
    let url = "https://stuck.example/in-flight";
    let template = local_template(&services, "模板").await;
    let id = local_streamer(
        &services,
        json!({ "url": url, "remark": "在投", "upload_streamers_id": template }),
    )
    .await;
    let worker = services.managers.get_room_by_id(id).await.unwrap();

    let fleet = ConnectionManager::new_pool_with(
        dir.path().join("fleet.sqlite3").to_str().unwrap(),
        &FLEET_MIGRATOR,
    )
    .await
    .unwrap();
    let primary = Primary::start(
        fleet,
        services.clone(),
        HaMode::DualRecord,
        HaParams::default(),
        10 * 60_000,
    )
    .await
    .unwrap();
    primary.set_rooms(std::collections::HashMap::from([(url.to_string(), 7)]));
    let (outbox, _frames) = tokio::sync::mpsc::unbounded_channel();
    primary.connected(Link::Controller(outbox));
    primary.standby_message(HaMessage::StandbyReport {
        sessions: Vec::new(),
    });
    set_role(Some(Role::Primary(primary.clone())));

    let ctx = Context::new(1, worker.clone(), services.pool.clone(), live_stream(url));
    crate::server::fleet::ha::unit_started(&ctx);
    let recording = dir.path().join("rec");
    std::fs::create_dir_all(&recording).unwrap();
    let (tx, rx) = async_channel::unbounded();
    for (name, size) in [("p1.flv", 3000), ("p2.flv", 2000)] {
        let path = recording.join(name);
        std::fs::write(&path, vec![0u8; size]).unwrap();
        tx.send(crate::server::core::downloader::SegmentInfo::new(
            path, None, None, 0,
        ))
        .await
        .unwrap();
    }
    drop(tx);
    crate::server::fleet::ha::unit_ended(&ctx, UnitOutput { seen: 2, sent: 2 });
    *worker.uploader_status.write().unwrap() = WorkerStatus::Pending;
    let config = ctx.upload_config().clone().unwrap();
    let upload =
        tokio::spawn(async move { process_with_upload(rx.inspect(|_| {}), &ctx, &config).await });
    let ops = |double: &Double| -> Vec<String> {
        double
            .entries()
            .iter()
            .map(|entry| entry["op"].as_str().unwrap_or_default().to_string())
            .collect()
    };
    eventually("the upload starts", Duration::from_secs(20), || {
        let double = double.clone();
        async move { ops(&double).contains(&"upload_start".to_string()) }
    })
    .await;

    crate::server::services::streamers::delete_streamer(&services.pool, &services.managers, id)
        .await
        .unwrap();
    assert!(row_by_url(&services, url).await.is_none());
    assert!(services.managers.get_room_by_id(id).await.is_none());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!upload.is_finished(), "释放那一行不打断在投的一场");

    std::fs::remove_file(control.join("stall")).unwrap();
    tokio::time::timeout(Duration::from_secs(30), upload)
        .await
        .expect("放开之后投完")
        .unwrap()
        .unwrap();
    let ops = ops(&double);
    let count = |op: &str| ops.iter().filter(|o| o.as_str() == op).count();
    assert_eq!(
        (count("login"), count("upload_start"), count("submit")),
        (1, 2, 1),
        "{ops:?}"
    );
    primary.flush().await;
    set_role(None);
    double::install(None);
}
