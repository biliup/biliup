//! 告警从哪来（F4）：每 [`EVALUATE_EVERY`] 按节点此刻的状况核对一遍状态类告警，
//! 节点上报的事件随到随记。告警本身的规矩见 [`crate::server::fleet::alerts`]。

use super::Controller;
use super::dispatch::heartbeat_status;
use crate::server::errors::AppResult;
use crate::server::fleet::alerts::{
    self, Alert, AlertKey, AlertKind, Condition, MAX_ALERTS, disk_low, disk_threshold,
};
use crate::server::fleet::assignments::{self, Room};
use crate::server::fleet::now_ms;
use crate::server::fleet::protocol::{
    EVENT_RECORDING_ERROR, EVENT_UPLOAD_FAILED, EVENTS_SINCE, Event, RoomEvent,
};
use crate::server::fleet::store;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Weak;
use std::time::Duration;
use tracing::{debug, info, warn};

pub const EVALUATE_EVERY: Duration = Duration::from_secs(5);
/// 节点至少这么久没消息才告警离线：短暂断线重连（重启进程、换网络）不算。
/// 控制面刚启动时同样先等这么久，给节点重连的时间
pub const OFFLINE_ALERT_AFTER_MS: i64 = 60 * 1000;

/// `GET /v1/fleet/alerts`
#[derive(Debug, Serialize)]
pub struct AlertList {
    pub now: i64,
    pub alerts: Vec<Alert>,
    /// 列表最多留几条
    pub capacity: usize,
    /// 协议次版本低于它的节点不上报录制出错与投稿失败
    pub events_since: u32,
}

/// `GET /v1/fleet/summary`：控制台上的 Fleet 汇总
#[derive(Debug, Serialize)]
pub struct FleetSummary {
    pub nodes_total: usize,
    pub nodes_online: usize,
    /// 在线节点正在录制的房间数之和
    pub recording: usize,
    /// 还没「知道了」的告警
    pub alerts: usize,
    /// 其中还在持续的
    pub alerts_open: usize,
}

fn key(kind: AlertKind, node_id: i64, subject: Option<String>) -> AlertKey {
    AlertKey {
        kind,
        node_id,
        subject,
    }
}

fn room_by_url<'a>(rooms: &'a [Room], url: &str) -> Option<&'a Room> {
    rooms
        .iter()
        .find(|room| room.spec.url == url && room.deleted_at.is_none())
}

impl Controller {
    pub fn alert_list(&self) -> AlertList {
        AlertList {
            now: now_ms(),
            alerts: self.alerts.lock().unwrap().list(),
            capacity: MAX_ALERTS,
            events_since: EVENTS_SINCE,
        }
    }

    /// 测试里停掉定时核对，改由测试调用 [`Self::evaluate_alerts_at`]
    #[cfg(test)]
    pub(in crate::server::fleet) fn stop_alert_loop(&self) {
        if let Some(task) = self.alert_task.lock().unwrap().take() {
            task.abort();
        }
    }

    pub fn acknowledge_alert(&self, id: u64) -> bool {
        self.alerts.lock().unwrap().acknowledge(id)
    }

    pub fn acknowledge_alerts(&self) -> usize {
        self.alerts.lock().unwrap().acknowledge_all()
    }

    pub async fn summary(&self) -> AppResult<FleetSummary> {
        let rows = store::list_nodes(&self.pool).await?;
        let now = now_ms();
        let (nodes_online, recording) = {
            let live = self.live.lock().unwrap();
            rows.iter()
                .filter_map(|row| live.get(&row.id).filter(|node| node.online(now)))
                .fold((0, 0), |(online, recording), node| {
                    let rooms = node.summary.as_ref().map_or(0, |s| s.recording);
                    (online + 1, recording + rooms)
                })
        };
        let alerts = self.alerts.lock().unwrap();
        Ok(FleetSummary {
            nodes_total: rows.len(),
            nodes_online,
            recording,
            alerts: alerts.len(),
            alerts_open: alerts.open_count(),
        })
    }

    /// 按此刻的状况核对状态类告警，顺带把又录上了的「录制出错」记为恢复
    pub(super) async fn evaluate_alerts(&self) {
        self.evaluate_alerts_at(now_ms()).await;
    }

