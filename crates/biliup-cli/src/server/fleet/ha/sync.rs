//! 一主一备的双向同步（ha-pair 方案 H2）：同步哪些记录、谁赢、线上的形状。
//!
//! 两台都能改设置：配对里的房间与模板、空间配置、B 站账号凭据。每条记录带一个版本
//! [`Stamp`]（写入时刻 + 写入方），后写者赢，同一毫秒时当前的主机赢（[`Stamp::beats`]）。
//! 配对本身的设置（谁是上传主机、模式与参数）不走版本：只在控制面库里落一份，节点上改要两台都在线、
//! 经控制面提交（[`HaChange`]），所以不会有两边各自离线改出的冲突。
//! 删除留墓碑（[`Record::deleted`]），墓碑与修改按同一个版本比：旧的修改复活不了删掉的记录，
//! 旧的删除也吞不掉之后的修改。墓碑一直留到解除配对。
//!
//! 时刻取混合逻辑时钟（[`Clock`]）：本机墙钟与见过的最大版本取大，再比上一次写入至少大 1。
//! 看到对端的修改之后再改，一定排在它后面；两台各自离线改同一条时比的是两边的墙钟，墙钟快的一台占便宜。
//!
//! 记录的键：`room/<uid>`、`template/<uid>`（`uid` 是控制面 id 的十进制，或节点新建时起的 `n…`）、
//! `config/<键名>`、`account/<mid>`。内容摘要（[`digest`]）只在本机比较，用来认出本机上的改动，
//! 从不跨机器比较。

use super::params::{HaMode, HaParams};
use crate::server::fleet::model::RoomSpec;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::fmt;

pub const ROOM: &str = "room/";
pub const TEMPLATE: &str = "template/";
pub const CONFIG: &str = "config/";
pub const ACCOUNT: &str = "account/";

pub fn room_key(uid: &str) -> String {
    format!("{ROOM}{uid}")
}

pub fn template_key(uid: &str) -> String {
    format!("{TEMPLATE}{uid}")
}

pub fn config_key(name: &str) -> String {
    format!("{CONFIG}{name}")
}

pub fn account_key(mid: u64) -> String {
    format!("{ACCOUNT}{mid}")
}

/// 配对里的哪一台
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// 控制面进程（它的「本机」节点）
    Controller,
    /// 被指定进配对的那台普通节点
    Node,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Controller => Side::Node,
            Side::Node => Side::Controller,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Side::Controller => "controller",
            Side::Node => "node",
        }
    }
}

/// 一条记录的版本
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// 写入时刻（混合逻辑时钟，Unix 毫秒）
    pub at: i64,
    pub side: Side,
}

impl Stamp {
    /// `self` 是否胜过 `other`：时刻大的赢，同一时刻 `primary`（当前的主机）写的赢。相同的版本互不胜过
    pub fn beats(&self, other: &Stamp, primary: Side) -> bool {
        match self.at.cmp(&other.at) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => self.side != other.side && self.side == primary,
        }
    }
}

/// 混合逻辑时钟：本机最后一次写入或见过的最大时刻
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Clock(i64);

impl Clock {
    /// 本机写入一条：取墙钟与上一次加 1 里大的那个
    pub fn tick(&mut self, now: i64) -> i64 {
        self.0 = now.max(self.0.saturating_add(1));
        self.0
    }

    /// 收到对端的版本
    pub fn observe(&mut self, at: i64) {
        self.0 = self.0.max(at);
    }
}

/// 本机账本里的一条
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub stamp: Stamp,
    /// 本机这份内容的摘要；墓碑没有
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// 控制面上的 id（房间、模板）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<i64>,
    /// 本机主库里的行 id（节点上的房间、模板）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<i64>,
}

impl Record {
    pub fn deleted(&self) -> bool {
        self.digest.is_none()
    }
}

/// 对端发来的一条怎么处理
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// 比本机的新：收下
    Take,
    /// 本机的更新：不收，本机这份会发过去
    Keep,
    /// 与本机同一个版本（重放）：什么都不做
    Same,
}

/// 本机的账本：每条记录的版本与本机内容的摘要
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Book {
    #[serde(default)]
    pub clock: Clock,
    #[serde(default)]
    pub records: BTreeMap<String, Record>,
}

