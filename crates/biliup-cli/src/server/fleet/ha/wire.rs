//! 主副配对在控制通道上的形状（协议次版本 4，见 [`super::super::protocol::HA_SINCE`]）。
//!
//! 主机就是控制面进程，备机是节点：备机 → 主机走 `NodeMessage::Ha`，主机 → 备机走 `ControllerMessage::Ha`，
//! 两个方向用同一组 [`HaMessage`]。次版本低于 4 的一端不认识 `ha` 帧，所以只在两端都 ≥ 4 时出现：
//! 控制面只把次版本 ≥ 4 的节点指定为备机；节点只在这条连接上收到过带 [`HaAssignment`] 的期望状态之后才发。
//!
//! 次版本 5 起上传主备可以对调（[`HaAssignment::leader`]）：这时节点进程跑主机那一侧、控制面进程跑备机那一侧，
//! 场次消息照旧走同一条连接，只是方向反过来。控制面只在节点次版本 ≥ 5 时对调。

use super::params::{HaMode, HaParams};
use super::store::Pair;
use super::sync::Side;
use serde::{Deserialize, Serialize};

fn controller_side() -> Side {
    Side::Controller
}

fn is_controller(side: &Side) -> bool {
    *side == Side::Controller
}

/// 期望状态里的主副角色：控制面只发给配对里的那台节点
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaAssignment {
    pub mode: HaMode,
    #[serde(default)]
    pub params: HaParams,
    /// 此刻上传主机的节点 id
    pub primary: i64,
    /// 镜像过来的房间（控制面房间 id）；它们与本机被分派的房间一起在期望状态的 `rooms` 里
    #[serde(default)]
    pub rooms: Vec<i64>,
    /// 上传主机在哪一台。是控制面（H1 的默认）时不出现在帧里，与次版本 4 的帧逐字相同
    #[serde(default = "controller_side", skip_serializing_if = "is_controller")]
    pub leader: Side,
}

impl HaAssignment {
    pub fn of(pair: &Pair, rooms: Vec<i64>) -> Self {
        HaAssignment {
            mode: pair.mode,
            params: pair.params,
            primary: pair.leader_id(),
            rooms,
            leader: pair.leader(),
        }
    }
}

/// 主机那份没投的原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// 分段都小于 `filtering_threshold`，被过滤掉了
    Filtered,
    /// 没有录到文件
    NoFiles,
    /// 主机回来时这一场备机已经在录或在投，交给备机负责
    Handover,
}

impl SkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            SkipReason::Filtered => "filtered",
            SkipReason::NoFiles => "no_files",
            SkipReason::Handover => "handover",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "filtered" => Some(SkipReason::Filtered),
            "no_files" => Some(SkipReason::NoFiles),
            "handover" => Some(SkipReason::Handover),
            _ => None,
        }
    }
}

/// 备机上一场的状态（`StandbyReport`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportedState {
    Recording,
    /// 模式 1：录完了，等主机的结果
    Holding,
    Uploading,
    Uploaded,
    /// 模式 2：接手录完了，等主机回来先投它那半
    AwaitingPrimary,
    /// 等人工处理
    Manual,
    /// 主机投成了，备机这份不用投
    Done,
    Dropped,
    /// 备机自己投失败了
    Failed,
    /// 模式 2：主机在线，备机只监控不录（不会出现在上报里，留给界面用）
    Standby,
}

impl ReportedState {
    pub fn as_str(self) -> &'static str {
        match self {
            ReportedState::Recording => "recording",
            ReportedState::Holding => "holding",
            ReportedState::Uploading => "uploading",
            ReportedState::Uploaded => "uploaded",
            ReportedState::AwaitingPrimary => "awaiting_primary",
            ReportedState::Manual => "manual",
            ReportedState::Done => "done",
            ReportedState::Dropped => "dropped",
            ReportedState::Failed => "failed",
            ReportedState::Standby => "standby",
        }
    }
}

