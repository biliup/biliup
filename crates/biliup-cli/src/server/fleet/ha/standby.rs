//! 备机这一侧的决策（ha-pair 方案 §3、§4、§6 C–F）。
//!
//! 备机录的每一段（一次拉流任务）是一场 [`Session`]：录的时候只把分段收下、不投；录完按它属于哪种情形
//! （[`Kind`]）与主机发来的场次消息（记在 [`PrimaryView`]）决定这一场怎么办：
//!
//! - 模式 1（[`Kind::Backup`]）：等主机的结果。主机投成 → 不投；主机 `UploadFailed` / `UploadSkipped`、
//!   主机在线却下播后 `upload_start_timeout` 没开始投、开始了但进度停住 `upload_stall_timeout`、
//!   主机离线 ≥ `offline_grace` 再等 `standby_upload_delay` → 备机投完整的一份。
//! - 模式 2 接手（[`Kind::Takeover`]）：等主机回来先投它那半。主机投成 → 把备机这半追加为后续分 P
//!   （主机那份已经录到下播就不用追加）；主机那半被过滤 / 没有文件 → 备机投完整的一份；主机投稿失败或
//!   `manual_timeout` 内没回来 → 待人工处理（「备机直接投」/「放弃」）。
//! - 模式 2 主机离线期间新开播（[`Kind::Normal`]）：录完备机自己投。
//!
//! 主机的场次按房间 + 时间与备机的场次对齐（[`super::key`]）。[`StandbyCore`] 不碰时钟、不做 I/O，
//! 测试用虚拟时间驱动；[`super::agent`] 把它接到钩子、`data/ha-state.json` 与控制通道上。

use super::Hold;
use super::key::{self, Span};
use super::params::{HaMode, HaParams};
use super::wire::{
    HaAssignment, HaMessage, ManualAction, ReportedSession, ReportedState, SkipReason,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use tracing::{info, warn};

/// `StandbyReport` 带上多久以内有过变化的场次（还没了结的一律带上）
pub const REPORT_MS: i64 = 48 * 60 * 60 * 1000;
/// 了结的场次在 `ha-state.json` 里留多久
const PRUNE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// 备机这一场属于哪种情形
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// 主机也在录（模式 1 全部；模式 2 连接断了但主机其实一直在录）：等主机的结果，主机投不了才投
    #[default]
    Backup,
    /// 模式 2：主机离线时接手它正在录的一场，等主机回来先投它那半
    Takeover,
    /// 模式 2：主机离线期间新开播的一场，备机自己投
    Normal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Recording,
    /// 录完了，等主机的结果（模式 1：主机没投、在投都算）
    Holding,
    /// 模式 2 接手：录完了，等主机回来先投它那半
    AwaitingPrimary,
    /// 备机把自己这份作为完整稿件在投
    Uploading,
    /// 备机把自己这半追加到主机的稿件
    Appending,
    Uploaded,
    Appended,
    /// 主机投成了，备机这份不用投
    Done,
    /// 等人工处理
    Manual,
    Dropped,
    /// 备机自己投失败了（可以人工再投）
    Failed,
    /// 备机这一段没有交给投稿的文件
    Empty,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Recording => "recording",
            State::Holding => "holding",
            State::AwaitingPrimary => "awaiting_primary",
            State::Uploading => "uploading",
            State::Appending => "appending",
            State::Uploaded => "uploaded",
            State::Appended => "appended",
            State::Done => "done",
            State::Manual => "manual",
            State::Dropped => "dropped",
            State::Failed => "failed",
            State::Empty => "empty",
        }
    }

    /// 了结了：不会再变
    pub fn settled(self) -> bool {
        matches!(
            self,
            State::Uploaded | State::Appended | State::Done | State::Dropped | State::Empty
        )
    }

    fn reported(self) -> ReportedState {
        match self {
            State::Recording => ReportedState::Recording,
            State::Holding => ReportedState::Holding,
            State::AwaitingPrimary => ReportedState::AwaitingPrimary,
            State::Uploading | State::Appending => ReportedState::Uploading,
            State::Uploaded | State::Appended => ReportedState::Uploaded,
            State::Done | State::Empty => ReportedState::Done,
            State::Manual => ReportedState::Manual,
            State::Dropped => ReportedState::Dropped,
            State::Failed => ReportedState::Failed,
        }
    }
}

/// 收下的一个分段
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSegment {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danmaku: Option<PathBuf>,
    #[serde(default)]
    pub index: usize,
}

/// 录制段在备机本地的信息：之后投稿时用来重建上下文，决策不看
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UnitData {
    pub url: String,
    #[serde(default)]
    pub remark: String,
    /// 切片工作台的场次 id（`stream_sessions.id`）
    #[serde(default)]
    pub stream_session: i64,
    /// 开播信息（去掉了直链、请求头与弹幕 cookie）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<serde_json::Value>,
    #[serde(default)]
    pub segments: Vec<SavedSegment>,
}

/// 备机录的一场。时刻一律是备机时钟的 Unix 毫秒
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// 备机自己的键（`standby:{房间}:{开录毫秒}`），不变
    pub id: String,
    /// 场次消息里用的键：对上主机的场次后就是主机的键
    pub key: String,
    pub room: i64,
    pub kind: Kind,
    pub state: State,
    pub started_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    /// 模式 2 接手的主机场次键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub takeover_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bvid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 进入当前状态的时刻
    pub since: i64,
    pub updated_at: i64,
    /// 分段都收下了（投稿通道已关闭）
    #[serde(default)]
    pub collected: bool,
    /// 正在提交稿件。重启后看到它就不知道提交成没成功，转人工
    #[serde(default)]
    pub submitting: bool,
    #[serde(default)]
    pub unit: UnitData,
}

impl Session {
    fn span(&self) -> Span {
        Span::new(self.started_at, self.ended_at)
    }

    fn ready(&self) -> bool {
        self.ended_at.is_some() && self.collected
    }
}

/// 主机那一场的投稿进展
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryUpload {
    #[default]
    None,
    Started,
    Uploaded,
    Failed,
    Skipped,
}

/// 备机知道的主机的一场
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryView {
    pub key: String,
    pub room: i64,
    /// 主机时钟
    pub started_at: i64,
    /// 主机时钟
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(default)]
    pub upload: PrimaryUpload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip: Option<SkipReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bvid: Option<String>,
    /// 主机稿件覆盖到的时刻（主机时钟）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<i64>,
    /// 主机这份在备机接手后没有续录：不算录到下播
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub yielded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 最近一次收到这一场消息的时刻（备机时钟，下同）
    pub seen_at: i64,
    /// 收到 `SessionEnded` 的时刻
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_seen_at: Option<i64>,
    #[serde(default)]
    pub bytes: u64,
    /// 上传字节最近一次增长（或收到 `UploadStarted`）的时刻
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_at: Option<i64>,
}

impl PrimaryView {
    fn recording(&self) -> bool {
        self.ended_at.is_none()
            && matches!(self.upload, PrimaryUpload::None | PrimaryUpload::Started)
    }
}

/// [`StandbyCore`] 要做的事，由 [`super::agent::Standby`] 执行
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Out {
    /// 发给主机（只在连着时产生）
    Send(HaMessage),
    /// 把这一场作为完整稿件投
    Upload(String),
    /// 把这一场追加到主机的稿件
    Append { id: String, bvid: String },
    /// 主机投成了，删掉备机这份（`delete_standby_copy`）
    Discard(String),
    /// 需要人看一眼（待人工处理、备机投稿失败）
    Attention { id: String, message: String },
}

/// 一场录完、分段收齐后的去向
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    Wait,
    Done(String),
    Upload(String),
    Append(String),
    Manual(String),
}

/// 备机的决策部分。时刻一律由调用方传入（备机时钟，Unix 毫秒）
pub(crate) struct StandbyCore {
    mode: HaMode,
    params: HaParams,
    /// 场次对齐的窗口（`live_merge_minutes`）
    window: i64,
    sessions: BTreeMap<String, Session>,
    primary: BTreeMap<String, PrimaryView>,
    /// 这条连接上已经收到了配对（期望状态里的 `ha`），场次消息可以发
    linked: bool,
    /// 最近一次与主机断开（或备机进程启动）的时刻
    link_down_at: i64,
    /// 模式 2 只监控时最近一次看到房间在播的时刻
    live_seen: HashMap<i64, i64>,
    dirty: bool,
    outs: Vec<Out>,
}

