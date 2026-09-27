//! 主副配对在控制通道上的形状（协议次版本 4，见 [`super::super::protocol::HA_SINCE`]）。
//!
//! 主机就是控制面进程，备机是节点：备机 → 主机走 `NodeMessage::Ha`，主机 → 备机走 `ControllerMessage::Ha`，
//! 两个方向用同一组 [`HaMessage`]。次版本低于 4 的一端不认识 `ha` 帧，所以只在两端都 ≥ 4 时出现：
//! 控制面只把次版本 ≥ 4 的节点指定为备机；节点只在这条连接上收到过带 [`HaAssignment`] 的期望状态之后才发。

use super::params::{HaMode, HaParams};
use serde::{Deserialize, Serialize};

/// 期望状态里的主副角色：控制面只发给备机
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaAssignment {
    pub mode: HaMode,
    #[serde(default)]
    pub params: HaParams,
    /// 主机（控制面「本机」节点）的 id
    pub primary: i64,
    /// 镜像过来的房间（控制面房间 id）；它们与本机被分派的房间一起在期望状态的 `rooms` 里
    #[serde(default)]
    pub rooms: Vec<i64>,
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
    /// 投稿成功；`from` / `to` 是这份稿件覆盖的录制时段（发送方时钟）
    Uploaded {
        key: String,
        room: i64,
        bvid: String,
        from: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<i64>,
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
            HaMessage::Manual { .. } => "manual",
        }
    }
}
