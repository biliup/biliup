//! 节点与控制面之间的控制通道。
//!
//! 连接一律由节点发起。一条连接上只开一条长期存在的双向流，帧格式是
//! `u32 大端长度 + JSON`，单帧上限 [`MAX_FRAME`]。主版本号放在 ALPN 里（[`ALPN`]），
//! 次版本号在 [`Hello::proto`] 里，只增字段不改语义，所以两端都忽略不认识的字段。

use crate::server::common::system_stats::{CpuInfo, DiskUsage, MemoryUsage, SystemStats};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"biliup/fleet/1";
/// 协议次版本号
pub const PROTOCOL_MINOR: u32 = 0;
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
}

/// 控制面 → 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControllerMessage {
    /// 对 `Hello` 的应答：分配给这台节点的 id 与控制面当前的 relay 地址
    Welcome { node_id: i64, relays: Vec<String> },
    /// 控制面的 relay 地址变了；节点写回 `data/node.json`，下次重连就用新地址
    Relays { relays: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 节点上报的事件。F1 只定结构，节点还不发。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    /// Unix 毫秒
    pub at: i64,
    #[serde(default)]
    pub detail: serde_json::Value,
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
        assert!(matches!(
            hello,
            NodeMessage::Hello(Hello { join: None, .. })
        ));
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