impl Book {
    pub fn get(&self, key: &str) -> Option<&Record> {
        self.records.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Record> {
        self.records.get_mut(key)
    }

    /// 本机上改了（或删了，`digest` 为空）一条：记下新版本并返回它
    pub fn write(&mut self, key: &str, side: Side, now: i64, digest: Option<String>) -> Stamp {
        let stamp = Stamp {
            at: self.clock.tick(now),
            side,
        };
        self.put(key, stamp, digest);
        stamp
    }

    /// 按给定版本记下一条（收下对端的，或按对端的版本落地之后），保留已记的 id
    pub fn put(&mut self, key: &str, stamp: Stamp, digest: Option<String>) {
        self.clock.observe(stamp.at);
        match self.records.get_mut(key) {
            Some(record) => {
                record.stamp = stamp;
                record.digest = digest;
            }
            None => {
                self.records.insert(
                    key.to_string(),
                    Record {
                        stamp,
                        digest,
                        fleet: None,
                        local: None,
                    },
                );
            }
        }
    }

    /// 对端的一条该不该收。本机没有这条时收
    pub fn judge(&mut self, key: &str, stamp: &Stamp, primary: Side) -> Verdict {
        self.clock.observe(stamp.at);
        match self.records.get(key) {
            None => Verdict::Take,
            Some(record) if record.stamp == *stamp => Verdict::Same,
            Some(record) if stamp.beats(&record.stamp, primary) => Verdict::Take,
            Some(_) => Verdict::Keep,
        }
    }

    /// 本机这条的版本是否胜过 `stamp`（没有这条时不胜过）
    pub fn newer_than(&self, key: &str, stamp: &Stamp, primary: Side) -> bool {
        self.records
            .get(key)
            .is_some_and(|record| record.stamp.beats(stamp, primary))
    }

    /// 按前缀列出记录
    pub fn with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Record)> + 'a {
        self.records
            .range(prefix.to_string()..)
            .take_while(move |(key, _)| key.starts_with(prefix))
            .map(|(key, record)| (key.as_str(), record))
    }

    /// 按控制面 id 找房间或模板的键
    pub fn key_of_fleet(&self, prefix: &str, id: i64) -> Option<String> {
        self.with_prefix(prefix)
            .find(|(_, record)| record.fleet == Some(id))
            .map(|(key, _)| key.to_string())
    }

    /// 按本机行 id 找房间或模板的键
    pub fn key_of_local(&self, prefix: &str, id: i64) -> Option<String> {
        self.with_prefix(prefix)
            .find(|(_, record)| record.local == Some(id))
            .map(|(key, _)| key.to_string())
    }
}

/// 内容摘要：键排好序的 JSON 的 SHA-256
pub fn digest(value: &Value) -> String {
    digest_bytes(canonical(value).as_bytes())
}

/// 文件内容（凭据文件）的摘要
pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    crate::server::fleet::store::hex(&hasher.finalize())
}

fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<&String, String> =
                map.iter().map(|(k, v)| (k, canonical(v))).collect();
            let body: Vec<String> = sorted
                .into_iter()
                .map(|(k, v)| format!("{}:{v}", Value::String(k.clone())))
                .collect();
            format!("{{{}}}", body.join(","))
        }
        Value::Array(items) => {
            let body: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", body.join(","))
        }
        other => other.to_string(),
    }
}

/// 节点新建的房间或模板的 uid
pub fn new_uid() -> String {
    format!("n{:016x}", rand::random::<u64>())
}

/// 配对里一个房间的内容：录制设置、用哪个配对模板（uid）、是否暂停
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomValue {
    #[serde(flatten)]
    pub spec: RoomSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default)]
    pub paused: bool,
}

/// 配对的模式与参数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaValue {
    pub mode: HaMode,
    #[serde(default)]
    pub params: HaParams,
}

/// 两台之间的同步消息（协议次版本 5 起，只在配对的两台之间、同一条加密连接上）。
/// 节点 → 控制面走 `NodeMessage::Pair`，控制面 → 节点走 `ControllerMessage::Pair`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PairMessage {
    /// 一条设置的新版本（`value` 为空表示删除）
    Edit(PairEdit),
    /// 一个 B 站账号的凭据文件
    Secret(PairSecret),
    /// 收到并处理完了 `upto` 及之前的消息
    Ack(PairAck),
    /// 改配对设置：控制面问节点能不能换上传主机，或节点请控制面换上传主机、改模式与参数
    Ha(HaChange),
    HaResult(HaResult),
    /// 控制面问节点有哪些还没纳入配对的本地主播与模板（指定备机之前也问，节点不在配对里也回）
    InventoryAsk(InventoryAsk),
    Inventory(Inventory),
}

