//! 控制面的一帧分两段到达、两段之间节点会话里别的分支先就绪：节点照样收下整帧，之后照常收发。
//! 控制面换成按脚本发帧的假控制面，经内嵌 relay 连真实的节点代理。

use super::*;
use crate::server::fleet::protocol::{self, ControllerMessage, DesiredState, Event, NodeMessage};
use crate::server::fleet::{bind_endpoint, events};
use iroh::endpoint::RecvStream;
use iroh::{RelayConfig, RelayMap, SecretKey};
use tokio::time::timeout;

const REPLY_WITHIN: Duration = Duration::from_secs(5);
const PROBE_EVENT: &str = "split_frame_probe";

fn encoded(message: &ControllerMessage) -> Vec<u8> {
    let body = serde_json::to_vec(message).unwrap();
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend(body);
    frame
}

fn desired(version: u64) -> ControllerMessage {
    ControllerMessage::DesiredState(DesiredState {
        version,
        ..DesiredState::default()
    })
}

/// 读到节点发来的下一个想要的帧；心跳与别的事件跳过
async fn from_node(recv: &mut RecvStream, what: &str, wanted: impl Fn(&NodeMessage) -> bool) {
    let found = timeout(REPLY_WITHIN, async {
        loop {
            let message = protocol::read_frame::<_, NodeMessage>(&mut *recv)
                .await
                .unwrap()
                .expect("the node closed its stream");
            if wanted(&message) {
                break;
            }
        }
    })
    .await;
    assert!(
        found.is_ok(),
        "the node did not send {what} within {REPLY_WITHIN:?}"
    );
}

async fn acked(recv: &mut RecvStream, version: u64) {
    from_node(
        recv,
        &format!("the ack of version {version}"),
        |message| matches!(message, NodeMessage::Ack(ack) if ack.version == version),
    )
    .await;
}

/// 让节点会话里的事件分支就绪，等它把这条事件转上来
async fn reported(recv: &mut RecvStream, seq: i64) {
    events::inject(Event {
        kind: PROBE_EVENT.into(),
        at: seq,
        detail: serde_json::Value::Null,
    });
    from_node(recv, &format!("probe event {seq}"), |message| {
        matches!(message, NodeMessage::Event(event) if event.kind == PROBE_EVENT && event.at == seq)
    })
    .await;
}

/// 必须是单线程 runtime（`#[tokio::test]` 的缺省）：节点、relay 与假控制面轮流跑。节点发完 `Ack` 在同一轮里
/// 就进下一次 `select!`，读帧那一支把已经到了的半帧收进去才让出，假控制面这才读得到这个 `Ack`
#[tokio::test]
async fn a_controller_frame_split_around_another_ready_branch_is_read_whole() {
    let _guard = events::test_guard().await;
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
    let relays = RelayMap::from_iter([RelayConfig::new(url.clone().into(), None)]);
    let endpoint = bind_endpoint(secret.clone(), relays, true).await.unwrap();

    let node_secret = SecretKey::generate();
    let now = now_ms();
    let (token, join_secret) = store::create_token(&pool, None, now, now + 60_000)
        .await
        .unwrap();
    let store::Redeem::Joined(row) = store::redeem_token(
        &pool,
        &token.id,
        &join_secret,
        &node_secret.public().to_string(),
        "split",
        false,
        now,
    )
    .await
    .unwrap() else {
        panic!("the join token was not redeemed");
    };
    let node_file = dir.path().join("node/data/node.json");
    node::NodeFile::new(
        secret.public(),
        vec![url.to_string()],
        row.id,
        &node_secret,
        false,
    )
    .save(&node_file)
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

    let connection = timeout(Duration::from_secs(30), async {
        endpoint.accept().await.unwrap().await.unwrap()
    })
    .await
    .expect("the node did not connect");
    let (mut send, mut recv) = connection.accept_bi().await.unwrap();
    let hello = protocol::read_frame::<_, NodeMessage>(&mut recv)
        .await
        .unwrap();
    assert!(matches!(hello, Some(NodeMessage::Hello(_))), "{hello:?}");
    let welcome = ControllerMessage::Welcome {
        node_id: row.id,
        relays: Vec::new(),
    };
    send.write_all(&encoded(&welcome)).await.unwrap();

    // 第 2 版的长度与前半截跟着第 1 版一起发出去：节点落地第 1 版、回完 `Ack` 就把这半帧读进来等后半截
    let second = encoded(&desired(2));
    let (head, tail) = second.split_at(second.len() / 2);
    let mut first = encoded(&desired(1));
    first.extend_from_slice(head);
    send.write_all(&first).await.unwrap();
    acked(&mut recv, 1).await;
    reported(&mut recv, 1).await;
    send.write_all(tail).await.unwrap();
    acked(&mut recv, 2).await;

    send.write_all(&encoded(&desired(3))).await.unwrap();
    acked(&mut recv, 3).await;
    reported(&mut recv, 2).await;

    agent.shutdown().await;
    endpoint.close().await;
    relay.shutdown().await;
}