impl StandbyCore {
    /// 备机进程启动（或刚被指定为备机）：录到一半退出的场次按录完处理；投到一半退出的，还没提交就重投，
    /// 正在提交的不知道成没成功，转人工
    pub(crate) fn new(
        assignment: &HaAssignment,
        window: i64,
        sessions: Vec<Session>,
        primary: Vec<PrimaryView>,
        now: i64,
    ) -> Self {
        let mut core = StandbyCore {
            mode: assignment.mode,
            params: assignment.params,
            window,
            sessions: BTreeMap::new(),
            primary: primary
                .into_iter()
                .map(|view| (view.key.clone(), view))
                .collect(),
            linked: false,
            link_down_at: now,
            live_seen: HashMap::new(),
            dirty: false,
            outs: Vec::new(),
        };
        for mut session in sessions {
            if session.ended_at.is_some() || session.state == State::Recording {
                session.collected = true;
            }
            let id = session.id.clone();
            let state = session.state;
            let submitting = session.submitting;
            core.sessions.insert(id.clone(), session);
            match state {
                State::Recording => {
                    let produced = core.sessions[&id].unit.segments.len();
                    let ended = core.sessions[&id].updated_at;
                    warn!(id, "HA：备机上次在录这一场时退出了，按录完处理");
                    core.finish(now, &id, ended, produced > 0);
                }
                State::Uploading | State::Appending if submitting => core.require_manual(
                    now,
                    &id,
                    "备机在提交稿件时退出，不知道提交成没成功：请到 B 站稿件页确认后再选「备机直接投」或「放弃」"
                        .into(),
                ),
                State::Uploading => {
                    info!(id, "HA：备机上次投到一半退出了，重新投");
                    core.outs.push(Out::Upload(id));
                }
                State::Appending => {
                    let bvid = core.sessions[&id].bvid.clone().unwrap_or_default();
                    info!(id, bvid, "HA：备机上次追加分 P 到一半退出了，重新追加");
                    core.outs.push(Out::Append { id, bvid });
                }
                _ => {}
            }
        }
        core
    }

    pub(crate) fn take(&mut self) -> Vec<Out> {
        std::mem::take(&mut self.outs)
    }

    /// 有没有要写进 `ha-state.json` 的变化（取走标记）
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub(crate) fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub(crate) fn primary_views(&self) -> impl Iterator<Item = &PrimaryView> {
        self.primary.values()
    }

    pub(crate) fn session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    #[cfg(test)]
    pub(crate) fn linked(&self) -> bool {
        self.linked
    }

    /// 控制面改了模式或参数；已经在进行的场次保持原来的情形
    pub(crate) fn assign(&mut self, assignment: &HaAssignment) {
        self.mode = assignment.mode;
        self.params = assignment.params;
    }

    fn send(&mut self, message: HaMessage) {
        if self.linked {
            self.outs.push(Out::Send(message));
        }
    }

    fn touch(&mut self, id: &str, now: i64) -> Option<&mut Session> {
        let session = self.sessions.get_mut(id)?;
        session.updated_at = now;
        self.dirty = true;
        Some(session)
    }

    /// 连上并收到配对：先把手里每一场的状态报给主机
    pub(crate) fn link_up(&mut self, now: i64) -> HaMessage {
        self.linked = true;
        HaMessage::StandbyReport {
            sessions: self.report(now),
        }
    }

    pub(crate) fn link_down(&mut self, now: i64) {
        if self.linked {
            self.linked = false;
            self.link_down_at = now;
        }
    }

    /// 与控制面断开且 `offline_grace` 内没重连上 = 主机离线（§1）
    pub(crate) fn primary_offline(&self, now: i64) -> bool {
        !self.linked && now - self.link_down_at >= HaParams::ms(self.params.offline_grace)
    }

    fn report(&self, now: i64) -> Vec<ReportedSession> {
        self.sessions
            .values()
            .filter(|session| !session.state.settled() || session.updated_at >= now - REPORT_MS)
            .map(|session| ReportedSession {
                key: session.key.clone(),
                room: session.room,
                started_at: session.started_at,
                ended_at: session.ended_at,
                state: session.state.reported(),
                takeover_of: session.takeover_of.clone(),
                bvid: session.bvid.clone(),
                adopted_from: (session.key != session.id).then(|| session.id.clone()),
            })
            .collect()
    }

    /// 监控循环检测到开播（`core/monitor.rs`）：模式 2 主机在线时只监控不录，记下房间在播
    pub(crate) fn hold(&mut self, now: i64, room: i64) -> Option<Hold> {
        if self.mode != HaMode::Takeover || self.primary_offline(now) {
            return None;
        }
        self.live_seen.insert(room, now);
        Some(Hold {
            reason: "模式 2：主机在线，备机只监控不录".into(),
            quick: false,
        })
    }

    /// 主机在录、备机能接手的那一场：有「只监控」看到它在播的证据，或它开播得不久
    fn takeover_candidate(&self, room: i64, now: i64) -> Option<String> {
        let seen_live = self
            .live_seen
            .get(&room)
            .is_some_and(|seen| now - seen <= self.window);
        self.primary
            .values()
            .filter(|view| view.room == room && view.recording() && !key::is_standby_key(&view.key))
            .filter(|view| seen_live || (now - view.started_at).abs() <= self.window)
            .max_by_key(|view| view.started_at)
            .map(|view| view.key.clone())
    }

    /// 备机开录一段
    pub(crate) fn unit_started(
        &mut self,
        now: i64,
        id: &str,
        room: i64,
        started_at: i64,
        unit: UnitData,
    ) {
        if self.sessions.contains_key(id) {
            return;
        }
        let (kind, takeover_of) = match self.mode {
            HaMode::DualRecord => (Kind::Backup, None),
            HaMode::Takeover => match self.takeover_candidate(room, now) {
                Some(of) => (Kind::Takeover, Some(of)),
                None => (Kind::Normal, None),
            },
        };
        let key = match kind {
            Kind::Takeover => id.to_string(),
            Kind::Backup | Kind::Normal => self
                .adoptable(room, started_at)
                .unwrap_or_else(|| id.to_string()),
        };
        info!(id, key, ?kind, takeover_of, "HA：备机开录");
        let session = Session {
            id: id.to_string(),
            key: key.clone(),
            room,
            kind,
            started_at,
            takeover_of,
            since: now,
            updated_at: now,
            unit,
            ..Session::default()
        };
        self.sessions.insert(id.to_string(), session);
        self.dirty = true;
        self.send(HaMessage::SessionStarted {
            key,
            room,
            started_at,
            at: now,
        });
    }

    /// 主机那边与 `started_at` 开播时刻最接近（窗口内）的一场的键
    fn adoptable(&self, room: i64, started_at: i64) -> Option<String> {
        let candidates = self
            .primary
            .values()
            .filter(|view| view.room == room && !key::is_standby_key(&view.key))
            .map(|view| (view.key.as_str(), Span::new(view.started_at, view.ended_at)));
        key::best_match(Span::new(started_at, None), candidates, self.window).map(str::to_string)
    }

    /// 投稿通道收下一个分段
    pub(crate) fn segment(&mut self, now: i64, id: &str, segment: SavedSegment) {
        if let Some(session) = self.touch(id, now)
            && !session.unit.segments.iter().any(|s| s.path == segment.path)
        {
            session.unit.segments.push(segment);
        }
    }

    /// 投稿通道关闭：分段都收齐了
    pub(crate) fn collected(&mut self, now: i64, id: &str) {
        if let Some(session) = self.touch(id, now) {
            session.collected = true;
        }
        self.evaluate(now, id);
    }

    /// 备机录完一段；`produced` 为假表示没有交给投稿的文件
    pub(crate) fn unit_ended(&mut self, now: i64, id: &str, produced: bool) {
        let Some(session) = self.sessions.get(id) else {
            return;
        };
        let message = HaMessage::SessionEnded {
            key: session.key.clone(),
            room: session.room,
            started_at: session.started_at,
            at: now,
            produced,
        };
        self.send(message);
        self.finish(now, id, now, produced);
    }

