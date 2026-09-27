//! 控制面的告警（F4）：只在界面上显示、只放在内存里，控制面重启就清空，也不往外发通知。
//!
//! 两类告警：
//! - 状态类（节点离线、磁盘空间不足、配置应用失败、房间落地失败）：控制面每隔几秒按此刻的情况
//!   整体核对一遍（[`Alerts::sync`]），条件消失就记下恢复时间；
//! - 事件类（录制出错、投稿失败）：节点上报一次记一次（[`Alerts::event`]），同一台节点、同一个
//!   直播间的同类告警合并计数。录制出错在节点重新录上之后也算恢复。
//!
//! 恢复了还没被「知道了」的告警再次出现时原地重开、次数加一，不另起一条，免得来回抖动刷屏。
//! 「知道了」把告警从列表里拿掉；状态类的条件若还在，要等它先恢复、再出现才会重新告警。
//! 列表最多 [`MAX_ALERTS`] 条，满了先挤掉最早恢复的，再挤掉最早的事件类，最后才动还在持续的。

use serde::Serialize;
use std::collections::HashSet;

pub const MAX_ALERTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    NodeOffline,
    DiskLow,
    ConfigFailed,
    RoomFailed,
    RecordingError,
    UploadFailed,
}

impl AlertKind {
    /// 状态类：由 [`Alerts::sync`] 核对；其余是事件类
    pub fn stateful(self) -> bool {
        !matches!(self, AlertKind::RecordingError | AlertKind::UploadFailed)
    }

    /// 只看节点本身、离线时无从判断的状态类
    fn needs_online_node(self) -> bool {
        matches!(
            self,
            AlertKind::DiskLow | AlertKind::ConfigFailed | AlertKind::RoomFailed
        )
    }
}

/// 同一件事：种类 + 节点 + 房间（状态类按控制面房间 id，事件类按直播间地址）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlertKey {
    pub kind: AlertKind,
    pub node_id: i64,
    pub subject: Option<String>,
}

