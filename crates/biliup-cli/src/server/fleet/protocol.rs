//! 节点与控制面之间的控制通道。
//!
//! 连接一律由节点发起。一条连接上只开一条长期存在的双向流，帧格式是
//! `u32 大端长度 + JSON`，单帧上限 [`MAX_FRAME`]。主版本号放在 ALPN 里（[`ALPN`]），
//! 次版本号在 [`Hello::proto`] 里，只增字段不改语义，所以两端都忽略不认识的字段。

use super::ha::sync::{PairMessage, PairState};
use super::ha::wire::{HaAssignment, HaMessage};
use super::model::{Account, DesiredRoom, DesiredTemplate};
use crate::server::common::system_stats::{CpuInfo, DiskUsage, MemoryUsage, SystemStats};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"biliup/fleet/1";
/// 协议次版本号。1：`Hello` 带账号、工具与已持有的房间，控制面下发 `DesiredState`，节点回 `Ack`。
/// 2：`DesiredState` 带配置（[`DesiredConfig`]），`Ack` 带配置是否生效（[`ConfigAck`]）。
/// 3：节点发 `Event`（录制出错、投稿失败，见 [`RoomEvent`]），`Heartbeat` 带 `min_free_space`。
/// 4：主副配对（[`super::ha`]）：给备机的 `DesiredState` 带 `ha`，主备之间互发 `Ha` 场次消息。
/// 5：配对双向同步（[`super::ha::sync`]）：给配对节点的 `DesiredState` 带 `pair`，两台之间互发 `Pair`。
pub const PROTOCOL_MINOR: u32 = 5;
/// 能收 `DesiredState` 的最低次版本号。更旧的节点收到不认识的帧会卡住，控制面不给它们发。
pub const DESIRED_STATE_SINCE: u32 = 1;
/// 能收配置的最低次版本号。次版本 1 的节点照常收房间，配置不发给它。
pub const CONFIG_SINCE: u32 = 2;
/// 会上报 `Event` 的最低次版本号。更旧的节点不发，控制面对它们只有离线、磁盘、配置与落地这几类告警。
/// `Event` 帧自 F1 就在协议里，旧控制面收到只记一行 debug，所以新节点连旧控制面照发无妨。
pub const EVENTS_SINCE: u32 = 3;
/// 能当备机、收发 `Ha` 帧的最低次版本号。更旧的节点收到 `ha` 帧解不了会断开重连，控制面不把它们指定为备机。
pub const HA_SINCE: u32 = 4;
/// 能与控制面双向同步设置、收发 `Pair` 帧、切换主备的最低次版本号。次版本 4 的备机照 H1 只收镜像
pub const PAIR_SINCE: u32 = 5;
/// [`Event::kind`]：一次拉流以错误结束（不含用户停止、迁移等取消）
pub const EVENT_RECORDING_ERROR: &str = "recording_error";
/// [`Event::kind`]：一场投稿流程失败（登录、上传、提交或之后的后处理）
pub const EVENT_UPLOAD_FAILED: &str = "upload_failed";
pub const MAX_FRAME: usize = 4 * 1024 * 1024;
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
/// 控制面超过这么久没收到节点的任何帧就判离线并关掉连接
pub const OFFLINE_AFTER: Duration = Duration::from_secs(30);

/// 控制面关连接时带的应用错误码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseCode {
    /// 正常结束：join 完成、进程退出、节点 leave
    Normal = 0,
    /// 节点已被移除，节点停止重连
    Revoked = 1,
    /// 同一节点又建了一条新连接，旧的关掉
    Superseded = 2,
    /// 版本不兼容或帧格式错误
    Protocol = 3,
    /// 公钥不在节点表里、join 秘密不对或票据已失效
    Unauthorized = 4,
}