    fn finish(&mut self, now: i64, id: &str, ended_at: i64, produced: bool) {
        let Some(session) = self.touch(id, now) else {
            return;
        };
        session.ended_at = Some(ended_at);
        session.since = now;
        session.state = if !produced {
            State::Empty
        } else if session.kind == Kind::Takeover {
            State::AwaitingPrimary
        } else {
            State::Holding
        };
        info!(id, state = session.state.as_str(), "HA：备机录完一段");
        self.tell_state(id);
        self.evaluate(now, id);
    }

    /// 与备机这一场对得上的主机场次：键相同、是它接手的那一场，或真的在时间上重叠
    fn matched(&self, session: &Session, now: i64) -> Vec<&PrimaryView> {
        let span = session.span();
        self.primary
            .values()
            .filter(|view| view.room == session.room)
            .filter(|view| {
                view.key == session.key
                    || view.key == session.id
                    || session.takeover_of.as_deref() == Some(view.key.as_str())
                    || (!key::is_standby_key(&view.key)
                        && key::overlaps(self.primary_span(view), span, now))
            })
            .collect()
    }

    /// 主机那一场在时间轴上的范围。还没结束的：连着就按录到此刻算，断了就只算到断开的时刻
    fn primary_span(&self, view: &PrimaryView) -> Span {
        let end = match view.ended_at {
            Some(end) => Some(end),
            None if self.linked => None,
            None => Some(self.link_down_at.max(view.started_at)),
        };
        Span::new(view.started_at, end)
    }

    /// 模式 1：主机那一场还没结果时，最晚等到什么时候（主机还在录就不设期限）
    fn pending_deadline(&self, view: &PrimaryView) -> Option<(i64, &'static str)> {
        let ended = view.ended_seen_at?;
        match view.upload {
            PrimaryUpload::None => Some((
                ended + HaParams::ms(self.params.upload_start_timeout),
                "主机在线，但下播后 upload_start_timeout 内没有开始投",
            )),
            PrimaryUpload::Started => Some((
                view.progress_at.unwrap_or(ended).max(ended)
                    + HaParams::ms(self.params.upload_stall_timeout),
                "主机开始投了，但上传进度停住超过 upload_stall_timeout",
            )),
            _ => None,
        }
    }

    fn backup_verdict(&self, session: &Session, now: i64) -> Verdict {
        let set = self.matched(session, now);
        let ended = session.ended_at.unwrap_or(now);
        let offline = self.primary_offline(now);
        let offline_due = offline
            && now
                >= (self.link_down_at + HaParams::ms(self.params.offline_grace)).max(ended)
                    + HaParams::ms(self.params.standby_upload_delay);
        if set.is_empty() {
            if offline {
                return if offline_due {
                    Verdict::Upload("主机离线超过 offline_grace + standby_upload_delay".into())
                } else {
                    Verdict::Wait
                };
            }
            return if now >= ended + HaParams::ms(self.params.upload_start_timeout) {
                Verdict::Upload("主机在线，但下播后 upload_start_timeout 内没有这一场的投稿".into())
            } else {
                Verdict::Wait
            };
        }
        let mut uploaded: Vec<&PrimaryView> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        let mut pending = false;
        for view in &set {
            match view.upload {
                PrimaryUpload::Uploaded => uploaded.push(view),
                PrimaryUpload::Failed => failed.push(format!(
                    "主机投稿失败：{}",
                    view.reason.as_deref().unwrap_or("未知原因")
                )),
                PrimaryUpload::Skipped => match view.skip {
                    Some(SkipReason::Handover) => failed.push("主机那份没有录完或没有投完".into()),
                    Some(SkipReason::Filtered) | Some(SkipReason::NoFiles) | None => {}
                },
                PrimaryUpload::None | PrimaryUpload::Started => match self.pending_deadline(view) {
                    Some((deadline, reason)) if now >= deadline => failed.push(reason.into()),
                    _ if offline_due => {
                        failed.push("主机离线超过 offline_grace + standby_upload_delay".into())
                    }
                    _ => pending = true,
                },
            }
        }
        if pending {
            return Verdict::Wait;
        }
        match (uploaded.is_empty(), failed.first()) {
            (false, None) => Verdict::Done(format!("主机投成了（{}）", bvids(&uploaded))),
            (true, Some(reason)) => Verdict::Upload(reason.clone()),
            (true, None) => Verdict::Upload("主机那份被过滤或没有文件".into()),
            (false, Some(reason)) => Verdict::Manual(format!(
                "主机只投成了这一场的一部分（{}），另一部分{}；备机再投整场会重复",
                bvids(&uploaded),
                reason
            )),
        }
    }

    fn takeover_verdict(&self, session: &Session, now: i64) -> Verdict {
        let set = self.matched(session, now);
        if let Some(view) = set.iter().find(|view| view.upload == PrimaryUpload::Failed) {
            return Verdict::Manual(format!(
                "主机那半投稿失败：{}",
                view.reason.as_deref().unwrap_or("未知原因")
            ));
        }
        let pending = set
            .iter()
            .any(|view| matches!(view.upload, PrimaryUpload::None | PrimaryUpload::Started));
        if set.is_empty() || pending {
            return if now >= session.since + HaParams::ms(self.params.manual_timeout) {
                Verdict::Manual("主机在 manual_timeout 内没有回来投它那半".into())
            } else {
                Verdict::Wait
            };
        }
        let latest = set
            .iter()
            .filter(|view| view.upload == PrimaryUpload::Uploaded)
            .max_by_key(|view| view.to.unwrap_or(view.started_at));
        let Some(latest) = latest else {
            return Verdict::Upload("主机那半被过滤或没有文件，备机投完整的一份".into());
        };
        let ended = session.ended_at.unwrap_or(now);
        if !latest.yielded && latest.to.is_some_and(|to| to >= ended - key::SKEW_MS) {
            return Verdict::Done(format!(
                "主机那份（{}）已经录到下播",
                latest.bvid.as_deref().unwrap_or_default()
            ));
        }
        Verdict::Append(latest.bvid.clone().unwrap_or_default())
    }

    /// 录完、分段收齐、还在等的场次，按模式与主机的消息决定去向
    fn evaluate(&mut self, now: i64, id: &str) {
        let Some(session) = self.sessions.get(id) else {
            return;
        };
        if !session.ready() || !matches!(session.state, State::Holding | State::AwaitingPrimary) {
            return;
        }
        let verdict = match session.kind {
            Kind::Backup => self.backup_verdict(session, now),
            Kind::Takeover => self.takeover_verdict(session, now),
            Kind::Normal => {
                let real = self
                    .matched(session, now)
                    .iter()
                    .any(|view| !key::is_standby_key(&view.key));
                if real {
                    // 主机其实一直在录（断线而不是宕机）：按模式 1 等它的结果，免得两份稿件
                    info!(id, "HA：主机也录了这一场，备机改为等主机的结果");
                    if let Some(session) = self.touch(id, now) {
                        session.kind = Kind::Backup;
                    }
                    return self.evaluate(now, id);
                }
                Verdict::Upload("主机离线期间开播的一场".into())
            }
        };
        match verdict {
            Verdict::Wait => {}
            Verdict::Done(reason) => {
                info!(id, reason, "HA：备机这份不用投");
                let discard = self.params.delete_standby_copy;
                self.settle(now, id, State::Done, reason);
                if discard {
                    self.outs.push(Out::Discard(id.to_string()));
                }
            }
            Verdict::Upload(reason) => self.start_upload(now, id, reason),
            Verdict::Append(bvid) => {
                info!(
                    id,
                    bvid, "HA：主机投成了它那半，备机把自己这半追加为后续分 P"
                );
                if let Some(session) = self.touch(id, now) {
                    session.state = State::Appending;
                    session.since = now;
                    session.bvid = Some(bvid.clone());
                    session.reason = Some("主机投成了它那半".into());
                }
                self.outs.push(Out::Append {
                    id: id.to_string(),
                    bvid,
                });
            }
            Verdict::Manual(reason) => self.require_manual(now, id, reason),
        }
    }

    fn settle(&mut self, now: i64, id: &str, state: State, reason: String) {
        if let Some(session) = self.touch(id, now) {
            session.state = state;
            session.since = now;
            session.reason = Some(reason);
        }
        self.tell_state(id);
    }