impl PairMessage {
    pub fn op(&self) -> &'static str {
        match self {
            PairMessage::Edit(_) => "edit",
            PairMessage::Secret(_) => "secret",
            PairMessage::Ack(_) => "ack",
            PairMessage::Ha(_) => "ha",
            PairMessage::HaResult(_) => "ha_result",
            PairMessage::InventoryAsk(_) => "inventory_ask",
            PairMessage::Inventory(_) => "inventory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairEdit {
    /// 发送方队列里的序号，对端按它应答
    pub seq: u64,
    pub key: String,
    pub stamp: Stamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// 单独加入配对的模板：没有房间用它也留在配对里
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pin: bool,
}

/// 凭据文件的内容只在这里出现：不进日志（`Debug` 抹掉，帧日志也抹掉）、不进控制面库
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PairSecret {
    pub seq: u64,
    pub mid: u64,
    pub stamp: Stamp,
    /// 凭据文件的原文；为空表示这个账号在对端被删掉了
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl fmt::Debug for PairSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PairSecret")
            .field("seq", &self.seq)
            .field("mid", &self.mid)
            .field("stamp", &self.stamp)
            .field("content", &self.content.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairAck {
    pub upto: u64,
    /// 没有生效的记录与原因（版本旧了的不算，那是正常的后写者赢）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<Rejected>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejected {
    pub key: String,
    pub reason: String,
}

/// 对端按 `id` 回一条 [`HaResult`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaChange {
    pub id: u64,
    /// 换成由这一台上传
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<Side>,
    /// 新的模式与参数（只由节点发给控制面）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ha: Option<HaValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaResult {
    pub id: u64,
    /// 为空表示照办了（或本来就是这样）；不然是给人看的原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 控制面给配对节点的期望状态里带的同步版本（自次版本 5 起）：镜像房间与模板各自的 uid 与版本、
/// 墓碑。节点按它认出期望状态里哪些已经比本机旧（本机的修改还没送到控制面）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairState {
    #[serde(default)]
    pub rooms: Vec<PairRef>,
    #[serde(default)]
    pub templates: Vec<PairRef>,
    #[serde(default)]
    pub gone: Vec<Gone>,
    /// 请节点把这些本地行加入配对（指定备机时纳入的既有主播与模板、之后按 id 加入的）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopt: Option<Box<AdoptRequest>>,
}

/// 控制面请节点加入配对的本地行：节点上的本地 id → 请求它的 `generation`。一直随期望状态带着、只增不减，
/// 节点只收 `generation` 比它处理过的大的那些（同一行再请求一次时它的 `generation` 跟着变大）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptRequest {
    pub generation: u64,
    #[serde(default, with = "id_pairs")]
    pub rooms: BTreeMap<i64, u64>,
    #[serde(default, with = "id_pairs")]
    pub templates: BTreeMap<i64, u64>,
}

/// 按 `[[id, generation], …]` 收发：帧是按 `type` 区分的枚举，serde 先整帧缓存再解，
/// 缓存里映射的键都是字符串，解不成 `i64`
mod id_pairs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<i64, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(map)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<i64, u64>, D::Error> {
        Ok(Vec::<(i64, u64)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryAsk {
    pub id: u64,
}

/// 节点上还没纳入配对的本地主播与模板
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub id: u64,
    #[serde(default)]
    pub rooms: Vec<LocalRow>,
    #[serde(default)]
    pub templates: Vec<LocalRow>,
}

/// 一台机器上还没纳入配对的一个本地主播或模板。只有名字、地址与判断能不能加入要的几项，
/// 钩子命令、覆写里的 Cookie、凭据路径都不带
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRow {
    /// 这台机器上的本地 id
    pub id: i64,
    /// 主播的备注、模板的模板名
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 主播用的本地模板 id
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<i64>,
    /// 主播带 run 命令的钩子
    #[serde(default)]
    pub hooks: bool,
    /// 主播覆写里的下载器（控制面按自己的配置判断是不是边录边传）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloader: Option<Value>,
    /// 正在录，或上一场还没投完：空闲了才加入
    #[serde(default)]
    pub busy: bool,
    #[serde(default)]
    pub state: RowState,
    /// 不能加入的原因；为空就能加入
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 上次加入时控制面没收下的原因（改好之后可以再加）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// 这一次请求有没有纳入（指定备机与加入接口的应答里才有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowState {
    /// 本地行，没有要加入
    #[default]
    Local,
    /// 要加入，等它空闲（没在录、上一场投完了）
    Waiting,
    /// 已经发出，等控制面收下、两台落地
    Joining,
}

impl AdoptRequest {
    /// 再请求这些行（`generation` 至少是 `now`，时钟回拨也不变小）
    pub fn add(&mut self, rooms: &[i64], templates: &[i64], now: u64) {
        self.generation = self.generation.saturating_add(1).max(now);
        for id in rooms {
            self.rooms.insert(*id, self.generation);
        }
        for id in templates {
            self.templates.insert(*id, self.generation);
        }
    }