impl CloseCode {
    pub fn from_code(code: u64) -> Option<Self> {
        Some(match code {
            0 => CloseCode::Normal,
            1 => CloseCode::Revoked,
            2 => CloseCode::Superseded,
            3 => CloseCode::Protocol,
            4 => CloseCode::Unauthorized,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CloseCode::Normal => "normal",
            CloseCode::Revoked => "revoked",
            CloseCode::Superseded => "superseded",
            CloseCode::Protocol => "protocol",
            CloseCode::Unauthorized => "unauthorized",
        }
    }
}

/// 节点 → 控制面
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeMessage {
    Hello(Hello),
    Heartbeat(Heartbeat),
    Event(Event),
    /// `biliup node leave`：请控制面把自己移出节点表
    Leave,
    /// 对 `DesiredState` 的应答：按哪一版对的账、现在持有哪些房间
    Ack(Ack),
    /// 配对节点发给控制面的场次消息（自次版本 4 起，只在收到过带 `ha` 的期望状态之后）。
    /// 节点是备机时是备机 → 主机，切换主备之后节点是主机时是主机 → 备机
    Ha(HaMessage),
    /// 配对节点发给控制面的同步消息（自次版本 5 起，只在收到过带 `pair` 的期望状态之后）
    Pair(PairMessage),
}

/// 控制面 → 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControllerMessage {
    /// 对 `Hello` 的应答：分配给这台节点的 id 与控制面当前的 relay 地址
    Welcome { node_id: i64, relays: Vec<String> },
    /// 控制面的 relay 地址变了；节点写回 `data/node.json`，下次重连就用新地址
    Relays { relays: Vec<String> },
    /// 这台节点应该录的全部房间与它们用到的模板（整份快照，不是增量）。
    /// 节点只动自己按控制面建的那些本地行，本机自己加的房间与模板不受影响。
    DesiredState(DesiredState),
    /// 控制面发给配对节点的场次消息（自次版本 4 起，只发给配对节点）
    Ha(HaMessage),
    /// 控制面发给配对节点的同步消息（自次版本 5 起，只发给次版本 ≥ 5 的配对节点）
    Pair(PairMessage),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesiredState {
    /// 控制面的期望状态版本号，单调递增；节点在 `Ack` 里原样带回
    pub version: u64,
    pub rooms: Vec<DesiredRoom>,
    pub templates: Vec<DesiredTemplate>,
    /// 这台节点的配置（自次版本 2 起）。控制面对次版本 ≥ 2 的节点总是带上；
    /// 没有这个字段表示控制面不管配置，节点的配置照旧在本机改。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<DesiredConfig>,
    /// 这台节点在配对里时才有（自次版本 4 起）：模式、参数、谁是主机与镜像过来的房间
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ha: Option<HaAssignment>,
    /// 与 `ha` 一起、只给次版本 ≥ 5 的配对节点（自次版本 5 起）：镜像房间与模板的同步版本
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair: Option<PairState>,
}

/// 下发给一台节点的配置：Fleet 全局 ⊕ 这台节点的覆盖，只含白名单键（D11），
/// 节点自己再叠上本机的密钥，见 [`super::layers`]。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DesiredConfig {
    /// 全局配置的版本号，0 表示控制面还没保存过全局配置（此时只有覆盖）
    pub global_version: i64,
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// 节点对 [`DesiredConfig`] 的应答
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigAck {
    /// 本机配置已经是下发的那份（包括本来就一样、无需改动）
    pub applied: bool,
    /// 没能应用的原因；此时节点保持原来的配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ConfigAck {
    pub fn applied() -> Self {
        ConfigAck {
            applied: true,
            error: None,
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        ConfigAck {
            applied: false,
            error: Some(reason.into()),
        }
    }
}

/// 节点持有的一个托管房间：已经落进本地 `livestreamers`、正在按它录
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldRoom {
    pub id: i64,
    pub epoch: i64,
}

/// 期望状态里没能落地的房间
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedRoom {
    pub id: i64,
    pub error: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ack {
    pub version: u64,
    #[serde(default)]
    pub held: Vec<HeldRoom>,
    #[serde(default)]
    pub failed: Vec<FailedRoom>,
    /// 期望状态带了配置时才有（自次版本 2 起）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<ConfigAck>,
}

/// 节点上外部工具的可用情况，只带是否可用与版本，不带本机路径
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tools {
    pub ffmpeg: ToolStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Hello {
    /// 协议次版本号
    pub proto: u32,
    /// biliup 版本
    pub version: String,
    /// 节点名，默认是主机名；只在首次加入时写进节点表
    pub name: String,
    pub allow_hooks: bool,
    /// 首次加入时出示的一次性秘密；之后的连接只凭节点私钥
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<JoinProof>,
    /// 本机登记的 B 站账号，只有 mid 与昵称（凭据不出节点）。以下字段自次版本 1 起。
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,
    /// 按本地缓存（`data/fleet-state.json`）此刻持有的托管房间，控制面据此按 epoch 对账
    #[serde(default)]
    pub rooms: Vec<HeldRoom>,
    /// 本地缓存对应的期望状态版本号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_version: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JoinProof {
    pub token: String,
    /// 16 字节秘密的小写十六进制
    pub secret: String,
}

impl fmt::Debug for JoinProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinProof")
            .field("token", &self.token)
            .field("secret", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// `SystemMonitor::snapshot(since)`：连上后的第一帧带全部历史，之后只带新采样
    pub stats: SystemStats,
    pub pools: Pools,
    /// 与 `/v1/status` 的 `rooms` 同形，按查看者权限脱敏（钩子、账号凭据路径置空）
    pub rooms: Vec<serde_json::Value>,
    /// 正在录制的房间数
    pub recording: usize,
    /// 本机登记的 B 站账号变了才带（自次版本 1 起）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<Account>>,
    /// 此刻生效的 `min_free_space`（字节），没设或为 0 时不带（自次版本 3 起）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_free_space: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pools {
    pub download: PoolUsage,
    pub upload: PoolUsage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolUsage {
    pub capacity: usize,
    pub occupied: usize,
}

/// 节点上报的事件（自次版本 3 起节点才发）。`kind` 见 [`EVENT_RECORDING_ERROR`]、[`EVENT_UPLOAD_FAILED`]，
/// 不认识的 `kind` 控制面忽略。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    /// Unix 毫秒（节点时钟）
    pub at: i64,
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// 录制出错与投稿失败事件的 `detail`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomEvent {
    pub url: String,
    pub remark: String,
    /// 已去掉链接查询串并截短（见 [`super::events::scrub`]）
    pub error: String,
}

/// 最近一次心跳去掉采样曲线与房间明细后的摘要，控制面落进 `fleet_nodes.last_summary`，
/// 节点离线时列表靠它显示。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub pools: Pools,
    pub rooms: usize,
    pub recording: usize,
    pub cpu: Option<CpuInfo>,
    pub memory: Option<MemoryUsage>,
    pub disk: Option<DiskUsage>,
    pub interfaces: Vec<String>,
}

impl Summary {
    pub fn of(heartbeat: &Heartbeat) -> Self {
        Summary {
            pools: heartbeat.pools,
            rooms: heartbeat.rooms.len(),
            recording: heartbeat.recording,
            cpu: heartbeat.stats.cpu,
            memory: heartbeat.stats.memory,
            disk: heartbeat.stats.disk.clone(),
            interfaces: heartbeat.stats.interfaces.clone(),
        }
    }
}

#[derive(Debug)]
pub enum FrameError {
    Io(std::io::Error),
    TooLarge(usize),
    Json(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Io(e) => write!(f, "{e}"),
            FrameError::TooLarge(len) => write!(f, "frame of {len} bytes exceeds {MAX_FRAME}"),
            FrameError::Json(e) => write!(f, "malformed frame: {e}"),
        }
    }
}

impl std::error::Error for FrameError {}

pub async fn write_frame<W, T>(writer: &mut W, message: &T) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let body = serde_json::to_vec(message).map_err(FrameError::Json)?;
    if body.len() > MAX_FRAME {
        return Err(FrameError::TooLarge(body.len()));
    }
    writer
        .write_all(&(body.len() as u32).to_be_bytes())
        .await
        .map_err(FrameError::Io)?;
    writer.write_all(&body).await.map_err(FrameError::Io)?;
    writer.flush().await.map_err(FrameError::Io)
}

/// 读一帧的原始 JSON 字节。对端正常关流时返回 `None`。
pub async fn read_frame_bytes<R>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError>
where
    R: AsyncRead + Unpin,
{
    let mut len = [0u8; 4];
    match reader.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(FrameError::Io(e)),
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(FrameError::TooLarge(len));
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await.map_err(FrameError::Io)?;
    Ok(Some(body))
}

pub fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, FrameError> {
    serde_json::from_slice(body).map_err(FrameError::Json)
}

pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>, FrameError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    match read_frame_bytes(reader).await? {
        Some(body) => decode(&body).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_round_trip_with_big_endian_length() {
        let (mut a, mut b) = tokio::io::duplex(64 * 1024);
        let message = ControllerMessage::Welcome {
            node_id: 7,
            relays: vec!["http://10.0.0.2:19160/".into()],
        };
        write_frame(&mut a, &message).await.unwrap();
        drop(a);

        let mut raw = Vec::new();
        b.read_to_end(&mut raw).await.unwrap();
        let body = serde_json::to_vec(&message).unwrap();
        assert_eq!(&raw[..4], &(body.len() as u32).to_be_bytes());
        assert_eq!(&raw[4..], &body[..]);

        let mut reader = &raw[..];
        let decoded: ControllerMessage = read_frame(&mut reader).await.unwrap().unwrap();
        assert!(matches!(
            decoded,
            ControllerMessage::Welcome { node_id: 7, .. }
        ));
        assert!(
            read_frame::<_, ControllerMessage>(&mut reader)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn oversized_frames_are_rejected_before_reading_the_body() {
        let raw = ((MAX_FRAME + 1) as u32).to_be_bytes();
        let mut reader = &raw[..];
        assert!(matches!(
            read_frame_bytes(&mut reader).await,
            Err(FrameError::TooLarge(_))
        ));
    }

    #[test]
    fn messages_are_tagged_by_type() {
        let json = serde_json::to_value(NodeMessage::Leave).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "leave" }));
        let hello: NodeMessage = serde_json::from_value(serde_json::json!({
            "type": "hello",
            "proto": 0,
            "version": "1.2.8",
            "name": "nas",
            "allow_hooks": false,
        }))
        .unwrap();
        let NodeMessage::Hello(hello) = hello else {
            panic!()
        };
        assert!(hello.join.is_none());
        assert!(hello.accounts.is_empty() && hello.rooms.is_empty());
        assert!(hello.tools.is_none() && hello.state_version.is_none());
    }