    /// 等主机、待人工、放弃、不用投：主机从别的消息推不出来，单独告诉它
    fn tell_state(&mut self, id: &str) {
        let Some(session) = self.sessions.get(id) else {
            return;
        };
        if !matches!(
            session.state,
            State::AwaitingPrimary | State::Manual | State::Done | State::Dropped
        ) {
            return;
        }
        let message = HaMessage::SessionState {
            key: session.key.clone(),
            room: session.room,
            state: session.state.reported(),
            reason: session.reason.clone(),
        };
        self.send(message);
    }

    fn start_upload(&mut self, now: i64, id: &str, reason: String) {
        info!(id, reason, "HA：备机投这一场");
        self.settle(now, id, State::Uploading, reason);
        self.outs.push(Out::Upload(id.to_string()));
    }

    pub(crate) fn require_manual(&mut self, now: i64, id: &str, reason: String) {
        warn!(
            id,
            reason, "HA：这一场待人工处理（「备机直接投」或「放弃」）"
        );
        self.settle(now, id, State::Manual, reason.clone());
        self.outs.push(Out::Attention {
            id: id.to_string(),
            message: format!("一主一备：这一场待人工处理——{reason}"),
        });
    }

    fn find(&self, key: &str) -> Option<String> {
        if self.sessions.contains_key(key) {
            return Some(key.to_string());
        }
        self.sessions
            .values()
            .find(|session| session.key == key)
            .map(|session| session.id.clone())
    }

    /// 人工处理（主机面板转来的，或备机本地节点页上点的）
    pub(crate) fn manual(
        &mut self,
        now: i64,
        key: &str,
        action: ManualAction,
    ) -> Result<(), String> {
        let id = self.find(key).ok_or_else(|| format!("没有场次 {key}"))?;
        let session = &self.sessions[&id];
        if !matches!(
            session.state,
            State::AwaitingPrimary | State::Manual | State::Failed
        ) {
            return Err(format!(
                "场次 {key} 现在是 {}，不需要人工处理",
                session.state.as_str()
            ));
        }
        if !session.ready() {
            return Err(format!("场次 {key} 还没录完"));
        }
        match action {
            ManualAction::StandbyUpload => {
                self.start_upload(now, &id, "人工选择「备机直接投」".into())
            }
            ManualAction::Drop => {
                info!(id, "HA：人工选择「放弃」这一场");
                self.settle(now, &id, State::Dropped, "人工选择「放弃」".into());
            }
        }
        Ok(())
    }

    /// 备机的投稿开始传分段
    pub(crate) fn upload_started(&mut self, now: i64, id: &str) {
        let Some(session) = self.sessions.get(id) else {
            return;
        };
        let message = HaMessage::UploadStarted {
            key: session.key.clone(),
            room: session.room,
            at: now,
        };
        self.send(message);
    }

    /// 提交前最后确认一次：作为完整稿件投的，主机那边已经投成了就不提交。确认后记下「正在提交」
    pub(crate) fn begin_submit(&mut self, now: i64, id: &str) -> bool {
        let Some(session) = self.sessions.get(id) else {
            return false;
        };
        if session.state == State::Uploading
            && self
                .matched(session, now)
                .iter()
                .any(|view| view.upload == PrimaryUpload::Uploaded)
        {
            return false;
        }
        if let Some(session) = self.touch(id, now) {
            session.submitting = true;
        }
        true
    }

    fn sent_upload(&mut self, now: i64, id: &str, state: State, bvid: &str) {
        let Some(session) = self.touch(id, now) else {
            return;
        };
        session.state = state;
        session.since = now;
        session.submitting = false;
        session.bvid = Some(bvid.to_string());
        let message = HaMessage::Uploaded {
            key: session.key.clone(),
            room: session.room,
            bvid: bvid.to_string(),
            from: session.started_at,
            to: session.ended_at,
            yielded: false,
        };
        self.send(message);
    }

    pub(crate) fn uploaded(&mut self, now: i64, id: &str, bvid: &str) {
        info!(id, bvid, "HA：备机投成");
        self.sent_upload(now, id, State::Uploaded, bvid);
    }

    /// 备机这半追加到了主机的稿件（同一个稿件号）
    pub(crate) fn appended(&mut self, now: i64, id: &str, bvid: &str) {
        info!(id, bvid, "HA：备机这半已追加为主机稿件的后续分 P");
        self.sent_upload(now, id, State::Appended, bvid);
    }

    pub(crate) fn upload_failed(&mut self, now: i64, id: &str, reason: &str) {
        let Some(session) = self.touch(id, now) else {
            return;
        };
        session.state = State::Failed;
        session.since = now;
        session.submitting = false;
        session.reason = Some(reason.to_string());
        let message = HaMessage::UploadFailed {
            key: session.key.clone(),
            room: session.room,
            reason: reason.to_string(),
        };
        warn!(id, reason, "HA：备机投稿失败");
        self.send(message);
        self.outs.push(Out::Attention {
            id: id.to_string(),
            message: format!("一主一备：备机投这一场失败——{reason}"),
        });
    }

    /// 分段传完了，但提交前发现主机已经投成：不提交，回到等待重新判断
    pub(crate) fn upload_fenced(&mut self, now: i64, id: &str) {
        let Some(session) = self.touch(id, now) else {
            return;
        };
        session.submitting = false;
        session.since = now;
        session.state = if session.kind == Kind::Takeover {
            State::AwaitingPrimary
        } else {
            State::Holding
        };
        info!(id, "HA：提交前发现主机已经投成，备机不提交");
        self.evaluate(now, id);
    }

    /// 主机发来的场次消息
    pub(crate) fn primary_message(&mut self, now: i64, message: HaMessage) {
        let room = match &message {
            HaMessage::Manual { key, action } => {
                if let Err(e) = self.manual(now, key, *action) {
                    warn!(key, error = e, "HA：主机转来的人工处理没有执行");
                }
                return;
            }
            HaMessage::StandbyReport { .. }
            | HaMessage::SessionState { .. }
            | HaMessage::Adopted { .. } => return,
            HaMessage::SessionStarted { room, .. }
            | HaMessage::SessionEnded { room, .. }
            | HaMessage::UploadStarted { room, .. }
            | HaMessage::UploadProgress { room, .. }
            | HaMessage::Uploaded { room, .. }
            | HaMessage::UploadFailed { room, .. }
            | HaMessage::UploadSkipped { room, .. } => *room,
        };
        let key = message.key().unwrap_or_default().to_string();
        let started_at = match &message {
            HaMessage::SessionStarted { started_at, .. }
            | HaMessage::SessionEnded { started_at, .. } => *started_at,
            HaMessage::Uploaded { from, .. } => *from,
            _ => key::parse(&key).map_or(now, |(_, started_at)| started_at),
        };
        let view = self
            .primary
            .entry(key.clone())
            .or_insert_with(|| PrimaryView {
                key: key.clone(),
                room,
                started_at,
                ..PrimaryView::default()
            });
        view.seen_at = now;
        let settled = matches!(
            view.upload,
            PrimaryUpload::Uploaded | PrimaryUpload::Failed | PrimaryUpload::Skipped
        );
        match message {
            HaMessage::SessionStarted { .. } => {}
            HaMessage::SessionEnded { at, .. } => {
                view.ended_at = Some(at);
                view.ended_seen_at.get_or_insert(now);
            }
            HaMessage::UploadStarted { .. } => {
                if !settled {
                    view.upload = PrimaryUpload::Started;
                    view.progress_at = Some(now);
                }
            }
            HaMessage::UploadProgress { bytes, .. } => {
                if !settled {
                    view.upload = PrimaryUpload::Started;
                    if bytes > view.bytes || view.progress_at.is_none() {
                        view.bytes = bytes;
                        view.progress_at = Some(now);
                    }
                }
            }
            HaMessage::Uploaded {
                bvid, to, yielded, ..
            } => {
                view.upload = PrimaryUpload::Uploaded;
                view.bvid = Some(bvid);
                view.to = to;
                view.yielded = yielded;
                view.ended_at = view.ended_at.or(to);
                view.reason = None;
            }
            HaMessage::UploadFailed { reason, .. } => {
                if view.upload != PrimaryUpload::Uploaded {
                    view.upload = PrimaryUpload::Failed;
                    view.reason = Some(reason);
                }
            }
            HaMessage::UploadSkipped { reason, detail, .. } => {
                if view.upload != PrimaryUpload::Uploaded {
                    view.upload = PrimaryUpload::Skipped;
                    view.skip = Some(reason);
                    view.reason = detail;
                }
            }
            HaMessage::StandbyReport { .. }
            | HaMessage::SessionState { .. }
            | HaMessage::Adopted { .. }
            | HaMessage::Manual { .. } => {}
        }
        self.dirty = true;
        if !key::is_standby_key(&key) {
            self.adopt(now, &key, room, started_at);
        }
        let ids: Vec<String> = self
            .sessions
            .values()
            .filter(|session| session.room == room)
            .map(|session| session.id.clone())
            .collect();
        for id in ids {
            self.evaluate(now, &id);
        }
    }

