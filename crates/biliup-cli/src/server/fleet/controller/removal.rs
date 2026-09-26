//! 「移除并自动改派」（`DELETE /v1/fleet/nodes/{id}?reassign=auto`）。
//!
//! 在线节点先按正常迁移把房间交出去：每个房间改派到按负载选出的节点（先释放后接手），
//! 等它在 `Ack` 里确认释放全部房间、或离线、或超过 [`REMOVAL_WAIT`]，再吊销。新节点在它停下之后才接手，
//! 不会重复录制。离线（或收不了期望状态）的节点没法确认释放，直接吊销再改派；它离线期间两边可能同时录，
//! 它连回来发现被移除后会把这些房间转成本地并暂停（见 `node.rs`）。
//!
//! 等待在后台做，接口当场返回 202；进度与结果放在内存里，由 `GET /v1/fleet/nodes` 的 `removals` 带给界面。
//! 控制面在等待中途重启时进度随之丢失：房间已经改派，迁移照常完成，节点没被吊销，界面上可以再移除一次。

use super::{Controller, DispatchError};
use crate::server::fleet::assignments::{self, Room};
use crate::server::fleet::now_ms;
use crate::server::fleet::store;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::Instant;
use tracing::{info, warn};

/// 在线节点确认释放的等待上限。释放是一次期望状态往返加上停掉录制（收尾当前分段、写库），
/// 正常几秒内完成（F2 实测旧节点重连后 2.5–3.6 s 按 epoch 释放）；60 s 给慢盘收尾留足余量，
/// 又不让节点长时间停在「移除中」。超时照样吊销：节点被吊销后会把没交出去的房间转成本地并暂停，
/// 重叠只到它收到吊销为止。
pub const REMOVAL_WAIT: Duration = Duration::from_secs(60);
/// 没有 `Ack` 唤醒时多久重新查一次（兜底，正常由 `Ack` 与掉线唤醒）
const RECHECK: Duration = Duration::from_secs(1);
/// 移除结束后结果留多久，给没开着页面的管理员回来看
const REMOVAL_KEEP_MS: i64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemovalState {
    /// 等节点确认释放
    Removing,
    /// 已吊销
    Done,
}