    /// `seen` 之后才请求的行
    pub fn since(&self, seen: u64) -> (Vec<i64>, Vec<i64>) {
        let fresh = |ids: &BTreeMap<i64, u64>| {
            ids.iter()
                .filter(|(_, generation)| **generation > seen)
                .map(|(id, _)| *id)
                .collect()
        };
        (fresh(&self.rooms), fresh(&self.templates))
    }
}

impl Inventory {
    pub fn new(id: u64, (rooms, templates): (Vec<LocalRow>, Vec<LocalRow>)) -> Self {
        Inventory {
            id,
            rooms,
            templates,
        }
    }
}

impl PairState {
    pub fn room(&self, id: i64) -> Option<&PairRef> {
        self.rooms.iter().find(|room| room.id == id)
    }

    pub fn template(&self, id: i64) -> Option<&PairRef> {
        self.templates.iter().find(|template| template.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairRef {
    /// 控制面 id
    pub id: i64,
    pub uid: String,
    pub stamp: Stamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gone {
    pub key: String,
    pub stamp: Stamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const C: Side = Side::Controller;
    const N: Side = Side::Node;

    fn stamp(at: i64, side: Side) -> Stamp {
        Stamp { at, side }
    }

    #[test]
    fn later_writes_win_and_ties_go_to_the_current_primary() {
        assert!(stamp(2, N).beats(&stamp(1, C), C));
        assert!(!stamp(1, N).beats(&stamp(2, C), N));
        // 同一毫秒：主机赢，与主机在哪一边无关
        assert!(stamp(5, C).beats(&stamp(5, N), C));
        assert!(!stamp(5, N).beats(&stamp(5, C), C));
        assert!(stamp(5, N).beats(&stamp(5, C), N));
        assert!(!stamp(5, C).beats(&stamp(5, N), N));
        // 同一个版本谁也不胜过谁
        assert!(!stamp(5, C).beats(&stamp(5, C), C));
    }

    #[test]
    fn the_clock_never_goes_back_and_orders_after_what_it_saw() {
        let mut clock = Clock::default();
        assert_eq!(clock.tick(100), 100);
        assert_eq!(clock.tick(100), 101, "同一毫秒两次写入也有先后");
        assert_eq!(clock.tick(50), 102, "墙钟回拨不影响");
        clock.observe(1_000);
        assert_eq!(clock.tick(200), 1_001, "看到对端的修改之后再改，排在它后面");
    }

    #[test]
    fn a_delete_and_a_modify_are_ordered_by_their_stamps() {
        let key = room_key("7");
        // 节点删了，控制面之后又改了：改的赢，房间回来
        let mut node = Book::default();
        node.write(&key, N, 1_000, None);
        assert!(node.get(&key).unwrap().deleted());
        assert_eq!(node.judge(&key, &stamp(2_000, C), C), Verdict::Take);
        // 控制面改在前，节点删在后：删的赢，旧的修改复活不了
        let mut node = Book::default();
        node.write(&key, N, 3_000, None);
        assert_eq!(node.judge(&key, &stamp(2_000, C), C), Verdict::Keep);
        // 同一毫秒：主机赢
        let mut node = Book::default();
        node.write(&key, N, 2_000, None);
        assert_eq!(node.judge(&key, &stamp(2_000, C), C), Verdict::Take);
        assert_eq!(node.judge(&key, &stamp(2_000, C), N), Verdict::Keep);
        // 重放同一个版本什么都不做
        let mut controller = Book::default();
        controller.put(&key, stamp(3_000, N), None);
        assert_eq!(controller.judge(&key, &stamp(3_000, N), C), Verdict::Same);
        // 没见过的记录收下，时钟跟上
        let mut fresh = Book::default();
        assert_eq!(fresh.judge("config/a", &stamp(9_000, N), C), Verdict::Take);
        assert_eq!(fresh.write("config/b", C, 10, None).at, 9_001);
    }

    /// 加入请求随期望状态走：节点那边经按 `type` 区分的帧解出来仍是同一份，只收比处理过的新的
    #[test]
    fn an_adopt_request_survives_the_desired_state_frame() {
        use crate::server::fleet::protocol::{ControllerMessage, DesiredState, decode};
        let mut request = AdoptRequest::default();
        request.add(&[1, 3], &[2], 7);
        request.add(&[3], &[], 7);
        assert_eq!(request.generation, 8);
        let desired = DesiredState {
            pair: Some(PairState {
                adopt: Some(Box::new(request.clone())),
                ..PairState::default()
            }),
            ..DesiredState::default()
        };
        let body = serde_json::to_vec(&ControllerMessage::DesiredState(desired)).unwrap();
        let ControllerMessage::DesiredState(decoded) = decode(&body).unwrap() else {
            panic!("不是期望状态");
        };
        let decoded = *decoded.pair.and_then(|pair| pair.adopt).unwrap();
        assert_eq!(decoded, request);
        assert_eq!(decoded.since(0), (vec![1, 3], vec![2]));
        assert_eq!(decoded.since(7), (vec![3], vec![]));
    }

    #[test]
    fn put_keeps_known_ids_and_prefix_lookup_is_exact() {
        let mut book = Book::default();
        book.put(&room_key("7"), stamp(1, C), Some("a".into()));
        book.get_mut(&room_key("7")).unwrap().fleet = Some(7);
        book.get_mut(&room_key("7")).unwrap().local = Some(3);
        book.put(&room_key("7"), stamp(2, N), Some("b".into()));
        let record = book.get(&room_key("7")).unwrap();
        assert_eq!((record.fleet, record.local), (Some(7), Some(3)));
        book.put(&template_key("7"), stamp(1, C), Some("t".into()));
        book.put("roomy", stamp(1, C), None);
        assert_eq!(book.with_prefix(ROOM).count(), 1);
        assert_eq!(book.key_of_fleet(ROOM, 7), Some(room_key("7")));
        assert_eq!(book.key_of_local(ROOM, 3), Some(room_key("7")));
        assert_eq!(book.key_of_fleet(TEMPLATE, 7), None);
        assert!(book.newer_than(&room_key("7"), &stamp(1, C), C));
        assert!(!book.newer_than(&room_key("8"), &stamp(1, C), C));
    }

    #[test]
    fn digests_ignore_key_order() {
        let a = json!({ "b": 1, "a": { "y": [1, { "q": 2, "p": 1 }], "x": null } });
        let b: Value =
            serde_json::from_str(r#"{"a":{"x":null,"y":[1,{"p":1,"q":2}]},"b":1}"#).unwrap();
        assert_eq!(digest(&a), digest(&b));
        assert_ne!(digest(&a), digest(&json!({ "b": 2 })));
        assert_eq!(digest(&a).len(), 64);
        assert!(new_uid().starts_with('n') && new_uid() != new_uid());
    }

    #[test]
    fn secrets_are_not_debug_printed_and_frames_are_tagged_by_op() {
        let secret = PairMessage::Secret(PairSecret {
            seq: 3,
            mid: 42,
            stamp: stamp(1, N),
            content: Some("SESSDATA=placeholder".into()),
        });
        let debug = format!("{secret:?}");
        assert!(debug.contains("42") && !debug.contains("placeholder"));
        let json = serde_json::to_value(&secret).unwrap();
        assert_eq!(json["op"], "secret");
        assert_eq!(json["stamp"], json!({ "at": 1, "side": "node" }));
        let edit: PairMessage = serde_json::from_value(json!({
            "op": "edit", "seq": 1, "key": "config/segment_time",
            "stamp": { "at": 5, "side": "controller" },
        }))
        .unwrap();
        assert!(matches!(
            edit,
            PairMessage::Edit(PairEdit { value: None, .. })
        ));
        let ack = serde_json::to_value(PairMessage::Ack(PairAck {
            upto: 4,
            rejected: Vec::new(),
        }))
        .unwrap();
        assert_eq!(ack, json!({ "op": "ack", "upto": 4 }));
    }
}