    /// 备机先开录、主机后开录的一场：还在用自己键的场次改用主机的键
    fn adopt(&mut self, now: i64, key: &str, room: i64, started_at: i64) {
        let window = self.window;
        let adopters: Vec<String> = self
            .sessions
            .values()
            .filter(|session| {
                session.room == room
                    && session.key == session.id
                    && session.kind != Kind::Takeover
                    && !session.state.settled()
                    && (session.started_at - started_at).abs() <= window
            })
            .map(|session| session.id.clone())
            .collect();
        for id in adopters {
            if let Some(session) = self.touch(&id, now) {
                info!(id, key, "HA：备机这一场对上了主机的场次");
                session.key = key.to_string();
                self.send(HaMessage::Adopted {
                    key: key.to_string(),
                    room,
                    from: id,
                });
            }
        }
    }

    /// 每秒一次：到期的等待按规则往下走；清掉很久以前了结的场次
    pub(crate) fn tick(&mut self, now: i64) {
        let waiting: Vec<String> = self
            .sessions
            .values()
            .filter(|session| matches!(session.state, State::Holding | State::AwaitingPrimary))
            .map(|session| session.id.clone())
            .collect();
        for id in waiting {
            self.evaluate(now, &id);
        }
        let before = self.sessions.len();
        self.sessions
            .retain(|_, session| !session.state.settled() || session.updated_at >= now - PRUNE_MS);
        let referenced: Vec<(i64, Span)> = self
            .sessions
            .values()
            .filter(|session| !session.state.settled())
            .map(|session| (session.room, session.span()))
            .collect();
        let views = self.primary.len();
        let linked = self.linked;
        let link_down_at = self.link_down_at;
        self.primary.retain(|_, view| {
            view.seen_at >= now - REPORT_MS
                || referenced.iter().any(|(room, span)| {
                    let end = view.ended_at.or((!linked).then_some(link_down_at));
                    *room == view.room && key::overlaps(Span::new(view.started_at, end), *span, now)
                })
        });
        if self.sessions.len() != before || self.primary.len() != views {
            self.dirty = true;
        }
        let window = self.window;
        self.live_seen.retain(|_, seen| now - *seen <= window);
    }
}