/// 一个房间在原节点上的释放情况
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Release {
    /// 还在等原节点确认
    Waiting,
    /// 原节点已确认释放，新节点接手时它已经停了
    Released,
    /// 原节点离线，没等到确认就吊销了；它发现被移除后会暂停这个房间
    Offline,
    /// 等满 [`REMOVAL_WAIT`] 没确认，吊销了；它发现被移除后会暂停这个房间
    Timeout,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemovedRoom {
    pub room_id: i64,
    pub remark: String,
    /// 改派到的节点；`None` 为没找到合适的节点、留在未分派，原因在 `unplaced`
    pub node_id: Option<i64>,
    pub unplaced: Option<String>,
    pub release: Release,
}

/// 一次「移除并自动改派」的进度与结果
#[derive(Debug, Clone, Serialize)]
pub struct Removal {
    pub node_id: i64,
    pub node_name: String,
    pub state: RemovalState,
    pub started_at: i64,
    /// 最晚这时吊销
    pub deadline: i64,
    pub finished_at: Option<i64>,
    pub rooms: Vec<RemovedRoom>,
}

/// 等待中每次查到的情况
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Probe {
    /// 还在等这台节点释放的房间数
    waiting: usize,
    online: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settled {
    Released,
    Offline,
    Timeout,
}

/// 等到没有房间在等这台节点释放、节点离线或到 `deadline`，按这个顺序判断
async fn settle<F, Fut>(deadline: Instant, wake: &Notify, mut probe: F) -> Settled
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Probe>,
{
    loop {
        let now = probe().await;
        if now.waiting == 0 {
            return Settled::Released;
        }
        if !now.online {
            return Settled::Offline;
        }
        if Instant::now() >= deadline {
            return Settled::Timeout;
        }
        tokio::select! {
            () = wake.notified() => {}
            () = tokio::time::sleep(RECHECK) => {}
            () = tokio::time::sleep_until(deadline) => {}
        }
    }
}

fn prune(removals: &mut HashMap<i64, Removal>, now: i64) {
    removals.retain(|_, removal| {
        removal
            .finished_at
            .is_none_or(|finished| now - finished < REMOVAL_KEEP_MS)
    });
}

impl Controller {
    /// 这台节点正在「移除并自动改派」：不再接收新房间
    pub(crate) fn is_removing(&self, id: i64) -> bool {
        self.removals
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|removal| removal.state == RemovalState::Removing)
    }

    /// 进行中与最近结束的移除，新的在前
    pub fn removals(&self) -> Vec<Removal> {
        let mut removals = self.removals.lock().unwrap();
        prune(&mut removals, now_ms());
        let mut list: Vec<Removal> = removals.values().cloned().collect();
        list.sort_by_key(|removal| std::cmp::Reverse(removal.started_at));
        list
    }

    /// `Ack` 确认了释放、节点掉线时唤醒等待中的移除
    pub(super) fn wake_removals(&self) {
        self.released.notify_waiters();
    }

    async fn place(&self, room: &Room) -> Result<i64, DispatchError> {
        let template = self.template_or_invalid(room.template_id).await?;
        let target = self.auto_node(&room.spec, template.as_ref()).await?;
        self.assign(room.id, Some(target), false).await?;
        Ok(target)
    }

    /// 移除节点，并把分派给它的房间按负载改派到其他节点。
    ///
    /// 节点在线时先迁移、等它确认释放再吊销，返回的 `state` 为 `removing`，结果随后出现在 [`Self::removals`]；
    /// 离线时当场吊销并改派，返回 `done`。返回 `None` 表示节点不存在或已被移除。
    pub async fn revoke_and_reassign(
        self: &Arc<Self>,
        id: i64,
    ) -> Result<Option<Removal>, DispatchError> {
        let Some(node) = store::node(&self.pool, id)
            .await?
            .filter(|node| node.revoked_at.is_none())
        else {
            return Ok(None);
        };
        let now = now_ms();
        let online = self
            .live
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|live| live.online(now) && live.accepts_desired_state());
        let deadline = now + i64::try_from(REMOVAL_WAIT.as_millis()).unwrap_or(i64::MAX);
        {
            let mut removals = self.removals.lock().unwrap();
            prune(&mut removals, now);
            if removals
                .get(&id)
                .is_some_and(|removal| removal.state == RemovalState::Removing)
            {
                return Err(DispatchError::Conflict(format!(
                    "节点「{}」正在移除：等它确认释放房间，最多 {} 秒",
                    node.name,
                    REMOVAL_WAIT.as_secs()
                )));
            }
            // 先挂上「移除中」，下面自动选节点时就不会再选回它
            removals.insert(
                id,
                Removal {
                    node_id: id,
                    node_name: node.name.clone(),
                    state: RemovalState::Removing,
                    started_at: now,
                    deadline,
                    finished_at: None,
                    rooms: Vec::new(),
                },
            );
        }
        let rooms: Vec<Room> = assignments::list_rooms(&self.pool)
            .await?
            .into_iter()
            .filter(|room| room.node_id == Some(id) && room.deleted_at.is_none())
            .collect();
        let result = if online {
            self.migrate_then_revoke(id, rooms, deadline).await
        } else {
            self.revoke_then_place(id, rooms).await
        };
        let mut removals = self.removals.lock().unwrap();
        match result {
            Ok(Some(removal)) => {
                removals.insert(id, removal.clone());
                Ok(Some(removal))
            }
            other => {
                removals.remove(&id);
                other
            }
        }
    }

    /// 离线节点：没法等它确认，先吊销（房间当场变成未分派），再改派
    async fn revoke_then_place(
        &self,
        id: i64,
        rooms: Vec<Room>,
    ) -> Result<Option<Removal>, DispatchError> {
        let Some(mut removal) = self.removals.lock().unwrap().get(&id).cloned() else {
            return Ok(None);
        };
        if !self.revoke(id).await? {
            return Ok(None);
        }
        for room in rooms {
            let placed = self.place(&room).await;
            removal.rooms.push(RemovedRoom {
                room_id: room.id,
                remark: room.spec.remark.clone(),
                node_id: placed.as_ref().ok().copied(),
                unplaced: placed.err().map(|error| error.message()),
                release: Release::Offline,
            });
        }
        removal.state = RemovalState::Done;
        removal.finished_at = Some(now_ms());
        info!(
            node = id,
            rooms = removal.rooms.len(),
            "offline fleet node revoked with automatic reassignment"
        );
        Ok(Some(removal))
    }

    /// 在线节点：房间按正常迁移交出去（找不到去处的取消分派，同样让它先停），后台等它确认释放再吊销
    async fn migrate_then_revoke(
        self: &Arc<Self>,
        id: i64,
        rooms: Vec<Room>,
        deadline: i64,
    ) -> Result<Option<Removal>, DispatchError> {
        let Some(mut removal) = self.removals.lock().unwrap().get(&id).cloned() else {
            return Ok(None);
        };
        for room in rooms {
            let placed = self.place(&room).await;
            let unplaced = match &placed {
                Ok(_) => None,
                Err(error) => {
                    if let Err(e) = self.assign(room.id, None, false).await {
                        warn!(
                            room = room.id,
                            error = e.message(),
                            "could not unassign a room from a node being removed"
                        );
                    }
                    Some(error.message())
                }
            };
            let current = assignments::room(&self.pool, room.id).await?;
            let release = if current.is_some_and(|room| room.releasing_node_id == Some(id)) {
                Release::Waiting
            } else {
                Release::Released
            };
            removal.rooms.push(RemovedRoom {
                room_id: room.id,
                remark: room.spec.remark.clone(),
                node_id: placed.ok(),
                unplaced,
                release,
            });
        }
        info!(
            node = id,
            rooms = removal.rooms.len(),
            "fleet node removal started, waiting for it to release its rooms"
        );
        let wait = Duration::from_millis(u64::try_from(deadline - now_ms()).unwrap_or(0));
        let controller = Arc::clone(self);
        let started = removal.clone();
        tokio::spawn(async move {
            controller
                .finish_removal(started, Instant::now() + wait)
                .await;
        });
        Ok(Some(removal))
    }

    async fn release_probe(&self, id: i64) -> Probe {
        let waiting = match assignments::list_rooms(&self.pool).await {
            Ok(rooms) => rooms
                .iter()
                .filter(|room| room.releasing_node_id == Some(id))
                .count(),
            Err(e) => {
                warn!(node = id, error = ?e, "could not read rooms while removing a node");
                usize::MAX
            }
        };
        let now = now_ms();
        let online = self
            .live
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|live| live.online(now));
        Probe { waiting, online }
    }

    async fn finish_removal(&self, mut removal: Removal, deadline: Instant) {
        let id = removal.node_id;
        let settled = settle(deadline, &self.released, || self.release_probe(id)).await;
        let unconfirmed = match settled {
            Settled::Released | Settled::Offline => Release::Offline,
            Settled::Timeout => Release::Timeout,
        };
        for room in &mut removal.rooms {
            if room.release != Release::Waiting {
                continue;
            }
            let still_held = assignments::room(&self.pool, room.room_id)
                .await
                .ok()
                .flatten()
                .is_some_and(|current| current.releasing_node_id == Some(id));
            room.release = if still_held {
                unconfirmed
            } else {
                Release::Released
            };
        }
        match self.revoke(id).await {
            Ok(_) => {}
            Err(e) => {
                warn!(node = id, error = ?e, "could not revoke a node after migrating its rooms")
            }
        }
        removal.state = RemovalState::Done;
        removal.finished_at = Some(now_ms());
        info!(
            node = id,
            ?settled,
            released = removal
                .rooms
                .iter()
                .filter(|room| room.release == Release::Released)
                .count(),
            rooms = removal.rooms.len(),
            "fleet node removed after migrating its rooms"
        );
        self.removals.lock().unwrap().insert(id, removal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn probe(waiting: usize, online: bool) -> Probe {
        Probe { waiting, online }
    }

    #[tokio::test(start_paused = true)]
    async fn settles_as_soon_as_everything_is_released() {
        let wake = Notify::new();
        let calls = AtomicUsize::new(0);
        let started = Instant::now();
        let settled = settle(started + REMOVAL_WAIT, &wake, || {
            let call = calls.fetch_add(1, Ordering::Relaxed);
            async move { probe(2usize.saturating_sub(call), true) }
        })
        .await;
        assert_eq!(settled, Settled::Released);
        assert!(started.elapsed() < REMOVAL_WAIT);
    }

    #[tokio::test(start_paused = true)]
    async fn an_offline_node_is_not_waited_for() {
        let wake = Notify::new();
        let started = Instant::now();
        let settled = settle(started + REMOVAL_WAIT, &wake, || async { probe(3, false) }).await;
        assert_eq!(settled, Settled::Offline);
        assert_eq!(started.elapsed(), Duration::ZERO);
        // 释放完了就不算离线
        let settled = settle(started + REMOVAL_WAIT, &wake, || async { probe(0, false) }).await;
        assert_eq!(settled, Settled::Released);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_at_the_deadline_while_the_node_holds_on() {
        let wake = Notify::new();
        let started = Instant::now();
        let settled = settle(started + REMOVAL_WAIT, &wake, || async { probe(1, true) }).await;
        assert_eq!(settled, Settled::Timeout);
        assert_eq!(started.elapsed(), REMOVAL_WAIT);
    }

    #[tokio::test(start_paused = true)]
    async fn a_node_going_offline_midway_ends_the_wait() {
        let wake = Notify::new();
        let started = Instant::now();
        let offline_at = started + Duration::from_secs(5);
        let settled = settle(started + REMOVAL_WAIT, &wake, || async move {
            probe(1, Instant::now() < offline_at)
        })
        .await;
        assert_eq!(settled, Settled::Offline);
        assert!(started.elapsed() >= Duration::from_secs(5));
        assert!(started.elapsed() < Duration::from_secs(7));
    }

    async fn controller(dir: &std::path::Path) -> Arc<Controller> {
        use crate::server::fleet::FLEET_MIGRATOR;
        use crate::server::infrastructure::connection_pool::ConnectionManager;
        let pool = ConnectionManager::new_pool_with(
            dir.join("fleet.sqlite3").to_str().unwrap(),
            &FLEET_MIGRATOR,
        )
        .await
        .unwrap();
        let secret = store::identity(&pool, now_ms()).await.unwrap();
        let relay: url::Url = "http://192.168.7.2:19160/".parse().unwrap();
        let relays = super::super::RelaySetup {
            local: vec![relay.clone()],
            advertised: vec![relay],
            embedded_port: None,
        };
        Controller::start(pool, secret, relays, None).await.unwrap()
    }

    async fn joined(controller: &Controller, endpoint: &str) -> i64 {
        let (token, secret) = store::create_token(controller.pool(), None, 1, i64::MAX)
            .await
            .unwrap();
        let store::Redeem::Joined(node) = store::redeem_token(
            controller.pool(),
            &token.id,
            &secret,
            endpoint,
            endpoint,
            false,
            2,
        )
        .await
        .unwrap() else {
            panic!("token was not redeemed")
        };
        node.id
    }

    fn room_on(node: Option<i64>, url: &str, auto: bool) -> super::super::CreateRoom {
        serde_json::from_value(serde_json::json!({
            "url": url,
            "remark": "房间",
            "node_id": node,
            "auto_node": auto,
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn a_node_being_removed_takes_no_rooms_and_cannot_be_removed_twice() {
        let dir = tempfile::tempdir().unwrap();
        let controller = controller(dir.path()).await;
        let a = joined(&controller, "aa").await;
        let room = controller
            .create_room(room_on(Some(a), "https://live.example/1", false))
            .await
            .unwrap();
        controller.removals.lock().unwrap().insert(
            a,
            Removal {
                node_id: a,
                node_name: "aa".into(),
                state: RemovalState::Removing,
                started_at: now_ms(),
                deadline: now_ms() + 60_000,
                finished_at: None,
                rooms: Vec::new(),
            },
        );
        let rejected = controller
            .create_room(room_on(Some(a), "https://live.example/2", false))
            .await
            .unwrap_err();
        assert!(
            matches!(&rejected, DispatchError::Invalid(m) if m.contains("正在移除")),
            "{rejected:?}"
        );
        let rejected = controller
            .create_room(room_on(None, "https://live.example/2", true))
            .await
            .unwrap_err();
        assert!(
            matches!(&rejected, DispatchError::Invalid(m) if m.contains("「aa」正在移除")),
            "{rejected:?}"
        );
        let rejected = controller.revoke_and_reassign(a).await.unwrap_err();
        assert!(
            matches!(rejected, DispatchError::Conflict(_)),
            "{rejected:?}"
        );
        assert!(controller.nodes().await.unwrap()[0].removing);

        // 离线节点：当场吊销再改派，房间标「节点离线」
        controller.removals.lock().unwrap().clear();
        let done = controller.revoke_and_reassign(a).await.unwrap().unwrap();
        assert_eq!(done.state, RemovalState::Done);
        assert_eq!(done.rooms.len(), 1);
        assert_eq!(done.rooms[0].room_id, room.id);
        assert_eq!(done.rooms[0].release, Release::Offline);
        assert_eq!(done.rooms[0].node_id, None);
        assert!(done.rooms[0].unplaced.is_some());
        assert_eq!(controller.removals()[0].node_id, a);
        assert!(!controller.is_removing(a));
        let orphan = assignments::room(controller.pool(), room.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((orphan.node_id, orphan.releasing_node_id), (None, None));
        assert!(controller.revoke_and_reassign(a).await.unwrap().is_none());
        controller.shutdown().await;
    }

    #[test]
    fn finished_removals_are_kept_for_a_while() {
        let removal = |finished_at| Removal {
            node_id: 1,
            node_name: "a".into(),
            state: RemovalState::Done,
            started_at: 0,
            deadline: 0,
            finished_at,
            rooms: Vec::new(),
        };
        let mut removals = HashMap::from([
            (1, removal(Some(1_000))),
            (2, removal(Some(1_000 + REMOVAL_KEEP_MS))),
            (3, removal(None)),
        ]);
        prune(&mut removals, 1_000 + REMOVAL_KEEP_MS);
        let mut kept: Vec<i64> = removals.keys().copied().collect();
        kept.sort_unstable();
        assert_eq!(kept, [2, 3]);
    }
}