/// 此刻成立的一个条件，或节点上报的一次事件
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Condition {
    pub node_name: String,
    /// 控制面的房间 id；本机房间或找不到时为空
    pub room_id: Option<i64>,
    /// 房间备注
    pub room: Option<String>,
    pub url: Option<String>,
    pub message: String,
    /// 条件从什么时候开始（离线从最后一次收到消息算）；没有就按发现的时刻
    pub since: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alert {
    pub id: u64,
    pub kind: AlertKind,
    pub node_id: i64,
    pub node_name: String,
    pub room_id: Option<i64>,
    pub room: Option<String>,
    pub url: Option<String>,
    pub message: String,
    /// 第一次出现
    pub first_at: i64,
    /// 最近一次出现（重开或再次上报）
    pub last_at: i64,
    /// 出现了几次
    pub count: u32,
    /// 恢复时间；还在持续为空。事件类只有录制出错会恢复
    pub resolved_at: Option<i64>,
    #[serde(skip)]
    key: Option<String>,
}

impl Alert {
    fn key(&self) -> AlertKey {
        AlertKey {
            kind: self.kind,
            node_id: self.node_id,
            subject: self.key.clone(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.resolved_at.is_none()
    }
}

#[derive(Debug, Default)]
pub struct Alerts {
    list: Vec<Alert>,
    next_id: u64,
    /// 「知道了」时条件还在的状态类：等它恢复之前不再告警
    suppressed: HashSet<AlertKey>,
}

impl Alerts {
    /// 新的在前：还在持续的排前面，再按最近一次出现
    pub fn list(&self) -> Vec<Alert> {
        let mut list = self.list.clone();
        list.sort_by(|a, b| {
            b.is_open()
                .cmp(&a.is_open())
                .then(b.last_at.cmp(&a.last_at))
                .then(b.id.cmp(&a.id))
        });
        list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn open_count(&self) -> usize {
        self.list.iter().filter(|alert| alert.is_open()).count()
    }

    pub fn is_open(&self, key: &AlertKey) -> bool {
        self.list
            .iter()
            .any(|alert| alert.is_open() && alert.key() == *key)
    }

    /// 还在持续的某类告警，给核对「录制出错」是否已经恢复用
    pub fn open_of(&self, kind: AlertKind) -> Vec<(AlertKey, i64)> {
        self.list
            .iter()
            .filter(|alert| alert.kind == kind && alert.is_open())
            .map(|alert| (alert.key(), alert.last_at))
            .collect()
    }

    fn position(&self, key: &AlertKey) -> Option<usize> {
        self.list.iter().position(|alert| alert.key() == *key)
    }

    /// 出现一次：已有同一件事就更新它（恢复了的重开），否则新增
    fn raise(&mut self, key: AlertKey, condition: Condition, at: i64) {
        if let Some(index) = self.position(&key) {
            let alert = &mut self.list[index];
            if alert.resolved_at.take().is_some() || !key.kind.stateful() {
                alert.count = alert.count.saturating_add(1);
                alert.last_at = alert.last_at.max(at);
            }
            alert.node_name = condition.node_name;
            alert.room_id = condition.room_id.or(alert.room_id);
            alert.room = condition.room.or(alert.room.take());
            alert.url = condition.url.or(alert.url.take());
            alert.message = condition.message;
            return;
        }
        self.next_id += 1;
        let first_at = condition.since.unwrap_or(at).min(at);
        self.list.push(Alert {
            id: self.next_id,
            kind: key.kind,
            node_id: key.node_id,
            node_name: condition.node_name,
            room_id: condition.room_id,
            room: condition.room,
            url: condition.url,
            message: condition.message,
            first_at,
            last_at: first_at,
            count: 1,
            resolved_at: None,
            key: key.subject,
        });
        self.evict();
    }

    fn evict(&mut self) {
        while self.list.len() > MAX_ALERTS {
            let victim = [
                |alert: &Alert| !alert.is_open(),
                |alert: &Alert| !alert.kind.stateful(),
                |_: &Alert| true,
            ]
            .iter()
            .find_map(|pick| {
                self.list
                    .iter()
                    .enumerate()
                    .filter(|(_, alert)| pick(alert))
                    .min_by_key(|(_, alert)| (alert.resolved_at.unwrap_or(alert.last_at), alert.id))
                    .map(|(index, _)| index)
            });
            match victim {
                Some(index) => {
                    self.list.remove(index);
                }
                None => break,
            }
        }
    }

    /// 按此刻成立的全部状态类条件核对一遍。`stale` 是在册但离线的节点：它们的磁盘、配置、
    /// 落地告警无从判断，保持原样；不在册（已移除）的节点的状态类告警一律恢复。
    pub fn sync(&mut self, conditions: Vec<(AlertKey, Condition)>, stale: &HashSet<i64>, now: i64) {
        let active: HashSet<AlertKey> = conditions.iter().map(|(key, _)| key.clone()).collect();
        for alert in &mut self.list {
            if !alert.kind.stateful() || !alert.is_open() || active.contains(&alert.key()) {
                continue;
            }
            if alert.kind.needs_online_node() && stale.contains(&alert.node_id) {
                continue;
            }
            alert.resolved_at = Some(now);
        }
        self.suppressed.retain(|key| {
            active.contains(key) || (key.kind.needs_online_node() && stale.contains(&key.node_id))
        });
        for (key, condition) in conditions {
            debug_assert!(key.kind.stateful());
            if self.suppressed.contains(&key) {
                continue;
            }
            self.raise(key, condition, now);
        }
    }

    /// 节点上报的一次事件
    pub fn event(&mut self, key: AlertKey, condition: Condition, at: i64) {
        debug_assert!(!key.kind.stateful());
        self.raise(key, condition, at);
    }

    /// 事件类告警恢复（录制出错之后又录上了）
    pub fn resolve(&mut self, key: &AlertKey, now: i64) {
        if let Some(index) = self.position(key) {
            let alert = &mut self.list[index];
            if alert.is_open() {
                alert.resolved_at = Some(now);
            }
        }
    }

    /// 「知道了」：拿掉这一条。返回是否找到
    pub fn acknowledge(&mut self, id: u64) -> bool {
        let Some(index) = self.list.iter().position(|alert| alert.id == id) else {
            return false;
        };
        let alert = self.list.remove(index);
        if alert.kind.stateful() && alert.is_open() {
            self.suppressed.insert(alert.key());
        }
        true
    }

    /// 全部「知道了」，返回拿掉了几条
    pub fn acknowledge_all(&mut self) -> usize {
        let ids: Vec<u64> = self.list.iter().map(|alert| alert.id).collect();
        for id in &ids {
            self.acknowledge(*id);
        }
        ids.len()
    }
}

/// 字节数写成界面上的 `12.3 GiB`
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// 磁盘告警的阈值（字节）与它的来历：节点设了 `min_free_space` 用它，否则总容量的 5%
pub fn disk_threshold(total: u64, min_free_space: Option<u64>) -> (u64, &'static str) {
    match min_free_space.filter(|bytes| *bytes > 0) {
        Some(bytes) => (bytes, "min_free_space"),
        None => (total / 20, "总容量的 5%"),
    }
}

/// 可用空间低于阈值就告警；已经在告警时要回到阈值再加总容量的 1% 以上才算恢复，免得在阈值附近来回跳
pub fn disk_low(available: u64, total: u64, threshold: u64, alerting: bool) -> bool {
    let limit = if alerting {
        threshold.saturating_add(total / 100)
    } else {
        threshold
    };
    available < limit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(kind: AlertKind, node: i64, subject: Option<&str>) -> AlertKey {
        AlertKey {
            kind,
            node_id: node,
            subject: subject.map(str::to_string),
        }
    }

    fn condition(message: &str) -> Condition {
        Condition {
            node_name: "n".into(),
            message: message.into(),
            ..Condition::default()
        }
    }

    fn offline(node: i64, since: i64) -> (AlertKey, Condition) {
        (
            key(AlertKind::NodeOffline, node, None),
            Condition {
                since: Some(since),
                ..condition("离线")
            },
        )
    }

    #[test]
    fn stateful_alerts_appear_recover_and_reopen() {
        let mut alerts = Alerts::default();
        let none = HashSet::new();
        alerts.sync(vec![offline(1, 900)], &none, 1_000);
        let list = alerts.list();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].first_at, list[0].count), (900, 1));
        assert!(list[0].is_open());
        // 条件还在：不重复计数
        alerts.sync(vec![offline(1, 900)], &none, 2_000);
        assert_eq!(alerts.list()[0].count, 1);
        // 恢复
        alerts.sync(vec![], &none, 3_000);
        assert_eq!(alerts.list()[0].resolved_at, Some(3_000));
        assert_eq!(alerts.open_count(), 0);
        // 没「知道了」又出现：同一条重开
        alerts.sync(vec![offline(1, 3_500)], &none, 4_000);
        let list = alerts.list();
        assert_eq!(list.len(), 1);
        assert_eq!(
            (list[0].count, list[0].first_at, list[0].last_at),
            (2, 900, 4_000)
        );
        assert!(list[0].is_open());
    }

    #[test]
    fn acknowledged_conditions_stay_quiet_until_they_recover() {
        let mut alerts = Alerts::default();
        let none = HashSet::new();
        alerts.sync(vec![offline(1, 900)], &none, 1_000);
        let id = alerts.list()[0].id;
        assert!(alerts.acknowledge(id));
        assert!(!alerts.acknowledge(id));
        alerts.sync(vec![offline(1, 900)], &none, 2_000);
        assert!(alerts.is_empty());
        // 恢复之后再出现，重新告警（新的一条）
        alerts.sync(vec![], &none, 3_000);
        alerts.sync(vec![offline(1, 3_000)], &none, 4_000);
        let list = alerts.list();
        assert_eq!(list.len(), 1);
        assert_ne!(list[0].id, id);
        assert_eq!(list[0].count, 1);
    }

    #[test]
    fn offline_nodes_keep_their_disk_alerts_and_removed_nodes_lose_them() {
        let mut alerts = Alerts::default();
        let disk = (key(AlertKind::DiskLow, 1, None), condition("磁盘"));
        alerts.sync(vec![disk.clone()], &HashSet::new(), 1_000);
        // 节点离线：磁盘无从判断，保持原样
        let stale = HashSet::from([1]);
        alerts.sync(vec![offline(1, 1_000)], &stale, 2_000);
        let list = alerts.list();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(Alert::is_open));
        // 节点被移除：什么条件都不在、也不在 stale 里，全部恢复
        alerts.sync(vec![], &HashSet::new(), 3_000);
        assert_eq!(alerts.open_count(), 0);
    }

    #[test]
    fn events_merge_per_room_and_recordings_recover() {
        let mut alerts = Alerts::default();
        let room = |url: &str| key(AlertKind::RecordingError, 1, Some(url));
        alerts.event(room("https://a"), condition("boom"), 1_000);
        alerts.event(room("https://a"), condition("boom again"), 1_500);
        alerts.event(room("https://b"), condition("other"), 1_200);
        alerts.event(
            key(AlertKind::UploadFailed, 1, Some("https://a")),
            condition("cookie"),
            1_300,
        );
        let list = alerts.list();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].message, "boom again");
        assert_eq!(
            (list[0].count, list[0].first_at, list[0].last_at),
            (2, 1_000, 1_500)
        );
        // 事件类不受 sync 影响
        alerts.sync(vec![], &HashSet::new(), 2_000);
        assert_eq!(alerts.open_count(), 3);
        assert_eq!(alerts.open_of(AlertKind::RecordingError).len(), 2);
        alerts.resolve(&room("https://a"), 2_500);
        assert_eq!(alerts.open_count(), 2);
        // 再出错：重开
        alerts.event(room("https://a"), condition("third"), 3_000);
        assert_eq!(alerts.open_count(), 3);
        assert_eq!(alerts.acknowledge_all(), 3);
        assert!(alerts.is_empty());
        // 事件类「知道了」之后再来就是新的一条
        alerts.event(room("https://a"), condition("fourth"), 4_000);
        assert_eq!(alerts.list()[0].count, 1);
    }

    #[test]
    fn the_list_is_capped_evicting_resolved_then_events_then_oldest() {
        let mut alerts = Alerts::default();
        let none = HashSet::new();
        // 1 条会恢复的离线
        alerts.sync(vec![offline(0, 0)], &none, 1);
        alerts.sync(vec![], &none, 2);
        // 事件类填满
        for i in 1..MAX_ALERTS as i64 {
            alerts.event(
                key(AlertKind::UploadFailed, i, Some("u")),
                condition("x"),
                10 + i,
            );
        }
        assert_eq!(alerts.len(), MAX_ALERTS);
        // 再来一条持续中的状态类：挤掉已恢复的那条
        let disk = |node| (key(AlertKind::DiskLow, node, None), condition("磁盘"));
        alerts.sync(vec![disk(1000)], &none, 5_000);
        assert_eq!(alerts.len(), MAX_ALERTS);
        assert!(
            alerts
                .list()
                .iter()
                .all(|alert| alert.kind != AlertKind::NodeOffline)
        );
        // 再来：挤掉最早的事件类（node 1）
        alerts.sync(vec![disk(1000), disk(1001)], &none, 6_000);
        let list = alerts.list();
        assert_eq!(list.len(), MAX_ALERTS);
        assert!(!list.iter().any(|alert| alert.node_id == 1));
        assert!(list.iter().any(|alert| alert.node_id == 2));
        assert_eq!(alerts.open_count(), MAX_ALERTS);
    }

    #[test]
    fn disk_threshold_prefers_min_free_space_then_five_percent() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            disk_threshold(100 * gib, Some(10 * gib)),
            (10 * gib, "min_free_space")
        );
        assert_eq!(disk_threshold(100 * gib, Some(0)), (5 * gib, "总容量的 5%"));
        assert_eq!(disk_threshold(100 * gib, None), (5 * gib, "总容量的 5%"));
        // 低于阈值告警；告警中要回到阈值 + 1% 才恢复
        assert!(disk_low(4 * gib, 100 * gib, 5 * gib, false));
        assert!(!disk_low(5 * gib, 100 * gib, 5 * gib, false));
        assert!(disk_low(5 * gib + 1, 100 * gib, 5 * gib, true));
        assert!(!disk_low(6 * gib, 100 * gib, 5 * gib, true));
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(3 * gib / 2), "1.5 GiB");
    }
}