fn bvids(views: &[&PrimaryView]) -> String {
    views
        .iter()
        .filter_map(|view| view.bvid.as_deref())
        .collect::<Vec<_>>()
        .join("、")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;
    const WINDOW: i64 = 10 * MIN;
    const ROOM: i64 = 7;

    fn assignment(mode: HaMode) -> HaAssignment {
        HaAssignment {
            mode,
            params: HaParams::default(),
            primary: 1,
            rooms: vec![ROOM],
        }
    }

    fn core(mode: HaMode) -> StandbyCore {
        StandbyCore::new(&assignment(mode), WINDOW, Vec::new(), Vec::new(), 0)
    }

    /// 连上、收到配对
    fn linked(mode: HaMode) -> StandbyCore {
        let mut core = core(mode);
        core.link_up(0);
        core
    }

    fn id(started_at: i64) -> String {
        key::standby_key(ROOM, started_at)
    }

    fn primary_key(started_at: i64) -> String {
        key::primary_key(ROOM, started_at)
    }

    fn unit() -> UnitData {
        UnitData {
            url: "https://live.example/7".into(),
            segments: vec![SavedSegment {
                path: "a.flv".into(),
                ..SavedSegment::default()
            }],
            ..UnitData::default()
        }
    }

    /// 录一段 `[start, end]`，分段收齐
    fn record(core: &mut StandbyCore, start: i64, end: i64) -> String {
        let id = id(start);
        core.unit_started(start, &id, ROOM, start, unit());
        core.collected(end, &id);
        core.unit_ended(end, &id, true);
        id
    }

    fn state(core: &StandbyCore, id: &str) -> State {
        core.session(id).unwrap().state
    }

    fn outs(core: &mut StandbyCore) -> Vec<Out> {
        core.take()
    }

    fn uploads(outs: &[Out]) -> Vec<String> {
        outs.iter()
            .filter_map(|out| match out {
                Out::Upload(id) => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    fn started(key: &str, started_at: i64) -> HaMessage {
        HaMessage::SessionStarted {
            key: key.into(),
            room: ROOM,
            started_at,
            at: started_at,
        }
    }

    fn ended(key: &str, started_at: i64, at: i64) -> HaMessage {
        HaMessage::SessionEnded {
            key: key.into(),
            room: ROOM,
            started_at,
            at,
            produced: true,
        }
    }

    fn uploaded(key: &str, from: i64, to: Option<i64>) -> HaMessage {
        HaMessage::Uploaded {
            key: key.into(),
            room: ROOM,
            bvid: "BVP".into(),
            from,
            to,
            yielded: false,
        }
    }

    fn upload_started(key: &str) -> HaMessage {
        HaMessage::UploadStarted {
            key: key.into(),
            room: ROOM,
            at: 0,
        }
    }

    fn progress(key: &str, bytes: u64) -> HaMessage {
        HaMessage::UploadProgress {
            key: key.into(),
            room: ROOM,
            bytes,
            at: 0,
        }
    }

    fn skipped(key: &str, reason: SkipReason) -> HaMessage {
        HaMessage::UploadSkipped {
            key: key.into(),
            room: ROOM,
            reason,
            detail: None,
        }
    }

    fn failed(key: &str) -> HaMessage {
        HaMessage::UploadFailed {
            key: key.into(),
            room: ROOM,
            reason: "拒稿".into(),
        }
    }

    // ---------- 场次键 ----------

    #[test]
    fn the_standby_adopts_the_primary_key_whichever_side_starts_first() {
        let mut core = linked(HaMode::DualRecord);
        // 主机先开录
        core.primary_message(MIN, started(&primary_key(MIN), MIN));
        let first = id(MIN + 20_000);
        core.unit_started(MIN + 20_000, &first, ROOM, MIN + 20_000, unit());
        assert_eq!(core.session(&first).unwrap().key, primary_key(MIN));
        let sent = outs(&mut core);
        assert!(sent.contains(&Out::Send(started(&primary_key(MIN), MIN + 20_000))));
        // 备机先开录，主机 3 分钟后才开录
        let second = id(100 * MIN);
        core.unit_started(100 * MIN, &second, ROOM, 100 * MIN, unit());
        assert_eq!(core.session(&second).unwrap().key, second);
        outs(&mut core);
        core.primary_message(103 * MIN, started(&primary_key(103 * MIN), 103 * MIN));
        assert_eq!(core.session(&second).unwrap().key, primary_key(103 * MIN));
        // 告诉主机：之前用备机键记的那一场改用主机的键；重连后的上报也带上原来的键
        assert!(outs(&mut core).contains(&Out::Send(HaMessage::Adopted {
            key: primary_key(103 * MIN),
            room: ROOM,
            from: second.clone(),
        })));
        core.link_down(104 * MIN);
        let HaMessage::StandbyReport { sessions } = core.link_up(104 * MIN + 1000) else {
            panic!("expected a report");
        };
        let adopted = sessions
            .iter()
            .find(|session| session.key == primary_key(103 * MIN))
            .unwrap();
        assert_eq!(adopted.adopted_from.as_deref(), Some(second.as_str()));
        // 窗口外开播的是另一场
        let third = id(200 * MIN);
        core.unit_started(200 * MIN, &third, ROOM, 200 * MIN, unit());
        core.primary_message(215 * MIN, started(&primary_key(215 * MIN), 215 * MIN));
        assert_eq!(core.session(&third).unwrap().key, third);
    }

    #[test]
    fn the_report_lists_unsettled_and_recent_sessions() {
        let mut core = linked(HaMode::DualRecord);
        let old = record(&mut core, 0, 10 * MIN);
        core.primary_message(11 * MIN, uploaded(&old, 0, Some(10 * MIN)));
        assert_eq!(state(&core, &old), State::Done);
        let recording = id(REPORT_MS + 20 * MIN);
        core.unit_started(
            REPORT_MS + 20 * MIN,
            &recording,
            ROOM,
            REPORT_MS + 20 * MIN,
            unit(),
        );
        core.link_down(REPORT_MS + 21 * MIN);
        let HaMessage::StandbyReport { sessions } = core.link_up(REPORT_MS + 22 * MIN) else {
            panic!("expected a report");
        };
        let listed: Vec<(String, ReportedState)> = sessions
            .into_iter()
            .map(|session| (session.key, session.state))
            .collect();
        assert_eq!(listed, [(recording, ReportedState::Recording)]);
        let HaMessage::StandbyReport { sessions } = core.link_up(20 * MIN) else {
            panic!("expected a report");
        };
        assert_eq!(sessions.len(), 2, "48 小时内了结的也带上");
    }

    #[test]
    fn nothing_is_sent_before_the_pairing_reaches_this_connection() {
        let mut core = core(HaMode::DualRecord);
        record(&mut core, 0, MIN);
        assert!(
            outs(&mut core)
                .iter()
                .all(|out| !matches!(out, Out::Send(_)))
        );
    }

    // ---------- 模式 1（§3、§6 C） ----------

    #[test]
    fn mode_one_holds_and_settles_when_the_primary_uploads() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 10_000, 60 * MIN);
        assert_eq!(state(&core, &id), State::Holding);
        core.primary_message(60 * MIN, ended(&key, 0, 60 * MIN));
        core.primary_message(61 * MIN, upload_started(&key));
        core.tick(70 * MIN);
        assert_eq!(state(&core, &id), State::Holding, "主机在投：继续等");
        outs(&mut core);
        core.primary_message(72 * MIN, uploaded(&key, 0, Some(60 * MIN)));
        assert_eq!(state(&core, &id), State::Done);
        assert!(uploads(&outs(&mut core)).is_empty());
    }

    #[test]
    fn mode_one_uploads_when_the_primary_fails_or_skips() {
        let cases: [fn(&str) -> HaMessage; 3] = [
            failed,
            |key| skipped(key, SkipReason::Filtered),
            |key| skipped(key, SkipReason::NoFiles),
        ];
        for message in cases {
            let mut core = linked(HaMode::DualRecord);
            let key = primary_key(0);
            core.primary_message(0, started(&key, 0));
            let id = record(&mut core, 0, 30 * MIN);
            core.primary_message(30 * MIN, ended(&key, 0, 30 * MIN));
            outs(&mut core);
            core.primary_message(31 * MIN, message(&key));
            assert_eq!(state(&core, &id), State::Uploading);
            assert_eq!(uploads(&outs(&mut core)), [id]);
        }
    }

    #[test]
    fn mode_one_uploads_when_the_primary_does_not_start_within_the_timeout() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(31 * MIN, ended(&key, 0, 31 * MIN));
        let timeout = HaParams::ms(HaParams::default().upload_start_timeout);
        core.tick(31 * MIN + timeout - 1);
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(31 * MIN + timeout);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn mode_one_waits_while_the_primary_is_still_recording() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.tick(30 * MIN + 10 * 60 * MIN);
        assert_eq!(state(&core, &id), State::Holding, "主机还在录就不设期限");
    }

    #[test]
    fn mode_one_keeps_waiting_while_progress_moves_and_takes_over_when_it_stalls() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(30 * MIN, ended(&key, 0, 30 * MIN));
        core.primary_message(31 * MIN, upload_started(&key));
        // 进度一直在动：超过 upload_start_timeout 很久也继续等
        for minute in 32..=100 {
            core.primary_message(minute * MIN, progress(&key, minute as u64 * 1000));
            core.tick(minute * MIN);
        }
        assert_eq!(state(&core, &id), State::Holding);
        // 进度停住（字节不再增长，进度消息照发）
        let stall = HaParams::ms(HaParams::default().upload_stall_timeout);
        for minute in 101..=114 {
            core.primary_message(minute * MIN, progress(&key, 100_000));
            core.tick(minute * MIN);
        }
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(100 * MIN + stall - 1);
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(100 * MIN + stall);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_stall_while_the_primary_is_still_recording_is_not_a_stall() {
        // 主机边录边传：上一段传完、下一段还没录完时字节不动
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        core.primary_message(MIN, upload_started(&key));
        let id = record(&mut core, 0, 30 * MIN);
        core.tick(100 * MIN);
        assert_eq!(state(&core, &id), State::Holding);
        core.primary_message(100 * MIN, ended(&key, 0, 100 * MIN));
        core.tick(114 * MIN);
        assert_eq!(state(&core, &id), State::Holding, "从主机下播起算");
        core.tick(115 * MIN);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn mode_one_uploads_after_grace_and_delay_when_the_primary_goes_offline() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.link_down(40 * MIN);
        let grace = HaParams::ms(HaParams::default().offline_grace);
        let delay = HaParams::ms(HaParams::default().standby_upload_delay);
        core.tick(40 * MIN + grace + delay - 1);
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(40 * MIN + grace + delay);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn the_delay_counts_from_the_live_end_when_the_primary_was_already_offline() {
        let mut core = linked(HaMode::DualRecord);
        core.link_down(0);
        let id = record(&mut core, MIN, 60 * MIN);
        let delay = HaParams::ms(HaParams::default().standby_upload_delay);
        core.tick(60 * MIN + delay - 1);
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(60 * MIN + delay);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_flap_shorter_than_the_grace_changes_nothing() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(30 * MIN, ended(&key, 0, 30 * MIN));
        core.primary_message(31 * MIN, upload_started(&key));
        core.link_down(32 * MIN);
        core.tick(32 * MIN + 50_000);
        assert!(!core.primary_offline(32 * MIN + 50_000));
        core.link_up(32 * MIN + 55_000);
        core.primary_message(33 * MIN, progress(&key, 10));
        core.tick(40 * MIN);
        assert_eq!(state(&core, &id), State::Holding);
        core.primary_message(41 * MIN, uploaded(&key, 0, Some(30 * MIN)));
        assert_eq!(state(&core, &id), State::Done);
    }

    #[test]
    fn a_resent_upload_during_the_delay_cancels_the_standby_upload() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.link_down(31 * MIN);
        core.tick(36 * MIN);
        core.link_up(37 * MIN);
        core.primary_message(37 * MIN, uploaded(&key, 0, Some(30 * MIN)));
        assert_eq!(state(&core, &id), State::Done);
    }

    #[test]
    fn a_standby_upload_is_fenced_when_the_primary_upload_arrives_before_submit() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(31 * MIN, failed(&key));
        assert_eq!(state(&core, &id), State::Uploading);
        core.upload_started(32 * MIN, &id);
        // 主机重试成功（24 小时内的 Uploaded 重发同理）
        core.primary_message(33 * MIN, uploaded(&key, 0, Some(30 * MIN)));
        assert!(!core.begin_submit(34 * MIN, &id));
        core.upload_fenced(34 * MIN, &id);
        assert_eq!(state(&core, &id), State::Done);
    }

    #[test]
    fn a_standby_upload_reports_its_lifecycle() {
        let mut core = linked(HaMode::DualRecord);
        core.link_down(0);
        let id = record(&mut core, 0, 30 * MIN);
        core.tick(60 * MIN);
        assert_eq!(state(&core, &id), State::Uploading);
        core.link_up(61 * MIN);
        outs(&mut core);
        core.upload_started(61 * MIN, &id);
        assert!(core.begin_submit(62 * MIN, &id));
        assert!(core.session(&id).unwrap().submitting);
        core.uploaded(63 * MIN, &id, "BVS");
        let kinds: Vec<&str> = outs(&mut core)
            .iter()
            .filter_map(|out| match out {
                Out::Send(message) => Some(message.kind()),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, ["upload_started", "uploaded"]);
        let session = core.session(&id).unwrap();
        assert_eq!(
            (session.state, session.bvid.as_deref()),
            (State::Uploaded, Some("BVS"))
        );
        assert!(!session.submitting);
    }

    #[test]
    fn mixed_primary_results_go_to_manual_instead_of_a_duplicate() {
        // 主机这一场断成两段：第一段投成，第二段失败；备机那份是整场
        let mut core = linked(HaMode::DualRecord);
        let first = primary_key(0);
        let second = primary_key(20 * MIN);
        core.primary_message(0, started(&first, 0));
        core.primary_message(15 * MIN, ended(&first, 0, 15 * MIN));
        core.primary_message(20 * MIN, started(&second, 20 * MIN));
        let id = record(&mut core, 0, 60 * MIN);
        core.primary_message(60 * MIN, ended(&second, 20 * MIN, 60 * MIN));
        core.primary_message(61 * MIN, uploaded(&first, 0, Some(15 * MIN)));
        assert_eq!(state(&core, &id), State::Holding);
        outs(&mut core);
        core.primary_message(62 * MIN, failed(&second));
        assert_eq!(state(&core, &id), State::Manual);
        assert!(
            outs(&mut core)
                .iter()
                .any(|out| matches!(out, Out::Attention { .. }))
        );
        // 两段都投成就了结
        let mut core = linked(HaMode::DualRecord);
        core.primary_message(0, started(&first, 0));
        core.primary_message(20 * MIN, started(&second, 20 * MIN));
        let id = record(&mut core, 0, 60 * MIN);
        core.primary_message(61 * MIN, uploaded(&first, 0, Some(15 * MIN)));
        core.primary_message(62 * MIN, uploaded(&second, 20 * MIN, Some(60 * MIN)));
        assert_eq!(state(&core, &id), State::Done);
    }

    #[test]
    fn a_session_the_primary_never_recorded_is_uploaded_after_the_start_timeout() {
        let mut core = linked(HaMode::DualRecord);
        let id = record(&mut core, 0, 30 * MIN);
        let timeout = HaParams::ms(HaParams::default().upload_start_timeout);
        core.tick(30 * MIN + timeout - 1);
        assert_eq!(state(&core, &id), State::Holding);
        core.tick(30 * MIN + timeout);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_handover_for_the_standby_key_uploads_at_once() {
        let mut core = linked(HaMode::DualRecord);
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(31 * MIN, skipped(&id, SkipReason::Handover));
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_standby_copy_is_deleted_after_the_primary_upload_when_asked() {
        let mut assignment = assignment(HaMode::DualRecord);
        assignment.params.delete_standby_copy = true;
        let mut core = StandbyCore::new(&assignment, WINDOW, Vec::new(), Vec::new(), 0);
        core.link_up(0);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, 30 * MIN);
        core.primary_message(31 * MIN, uploaded(&key, 0, Some(30 * MIN)));
        assert!(outs(&mut core).contains(&Out::Discard(id)));
    }

    #[test]
    fn an_empty_standby_unit_is_settled_without_upload() {
        let mut core = linked(HaMode::DualRecord);
        let id = id(0);
        core.unit_started(0, &id, ROOM, 0, unit());
        core.unit_ended(MIN, &id, false);
        assert_eq!(state(&core, &id), State::Empty);
        let sent = outs(&mut core);
        assert!(sent.iter().any(|out| matches!(
            out,
            Out::Send(HaMessage::SessionEnded {
                produced: false,
                ..
            })
        )));
        assert!(uploads(&sent).is_empty());
    }

    #[test]
    fn nothing_is_decided_until_the_segments_are_collected() {
        let mut core = linked(HaMode::DualRecord);
        core.link_down(0);
        let id = id(0);
        core.unit_started(0, &id, ROOM, 0, unit());
        core.unit_ended(MIN, &id, true);
        core.tick(60 * MIN);
        assert_eq!(state(&core, &id), State::Holding);
        core.collected(60 * MIN, &id);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_failed_standby_upload_can_be_retried_or_dropped_by_hand() {
        let mut core = linked(HaMode::DualRecord);
        core.link_down(0);
        let id = record(&mut core, 0, MIN);
        core.tick(60 * MIN);
        core.upload_failed(61 * MIN, &id, "网络错误");
        assert_eq!(state(&core, &id), State::Failed);
        assert!(
            core.manual(62 * MIN, &id, ManualAction::StandbyUpload)
                .is_ok()
        );
        assert_eq!(state(&core, &id), State::Uploading);
        core.upload_failed(63 * MIN, &id, "网络错误");
        assert!(core.manual(64 * MIN, &id, ManualAction::Drop).is_ok());
        assert_eq!(state(&core, &id), State::Dropped);
        assert!(core.manual(65 * MIN, &id, ManualAction::Drop).is_err());
    }

    #[test]
    fn mode_one_never_holds_recording() {
        let mut core = linked(HaMode::DualRecord);
        assert_eq!(core.hold(0, ROOM), None);
    }

    // ---------- 模式 2（§4、§6 E / F / H） ----------

    #[test]
    fn mode_two_only_monitors_while_the_primary_is_online() {
        let mut core = linked(HaMode::Takeover);
        assert!(core.hold(MIN, ROOM).is_some());
        core.link_down(2 * MIN);
        let grace = HaParams::ms(HaParams::default().offline_grace);
        assert!(core.hold(2 * MIN + grace - 1, ROOM).is_some(), "抖动不接手");
        assert_eq!(core.hold(2 * MIN + grace, ROOM), None);
        core.link_up(10 * MIN);
        assert!(
            core.hold(10 * MIN, ROOM).is_some(),
            "主机回来：新开播的主机录"
        );
    }

    fn takeover(core: &mut StandbyCore, primary_start: i64, crash: i64, end: i64) -> String {
        let key = primary_key(primary_start);
        core.primary_message(primary_start, started(&key, primary_start));
        core.hold(crash - 1000, ROOM);
        core.link_down(crash);
        let start = crash + HaParams::ms(HaParams::default().offline_grace);
        assert_eq!(core.hold(start, ROOM), None);
        let id = record(core, start, end);
        let session = core.session(&id).unwrap();
        assert_eq!(session.kind, Kind::Takeover);
        assert_eq!(session.takeover_of.as_deref(), Some(key.as_str()));
        id
    }

    #[test]
    fn mode_two_takes_over_the_session_the_primary_was_recording() {
        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        assert_eq!(state(&core, &id), State::AwaitingPrimary);
        assert!(uploads(&outs(&mut core)).is_empty(), "接手的一场先不投");
    }

    #[test]
    fn mode_two_records_a_new_live_normally_and_uploads_it() {
        let mut core = linked(HaMode::Takeover);
        core.link_down(0);
        let id = record(&mut core, 5 * MIN, 60 * MIN);
        assert_eq!(core.session(&id).unwrap().kind, Kind::Normal);
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn a_stale_recording_view_is_not_taken_over_the_next_day() {
        let mut core = linked(HaMode::Takeover);
        core.primary_message(0, started(&primary_key(0), 0));
        core.link_down(MIN);
        let id = record(&mut core, 24 * 60 * MIN, 25 * 60 * MIN);
        assert_eq!(core.session(&id).unwrap().kind, Kind::Normal);
    }

    #[test]
    fn the_primary_half_then_the_standby_half_is_appended() {
        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        // 主机回来，先投它那半（主机那段中断，没有结束时刻）
        core.link_up(200 * MIN);
        outs(&mut core);
        core.primary_message(201 * MIN, uploaded(&primary_key(0), 0, None));
        assert_eq!(state(&core, &id), State::Appending);
        assert!(outs(&mut core).contains(&Out::Append {
            id: id.clone(),
            bvid: "BVP".into()
        }));
        // 提交前确认一次（追加不受「主机已投成」的栅栏影响），追加成功后告诉主机同一个稿件号
        assert!(core.begin_submit(202 * MIN, &id));
        core.appended(203 * MIN, &id, "BVP");
        assert_eq!(state(&core, &id), State::Appended);
        assert!(state(&core, &id).settled());
        let started_at = core.session(&id).unwrap().started_at;
        let sent = outs(&mut core);
        assert!(
            sent.iter().any(|out| matches!(out,
                Out::Send(HaMessage::Uploaded { key, bvid, from, to, .. })
                    if *key == id && bvid == "BVP" && *from == started_at && *to == Some(120 * MIN))),
            "{sent:?}"
        );
        assert!(!core.session(&id).unwrap().submitting);
        assert_eq!(
            core.report(204 * MIN)[0].state,
            ReportedState::Uploaded,
            "重连时报成投成，主机不会再让谁投"
        );
    }

    #[test]
    fn a_filtered_or_lost_primary_half_makes_the_standby_upload_the_whole_session() {
        for reason in [
            SkipReason::Filtered,
            SkipReason::NoFiles,
            SkipReason::Handover,
        ] {
            let mut core = linked(HaMode::Takeover);
            let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
            core.link_up(200 * MIN);
            core.primary_message(200 * MIN, skipped(&primary_key(0), reason));
            assert_eq!(state(&core, &id), State::Uploading, "{reason:?}");
        }
    }

    #[test]
    fn a_failed_primary_half_or_a_long_absence_needs_a_human() {
        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        core.link_up(200 * MIN);
        core.primary_message(200 * MIN, failed(&primary_key(0)));
        assert_eq!(state(&core, &id), State::Manual);

        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        let timeout = HaParams::ms(HaParams::default().manual_timeout);
        core.tick(120 * MIN + timeout - 1);
        assert_eq!(state(&core, &id), State::AwaitingPrimary);
        core.tick(120 * MIN + timeout);
        assert_eq!(state(&core, &id), State::Manual);
        assert!(
            core.manual(121 * MIN + timeout, &id, ManualAction::StandbyUpload)
                .is_ok()
        );
        assert_eq!(state(&core, &id), State::Uploading);
    }

    #[test]
    fn both_manual_actions_work_while_awaiting_the_primary() {
        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        // 按主机场次键也认得（主机面板转来的）
        core.primary_message(
            130 * MIN,
            HaMessage::Manual {
                key: id.clone(),
                action: ManualAction::Drop,
            },
        );
        assert_eq!(state(&core, &id), State::Dropped);

        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        assert!(
            core.manual(130 * MIN, &id, ManualAction::StandbyUpload)
                .is_ok()
        );
        assert_eq!(uploads(&outs(&mut core)), [id]);
    }

    #[test]
    fn states_the_primary_cannot_infer_are_sent_to_it() {
        fn states(outs: &[Out]) -> Vec<(String, ReportedState)> {
            outs.iter()
                .filter_map(|out| match out {
                    Out::Send(HaMessage::SessionState { key, state, .. }) => {
                        Some((key.clone(), *state))
                    }
                    _ => None,
                })
                .collect()
        }
        let mut core = linked(HaMode::Takeover);
        let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        assert!(states(&outs(&mut core)).is_empty(), "断着时不发");
        core.link_up(200 * MIN);
        core.primary_message(200 * MIN, failed(&primary_key(0)));
        assert_eq!(
            states(&outs(&mut core)),
            [(id.clone(), ReportedState::Manual)]
        );
        core.manual(201 * MIN, &id, ManualAction::Drop).unwrap();
        assert_eq!(states(&outs(&mut core)), [(id, ReportedState::Dropped)]);

        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        record(&mut core, 0, 30 * MIN);
        outs(&mut core);
        core.primary_message(40 * MIN, uploaded(&key, 0, Some(30 * MIN)));
        assert_eq!(states(&outs(&mut core)), [(key, ReportedState::Done)]);
    }

    #[test]
    fn the_primary_returning_mid_recording_does_not_cut_the_takeover() {
        let mut core = linked(HaMode::Takeover);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        core.hold(59 * MIN, ROOM);
        core.link_down(60 * MIN);
        let start = 61 * MIN;
        let id = id(start);
        core.unit_started(start, &id, ROOM, start, unit());
        core.link_up(70 * MIN);
        core.tick(80 * MIN);
        assert_eq!(state(&core, &id), State::Recording, "录制不受影响");
        core.collected(120 * MIN, &id);
        core.unit_ended(120 * MIN, &id, true);
        assert_eq!(state(&core, &id), State::AwaitingPrimary);
    }

    #[test]
    fn a_partition_where_the_primary_kept_recording_ends_without_a_second_upload() {
        // 连接断了但主机一直在录：主机的稿件录到了下播，备机不追加也不投
        let mut core = linked(HaMode::Takeover);
        let taken = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
        core.link_up(121 * MIN);
        let key = primary_key(0);
        core.primary_message(121 * MIN, ended(&key, 0, 120 * MIN));
        core.primary_message(122 * MIN, upload_started(&key));
        assert_eq!(state(&core, &taken), State::AwaitingPrimary);
        core.primary_message(130 * MIN, uploaded(&key, 0, Some(120 * MIN)));
        assert_eq!(state(&core, &taken), State::Done);
        // 主机离线期间开播的一场也一样：主机其实也在录，就按模式 1 等它
        let mut core = linked(HaMode::Takeover);
        core.link_down(0);
        let start = 5 * MIN;
        let id = id(start);
        core.unit_started(start, &id, ROOM, start, unit());
        core.link_up(20 * MIN);
        let key = primary_key(4 * MIN);
        core.primary_message(20 * MIN, started(&key, 4 * MIN));
        core.collected(60 * MIN, &id);
        core.unit_ended(60 * MIN, &id, true);
        assert_eq!(core.session(&id).unwrap().kind, Kind::Backup);
        assert_eq!(state(&core, &id), State::Holding);
        core.primary_message(61 * MIN, uploaded(&key, 4 * MIN, Some(60 * MIN)));
        assert_eq!(state(&core, &id), State::Done);
    }

    #[test]
    fn a_primary_half_that_yielded_near_the_live_end_is_still_appended() {
        // 主机回来时因为备机已接手而不续录：它那份结束得离下播再近，也不算录到了下播
        for (yielded, expected) in [(false, State::Done), (true, State::Appending)] {
            let mut core = linked(HaMode::Takeover);
            let id = takeover(&mut core, 0, 60 * MIN, 120 * MIN);
            core.link_up(121 * MIN);
            outs(&mut core);
            let mut message = uploaded(&primary_key(0), 0, Some(119 * MIN));
            if let HaMessage::Uploaded { yielded: flag, .. } = &mut message {
                *flag = yielded;
            }
            core.primary_message(122 * MIN, message);
            assert_eq!(state(&core, &id), expected, "yielded = {yielded}");
        }
    }

    // ---------- 重启 ----------

    #[test]
    fn a_restart_finishes_cut_recordings_and_resumes_or_parks_uploads() {
        let recording = Session {
            id: id(0),
            key: id(0),
            room: ROOM,
            state: State::Recording,
            started_at: 0,
            since: 0,
            updated_at: 20 * MIN,
            unit: unit(),
            ..Session::default()
        };
        let uploading = Session {
            id: id(MIN),
            key: id(MIN),
            state: State::Uploading,
            ended_at: Some(10 * MIN),
            ..recording.clone()
        };
        let submitting = Session {
            id: id(2 * MIN),
            key: id(2 * MIN),
            state: State::Uploading,
            ended_at: Some(10 * MIN),
            submitting: true,
            ..recording.clone()
        };
        let mut core = StandbyCore::new(
            &assignment(HaMode::DualRecord),
            WINDOW,
            vec![recording, uploading, submitting],
            Vec::new(),
            30 * MIN,
        );
        let sent = outs(&mut core);
        assert_eq!(state(&core, &id(0)), State::Holding);
        assert_eq!(core.session(&id(0)).unwrap().ended_at, Some(20 * MIN));
        assert_eq!(uploads(&sent), [id(MIN)]);
        assert_eq!(state(&core, &id(2 * MIN)), State::Manual);
        assert!(!core.linked(), "重启后先当主机断开");
        assert!(!core.primary_offline(30 * MIN));
    }

    #[test]
    fn old_settled_sessions_and_views_are_pruned() {
        let mut core = linked(HaMode::DualRecord);
        let key = primary_key(0);
        core.primary_message(0, started(&key, 0));
        let id = record(&mut core, 0, MIN);
        core.primary_message(2 * MIN, uploaded(&key, 0, Some(MIN)));
        core.take_dirty();
        core.tick(PRUNE_MS + 3 * MIN);
        assert!(core.session(&id).is_none());
        assert_eq!(core.primary_views().count(), 0);
        assert!(core.take_dirty());
    }
}