    /// 次版本 0 的节点不认识 `desired_state` / `ack`，这两种帧只在两端都 ≥ 1 时出现；
    /// 而 0 的节点发来的 `hello` / `heartbeat` 缺新字段也照样能解。
    #[test]
    fn new_frames_are_tagged_and_old_frames_still_decode() {
        let desired = serde_json::to_value(ControllerMessage::DesiredState(DesiredState {
            version: 5,
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(desired["type"], "desired_state");
        let ack: NodeMessage = serde_json::from_value(serde_json::json!({
            "type": "ack",
            "version": 5,
            "held": [{ "id": 1, "epoch": 3 }],
        }))
        .unwrap();
        let NodeMessage::Ack(ack) = ack else { panic!() };
        assert_eq!(ack.held, [HeldRoom { id: 1, epoch: 3 }]);
        assert!(ack.failed.is_empty());

        let heartbeat: NodeMessage = serde_json::from_value(serde_json::json!({
            "type": "heartbeat",
            "stats": {
                "ts": 1,
                "interval_ms": 1000,
                "history_ms": 300000,
                "cpu": null,
                "memory": null,
                "disk": null,
                "interfaces": [],
                "samples": [],
            },
            "pools": Pools::default(),
            "rooms": [],
            "recording": 0,
        }))
        .unwrap();
        let NodeMessage::Heartbeat(heartbeat) = heartbeat else {
            panic!()
        };
        assert!(heartbeat.accounts.is_none());
    }

    /// 次版本 1 与 2 混跑：1 的节点不认识 `config`，解 `DesiredState` 时忽略它；
    /// 1 的节点回的 `Ack` 没有 `config`；控制面不给 1 的节点发配置时帧里也没有这个键。
    #[test]
    fn config_fields_are_optional_in_both_directions() {
        let without = serde_json::to_value(DesiredState {
            version: 3,
            ..Default::default()
        })
        .unwrap();
        assert!(without.get("config").is_none());

        let with = serde_json::to_value(DesiredState {
            version: 4,
            config: Some(DesiredConfig {
                global_version: 2,
                values: serde_json::json!({"segment_time": "01:00:00"})
                    .as_object()
                    .unwrap()
                    .clone(),
            }),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(with["config"]["values"]["segment_time"], "01:00:00");

        /// 次版本 1 的 `DesiredState` 只有这三个字段
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct MinorOne {
            version: u64,
            rooms: Vec<DesiredRoom>,
            templates: Vec<DesiredTemplate>,
        }
        let old: MinorOne = serde_json::from_value(with.clone()).unwrap();
        assert_eq!(old.version, 4);
        let new: DesiredState = serde_json::from_value(with).unwrap();
        assert_eq!(new.config.unwrap().global_version, 2);

        let old_ack: Ack =
            serde_json::from_value(serde_json::json!({"version": 4, "held": []})).unwrap();
        assert!(old_ack.config.is_none());
        let ack = Ack {
            version: 4,
            config: Some(ConfigAck::failed("pool1_size")),
            ..Default::default()
        };
        let round: Ack = serde_json::from_value(serde_json::to_value(&ack).unwrap()).unwrap();
        assert_eq!(round.config, ack.config);
    }

    /// 次版本 2 与 3 混跑：2 的控制面解 3 的心跳时忽略 `min_free_space`，收到 `event` 照样能解；
    /// 2 的节点心跳里没有 `min_free_space`，也从不发 `event`。
    #[test]
    fn event_frames_and_min_free_space_are_compatible_both_ways() {
        let event = NodeMessage::Event(Event {
            kind: EVENT_RECORDING_ERROR.into(),
            at: 5,
            detail: serde_json::to_value(RoomEvent {
                url: "https://live.example/1".into(),
                remark: "r".into(),
                error: "mesio error: boom".into(),
            })
            .unwrap(),
        });
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "event");
        assert_eq!(json["kind"], "recording_error");
        let NodeMessage::Event(back) = serde_json::from_value(json).unwrap() else {
            panic!()
        };
        let detail: RoomEvent = serde_json::from_value(back.detail).unwrap();
        assert_eq!(detail.error, "mesio error: boom");
        // 不认识的 kind 与缺字段的 detail 也能解
        let odd: NodeMessage = serde_json::from_value(
            serde_json::json!({ "type": "event", "kind": "future", "at": 1 }),
        )
        .unwrap();
        let NodeMessage::Event(odd) = odd else {
            panic!()
        };
        assert_eq!(
            serde_json::from_value::<RoomEvent>(odd.detail).unwrap_or_default(),
            RoomEvent::default()
        );

        let heartbeat = |extra: serde_json::Value| {
            let mut json = serde_json::json!({
                "type": "heartbeat",
                "stats": {
                    "ts": 1, "interval_ms": 1000, "history_ms": 300000,
                    "cpu": null, "memory": null, "disk": null, "interfaces": [], "samples": [],
                },
                "pools": Pools::default(),
                "rooms": [],
                "recording": 0,
            });
            json.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            match serde_json::from_value::<NodeMessage>(json).unwrap() {
                NodeMessage::Heartbeat(heartbeat) => heartbeat,
                _ => panic!(),
            }
        };
        assert_eq!(heartbeat(serde_json::json!({})).min_free_space, None);
        let new = heartbeat(serde_json::json!({ "min_free_space": 1024 }));
        assert_eq!(new.min_free_space, Some(1024));
        let out = serde_json::to_value(NodeMessage::Heartbeat(Heartbeat {
            min_free_space: None,
            ..new
        }))
        .unwrap();
        assert!(out.get("min_free_space").is_none());
    }

    /// 次版本 3 与 4 混跑：3 的节点解期望状态时忽略 `ha`，不带 `ha` 的期望状态与以前逐字相同；
    /// `ha` 帧两个方向都按 `type` + `kind` 标记，3 的一端解不了（所以只在两端都 ≥ 4 时发）。
    #[test]
    fn ha_frames_and_assignment_are_compatible_both_ways() {
        use crate::server::fleet::ha::params::{HaMode, HaParams};
        use crate::server::fleet::ha::wire::{ReportedSession, ReportedState, SkipReason};

        let plain = serde_json::to_value(DesiredState {
            version: 3,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            plain,
            serde_json::json!({ "version": 3, "rooms": [], "templates": [] })
        );
        let with = serde_json::to_value(DesiredState {
            version: 4,
            ha: Some(HaAssignment {
                mode: HaMode::Takeover,
                params: HaParams::default(),
                primary: 1,
                rooms: vec![7, 8],
                leader: crate::server::fleet::ha::sync::Side::Controller,
            }),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(with["ha"]["mode"], 2);
        assert_eq!(with["ha"]["rooms"], serde_json::json!([7, 8]));
        assert!(with["ha"].get("leader").is_none(), "{with}");
        assert_eq!(with["ha"]["params"]["offline_grace"], 60);
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct MinorThree {
            version: u64,
            rooms: Vec<DesiredRoom>,
            templates: Vec<DesiredTemplate>,
            #[serde(default)]
            config: Option<DesiredConfig>,
        }
        assert_eq!(
            serde_json::from_value::<MinorThree>(with.clone())
                .unwrap()
                .version,
            4
        );
        let back: DesiredState = serde_json::from_value(with).unwrap();
        assert_eq!(back.ha.unwrap().primary, 1);
        // 参数缺了用默认值（以后加参数时旧控制面发来的也能解）
        let sparse: HaAssignment =
            serde_json::from_value(serde_json::json!({ "mode": 1, "primary": 1 })).unwrap();
        assert_eq!(sparse.params, HaParams::default());
        assert!(sparse.rooms.is_empty());

        let uploaded = NodeMessage::Ha(HaMessage::Uploaded {
            key: "7:1000".into(),
            room: 7,
            bvid: "BV1xx".into(),
            from: 1000,
            to: Some(5000),
            yielded: false,
        });
        let json = serde_json::to_value(&uploaded).unwrap();
        assert_eq!(json["type"], "ha");
        assert_eq!(json["kind"], "uploaded");
        assert_eq!(json["bvid"], "BV1xx");
        assert!(json.get("yielded").is_none());
        let NodeMessage::Ha(back) = serde_json::from_value(json).unwrap() else {
            panic!()
        };
        assert_eq!(back.key(), Some("7:1000"));

        let session: ReportedSession = serde_json::from_value(serde_json::json!({
            "key": "standby:7:2000", "room": 7, "started_at": 2000, "state": "recording",
        }))
        .unwrap();
        assert_eq!(session.state, ReportedState::Recording);
        let report = ControllerMessage::Ha(HaMessage::StandbyReport {
            sessions: vec![session],
        });
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["sessions"][0]["state"], "recording");
        assert!(json["sessions"][0].get("bvid").is_none());
        let ControllerMessage::Ha(HaMessage::StandbyReport { sessions }) =
            serde_json::from_value(json).unwrap()
        else {
            panic!()
        };
        assert_eq!(sessions[0].key, "standby:7:2000");

        let skipped: NodeMessage = serde_json::from_value(serde_json::json!({
            "type": "ha", "kind": "upload_skipped", "key": "7:1", "room": 7, "reason": "filtered",
        }))
        .unwrap();
        assert!(matches!(
            skipped,
            NodeMessage::Ha(HaMessage::UploadSkipped {
                reason: SkipReason::Filtered,
                detail: None,
                ..
            })
        ));
        let manual = NodeMessage::Ha(HaMessage::SessionState {
            key: "standby:7:3000".into(),
            room: 7,
            state: ReportedState::Manual,
            reason: Some("主机投稿失败，模式 2 待人工".into()),
        });
        let json = serde_json::to_value(&manual).unwrap();
        assert_eq!(json["kind"], "session_state");
        assert_eq!(json["state"], "manual");
        let NodeMessage::Ha(back) = serde_json::from_value(json).unwrap() else {
            panic!()
        };
        let NodeMessage::Ha(manual) = manual else {
            panic!()
        };
        assert_eq!(back, manual);
        let bare: NodeMessage = serde_json::from_value(serde_json::json!({
            "type": "ha", "kind": "session_state", "key": "7:1", "room": 7, "state": "done",
        }))
        .unwrap();
        assert!(matches!(
            bare,
            NodeMessage::Ha(HaMessage::SessionState { reason: None, .. })
        ));
        // 次版本 3 的一端不认识 `ha`
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        #[allow(dead_code)]
        enum MinorThreeNode {
            Hello(Hello),
            Heartbeat(Heartbeat),
            Event(Event),
            Leave,
            Ack(Ack),
        }
        let frame = serde_json::to_vec(&uploaded).unwrap();
        assert!(decode::<MinorThreeNode>(&frame).is_err());
    }

    /// 次版本 4 与 5 混跑：4 的节点解期望状态时忽略 `pair`，不带 `pair` 的期望状态与以前逐字相同；
    /// `pair` 帧两个方向都按 `type` + `op` 标记，4 的一端解不了（所以只在两端都 ≥ 5 时发）
    #[test]
    fn pair_frames_and_state_are_compatible_both_ways() {
        use crate::server::fleet::ha::params::{HaMode, HaParams};
        use crate::server::fleet::ha::sync::{Gone, PairEdit, PairRef, PairSecret, Side, Stamp};

        let assignment = HaAssignment {
            mode: HaMode::DualRecord,
            params: HaParams::default(),
            primary: 1,
            rooms: vec![7],
            leader: Side::Controller,
        };
        // 上传主机对调到节点时多一个 `leader`，次版本 4 的节点解得开（它不会被对调）
        let switched = serde_json::to_value(HaAssignment {
            primary: 5,
            leader: Side::Node,
            ..assignment.clone()
        })
        .unwrap();
        assert_eq!(switched["leader"], "node");
        assert_eq!(
            serde_json::from_value::<HaAssignment>(switched)
                .unwrap()
                .leader,
            Side::Node
        );
        let without = serde_json::to_value(DesiredState {
            version: 4,
            ha: Some(assignment.clone()),
            ..Default::default()
        })
        .unwrap();
        assert!(without.get("pair").is_none());
        let stamp = Stamp {
            at: 1_000,
            side: Side::Node,
        };
        let with = serde_json::to_value(DesiredState {
            version: 5,
            ha: Some(assignment),
            pair: Some(PairState {
                rooms: vec![PairRef {
                    id: 7,
                    uid: "n00ff".into(),
                    stamp,
                }],
                gone: vec![Gone {
                    key: "room/8".into(),
                    stamp,
                }],
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(with["pair"]["rooms"][0]["uid"], "n00ff");
        assert_eq!(with["pair"]["gone"][0]["stamp"]["side"], "node");
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct MinorFour {
            version: u64,
            rooms: Vec<DesiredRoom>,
            templates: Vec<DesiredTemplate>,
            #[serde(default)]
            config: Option<DesiredConfig>,
            #[serde(default)]
            ha: Option<HaAssignment>,
        }
        let old: MinorFour = serde_json::from_value(with.clone()).unwrap();
        assert_eq!(old.ha.unwrap().rooms, [7]);
        let back: DesiredState = serde_json::from_value(with).unwrap();
        assert_eq!(back.pair.unwrap().room(7).unwrap().stamp, stamp);

        let edit = NodeMessage::Pair(PairMessage::Edit(PairEdit {
            seq: 2,
            key: "config/segment_time".into(),
            stamp,
            value: Some(serde_json::json!("01:00:00")),
            pin: false,
        }));
        let json = serde_json::to_value(&edit).unwrap();
        assert_eq!(json["type"], "pair");
        assert_eq!(json["op"], "edit");
        let NodeMessage::Pair(PairMessage::Edit(back)) = serde_json::from_value(json).unwrap()
        else {
            panic!()
        };
        assert_eq!(back.seq, 2);
        let secret = ControllerMessage::Pair(PairMessage::Secret(PairSecret {
            seq: 1,
            mid: 42,
            stamp,
            content: Some("{}".into()),
        }));
        let frame = serde_json::to_vec(&secret).unwrap();
        assert!(decode::<ControllerMessage>(&frame).is_ok());
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        #[allow(dead_code)]
        enum MinorFourController {
            Welcome { node_id: i64, relays: Vec<String> },
            Relays { relays: Vec<String> },
            DesiredState(DesiredState),
            Ha(HaMessage),
        }
        assert!(decode::<MinorFourController>(&frame).is_err());
    }

    #[test]
    fn join_secret_is_not_debug_printed() {
        let proof = JoinProof {
            token: "abc123".into(),
            secret: "00112233445566778899aabbccddeeff".into(),
        };
        let debug = format!("{proof:?}");
        assert!(debug.contains("abc123"));
        assert!(!debug.contains("0011"));
    }

    #[test]
    fn close_codes_round_trip() {
        for code in [
            CloseCode::Normal,
            CloseCode::Revoked,
            CloseCode::Superseded,
            CloseCode::Protocol,
            CloseCode::Unauthorized,
        ] {
            assert_eq!(CloseCode::from_code(code as u64), Some(code));
        }
        assert_eq!(CloseCode::from_code(99), None);
    }
}