/// `StandbyReport` 里的一场
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedSession {
    pub key: String,
    pub room: i64,
    /// 这一场在备机上开始录的时刻（备机时钟）
    pub started_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    pub state: ReportedState,
    /// 模式 2 接手的主机场次键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub takeover_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bvid: Option<String>,
    /// 对上了主机场次（`key`）的一场在备机上自己的键：主机那边可能还留着按它记的一行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_from: Option<String>,
}

/// 人工处理：「备机直接投」或「放弃」
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualAction {
    StandbyUpload,
    Drop,
}

impl ManualAction {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "standby-upload" | "standby_upload" => Some(ManualAction::StandbyUpload),
            "drop" => Some(ManualAction::Drop),
            _ => None,
        }
    }
}

/// 场次消息。`key` 是场次键（[`super::key`]），`room` 是控制面房间 id，时刻一律是发送方的 Unix 毫秒。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HaMessage {
    /// 开始录一段（`started_at` 是这一场的开播时刻，断流合并接上的也不变；`at` 是这一段开始的时刻）
    SessionStarted {
        key: String,
        room: i64,
        started_at: i64,
        at: i64,
    },
    /// 这一段录完了；`produced` 为假表示没有交给投稿的文件
    SessionEnded {
        key: String,
        room: i64,
        started_at: i64,
        at: i64,
        produced: bool,
    },
    UploadStarted {
        key: String,
        room: i64,
        at: i64,
    },
    /// 上传中每隔 `progress_interval` 一条，`bytes` 是这一场累计已传的字节
    UploadProgress {
        key: String,
        room: i64,
        bytes: u64,
        at: i64,
    },
    /// 投稿成功；`from` / `to` 是这份稿件覆盖的录制时段（发送方时钟）。
    /// `yielded` 为真表示主机这份在备机接手后没有续录（模式 2），`to` 之后只有备机那份
    Uploaded {
        key: String,
        room: i64,
        bvid: String,
        from: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<i64>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        yielded: bool,
    },
    UploadFailed {
        key: String,
        room: i64,
        reason: String,
    },
    UploadSkipped {
        key: String,
        room: i64,
        reason: SkipReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// 备机重连后的第一条场次消息：它手里每一场的状态
    StandbyReport {
        sessions: Vec<ReportedSession>,
    },
    /// 备机先开录的一场对上了主机的场次：之后的消息都用主机的键 `key`，`from` 是它之前用的备机键
    Adopted {
        key: String,
        room: i64,
        from: String,
    },
    /// 备机的一场进入了主机从别的消息推不出来的状态（等主机、待人工、放弃、不用投）；主机据此列出待人工的场次
    SessionState {
        key: String,
        room: i64,
        state: ReportedState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// 主机界面上点的人工处理，转给备机执行
    Manual {
        key: String,
        action: ManualAction,
    },
}

impl HaMessage {
    /// 消息针对的场次键；`StandbyReport` 没有
    pub fn key(&self) -> Option<&str> {
        match self {
            HaMessage::SessionStarted { key, .. }
            | HaMessage::SessionEnded { key, .. }
            | HaMessage::UploadStarted { key, .. }
            | HaMessage::UploadProgress { key, .. }
            | HaMessage::Uploaded { key, .. }
            | HaMessage::UploadFailed { key, .. }
            | HaMessage::UploadSkipped { key, .. }
            | HaMessage::Adopted { key, .. }
            | HaMessage::SessionState { key, .. }
            | HaMessage::Manual { key, .. } => Some(key),
            HaMessage::StandbyReport { .. } => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            HaMessage::SessionStarted { .. } => "session_started",
            HaMessage::SessionEnded { .. } => "session_ended",
            HaMessage::UploadStarted { .. } => "upload_started",
            HaMessage::UploadProgress { .. } => "upload_progress",
            HaMessage::Uploaded { .. } => "uploaded",
            HaMessage::UploadFailed { .. } => "upload_failed",
            HaMessage::UploadSkipped { .. } => "upload_skipped",
            HaMessage::StandbyReport { .. } => "standby_report",
            HaMessage::Adopted { .. } => "adopted",
            HaMessage::SessionState { .. } => "session_state",
            HaMessage::Manual { .. } => "manual",
        }
    }
}