    /// [`Self::evaluate_alerts`]，时刻由调用方给（测试里用来跳过离线告警的等待）
    pub(in crate::server::fleet) async fn evaluate_alerts_at(&self, now: i64) {
        let (rows, rooms) = match tokio::try_join!(
            store::list_nodes(&self.pool),
            assignments::list_rooms(&self.pool)
        ) {
            Ok(loaded) => loaded,
            Err(e) => {
                warn!(error = ?e, "could not evaluate fleet alerts");
                return;
            }
        };
        let mut conditions = Vec::new();
        let mut stale = HashSet::new();
        let mut recovered = Vec::new();
        {
            let live = self.live.lock().unwrap();
            let current = self.alerts.lock().unwrap();
            let recording_errors = current.open_of(AlertKind::RecordingError);
            for row in &rows {
                let node_name = row.name.clone();
                let Some(node) = live.get(&row.id).filter(|node| node.online(now)) else {
                    stale.insert(row.id);
                    let last_seen = live
                        .get(&row.id)
                        .map(|node| node.last_message_at)
                        .or(row.last_seen_at)
                        .unwrap_or(row.created_at);
                    if now - last_seen.max(self.started_at) >= OFFLINE_ALERT_AFTER_MS {
                        conditions.push((
                            key(AlertKind::NodeOffline, row.id, None),
                            Condition {
                                node_name,
                                message: "节点离线：连不上控制面".into(),
                                since: Some(last_seen),
                                ..Condition::default()
                            },
                        ));
                    }
                    continue;
                };
                if let Some(disk) = node.summary.as_ref().and_then(|s| s.disk.as_ref())
                    && disk.total > 0
                {
                    let (threshold, source) = disk_threshold(disk.total, node.min_free_space);
                    let disk_key = key(AlertKind::DiskLow, row.id, None);
                    if disk_low(
                        disk.available,
                        disk.total,
                        threshold,
                        current.is_open(&disk_key),
                    ) {
                        conditions.push((
                            disk_key,
                            Condition {
                                node_name: node_name.clone(),
                                message: format!(
                                    "录制目录所在磁盘只剩 {}，低于 {}（{source}）",
                                    alerts::bytes(disk.available),
                                    alerts::bytes(threshold)
                                ),
                                ..Condition::default()
                            },
                        ));
                    }
                }
                // 按节点最近一次的应答：新配置刚推出去、还没应答时仍算失败，免得告警先恢复再重开
                if let Some(ack) = node.config_ack.as_ref().filter(|ack| !ack.applied) {
                    conditions.push((
                        key(AlertKind::ConfigFailed, row.id, None),
                        Condition {
                            node_name: node_name.clone(),
                            message: format!(
                                "下发的配置没能生效，节点保持原来的配置：{}",
                                ack.error.as_deref().unwrap_or("节点没有给出原因")
                            ),
                            ..Condition::default()
                        },
                    ));
                }
                for (room_id, error) in &node.failed {
                    let room = rooms.iter().find(|room| room.id == *room_id);
                    conditions.push((
                        key(AlertKind::RoomFailed, row.id, Some(room_id.to_string())),
                        Condition {
                            node_name: node_name.clone(),
                            room_id: Some(*room_id),
                            room: room.map(|room| room.spec.remark.clone()),
                            url: room.map(|room| room.spec.url.clone()),
                            message: error.clone(),
                            ..Condition::default()
                        },
                    ));
                }
                for (alert, last_at) in &recording_errors {
                    if alert.node_id == row.id
                        && node.heartbeat_at > *last_at
                        && let Some(url) = &alert.subject
                        && heartbeat_status(node, url) == Some("Working")
                    {
                        recovered.push(alert.clone());
                    }
                }
            }
        }
        let mut current = self.alerts.lock().unwrap();
        for alert in &recovered {
            current.resolve(alert, now);
        }
        current.sync(conditions, &stale, now);
    }

    /// 节点上报的事件；不认识的 `kind` 忽略
    pub(super) async fn record_event(&self, id: i64, event: Event) {
        let kind = match event.kind.as_str() {
            EVENT_RECORDING_ERROR => AlertKind::RecordingError,
            EVENT_UPLOAD_FAILED => AlertKind::UploadFailed,
            other => {
                debug!(node = id, kind = other, "ignoring unknown fleet node event");
                return;
            }
        };
        let detail: RoomEvent = serde_json::from_value(event.detail).unwrap_or_default();
        let now = now_ms();
        // 节点时钟可能比控制面快
        let at = event.at.min(now);
        let node_name = match store::node(&self.pool, id).await {
            Ok(Some(row)) => row.name,
            _ => id.to_string(),
        };
        let rooms = assignments::list_rooms(&self.pool)
            .await
            .unwrap_or_default();
        let room = room_by_url(&rooms, &detail.url);
        info!(node = id, kind = %event.kind, url = %detail.url, error = %detail.error, "fleet node reported a failure");
        let message = if detail.error.is_empty() {
            "节点没有给出原因".to_string()
        } else {
            detail.error
        };
        let condition = Condition {
            node_name,
            room_id: room.map(|room| room.id),
            room: Some(detail.remark).filter(|remark| !remark.is_empty()),
            url: Some(detail.url.clone()),
            message,
            since: None,
        };
        self.alerts
            .lock()
            .unwrap()
            .event(key(kind, id, Some(detail.url)), condition, at);
    }
}

/// 控制面的定时核对，随控制面一起停
pub(super) async fn alert_loop(controller: Weak<Controller>) {
    let mut ticker = tokio::time::interval(EVALUATE_EVERY);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let Some(controller) = controller.upgrade() else {
            break;
        };
        controller.evaluate_alerts().await;
    }
}
