//! 主机这一侧（ha-pair 方案 §2、§3、§4 里主机的部分）。
//!
//! 主机就是控制面进程，「本机」节点录的每一段都经钩子直接调到这里：开录、录完、开始投、进度、投成 / 失败，
//! 记进 `ha_sessions` 并发给备机。备机（重）连上后先等备机的 `StandbyReport`（最多 [`REPORT_WAIT_MS`]），
//! 主机进程刚启动时最多等 [`STARTUP_WAIT_MS`]：这期间不开始任何投稿、不开录配对里的房间。
//! 收到上报后把备机在录 / 在投的场次交给备机、按上报逐场回复主机这边的结果，再重发近 24 小时投成的场次。
//! 模式 2 里备机接手的正是主机上次中断的那一场时，主机先把盘上它那半投了，备机再追加它那半（§4）。
//!
//! [`PrimaryCore`] 不碰时钟、不做 I/O，测试用虚拟时间驱动；[`Primary`] 把它接到钩子、数据库与备机连接上。

use super::key::{self, Span};
use super::params::{HaMode, HaParams};
use super::store::{self, PrimaryState, SessionRecord, Uploader};
use super::upload::{self, Plan};
use super::wire::{HaMessage, ReportedSession, ReportedState, SkipReason};
use super::{Hold, Unit, UnitOutput};
use crate::server::errors::AppResult;
use crate::server::fleet::protocol::ControllerMessage;
use crate::server::fleet::{node, now_ms};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::service_register::ServiceRegister;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// 备机（重）连上后最多等 `StandbyReport` 这么久（§2 硬规则）
pub const REPORT_WAIT_MS: i64 = 10_000;
/// 主机进程启动后最多等 `StandbyReport` 这么久。备机这时可能还挂在旧连接上，要到 QUIC 空闲超时才发现断了；
/// 也可能卡在主机停机期间发起的一次拨号里，要等拨号超时再退避一次才重拨。早于它的上报就开录，会与备机
/// 正在录的场次撞上：主机那份从重启后才开始，备机那份反倒被当成多余
pub const STARTUP_WAIT_MS: i64 = {
    let idle = crate::server::fleet::IDLE_TIMEOUT.as_millis() as i64;
    let redial = (node::CONNECT_TIMEOUT.as_millis() + node::STANDBY_BACKOFF_MAX.as_millis()) as i64;
    let reconnect = if idle > redial { idle } else { redial };
    reconnect + REPORT_WAIT_MS
};
/// 重发 `Uploaded` 的回看范围（§3）
pub const RESEND_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
/// 内存里留多久的场次（更早的只在库里，界面按库查）
const KEEP_MS: i64 = 48 * 60 * 60 * 1000;
/// `ha_sessions` 里留多久的场次
const PRUNE_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const TICK: Duration = Duration::from_secs(1);

/// [`PrimaryCore`] 要做的事，由 [`Primary`] 执行
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Out {
    /// 发给备机（只在连着时产生）
    Send(HaMessage),
    /// 写进 `ha_sessions`
    Save(SessionRecord),
    /// 从 `ha_sessions` 删掉这一行
    Delete(String),
    /// 停掉这一段正在进行的拉流
    Stop(String),
    /// 补投主机上次中断的那一段
    Resume(Resume),
}

/// 模式 2：主机进程中断、备机接手了的一段，主机回来后补投盘上它那半
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resume {
    pub key: String,
    pub room: i64,
    /// 切片工作台的场次 id（`stream_sessions.id`）
    pub session: Option<i64>,
    /// 这一段检测到开播的时刻：场次里更早的分段属于之前的录制
    pub unit_started_at: i64,
}

/// 主机的投稿流程要不要真的投
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Begin {
    Proceed,
    /// 这一场由备机负责，或主机这份已经有了结果（失败后重开的上传管道）：不传、不跑后处理
    Skip,
}

fn standby_owns(row: &SessionRecord) -> bool {
    row.primary_state == PrimaryState::HandedOver
        || row.uploader == Some(Uploader::Standby)
        || matches!(row.standby_state.as_deref(), Some("uploading" | "uploaded"))
}

/// 主机的决策部分。时刻一律由调用方传入（主机时钟，Unix 毫秒）
pub(crate) struct PrimaryCore {
    mode: HaMode,
    params: HaParams,
    /// 场次对齐的窗口（`live_merge_minutes`）
    window: i64,
    rows: BTreeMap<String, SessionRecord>,
    /// 本进程正在录的段：场次键 → 房间
    active: HashMap<String, i64>,
    /// 正在录的段最近一次拉流中断后重连的时刻
    retried: HashMap<String, i64>,
    /// 备机连着的起始时刻
    connected: Option<i64>,
    /// 备机最近一次断开（或主机进程启动）的时刻
    disconnected_at: i64,
    /// 等备机上报的截止时刻；`None` 表示闸门开着
    gate_until: Option<i64>,
    /// 备机正在录、主机暂不开录的房间 → 备机那一场的键
    held: BTreeMap<i64, String>,
    /// 模式 2 里因为备机接手而没有续录的主机段
    yielded: BTreeSet<String>,
    dirty: BTreeSet<String>,
    outs: Vec<Out>,
}

impl PrimaryCore {
    /// 主机进程启动：上次在录、录完待投、在投的场次都没做完，标成中断
    pub(crate) fn new(
        mode: HaMode,
        params: HaParams,
        window: i64,
        rows: Vec<SessionRecord>,
        now: i64,
    ) -> Self {
        let mut core = PrimaryCore {
            mode,
            params,
            window,
            rows: BTreeMap::new(),
            active: HashMap::new(),
            retried: HashMap::new(),
            connected: None,
            disconnected_at: now,
            gate_until: Some(now + STARTUP_WAIT_MS),
            held: BTreeMap::new(),
            yielded: BTreeSet::new(),
            dirty: BTreeSet::new(),
            outs: Vec::new(),
        };
        for mut row in rows {
            if matches!(
                row.primary_state,
                PrimaryState::Recording | PrimaryState::Recorded | PrimaryState::Uploading
            ) {
                row.primary_state = PrimaryState::Interrupted;
                row.updated_at = now;
                core.dirty.insert(row.session_key.clone());
            }
            core.rows.insert(row.session_key.clone(), row);
        }
        core
    }

    /// 取走攒下的动作；改过的行各写一次
    pub(crate) fn take(&mut self) -> Vec<Out> {
        let dirty = std::mem::take(&mut self.dirty);
        for key in dirty {
            if let Some(row) = self.rows.get(&key) {
                self.outs.push(Out::Save(row.clone()));
            }
        }
        std::mem::take(&mut self.outs)
    }

    pub(crate) fn params(&self) -> HaParams {
        self.params
    }

    /// 控制面改了模式或参数；已经在进行的场次保持原来的情形
    pub(crate) fn configure(&mut self, mode: HaMode, params: HaParams) {
        self.mode = mode;
        self.params = params;
    }

    pub(crate) fn gate_open(&self) -> bool {
        self.gate_until.is_none()
    }

    pub(crate) fn tracks(&self, key: &str) -> bool {
        self.rows.contains_key(key)
    }

    fn send(&mut self, message: HaMessage) {
        if self.connected.is_some() {
            self.outs.push(Out::Send(message));
        }
    }

    fn touch(&mut self, key: &str, now: i64) -> Option<&mut SessionRecord> {
        let row = self.rows.get_mut(key)?;
        row.updated_at = now;
        self.dirty.insert(key.to_string());
        Some(row)
    }

    pub(crate) fn hold(&self, room: i64) -> Option<Hold> {
        if self.gate_until.is_some() {
            return Some(Hold {
                reason: "等备机上报它手里的场次".into(),
                quick: true,
            });
        }
        let key = self.held.get(&room)?;
        Some(Hold {
            reason: format!("备机正在录这一场（{key}），等它下播再开录"),
            quick: false,
        })
    }

    /// 主机这一段拉流中断、准备重连：模式 2 里备机已经接手这个房间就不续录，等备机下播（§4）。
    /// 返回真时调用方结束这一段
    pub(crate) fn retrying(&mut self, now: i64, key: &str) -> bool {
        if self.yielded.contains(key) {
            return true;
        }
        if self.mode == HaMode::Takeover
            && let Some(room) = self.rows.get(key).map(|row| row.room_id)
            && let Some(standby_key) = self.held.get(&room).cloned()
        {
            self.yield_to(now, key, &standby_key);
            return true;
        }
        if self.active.contains_key(key) {
            self.retried.insert(key.to_string(), now);
        }
        false
    }

    fn yield_to(&mut self, now: i64, key: &str, standby_key: &str) {
        self.yielded.insert(key.to_string());
        if let Some(row) = self.touch(key, now) {
            row.reason = Some(format!("备机已接手（{standby_key}），主机不续录"));
        }
        info!(key, standby_key, "HA：备机已接手这个房间，主机这一段不续录");
    }

    pub(crate) fn connected(&mut self, now: i64) {
        self.connected = Some(now);
        self.gate_until = Some(now + REPORT_WAIT_MS);
    }

    pub(crate) fn disconnected(&mut self, now: i64) {
        self.connected = None;
        self.disconnected_at = now;
    }

    pub(crate) fn tick(&mut self, now: i64) {
        if self.gate_until.is_some_and(|until| now >= until) {
            self.gate_until = None;
            info!("HA：等满了时限也没收到备机的场次上报，主机照常开录、开投");
        }
        let grace = HaParams::ms(self.params.offline_grace);
        if self.connected.is_none() && now - self.disconnected_at >= grace && !self.held.is_empty()
        {
            let rooms: Vec<i64> = self.held.keys().copied().collect();
            warn!(
                ?rooms,
                "HA：备机离线超过 offline_grace，不再等它录完，主机恢复开录这些房间"
            );
            self.held.clear();
        }
        let active = &self.active;
        self.rows
            .retain(|key, row| row.updated_at >= now - KEEP_MS || active.contains_key(key));
        self.retried.retain(|key, _| active.contains_key(key));
        let rows = &self.rows;
        self.yielded.retain(|key| rows.contains_key(key));
    }

    pub(crate) fn unit_started(
        &mut self,
        now: i64,
        key: &str,
        room: i64,
        started_at: i64,
        session: i64,
    ) {
        self.active.insert(key.to_string(), room);
        let row = SessionRecord {
            session_key: key.to_string(),
            room_id: room,
            started_at,
            primary_state: PrimaryState::Recording,
            local_session_id: Some(session),
            unit_started_at: Some(started_at),
            updated_at: now,
            ..SessionRecord::default()
        };
        self.rows.insert(key.to_string(), row);
        self.dirty.insert(key.to_string());
        self.send(HaMessage::SessionStarted {
            key: key.to_string(),
            room,
            started_at,
            at: now,
        });
    }

    pub(crate) fn unit_ended(&mut self, now: i64, key: &str, output: UnitOutput) {
        self.active.remove(key);
        self.retried.remove(key);
        let Some(row) = self.touch(key, now) else {
            return;
        };
        row.ended_at = Some(now);
        let produced = output.sent > 0;
        let skipped = match row.primary_state {
            PrimaryState::Recording if !produced => {
                let reason = if output.seen > 0 {
                    SkipReason::Filtered
                } else {
                    SkipReason::NoFiles
                };
                row.primary_state = PrimaryState::Skipped;
                row.reason = Some(reason.as_str().into());
                Some(reason)
            }
            PrimaryState::Recording => {
                row.primary_state = PrimaryState::Recorded;
                None
            }
            _ => None,
        };
        let (room, started_at) = (row.room_id, row.started_at);
        self.send(HaMessage::SessionEnded {
            key: key.to_string(),
            room,
            started_at,
            at: now,
            produced,
        });
        if let Some(reason) = skipped {
            info!(
                key,
                reason = reason.as_str(),
                "HA：主机这一段没有可投的文件"
            );
            self.send(HaMessage::UploadSkipped {
                key: key.to_string(),
                room,
                reason,
                detail: None,
            });
        }
    }

    pub(crate) fn upload_begin(&mut self, now: i64, key: &str) -> Begin {
        let Some(row) = self.rows.get(key) else {
            return Begin::Proceed;
        };
        if standby_owns(row)
            || !matches!(
                row.primary_state,
                PrimaryState::Recording | PrimaryState::Recorded
            )
        {
            return Begin::Skip;
        }
        let room = row.room_id;
        if let Some(row) = self.touch(key, now) {
            row.primary_state = PrimaryState::Uploading;
            row.progress_at = Some(now);
        }
        self.send(HaMessage::UploadStarted {
            key: key.to_string(),
            room,
            at: now,
        });
        Begin::Proceed
    }

    pub(crate) fn progress(&mut self, now: i64, key: &str, bytes: u64) {
        let Some(row) = self.touch(key, now) else {
            return;
        };
        if i64::try_from(bytes).unwrap_or(i64::MAX) > row.upload_bytes {
            row.upload_bytes = i64::try_from(bytes).unwrap_or(i64::MAX);
            row.progress_at = Some(now);
        }
        let room = row.room_id;
        self.send(HaMessage::UploadProgress {
            key: key.to_string(),
            room,
            bytes,
            at: now,
        });
    }

    /// 提交前最后确认一次：备机接手了（在投或投成）就不提交
    pub(crate) fn may_submit(&self, key: &str) -> bool {
        self.rows.get(key).is_none_or(|row| !standby_owns(row))
    }

    pub(crate) fn uploaded(&mut self, now: i64, key: &str, bvid: &str) {
        let Some(row) = self.touch(key, now) else {
            return;
        };
        row.primary_state = PrimaryState::Uploaded;
        row.bvid = Some(bvid.to_string());
        row.uploader = Some(Uploader::Primary);
        row.reason = None;
        let message = HaMessage::Uploaded {
            key: key.to_string(),
            room: row.room_id,
            bvid: bvid.to_string(),
            from: row.started_at,
            to: row.ended_at,
            yielded: self.yielded.contains(key),
        };
        self.send(message);
    }

    pub(crate) fn upload_failed(&mut self, now: i64, key: &str, reason: &str) {
        let Some(row) = self.touch(key, now) else {
            return;
        };
        row.primary_state = PrimaryState::Failed;
        row.reason = Some(reason.to_string());
        let room = row.room_id;
        self.send(HaMessage::UploadFailed {
            key: key.to_string(),
            room,
            reason: reason.to_string(),
        });
    }

    /// 分段传完了，但提交前发现备机已经接手：不提交，交给备机
    pub(crate) fn upload_fenced(&mut self, now: i64, key: &str) {
        if let Some(row) = self.touch(key, now) {
            row.primary_state = PrimaryState::HandedOver;
            row.reason = Some("备机已经在投这一场".into());
        }
    }

    /// 模式 2：备机接手的正是主机上次中断的这一段（`takeover_of`），主机先投它那半，投成后备机追加（§4）。
    /// 每段只补投一次：`uploader` 已是主机而没有稿件号，说明上次补投开始了却没结果；开始传了就不知道
    /// 提交成没成功，转人工，免得两份稿件。返回假时调用方照常回复这一段的现状
    fn resume(&mut self, now: i64, key: &str, takeover_of: Option<&str>) -> bool {
        if self.mode != HaMode::Takeover || takeover_of != Some(key) {
            return false;
        }
        let Some(row) = self.rows.get(key) else {
            return false;
        };
        if row.primary_state != PrimaryState::Interrupted || standby_owns(row) {
            return false;
        }
        let job = Resume {
            key: key.to_string(),
            room: row.room_id,
            session: row.local_session_id,
            unit_started_at: row.unit_started_at.unwrap_or(row.started_at),
        };
        let again = row.uploader == Some(Uploader::Primary);
        let began = again && row.progress_at.is_some();
        let Some(row) = self.touch(key, now) else {
            return false;
        };
        if began {
            row.primary_state = PrimaryState::Failed;
            row.reason = Some("主机补投它那半时进程又中断了，不知道提交成没成功".into());
            warn!(key, "HA：上次补投中途中断，转人工");
            return false;
        }
        row.primary_state = PrimaryState::Recorded;
        row.uploader = Some(Uploader::Primary);
        row.progress_at = None;
        row.upload_bytes = 0;
        row.reason = Some("主机回来补投它那半".into());
        self.yielded.insert(key.to_string());
        self.outs.push(Out::Resume(job));
        info!(key, "HA：备机接手了主机中断的这一段，主机先投它那半");
        true
    }

    /// 补投找到了主机那半：它录到最后一个分段写盘的时刻
    pub(crate) fn resume_ended(&mut self, now: i64, key: &str, ended_at: i64) {
        let Some(row) = self.touch(key, now) else {
            return;
        };
        if row.ended_at.is_some() {
            return;
        }
        row.ended_at = Some(ended_at);
        let (room, started_at) = (row.room_id, row.started_at);
        self.send(HaMessage::SessionEnded {
            key: key.to_string(),
            room,
            started_at,
            at: ended_at,
            produced: true,
        });
    }

    /// 补投时主机那半没有可投的文件（太小被过滤，或已不在盘上）：备机投完整的一份（§6 F）
    pub(crate) fn resume_skipped(&mut self, now: i64, key: &str, reason: SkipReason, detail: &str) {
        let Some(row) = self.touch(key, now) else {
            return;
        };
        row.primary_state = PrimaryState::Skipped;
        row.uploader = None;
        row.reason = Some(reason.as_str().into());
        let room = row.room_id;
        self.send(HaMessage::UploadSkipped {
            key: key.to_string(),
            room,
            reason,
            detail: Some(detail.into()),
        });
    }

    pub(crate) fn standby_message(&mut self, now: i64, message: HaMessage) {
        match message {
            HaMessage::StandbyReport { sessions } => self.report(now, sessions),
            HaMessage::SessionStarted {
                key,
                room,
                started_at,
                ..
            } => self.note(now, &key, room, started_at, None, "recording", None),
            HaMessage::SessionEnded {
                key,
                room,
                started_at,
                at,
                ..
            } => {
                // 备机一个房间同时只录一段；它中途可能改用了主机的键，按房间放开
                if self.held.remove(&room).is_some() {
                    info!(room, key, "HA：备机录完了这一场，主机恢复开录这个房间");
                }
                self.note(now, &key, room, started_at, Some(at), "recorded", None);
            }
            HaMessage::UploadStarted { key, room, .. } => {
                self.standby_took(now, &key, room, "uploading", None)
            }
            HaMessage::Uploaded {
                key, room, bvid, ..
            } => {
                info!(key, bvid, "HA：备机投成了这一场");
                self.standby_took(now, &key, room, "uploaded", Some(bvid));
            }
            HaMessage::UploadFailed { key, room, reason } => {
                warn!(key, reason, "HA：备机投这一场失败");
                let started_at = key::parse(&key).map_or(now, |(_, started_at)| started_at);
                self.note(now, &key, room, started_at, None, "failed", None);
                if let Some(row) = self.touch(&key, now) {
                    row.reason = Some(reason);
                }
            }
            HaMessage::SessionState {
                key,
                room,
                state,
                reason,
            } => {
                let started_at = key::parse(&key).map_or(now, |(_, started_at)| started_at);
                self.note(now, &key, room, started_at, None, state.as_str(), None);
                if reason.is_some()
                    && let Some(row) = self.touch(&key, now)
                {
                    row.reason = reason;
                }
            }
            HaMessage::Adopted { key, from, .. } => self.adopted(now, &key, &from),
            HaMessage::UploadProgress { .. }
            | HaMessage::UploadSkipped { .. }
            | HaMessage::Manual { .. } => {}
        }
    }

    /// 备机那一场改用了主机的键：它之前用备机键记下的那一行并进主机这一行
    fn adopted(&mut self, now: i64, key: &str, from: &str) {
        let Some(alias) = self.rows.remove(from) else {
            return;
        };
        self.dirty.remove(from);
        self.outs.push(Out::Delete(from.to_string()));
        for held in self.held.values_mut().filter(|held| held.as_str() == from) {
            *held = key.to_string();
        }
        if let Some(row) = self.touch(key, now)
            && row.standby_state.is_none()
        {
            row.standby_state = alias.standby_state;
        }
        debug!(key, from, "HA：备机那一场改用了主机的键");
    }

    /// 记下备机那边这一场的状态；备机自己起的场次（主机没有）新建一行
    #[allow(clippy::too_many_arguments)]
    fn note(
        &mut self,
        now: i64,
        key: &str,
        room: i64,
        started_at: i64,
        ended_at: Option<i64>,
        state: &str,
        bvid: Option<String>,
    ) {
        let row = self
            .rows
            .entry(key.to_string())
            .or_insert_with(|| SessionRecord {
                session_key: key.to_string(),
                room_id: room,
                started_at,
                ..SessionRecord::default()
            });
        if row.primary_state == PrimaryState::None && ended_at.is_some() {
            row.ended_at = ended_at;
        }
        row.standby_state = Some(state.to_string());
        if bvid.is_some() && row.bvid.is_none() {
            row.bvid = bvid;
            row.uploader = Some(Uploader::Standby);
        }
        row.updated_at = now;
        self.dirty.insert(key.to_string());
    }

    /// 备机在投 / 投成了：它那一场与重叠的主机场次都归备机，主机不再提交
    fn standby_took(&mut self, now: i64, key: &str, room: i64, state: &str, bvid: Option<String>) {
        let started_at = key::parse(key).map_or(now, |(_, started_at)| started_at);
        let span = match self.rows.get(key) {
            Some(row) => Span::new(row.started_at, row.ended_at),
            None => Span::new(started_at, None),
        };
        self.note(now, key, room, started_at, None, state, bvid.clone());
        for matched in self.matching(room, span, key, None, now) {
            self.mark_standby(now, &matched, state, bvid.clone());
        }
    }

    fn mark_standby(&mut self, now: i64, key: &str, state: &str, bvid: Option<String>) {
        if let Some(row) = self.touch(key, now) {
            row.standby_state = Some(state.to_string());
            if bvid.is_some() && row.bvid.is_none() {
                row.bvid = bvid;
                row.uploader = Some(Uploader::Standby);
            }
        }
    }

    /// 主机这边与备机那一场对得上的场次：键相同、是它接手的那一场，或按房间 + 时间对得上
    fn matching(
        &self,
        room: i64,
        span: Span,
        key: &str,
        takeover_of: Option<&str>,
        now: i64,
    ) -> Vec<String> {
        self.rows
            .values()
            .filter(|row| row.primary_state != PrimaryState::None && row.room_id == room)
            .filter(|row| {
                let end = match row.ended_at {
                    Some(end) => Some(end),
                    None if self.active.contains_key(&row.session_key) => None,
                    // 中断的段不知道录到了什么时候，按最后一次有动静算
                    None => Some(row.started_at.max(row.progress_at.unwrap_or_default())),
                };
                row.session_key == key
                    || Some(row.session_key.as_str()) == takeover_of
                    || key::same_session(Span::new(row.started_at, end), span, self.window, now)
            })
            .map(|row| row.session_key.clone())
            .collect()
    }

    /// 把主机这边一场的现状发给备机
    fn answer(&mut self, key: &str, now: i64) {
        let Some(row) = self.rows.get(key).cloned() else {
            return;
        };
        let key = row.session_key.clone();
        let room = row.room_id;
        let started = HaMessage::SessionStarted {
            key: key.clone(),
            room,
            started_at: row.started_at,
            at: row.started_at,
        };
        let ended = row.ended_at.map(|at| HaMessage::SessionEnded {
            key: key.clone(),
            room,
            started_at: row.started_at,
            at,
            produced: true,
        });
        let messages = match row.primary_state {
            PrimaryState::None => Vec::new(),
            PrimaryState::Recording => vec![started],
            PrimaryState::Recorded => {
                let mut messages = vec![started];
                messages.extend(ended);
                messages
            }
            PrimaryState::Uploading => {
                let mut messages = vec![started];
                messages.extend(ended);
                messages.push(HaMessage::UploadStarted {
                    key: key.clone(),
                    room,
                    at: row.progress_at.unwrap_or(now),
                });
                messages.push(HaMessage::UploadProgress {
                    key: key.clone(),
                    room,
                    bytes: u64::try_from(row.upload_bytes).unwrap_or_default(),
                    at: now,
                });
                messages
            }
            PrimaryState::Uploaded => vec![HaMessage::Uploaded {
                key: key.clone(),
                room,
                bvid: row.bvid.clone().unwrap_or_default(),
                from: row.started_at,
                to: row.ended_at,
                yielded: self.yielded.contains(&key),
            }],
            PrimaryState::Failed => vec![HaMessage::UploadFailed {
                key: key.clone(),
                room,
                reason: row.reason.clone().unwrap_or_default(),
            }],
            PrimaryState::Skipped => vec![HaMessage::UploadSkipped {
                key: key.clone(),
                room,
                reason: row
                    .reason
                    .as_deref()
                    .and_then(SkipReason::parse)
                    .unwrap_or(SkipReason::NoFiles),
                detail: None,
            }],
            PrimaryState::Interrupted | PrimaryState::HandedOver => {
                vec![HaMessage::UploadSkipped {
                    key: key.clone(),
                    room,
                    reason: SkipReason::Handover,
                    detail: Some("主机这份没有录完或没有投完".into()),
                }]
            }
        };
        for message in messages {
            self.send(message);
        }
    }

    /// 备机那一场主机不投了：中断的主机段标成交给备机，告诉备机
    fn hand_over(&mut self, now: i64, standby_key: &str, room: i64, matches: &[String]) {
        for key in matches {
            let interrupted = self
                .rows
                .get(key)
                .is_some_and(|row| row.primary_state == PrimaryState::Interrupted);
            if interrupted && let Some(row) = self.touch(key, now) {
                row.primary_state = PrimaryState::HandedOver;
                row.reason = Some("主机回来时备机已在录这一场".into());
            }
            self.answer(key, now);
        }
        if !matches.iter().any(|key| key == standby_key) {
            self.send(HaMessage::UploadSkipped {
                key: standby_key.to_string(),
                room,
                reason: SkipReason::Handover,
                detail: Some("主机没有这一场的完整录像".into()),
            });
        }
    }

    /// 备机接手的一场主机这边还开着：断线以来拉流重连过的，说明主机也断过、录像有缺口，
    /// 停掉这次拉流、让备机那半追加上来；一直没断过的（只是两台之间不通）照常录到下播
    fn stop_reconnected(&mut self, now: i64, standby_key: &str, matches: &[String]) {
        for key in matches {
            let reconnected = self.active.contains_key(key)
                && self
                    .retried
                    .get(key)
                    .is_some_and(|at| *at >= self.disconnected_at);
            if reconnected && !self.yielded.contains(key) {
                self.yield_to(now, key, standby_key);
                self.outs.push(Out::Stop(key.clone()));
            }
        }
    }

    fn report(&mut self, now: i64, sessions: Vec<ReportedSession>) {
        self.gate_until = None;
        info!(sessions = sessions.len(), "HA：收到备机的场次上报");
        let mut answered = BTreeSet::new();
        for session in &sessions {
            let span = Span::new(session.started_at, session.ended_at);
            self.note(
                now,
                &session.key,
                session.room,
                session.started_at,
                session.ended_at,
                session.state.as_str(),
                session
                    .bvid
                    .clone()
                    .filter(|_| session.state == ReportedState::Uploaded),
            );
            if let Some(from) = &session.adopted_from {
                self.adopted(now, &session.key, from);
            }
            let matches = self.matching(
                session.room,
                span,
                &session.key,
                session.takeover_of.as_deref(),
                now,
            );
            match session.state {
                ReportedState::Uploaded | ReportedState::Uploading => {
                    for key in &matches {
                        self.mark_standby(now, key, session.state.as_str(), session.bvid.clone());
                    }
                }
                ReportedState::Recording => {
                    let taken_over = self.mode == HaMode::Takeover && session.takeover_of.is_some();
                    if !taken_over && matches.iter().any(|key| self.active.contains_key(key)) {
                        // 连接抖了一下，两台都还在录：照常
                        for key in &matches {
                            self.answer(key, now);
                        }
                    } else {
                        info!(
                            room = session.room,
                            key = session.key,
                            "HA：备机正在录这个房间，主机等它下播再开录"
                        );
                        self.held.insert(session.room, session.key.clone());
                        if taken_over {
                            self.stop_reconnected(now, &session.key, &matches);
                        }
                        match (self.mode, &session.takeover_of) {
                            (HaMode::DualRecord, _) => {
                                self.hand_over(now, &session.key, session.room, &matches)
                            }
                            (HaMode::Takeover, Some(takeover_of)) => {
                                if matches.is_empty() {
                                    self.send(HaMessage::UploadSkipped {
                                        key: takeover_of.clone(),
                                        room: session.room,
                                        reason: SkipReason::NoFiles,
                                        detail: Some("主机没有这一场的录像".into()),
                                    });
                                }
                                for key in &matches {
                                    if !self.resume(now, key, Some(takeover_of)) {
                                        self.answer(key, now);
                                    }
                                }
                            }
                            // 主机离线期间新开播的一场：备机自己投
                            (HaMode::Takeover, None) => {}
                        }
                    }
                    answered.extend(matches);
                }
                ReportedState::Holding => {
                    let live = matches.iter().any(|key| {
                        self.rows.get(key).is_some_and(|row| {
                            !matches!(
                                row.primary_state,
                                PrimaryState::Interrupted | PrimaryState::HandedOver
                            )
                        })
                    });
                    if live {
                        for key in &matches {
                            self.answer(key, now);
                        }
                    } else {
                        self.hand_over(now, &session.key, session.room, &matches);
                    }
                    answered.extend(matches);
                }
                ReportedState::AwaitingPrimary => {
                    if matches.is_empty() {
                        self.send(HaMessage::UploadSkipped {
                            key: session
                                .takeover_of
                                .clone()
                                .unwrap_or_else(|| session.key.clone()),
                            room: session.room,
                            reason: SkipReason::NoFiles,
                            detail: Some("主机没有这一场的录像".into()),
                        });
                    }
                    for key in &matches {
                        if !self.resume(now, key, session.takeover_of.as_deref()) {
                            self.answer(key, now);
                        }
                    }
                    answered.extend(matches);
                }
                ReportedState::Manual
                | ReportedState::Done
                | ReportedState::Dropped
                | ReportedState::Failed
                | ReportedState::Standby => {}
            }
        }
        // 主机重启、断线期间投成的：近 24 小时的都再告诉备机一遍（§3 残余风险的缓解）
        let resend: Vec<String> = self
            .rows
            .values()
            .filter(|row| {
                row.bvid.is_some()
                    && row.uploader == Some(Uploader::Primary)
                    && row.updated_at >= now - RESEND_WINDOW_MS
                    && !answered.contains(&row.session_key)
            })
            .map(|row| row.session_key.clone())
            .collect();
        // 断线期间主机开录的段备机不知道，补一条开录，备机才能把自己那一场对上
        let active: Vec<String> = self
            .active
            .keys()
            .filter(|key| !answered.contains(*key) && !resend.contains(*key))
            .cloned()
            .collect();
        for key in resend.iter().chain(&active) {
            self.answer(key, now);
        }
    }
}

enum Write {
    Save(SessionRecord),
    Delete(String),
    #[cfg(test)]
    Flush(tokio::sync::oneshot::Sender<()>),
}

/// 控制面进程里的主机：钩子、`ha_sessions` 与备机连接都经它
pub struct Primary {
    core: Mutex<PrimaryCore>,
    /// 配对里的房间：主播地址 → 控制面房间 id
    rooms: RwLock<HashMap<String, i64>>,
    /// 录制段（地址, 开播时刻）→ 场次键
    keys: Mutex<HashMap<(String, i64), String>>,
    link: Mutex<Option<mpsc::UnboundedSender<ControllerMessage>>>,
    writer: mpsc::UnboundedSender<Write>,
    gate: watch::Sender<bool>,
    /// 该停掉拉流的段（场次键）
    stop: watch::Sender<BTreeSet<String>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    /// 补投要用的房间、上传池与主库
    services: ServiceRegister,
    /// 本主机的启动时刻：之后才写盘的分段不属于上次中断的录制
    started: i64,
    me: Weak<Primary>,
}

impl Primary {
    pub(crate) async fn start(
        pool: ConnectionPool,
        services: ServiceRegister,
        mode: HaMode,
        params: HaParams,
        window: i64,
    ) -> AppResult<Arc<Self>> {
        let now = now_ms();
        store::prune_sessions(&pool, now - PRUNE_MS).await?;
        let rows = store::updated_since(&pool, now - KEEP_MS).await?;
        let core = PrimaryCore::new(mode, params, window, rows, now);
        let (writer, mut queue) = mpsc::unbounded_channel();
        let primary = Arc::new_cyclic(|me| Primary {
            core: Mutex::new(core),
            rooms: RwLock::default(),
            keys: Mutex::default(),
            link: Mutex::default(),
            writer,
            gate: watch::channel(false).0,
            stop: watch::channel(BTreeSet::new()).0,
            tasks: Mutex::default(),
            services,
            started: now,
            me: me.clone(),
        });
        // 写入任务不随 `stop` 中止：主机被换掉、解除配对时排着的写入照样落盘，最后一个引用放掉时结束
        tokio::spawn(async move {
            while let Some(item) = queue.recv().await {
                match item {
                    Write::Save(record) => {
                        if let Err(e) = store::save_session(&pool, &record).await {
                            warn!(key = record.session_key, error = ?e, "HA：没能写入 ha_sessions");
                        }
                    }
                    Write::Delete(key) => {
                        if let Err(e) = store::delete_session(&pool, &key).await {
                            warn!(key, error = ?e, "HA：没能删掉 ha_sessions 里的行");
                        }
                    }
                    #[cfg(test)]
                    Write::Flush(done) => {
                        let _ = done.send(());
                    }
                }
            }
        });
        let ticking = tokio::spawn(tick_loop(Arc::downgrade(&primary)));
        primary.tasks.lock().unwrap().push(ticking);
        primary.update(|_, _| ());
        info!(%mode, "HA：本机是主机");
        Ok(primary)
    }

    /// 解除配对或控制面退出
    pub(crate) fn stop(&self) {
        for task in self.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        *self.link.lock().unwrap() = None;
    }

    fn update<R>(&self, f: impl FnOnce(&mut PrimaryCore, i64) -> R) -> R {
        let mut core = self.core.lock().unwrap();
        let result = f(&mut core, now_ms());
        let outs = core.take();
        let open = core.gate_open();
        let link = self.link.lock().unwrap();
        for out in outs {
            match out {
                Out::Send(message) => {
                    if let Some(link) = link.as_ref() {
                        let _ = link.send(ControllerMessage::Ha(message));
                    }
                }
                Out::Save(record) => {
                    let _ = self.writer.send(Write::Save(record));
                }
                Out::Delete(key) => {
                    let _ = self.writer.send(Write::Delete(key));
                }
                Out::Stop(key) => {
                    self.stop.send_if_modified(|keys| keys.insert(key));
                }
                Out::Resume(job) => {
                    // 与普通投稿一样，解除配对、换主机时不中止：提交中途被打断就说不清投没投成
                    if let Some(primary) = self.me.upgrade() {
                        let services = self.services.clone();
                        tokio::spawn(upload::run_resume(primary, services, job, self.started));
                    }
                }
            }
        }
        self.gate.send_if_modified(|current| {
            let changed = *current != open;
            *current = open;
            changed
        });
        result
    }

    /// 等之前的 `ha_sessions` 写入都落盘
    #[cfg(test)]
    pub(crate) async fn flush(&self) {
        let (done, wait) = tokio::sync::oneshot::channel();
        if self.writer.send(Write::Flush(done)).is_ok() {
            let _ = wait.await;
        }
    }

    pub(crate) fn params(&self) -> HaParams {
        self.core.lock().unwrap().params()
    }

    pub(crate) fn configure(&self, mode: HaMode, params: HaParams) {
        info!(%mode, "HA：配对的模式或参数改了");
        self.update(|core, _| core.configure(mode, params));
    }

    /// 备机连着（收得到场次消息）
    pub(crate) fn linked(&self) -> bool {
        self.link.lock().unwrap().is_some()
    }

    /// 已经收到备机的上报（或等满了时限）
    pub(crate) fn reported(&self) -> bool {
        self.core.lock().unwrap().gate_open()
    }

    /// 转给备机；没连着时返回 `false`
    pub(crate) fn forward(&self, message: HaMessage) -> bool {
        match self.link.lock().unwrap().as_ref() {
            Some(link) => link.send(ControllerMessage::Ha(message)).is_ok(),
            None => false,
        }
    }

    pub(crate) fn set_rooms(&self, rooms: HashMap<String, i64>) {
        *self.rooms.write().unwrap() = rooms;
    }

    fn room_of(&self, url: &str) -> Option<i64> {
        self.rooms.read().unwrap().get(url).copied()
    }

    pub(crate) fn connected(&self, outbox: mpsc::UnboundedSender<ControllerMessage>) {
        *self.link.lock().unwrap() = Some(outbox);
        info!("HA：备机连上了，等它上报场次");
        self.update(|core, now| core.connected(now));
    }

    pub(crate) fn disconnected(&self) {
        *self.link.lock().unwrap() = None;
        warn!("HA：备机离线");
        self.update(|core, now| core.disconnected(now));
    }

    pub(crate) fn standby_message(&self, message: HaMessage) {
        self.update(|core, now| core.standby_message(now, message));
    }

    pub(crate) fn hold(&self, url: &str) -> Option<Hold> {
        let room = self.room_of(url)?;
        self.core.lock().unwrap().hold(room)
    }

    pub(crate) fn unit_started(&self, unit: &Unit) {
        let Some(room) = self.room_of(&unit.url) else {
            return;
        };
        let key = key::primary_key(room, unit.started_at);
        {
            let mut keys = self.keys.lock().unwrap();
            keys.retain(|(_, started_at), _| *started_at >= unit.started_at - KEEP_MS);
            keys.insert((unit.url.clone(), unit.started_at), key.clone());
        }
        info!(key, url = unit.url, "HA：主机开录");
        self.update(|core, now| core.unit_started(now, &key, room, unit.started_at, unit.session));
    }

    fn key_of(&self, unit: &Unit) -> Option<String> {
        self.keys
            .lock()
            .unwrap()
            .get(&(unit.url.clone(), unit.started_at))
            .cloned()
    }

    pub(crate) fn unit_ended(&self, unit: &Unit, output: UnitOutput) {
        let Some(key) = self.key_of(unit) else {
            return;
        };
        info!(
            key,
            seen = output.seen,
            sent = output.sent,
            "HA：主机录完一段"
        );
        self.stop.send_if_modified(|keys| keys.remove(&key));
        self.update(|core, now| core.unit_ended(now, &key, output));
    }

    pub(crate) fn retrying(&self, unit: &Unit) -> bool {
        let Some(key) = self.key_of(unit) else {
            return false;
        };
        self.update(|core, now| core.retrying(now, &key))
    }

    /// 备机接手了这一段、而主机断线以来重连过拉流时就绪；不在配对里的段永远不就绪
    pub(crate) async fn stop_requested(&self, unit: &Unit) {
        if let Some(key) = self.key_of(unit) {
            let mut stop = self.stop.subscribe();
            if stop.wait_for(|keys| keys.contains(&key)).await.is_ok() {
                return;
            }
        }
        std::future::pending::<()>().await;
    }

    pub(crate) fn plan(self: &Arc<Self>, unit: &Unit) -> Option<Plan> {
        let key = self.key_of(unit)?;
        self.core
            .lock()
            .unwrap()
            .tracks(&key)
            .then(|| Plan::Primary {
                primary: self.clone(),
                key,
            })
    }

    pub(crate) async fn wait_gate(&self) {
        let mut gate = self.gate.subscribe();
        let _ = gate.wait_for(|open| *open).await;
    }

    pub(crate) fn upload_begin(&self, key: &str) -> Begin {
        self.update(|core, now| core.upload_begin(now, key))
    }

    pub(crate) fn progress(&self, key: &str, bytes: u64) {
        self.update(|core, now| core.progress(now, key, bytes));
    }

    pub(crate) fn may_submit(&self, key: &str) -> bool {
        self.core.lock().unwrap().may_submit(key)
    }

    pub(crate) fn uploaded(&self, key: &str, bvid: &str) {
        info!(key, bvid, "HA：主机投成");
        self.update(|core, now| core.uploaded(now, key, bvid));
    }

    pub(crate) fn upload_failed(&self, key: &str, reason: &str) {
        warn!(key, reason, "HA：主机投稿失败，交给备机按模式处理");
        self.update(|core, now| core.upload_failed(now, key, reason));
    }

    pub(crate) fn upload_fenced(&self, key: &str) {
        warn!(key, "HA：提交前发现备机已在投这一场，主机不提交");
        self.update(|core, now| core.upload_fenced(now, key));
    }

    /// 配对里这个控制面房间的主播地址
    pub(crate) fn room_url(&self, room: i64) -> Option<String> {
        self.rooms
            .read()
            .unwrap()
            .iter()
            .find(|(_, id)| **id == room)
            .map(|(url, _)| url.clone())
    }

    pub(crate) fn resume_ended(&self, key: &str, ended_at: i64) {
        self.update(|core, now| core.resume_ended(now, key, ended_at));
    }

    pub(crate) fn resume_skipped(&self, key: &str, reason: SkipReason, detail: &str) {
        info!(
            key,
            reason = reason.as_str(),
            detail,
            "HA：主机那半没有可投的文件，交给备机投完整的一份"
        );
        self.update(|core, now| core.resume_skipped(now, key, reason, detail));
    }
}

async fn tick_loop(primary: Weak<Primary>) {
    let mut ticker = tokio::time::interval(TICK);
    loop {
        ticker.tick().await;
        let Some(primary) = primary.upgrade() else {
            return;
        };
        primary.update(|core, now| core.tick(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::upload::double::{self, Double};
    use crate::server::fleet::ha::wire::ReportedSession;

    const MIN: i64 = 60_000;
    const WINDOW: i64 = 10 * MIN;

    fn core(mode: HaMode, rows: Vec<SessionRecord>) -> PrimaryCore {
        let mut core = PrimaryCore::new(mode, HaParams::default(), WINDOW, rows, 0);
        core.take();
        core
    }

    fn sends(outs: &[Out]) -> Vec<HaMessage> {
        outs.iter()
            .filter_map(|out| match out {
                Out::Send(message) => Some(message.clone()),
                Out::Save(_) | Out::Delete(_) | Out::Stop(_) | Out::Resume(_) => None,
            })
            .collect()
    }

    fn kinds(outs: &[Out]) -> Vec<&'static str> {
        outs.iter()
            .filter_map(|out| match out {
                Out::Send(message) => Some(message.kind()),
                Out::Save(_) | Out::Delete(_) | Out::Stop(_) | Out::Resume(_) => None,
            })
            .collect()
    }

    fn resumed(outs: &[Out]) -> Vec<String> {
        outs.iter()
            .filter_map(|out| match out {
                Out::Resume(job) => Some(job.key.clone()),
                _ => None,
            })
            .collect()
    }

    fn saved(outs: &[Out], key: &str) -> Option<SessionRecord> {
        outs.iter().rev().find_map(|out| match out {
            Out::Save(row) if row.session_key == key => Some(row.clone()),
            _ => None,
        })
    }

    fn reported(key: &str, room: i64, started_at: i64, state: ReportedState) -> ReportedSession {
        ReportedSession {
            key: key.into(),
            room,
            started_at,
            ended_at: None,
            state,
            takeover_of: None,
            bvid: None,
            adopted_from: None,
        }
    }

    fn report(sessions: Vec<ReportedSession>) -> HaMessage {
        HaMessage::StandbyReport { sessions }
    }

    /// 连上并收到空上报：闸门打开
    fn online(core: &mut PrimaryCore, now: i64) {
        core.connected(now);
        core.standby_message(now, report(Vec::new()));
        core.take();
    }

    fn row(key: &str, room: i64, started_at: i64, state: PrimaryState) -> SessionRecord {
        SessionRecord {
            session_key: key.into(),
            room_id: room,
            started_at,
            primary_state: state,
            updated_at: started_at,
            ..SessionRecord::default()
        }
    }

    #[test]
    fn a_restart_interrupts_unfinished_sessions_and_waits_for_the_report() {
        let rows = vec![
            row("1:0", 1, 0, PrimaryState::Recording),
            row("2:0", 2, 0, PrimaryState::Uploading),
            row("3:0", 3, 0, PrimaryState::Recorded),
            row("4:0", 4, 0, PrimaryState::Uploaded),
        ];
        let mut core = PrimaryCore::new(HaMode::DualRecord, HaParams::default(), WINDOW, rows, 0);
        let outs = core.take();
        for key in ["1:0", "2:0", "3:0"] {
            assert_eq!(
                saved(&outs, key).unwrap().primary_state,
                PrimaryState::Interrupted
            );
        }
        assert!(saved(&outs, "4:0").is_none());
        // 备机一直没连上：启动后 STARTUP_WAIT_MS 内不开录不开投，之后照常
        assert!(!core.gate_open());
        assert!(core.hold(1).is_some_and(|hold| hold.quick));
        assert!(core.hold(99).is_some(), "等上报期间配对里的房间都不开录");
        core.tick(STARTUP_WAIT_MS - 1);
        assert!(!core.gate_open());
        core.tick(STARTUP_WAIT_MS);
        assert!(core.gate_open());
        assert_eq!(core.hold(1), None);
    }

    #[test]
    fn a_restarted_primary_waits_until_the_standby_can_have_redialled() {
        // 主机进程被杀后重启，备机正在接手录 7 号房间：它要么到 QUIC 空闲超时才发现旧连接断了，
        // 要么卡在停机期间发起的那次拨号里，连上、上报之前主机都不能开录这个房间
        let idle = crate::server::fleet::IDLE_TIMEOUT.as_millis() as i64;
        let redial =
            (node::CONNECT_TIMEOUT.as_millis() + node::STANDBY_BACKOFF_MAX.as_millis()) as i64;
        for late in [idle, redial] {
            let rows = vec![row("7:0", 7, 0, PrimaryState::Recording)];
            let mut core =
                PrimaryCore::new(HaMode::Takeover, HaParams::default(), WINDOW, rows, MIN);
            core.take();
            let reported_at = MIN + late + 2_000;
            core.tick(reported_at);
            assert!(
                core.hold(7).is_some_and(|hold| hold.quick),
                "备机 {late} 毫秒后才重连：主机还在等"
            );
            core.connected(reported_at);
            core.standby_message(
                reported_at,
                report(vec![ReportedSession {
                    takeover_of: Some("7:0".into()),
                    ..reported("standby:7:40000", 7, 40_000, ReportedState::Recording)
                }]),
            );
            assert!(core.gate_open());
            assert!(
                core.hold(7).is_some_and(|hold| !hold.quick),
                "备机在录这一场：主机等它下播再开录"
            );
        }
    }

    #[test]
    fn a_reconnect_closes_the_gate_until_the_report_or_ten_seconds() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.tick(STARTUP_WAIT_MS);
        assert!(core.gate_open());
        core.connected(MIN);
        assert!(!core.gate_open());
        core.standby_message(MIN + 100, report(Vec::new()));
        assert!(core.gate_open());
        core.disconnected(2 * MIN);
        core.connected(3 * MIN);
        assert!(!core.gate_open());
        core.tick(3 * MIN + REPORT_WAIT_MS);
        assert!(core.gate_open(), "备机连上却不上报：最多等 10 秒");
    }

    #[test]
    fn the_lifecycle_of_a_primary_upload_reaches_the_standby() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        let outs = core.take();
        assert_eq!(kinds(&outs), ["session_started"]);
        assert_eq!(saved(&outs, "7:60000").unwrap().local_session_id, Some(3));
        assert_eq!(core.upload_begin(2 * MIN, "7:60000"), Begin::Proceed);
        core.progress(3 * MIN, "7:60000", 1000);
        core.unit_ended(10 * MIN, "7:60000", UnitOutput { seen: 2, sent: 2 });
        let outs = core.take();
        assert_eq!(
            kinds(&outs),
            ["upload_started", "upload_progress", "session_ended"]
        );
        let row = saved(&outs, "7:60000").unwrap();
        assert_eq!(row.primary_state, PrimaryState::Uploading);
        assert_eq!((row.upload_bytes, row.progress_at), (1000, Some(3 * MIN)));
        assert!(core.may_submit("7:60000"));
        core.uploaded(11 * MIN, "7:60000", "BV1");
        let outs = core.take();
        assert_eq!(
            sends(&outs),
            [HaMessage::Uploaded {
                key: "7:60000".into(),
                room: 7,
                bvid: "BV1".into(),
                from: MIN,
                to: Some(10 * MIN),
                yielded: false,
            }]
        );
        let row = saved(&outs, "7:60000").unwrap();
        assert_eq!(row.primary_state, PrimaryState::Uploaded);
        assert_eq!(row.uploader, Some(Uploader::Primary));
    }

    #[test]
    fn nothing_is_sent_while_the_standby_is_away() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        let outs = core.take();
        assert!(sends(&outs).is_empty());
        assert!(saved(&outs, "7:60000").is_some(), "状态照样落库");
    }

    #[test]
    fn filtered_or_empty_units_are_reported_as_skipped() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:1", 7, MIN, 3);
        core.unit_started(MIN, "8:1", 8, MIN, 4);
        core.unit_started(MIN, "9:1", 9, MIN, 5);
        core.take();
        core.unit_ended(2 * MIN, "7:1", UnitOutput { seen: 3, sent: 0 });
        core.unit_ended(2 * MIN, "8:1", UnitOutput { seen: 0, sent: 0 });
        core.unit_ended(2 * MIN, "9:1", UnitOutput { seen: 3, sent: 1 });
        let outs = core.take();
        let skipped: Vec<(String, SkipReason)> = sends(&outs)
            .into_iter()
            .filter_map(|message| match message {
                HaMessage::UploadSkipped { key, reason, .. } => Some((key, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(
            skipped,
            [
                ("7:1".to_string(), SkipReason::Filtered),
                ("8:1".to_string(), SkipReason::NoFiles)
            ]
        );
        assert_eq!(
            saved(&outs, "9:1").unwrap().primary_state,
            PrimaryState::Recorded
        );
        assert!(sends(&outs).contains(&HaMessage::SessionEnded {
            key: "7:1".into(),
            room: 7,
            started_at: MIN,
            at: 2 * MIN,
            produced: false,
        }));
    }

    #[test]
    fn a_failed_upload_is_reported_and_a_reopened_pipe_does_not_upload_again() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:1", 7, MIN, 3);
        assert_eq!(core.upload_begin(MIN, "7:1"), Begin::Proceed);
        core.take();
        core.upload_failed(2 * MIN, "7:1", "B 站拒稿");
        let outs = core.take();
        assert_eq!(
            sends(&outs),
            [HaMessage::UploadFailed {
                key: "7:1".into(),
                room: 7,
                reason: "B 站拒稿".into(),
            }]
        );
        assert_eq!(core.upload_begin(3 * MIN, "7:1"), Begin::Skip);
    }

    #[test]
    fn the_standby_taking_a_session_fences_the_primary_submit() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.upload_begin(MIN, "7:60000");
        core.take();
        // 备机的一场用自己的键，按时间与主机这段对上
        core.standby_message(
            20 * MIN,
            HaMessage::UploadStarted {
                key: "standby:7:120000".into(),
                room: 7,
                at: 20 * MIN,
            },
        );
        assert!(!core.may_submit("7:60000"));
        core.upload_fenced(21 * MIN, "7:60000");
        let outs = core.take();
        assert_eq!(
            saved(&outs, "7:60000").unwrap().primary_state,
            PrimaryState::HandedOver
        );
        assert_eq!(core.upload_begin(22 * MIN, "7:60000"), Begin::Skip);
        core.standby_message(
            30 * MIN,
            HaMessage::Uploaded {
                key: "standby:7:120000".into(),
                room: 7,
                bvid: "BVS".into(),
                from: 2 * MIN,
                to: Some(25 * MIN),
                yielded: false,
            },
        );
        let outs = core.take();
        let standby = saved(&outs, "standby:7:120000").unwrap();
        assert_eq!(standby.bvid.as_deref(), Some("BVS"));
        assert_eq!(standby.uploader, Some(Uploader::Standby));
        assert_eq!(standby.primary_state, PrimaryState::None);
    }

    #[test]
    fn a_report_of_uploading_or_uploaded_hands_the_session_to_the_standby() {
        // 主机回来时备机正在投（§3 用户特别指出的情形）：主机不投，等备机的 Uploaded
        let rows = vec![row("7:60000", 7, MIN, PrimaryState::Uploading)];
        let mut core = core(HaMode::DualRecord, rows);
        core.connected(40 * MIN);
        core.standby_message(
            40 * MIN,
            report(vec![ReportedSession {
                ended_at: Some(30 * MIN),
                ..reported("7:60000", 7, MIN + 5000, ReportedState::Uploading)
            }]),
        );
        assert!(core.gate_open());
        assert!(!core.may_submit("7:60000"));
        assert_eq!(core.upload_begin(41 * MIN, "7:60000"), Begin::Skip);
        core.standby_message(
            50 * MIN,
            HaMessage::Uploaded {
                key: "7:60000".into(),
                room: 7,
                bvid: "BVS".into(),
                from: MIN + 5000,
                to: Some(30 * MIN),
                yielded: false,
            },
        );
        let outs = core.take();
        let row = saved(&outs, "7:60000").unwrap();
        assert_eq!(row.bvid.as_deref(), Some("BVS"));
        assert_eq!(row.uploader, Some(Uploader::Standby));
    }

    #[test]
    fn a_flap_while_both_record_changes_nothing() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.disconnected(2 * MIN);
        core.connected(2 * MIN + 20_000);
        core.take();
        core.standby_message(
            2 * MIN + 21_000,
            report(vec![reported(
                "7:60000",
                7,
                MIN + 3000,
                ReportedState::Recording,
            )]),
        );
        let outs = core.take();
        assert_eq!(kinds(&outs), ["session_started"]);
        assert_eq!(core.hold(7), None, "主机自己也在录：不交给备机");
    }

    #[test]
    fn mode_one_hands_over_a_session_the_primary_lost_and_holds_the_room() {
        let rows = vec![row("7:60000", 7, MIN, PrimaryState::Recording)];
        let mut core = core(HaMode::DualRecord, rows);
        core.connected(20 * MIN);
        core.standby_message(
            20 * MIN,
            report(vec![reported(
                "7:60000",
                7,
                MIN + 2000,
                ReportedState::Recording,
            )]),
        );
        let outs = core.take();
        assert_eq!(
            sends(&outs),
            [HaMessage::UploadSkipped {
                key: "7:60000".into(),
                room: 7,
                reason: SkipReason::Handover,
                detail: Some("主机这份没有录完或没有投完".into()),
            }]
        );
        assert_eq!(
            saved(&outs, "7:60000").unwrap().primary_state,
            PrimaryState::HandedOver
        );
        let hold = core.hold(7).unwrap();
        assert!(!hold.quick);
        assert_eq!(core.hold(8), None, "只挡备机在录的房间");
        core.standby_message(
            30 * MIN,
            HaMessage::SessionEnded {
                key: "7:60000".into(),
                room: 7,
                started_at: MIN + 2000,
                at: 30 * MIN,
                produced: true,
            },
        );
        assert_eq!(core.hold(7), None, "备机下播后主机恢复开录");
    }

    #[test]
    fn a_session_the_primary_never_saw_is_handed_over_under_the_standby_key() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.connected(20 * MIN);
        core.standby_message(
            20 * MIN,
            report(vec![
                reported("standby:7:600000", 7, 10 * MIN, ReportedState::Recording),
                ReportedSession {
                    ended_at: Some(15 * MIN),
                    ..reported("standby:8:60000", 8, MIN, ReportedState::Holding)
                },
            ]),
        );
        let outs = core.take();
        let skipped: Vec<String> = sends(&outs)
            .into_iter()
            .filter_map(|message| match message {
                HaMessage::UploadSkipped { key, reason, .. } => {
                    assert_eq!(reason, SkipReason::Handover);
                    Some(key)
                }
                _ => None,
            })
            .collect();
        assert_eq!(skipped, ["standby:7:600000", "standby:8:60000"]);
        assert!(core.hold(7).is_some());
        assert_eq!(core.hold(8), None, "录完了的不挡");
    }

    #[test]
    fn held_rooms_are_released_when_the_standby_stays_away() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.connected(0);
        core.standby_message(
            0,
            report(vec![reported(
                "standby:7:0",
                7,
                0,
                ReportedState::Recording,
            )]),
        );
        core.disconnected(MIN);
        let grace = HaParams::ms(HaParams::default().offline_grace);
        core.tick(MIN + grace - 1);
        assert!(core.hold(7).is_some());
        core.tick(MIN + grace);
        assert_eq!(core.hold(7), None);
    }

    #[test]
    fn a_holding_standby_learns_the_primary_result() {
        let uploaded = SessionRecord {
            ended_at: Some(30 * MIN),
            bvid: Some("BVP".into()),
            uploader: Some(Uploader::Primary),
            ..row("7:60000", 7, MIN, PrimaryState::Uploaded)
        };
        let failed = SessionRecord {
            ended_at: Some(30 * MIN),
            reason: Some("拒稿".into()),
            ..row("8:60000", 8, MIN, PrimaryState::Failed)
        };
        let mut core = core(HaMode::DualRecord, vec![uploaded, failed]);
        core.connected(40 * MIN);
        core.standby_message(
            40 * MIN,
            report(vec![
                ReportedSession {
                    ended_at: Some(31 * MIN),
                    ..reported("7:60000", 7, MIN, ReportedState::Holding)
                },
                ReportedSession {
                    ended_at: Some(31 * MIN),
                    ..reported("standby:8:62000", 8, MIN + 2000, ReportedState::Holding)
                },
            ]),
        );
        let outs = core.take();
        assert_eq!(kinds(&outs), ["uploaded", "upload_failed"]);
    }

    /// 待人工、放弃这类状态备机单独告诉主机，主机面板据此列出待人工的场次
    #[test]
    fn standby_states_are_recorded_for_the_panel() {
        let failed = SessionRecord {
            reason: Some("拒稿".into()),
            ..row("7:60000", 7, MIN, PrimaryState::Failed)
        };
        let mut core = core(HaMode::Takeover, vec![failed]);
        online(&mut core, 0);
        core.standby_message(
            MIN,
            HaMessage::SessionState {
                key: "standby:7:120000".into(),
                room: 7,
                state: ReportedState::Manual,
                reason: Some("主机那半投稿失败：拒稿".into()),
            },
        );
        let outs = core.take();
        let row = saved(&outs, "standby:7:120000").unwrap();
        assert_eq!(row.standby_state.as_deref(), Some("manual"));
        assert_eq!(row.reason.as_deref(), Some("主机那半投稿失败：拒稿"));
        assert_eq!(row.primary_state, PrimaryState::None);
        assert_eq!(row.started_at, 120_000);
        assert!(sends(&outs).is_empty());
    }

    /// 备机先开录、后来对上了主机的场次：备机键那一行并进主机那一行，不留一行永远「录制中」
    #[test]
    fn a_standby_session_that_adopts_the_primary_key_is_merged_into_its_row() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.standby_message(
            MIN,
            HaMessage::SessionStarted {
                key: "standby:7:60000".into(),
                room: 7,
                started_at: MIN,
                at: MIN,
            },
        );
        core.unit_started(MIN + 30_000, "7:90000", 7, MIN + 30_000, 1);
        core.take();
        assert!(core.tracks("standby:7:60000"));

        core.standby_message(
            2 * MIN,
            HaMessage::Adopted {
                key: "7:90000".into(),
                room: 7,
                from: "standby:7:60000".into(),
            },
        );
        let outs = core.take();
        assert!(!core.tracks("standby:7:60000"));
        assert!(
            outs.iter()
                .any(|out| matches!(out, Out::Delete(key) if key == "standby:7:60000"))
        );
        assert!(saved(&outs, "standby:7:60000").is_none());
        let row = saved(&outs, "7:90000").unwrap();
        assert_eq!(row.primary_state, PrimaryState::Recording);
        assert_eq!(row.standby_state.as_deref(), Some("recording"));
        assert!(sends(&outs).is_empty());

        // 改键那条消息丢在断线里：重连后的上报带着原来的键，照样并掉
        core.standby_message(
            3 * MIN,
            HaMessage::SessionStarted {
                key: "standby:8:180000".into(),
                room: 8,
                started_at: 3 * MIN,
                at: 3 * MIN,
            },
        );
        core.unit_started(3 * MIN, "8:180000", 8, 3 * MIN, 2);
        core.disconnected(4 * MIN);
        core.connected(4 * MIN + 5_000);
        core.standby_message(
            4 * MIN + 5_000,
            report(vec![ReportedSession {
                adopted_from: Some("standby:8:180000".into()),
                ..reported("8:180000", 8, 3 * MIN, ReportedState::Recording)
            }]),
        );
        let outs = core.take();
        assert!(!core.tracks("standby:8:180000"));
        assert!(
            outs.iter()
                .any(|out| matches!(out, Out::Delete(key) if key == "standby:8:180000"))
        );
        assert_eq!(
            saved(&outs, "8:180000").unwrap().standby_state.as_deref(),
            Some("recording")
        );
        assert!(core.gate_open());
    }

    #[test]
    fn uploads_of_the_last_day_are_resent_after_the_report() {
        let day = RESEND_WINDOW_MS;
        let recent = SessionRecord {
            bvid: Some("BV1".into()),
            uploader: Some(Uploader::Primary),
            updated_at: 2 * day,
            ..row("7:1", 7, 2 * day - MIN, PrimaryState::Uploaded)
        };
        let old = SessionRecord {
            bvid: Some("BV0".into()),
            uploader: Some(Uploader::Primary),
            updated_at: day - MIN,
            ..row("7:0", 7, day - 2 * MIN, PrimaryState::Uploaded)
        };
        let by_standby = SessionRecord {
            bvid: Some("BVS".into()),
            uploader: Some(Uploader::Standby),
            updated_at: 2 * day,
            ..row("standby:8:1", 8, 2 * day - MIN, PrimaryState::None)
        };
        let mut core = PrimaryCore::new(
            HaMode::DualRecord,
            HaParams::default(),
            WINDOW,
            vec![recent, old, by_standby],
            2 * day,
        );
        core.take();
        core.connected(2 * day + 1000);
        core.standby_message(2 * day + 1000, report(Vec::new()));
        let outs = core.take();
        let resent: Vec<String> = sends(&outs)
            .into_iter()
            .filter_map(|message| match message {
                HaMessage::Uploaded { key, .. } => Some(key),
                _ => None,
            })
            .collect();
        assert_eq!(resent, ["7:1"]);
    }

    #[test]
    fn units_started_while_apart_are_announced_after_the_report() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.tick(STARTUP_WAIT_MS);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.connected(2 * MIN);
        core.take();
        core.standby_message(
            2 * MIN,
            report(vec![reported("standby:8:0", 8, 0, ReportedState::Uploaded)]),
        );
        let outs = core.take();
        assert_eq!(kinds(&outs), ["session_started"]);
    }

    #[test]
    fn mode_two_answers_takeovers_and_new_sessions() {
        let interrupted = row("7:60000", 7, MIN, PrimaryState::Recording);
        let uploaded = SessionRecord {
            ended_at: Some(20 * MIN),
            bvid: Some("BVP".into()),
            uploader: Some(Uploader::Primary),
            ..row("9:60000", 9, MIN, PrimaryState::Uploaded)
        };
        let mut core = core(HaMode::Takeover, vec![interrupted, uploaded]);
        core.connected(60 * MIN);
        core.standby_message(
            60 * MIN,
            report(vec![
                // 接手主机中断的那场，还在录
                ReportedSession {
                    takeover_of: Some("7:60000".into()),
                    ..reported("standby:7:300000", 7, 5 * MIN, ReportedState::Recording)
                },
                // 主机离线期间新开播的一场
                reported("standby:8:1800000", 8, 30 * MIN, ReportedState::Recording),
                // 接手的一场录完了，主机那半已经投成
                ReportedSession {
                    takeover_of: Some("9:60000".into()),
                    ended_at: Some(50 * MIN),
                    ..reported(
                        "standby:9:1500000",
                        9,
                        25 * MIN,
                        ReportedState::AwaitingPrimary,
                    )
                },
                // 主机根本没有的一场
                ReportedSession {
                    takeover_of: Some("10:60000".into()),
                    ended_at: Some(50 * MIN),
                    ..reported(
                        "standby:10:120000",
                        10,
                        2 * MIN,
                        ReportedState::AwaitingPrimary,
                    )
                },
            ]),
        );
        let outs = core.take();
        let messages = sends(&outs);
        assert_eq!(
            resumed(&outs),
            ["7:60000"],
            "备机接手的正是主机中断的那场：主机先投它那半"
        );
        assert!(
            !messages
                .iter()
                .any(|message| message.key() == Some("7:60000")),
            "补投开始前不回复这一场"
        );
        assert!(messages.iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, bvid, .. } if key == "9:60000" && bvid == "BVP"
        )));
        assert!(messages.iter().any(|message| matches!(
            message,
            HaMessage::UploadSkipped { key, reason: SkipReason::NoFiles, .. } if key == "10:60000"
        )));
        assert!(
            !messages
                .iter()
                .any(|message| message.key() == Some("standby:8:1800000")),
            "新开播的一场备机自己投，不用回复"
        );
        // 两个在录的房间都等备机下播（§4：不掐断）
        assert!(core.hold(7).is_some());
        assert!(core.hold(8).is_some());
        assert_eq!(core.hold(9), None);
    }

    /// 主机边录边传、录到一半进程没了的 7:60000
    fn crashed() -> SessionRecord {
        SessionRecord {
            local_session_id: Some(3),
            unit_started_at: Some(MIN),
            progress_at: Some(3 * MIN),
            upload_bytes: 500,
            ..row("7:60000", 7, MIN, PrimaryState::Uploading)
        }
    }

    /// 备机接手了 7:60000，它那场从第 4 分钟录起
    fn takeover(state: ReportedState) -> ReportedSession {
        ReportedSession {
            takeover_of: Some("7:60000".into()),
            ended_at: (state != ReportedState::Recording).then_some(30 * MIN),
            ..reported("standby:7:240000", 7, 4 * MIN, state)
        }
    }

    /// 模式 2 主机重启后收到上报：先补投它那半，投成的 `Uploaded` 带着「让给了备机」，备机据此追加
    #[test]
    fn mode_two_resumes_the_interrupted_half_so_the_standby_appends() {
        let mut core = core(HaMode::Takeover, vec![crashed()]);
        core.connected(40 * MIN);
        core.standby_message(
            40 * MIN,
            report(vec![takeover(ReportedState::AwaitingPrimary)]),
        );
        assert!(core.gate_open());
        let outs = core.take();
        assert!(outs.contains(&Out::Resume(Resume {
            key: "7:60000".into(),
            room: 7,
            session: Some(3),
            unit_started_at: MIN,
        })));
        assert!(kinds(&outs).is_empty(), "补投开始前不回复这一场");
        let row = saved(&outs, "7:60000").unwrap();
        assert_eq!(row.primary_state, PrimaryState::Recorded);
        assert_eq!(row.uploader, Some(Uploader::Primary));
        assert_eq!((row.progress_at, row.upload_bytes), (None, 0));

        core.resume_ended(41 * MIN, "7:60000", 3 * MIN);
        assert_eq!(core.upload_begin(41 * MIN, "7:60000"), Begin::Proceed);
        core.progress(42 * MIN, "7:60000", 800);
        assert!(core.may_submit("7:60000"));
        core.uploaded(43 * MIN, "7:60000", "BVP");
        let outs = core.take();
        assert_eq!(
            kinds(&outs),
            [
                "session_ended",
                "upload_started",
                "upload_progress",
                "uploaded"
            ]
        );
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, bvid, from: MIN, to: Some(to), yielded: true, .. }
                if key == "7:60000" && bvid == "BVP" && *to == 3 * MIN
        )));
        assert_eq!(
            saved(&outs, "7:60000").unwrap().primary_state,
            PrimaryState::Uploaded
        );

        // 重连后的上报：不再补投，回复投成的稿件号
        core.disconnected(44 * MIN);
        core.connected(45 * MIN);
        core.standby_message(
            45 * MIN,
            report(vec![takeover(ReportedState::AwaitingPrimary)]),
        );
        let outs = core.take();
        assert!(resumed(&outs).is_empty());
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, yielded: true, .. } if key == "7:60000"
        )));
    }

    /// 补投有了结果之后主机再重启：投成的回复稿件号，跳过 / 失败的照旧回复，都不再投
    #[test]
    fn a_restart_after_the_catch_up_does_not_upload_again() {
        let done = SessionRecord {
            primary_state: PrimaryState::Uploaded,
            bvid: Some("BVP".into()),
            uploader: Some(Uploader::Primary),
            ended_at: Some(3 * MIN),
            ..crashed()
        };
        let skipped = SessionRecord {
            primary_state: PrimaryState::Skipped,
            reason: Some(SkipReason::Filtered.as_str().into()),
            ..crashed()
        };
        let failed = SessionRecord {
            primary_state: PrimaryState::Failed,
            reason: Some("upload double: submit rejected".into()),
            uploader: Some(Uploader::Primary),
            ..crashed()
        };
        for (row, expected) in [
            (done, "uploaded"),
            (skipped, "upload_skipped"),
            (failed, "upload_failed"),
        ] {
            let mut core = core(HaMode::Takeover, vec![row]);
            core.connected(50 * MIN);
            core.standby_message(
                50 * MIN,
                report(vec![takeover(ReportedState::AwaitingPrimary)]),
            );
            let outs = core.take();
            assert!(resumed(&outs).is_empty(), "{expected}：只投一次");
            assert_eq!(kinds(&outs), [expected]);
        }
    }

    /// 补投中途进程又没了：还没开始传的再补投一次；开始传了就不知道提交成没成功，转人工
    #[test]
    fn a_catch_up_cut_short_is_retried_only_before_it_started_uploading() {
        let waiting = SessionRecord {
            primary_state: PrimaryState::Recorded,
            uploader: Some(Uploader::Primary),
            progress_at: None,
            upload_bytes: 0,
            ..crashed()
        };
        let uploading = SessionRecord {
            primary_state: PrimaryState::Uploading,
            uploader: Some(Uploader::Primary),
            progress_at: Some(41 * MIN),
            ..crashed()
        };
        let restart = |row: SessionRecord| {
            let mut core = core(HaMode::Takeover, vec![row]);
            core.connected(50 * MIN);
            core.standby_message(
                50 * MIN,
                report(vec![takeover(ReportedState::AwaitingPrimary)]),
            );
            core.take()
        };
        assert_eq!(resumed(&restart(waiting)), ["7:60000"]);

        let outs = restart(uploading);
        assert!(resumed(&outs).is_empty());
        assert_eq!(
            saved(&outs, "7:60000").unwrap().primary_state,
            PrimaryState::Failed
        );
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::UploadFailed { key, reason, .. }
                if key == "7:60000" && reason.contains("不知道提交成没成功")
        )));
    }

    /// 主机那半太小或已不在盘上：`UploadSkipped` 让备机投完整的一份（§6 F），重连后照旧回复
    #[test]
    fn a_catch_up_without_usable_files_hands_the_whole_session_to_the_standby() {
        for reason in [SkipReason::Filtered, SkipReason::NoFiles] {
            let mut core = core(HaMode::Takeover, vec![crashed()]);
            core.connected(40 * MIN);
            core.standby_message(40 * MIN, report(vec![takeover(ReportedState::Recording)]));
            assert_eq!(resumed(&core.take()), ["7:60000"]);
            core.resume_skipped(41 * MIN, "7:60000", reason, "说明");
            let outs = core.take();
            assert_eq!(
                sends(&outs),
                [HaMessage::UploadSkipped {
                    key: "7:60000".into(),
                    room: 7,
                    reason,
                    detail: Some("说明".into()),
                }]
            );
            let row = saved(&outs, "7:60000").unwrap();
            assert_eq!(row.primary_state, PrimaryState::Skipped);
            assert_eq!(row.uploader, None);

            core.disconnected(42 * MIN);
            core.connected(43 * MIN);
            core.standby_message(43 * MIN, report(vec![takeover(ReportedState::Recording)]));
            let outs = core.take();
            assert!(resumed(&outs).is_empty());
            assert!(sends(&outs).iter().any(|message| matches!(
                message,
                HaMessage::UploadSkipped { key, reason: skipped, .. }
                    if key == "7:60000" && *skipped == reason
            )));
        }
    }

    /// 备机已经处理过（在投、投成、转人工、放弃、不用投）的场次，模式 1，以及备机接手的不是这一段：不补投
    #[test]
    fn no_catch_up_unless_the_standby_is_waiting_for_this_very_unit() {
        for state in [
            ReportedState::Uploading,
            ReportedState::Uploaded,
            ReportedState::Manual,
            ReportedState::Dropped,
            ReportedState::Done,
            ReportedState::Failed,
        ] {
            let mut core = core(HaMode::Takeover, vec![crashed()]);
            core.connected(40 * MIN);
            let session = ReportedSession {
                bvid: Some("BVS".into()),
                ..takeover(state)
            };
            core.standby_message(40 * MIN, report(vec![session]));
            let outs = core.take();
            assert!(resumed(&outs).is_empty(), "{state:?}");
            assert!(
                !sends(&outs)
                    .iter()
                    .any(|message| message.kind() == "upload_started"),
                "{state:?}"
            );
        }

        let outs = {
            let mut core = core(HaMode::DualRecord, vec![crashed()]);
            core.connected(40 * MIN);
            core.standby_message(40 * MIN, report(vec![takeover(ReportedState::Recording)]));
            core.take()
        };
        assert!(resumed(&outs).is_empty());
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::UploadSkipped { key, reason: SkipReason::Handover, .. } if key == "7:60000"
        )));

        // 同一房间另一段中断的也按时间对得上，但备机接手的是 7:60000：那一段照旧交给备机
        let earlier = SessionRecord {
            local_session_id: Some(3),
            ..row("7:0", 7, 0, PrimaryState::Recording)
        };
        let outs = {
            let mut core = core(HaMode::Takeover, vec![earlier, crashed()]);
            core.connected(40 * MIN);
            core.standby_message(
                40 * MIN,
                report(vec![takeover(ReportedState::AwaitingPrimary)]),
            );
            core.take()
        };
        assert_eq!(resumed(&outs), ["7:60000"]);
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::UploadSkipped { key, reason: SkipReason::Handover, .. } if key == "7:0"
        )));
    }

    /// 模式 2 主机断网期间这一段一直开着，网络回来时备机已接手：主机不续录，
    /// 投成后告诉备机这份没录到下播，备机才会追加它那半
    #[test]
    fn mode_two_yields_an_open_unit_the_standby_took_over() {
        let mut core = core(HaMode::Takeover, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        assert!(!core.retrying(2 * MIN, "7:60000"), "备机没接手：照常续录");
        core.disconnected(3 * MIN);
        core.connected(6 * MIN);
        core.take();
        core.standby_message(
            6 * MIN,
            report(vec![ReportedSession {
                takeover_of: Some("7:60000".into()),
                ..reported("standby:7:240000", 7, 4 * MIN, ReportedState::Recording)
            }]),
        );
        let outs = core.take();
        assert_eq!(kinds(&outs), ["session_started"]);
        assert!(
            !outs.contains(&Out::Stop("7:60000".into())),
            "断线以来没重连过拉流（只是两台之间不通）：不停"
        );
        assert!(core.hold(7).is_some(), "主机这一段还开着，也等备机下播");

        assert!(core.retrying(6 * MIN + 2000, "7:60000"));
        core.unit_ended(6 * MIN + 3000, "7:60000", UnitOutput { seen: 2, sent: 2 });
        assert_eq!(core.upload_begin(7 * MIN, "7:60000"), Begin::Proceed);
        core.uploaded(8 * MIN, "7:60000", "BVP");
        let outs = core.take();
        assert!(sends(&outs).iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, to: Some(to), yielded: true, .. }
                if key == "7:60000" && *to == 6 * MIN + 3000
        )));

        // 重连后的上报再问一遍，回复里也带着
        core.disconnected(9 * MIN);
        core.connected(9 * MIN + 5000);
        core.standby_message(
            9 * MIN + 5000,
            report(vec![ReportedSession {
                takeover_of: Some("7:60000".into()),
                ..reported("standby:7:240000", 7, 4 * MIN, ReportedState::Recording)
            }]),
        );
        assert!(sends(&core.take()).iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, yielded: true, .. } if key == "7:60000"
        )));

        core.standby_message(
            20 * MIN,
            HaMessage::SessionEnded {
                key: "standby:7:240000".into(),
                room: 7,
                started_at: 4 * MIN,
                at: 20 * MIN,
                produced: true,
            },
        );
        assert_eq!(core.hold(7), None);
    }

    /// 模式 1、以及模式 2 里备机没有接手的房间：拉流重连照常
    #[test]
    fn units_are_not_yielded_without_a_takeover() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.disconnected(2 * MIN);
        core.connected(3 * MIN);
        core.standby_message(
            3 * MIN,
            report(vec![ReportedSession {
                takeover_of: Some("7:60000".into()),
                ..reported("standby:7:120000", 7, 2 * MIN, ReportedState::Recording)
            }]),
        );
        assert!(!core.retrying(3 * MIN + 2000, "7:60000"));

        let mut core = self::core(HaMode::Takeover, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.unit_started(MIN, "8:60000", 8, MIN, 4);
        core.disconnected(2 * MIN);
        core.connected(4 * MIN);
        core.standby_message(
            4 * MIN,
            report(vec![ReportedSession {
                takeover_of: Some("8:60000".into()),
                ..reported("standby:8:180000", 8, 3 * MIN, ReportedState::Recording)
            }]),
        );
        assert!(!core.retrying(4 * MIN + 2000, "7:60000"));
        assert!(core.retrying(4 * MIN + 2000, "8:60000"));
    }

    /// 主机断网期间拉流重连过、网络回来时又接着录上了：备机上报接手时停掉这次拉流，这份让给备机
    #[test]
    fn a_unit_that_reconnected_while_apart_is_stopped_by_the_takeover_report() {
        let mut core = core(HaMode::Takeover, Vec::new());
        online(&mut core, 0);
        core.unit_started(MIN, "7:60000", 7, MIN, 3);
        core.disconnected(2 * MIN);
        assert!(
            !core.retrying(3 * MIN, "7:60000"),
            "还不知道备机接手：照常重连"
        );
        core.connected(4 * MIN);
        core.take();
        core.standby_message(
            4 * MIN,
            report(vec![ReportedSession {
                takeover_of: Some("7:60000".into()),
                ..reported("standby:7:150000", 7, 150_000, ReportedState::Recording)
            }]),
        );
        let outs = core.take();
        assert!(outs.contains(&Out::Stop("7:60000".into())));
        assert_eq!(
            saved(&outs, "7:60000").unwrap().reason.as_deref(),
            Some("备机已接手（standby:7:150000），主机不续录")
        );
        core.standby_message(
            5 * MIN,
            HaMessage::SessionEnded {
                key: "standby:7:150000".into(),
                room: 7,
                started_at: 150_000,
                at: 5 * MIN,
                produced: true,
            },
        );
        assert!(
            core.retrying(5 * MIN + 1000, "7:60000"),
            "备机先下播了也不再重连：这一场已经让给备机"
        );
        core.unit_ended(5 * MIN + 2000, "7:60000", UnitOutput { seen: 1, sent: 1 });
        core.upload_begin(6 * MIN, "7:60000");
        core.uploaded(7 * MIN, "7:60000", "BVP");
        assert!(sends(&core.take()).iter().any(|message| matches!(
            message,
            HaMessage::Uploaded { key, yielded: true, .. } if key == "7:60000"
        )));
    }

    #[tokio::test]
    async fn the_stop_signal_reaches_only_the_unit_the_standby_took_over() {
        let dir = tempfile::tempdir().unwrap();
        let (_db, pool) = store::tests::pool().await;
        let primary = Primary::start(
            pool,
            services(dir.path()).await,
            HaMode::Takeover,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        let (url, other) = ("https://live.example/7", "https://live.example/8");
        primary.set_rooms(HashMap::from([
            (url.to_string(), 7),
            (other.to_string(), 8),
        ]));
        let (outbox, _frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        primary.standby_message(report(Vec::new()));
        let unit = Unit::of(&context(url, "2026-01-01T00:00:00Z"));
        let unrelated = Unit::of(&context(other, "2026-01-01T00:00:00Z"));
        primary.unit_started(&unit);
        primary.unit_started(&unrelated);
        primary.disconnected();
        assert!(!primary.retrying(&unit));
        assert!(!primary.retrying(&unrelated));

        let (outbox, _frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        let key = key::primary_key(7, unit.started_at);
        let taken = unit.started_at + MIN;
        primary.standby_message(report(vec![ReportedSession {
            takeover_of: Some(key.clone()),
            ..reported(
                &key::standby_key(7, taken),
                7,
                taken,
                ReportedState::Recording,
            )
        }]));
        tokio::time::timeout(Duration::from_secs(1), primary.stop_requested(&unit))
            .await
            .expect("备机接手的那一段收到停止");
        let pending = primary.stop_requested(&unrelated);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), pending)
                .await
                .is_err()
        );
        primary.unit_ended(&unit, UnitOutput { seen: 1, sent: 1 });
        assert!(primary.stop.borrow().is_empty());
        primary.stop();
    }

    fn context(url: &str, date: &str) -> crate::server::infrastructure::context::Context {
        use crate::server::config::Config;
        use crate::server::infrastructure::context::{Context, Worker};
        use serde_json::json;
        use std::sync::RwLock;
        let streamer =
            serde_json::from_value(json!({ "id": 1, "url": url, "remark": "房间" })).unwrap();
        let upload = serde_json::from_value(json!({
            "id": 1, "template_name": "模板", "title": "{title}", "tags": ["t"], "uploader": "bili_web",
        }))
        .unwrap();
        let worker = Arc::new(Worker::new(
            streamer,
            Some(upload),
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        ));
        let stream = serde_json::from_value(json!({
            "name": "n", "url": url, "title": "直播标题", "date": date,
            "live_cover_url": "", "raw_stream_url": "http://127.0.0.1:9/x.flv", "platform": "p",
            "stream_headers": {}, "suffix": "flv", "danmaku": null, "downloader_hint": "StreamGears",
            "runtime_options": null,
        }))
        .unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        Context::new(1, worker, pool, stream)
    }

    fn segment(
        dir: &std::path::Path,
        name: &str,
        size: usize,
    ) -> crate::server::core::downloader::SegmentInfo {
        let path = dir.join(name);
        std::fs::write(&path, vec![0u8; size]).unwrap();
        crate::server::core::downloader::SegmentInfo::new(path, None, None, 0)
    }

    fn drain(frames: &mut mpsc::UnboundedReceiver<ControllerMessage>) -> Vec<HaMessage> {
        let mut out = Vec::new();
        while let Ok(frame) = frames.try_recv() {
            if let ControllerMessage::Ha(message) = frame {
                out.push(message);
            }
        }
        out
    }

    /// 主机真实的投稿流程（换成测试替身的 B 站）：生命周期帧发给备机、结果落进 `ha_sessions`，
    /// 备机接手的场次不提交，失败报 `UploadFailed`
    #[tokio::test]
    async fn the_primary_upload_path_reports_through_the_double() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let dir = tempfile::tempdir().unwrap();
        let control = dir.path().join("control");
        std::fs::create_dir_all(&control).unwrap();
        let recording = dir.path().join("rec");
        std::fs::create_dir_all(&recording).unwrap();
        let double = Arc::new(Double::new(
            dir.path().join("double.jsonl"),
            control.clone(),
            0,
            "P",
        ));
        double::install(Some(double.clone()));

        let (_db, pool) = store::tests::pool().await;
        let primary = Primary::start(
            pool.clone(),
            services(dir.path()).await,
            HaMode::DualRecord,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        let url = "https://live.example/7";
        primary.set_rooms(HashMap::from([(url.to_string(), 7)]));
        let (outbox, mut frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        primary.standby_message(report(Vec::new()));

        let ctx = context(url, "2026-01-01T00:00:00Z");
        let unit = Unit::of(&ctx);
        let key = key::primary_key(7, unit.started_at);
        primary.unit_started(&unit);
        let segments = vec![
            segment(&recording, "a-part1.flv", 3000),
            segment(&recording, "a-part2.flv", 2000),
        ];
        primary.unit_ended(&unit, UnitOutput { seen: 2, sent: 2 });
        let plan = primary.plan(&unit).expect("配对里的房间走 HA 投稿");
        let config = ctx.upload_config().clone().unwrap();
        plan.run(futures::stream::iter(segments), &ctx, &config)
            .await
            .unwrap();
        let messages = drain(&mut frames);
        let kinds: Vec<&str> = messages.iter().map(HaMessage::kind).collect();
        assert_eq!(
            kinds,
            [
                "session_started",
                "session_ended",
                "upload_started",
                "uploaded"
            ]
        );
        assert!(matches!(&messages[3], HaMessage::Uploaded { bvid, .. } if bvid == "BVP0001"));
        let ops: Vec<String> = double
            .entries()
            .iter()
            .map(|entry| entry["op"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            ops,
            [
                "login",
                "upload_start",
                "upload",
                "upload_start",
                "upload",
                "submit"
            ]
        );
        let submit = double.entries().pop().unwrap();
        assert_eq!(submit["parts"], serde_json::json!(["a-part1", "a-part2"]));
        primary.flush().await;
        let stored = store::session(&pool, &key).await.unwrap().unwrap();
        assert_eq!(stored.primary_state, PrimaryState::Uploaded);
        assert_eq!(stored.bvid.as_deref(), Some("BVP0001"));

        // 备机已经在投的一场：主机不传不提交，录像留着
        let ctx = context(url, "2026-01-01T02:00:00Z");
        let unit = Unit::of(&ctx);
        primary.unit_started(&unit);
        let key = key::primary_key(7, unit.started_at);
        primary.standby_message(HaMessage::UploadStarted {
            key: key.clone(),
            room: 7,
            at: 0,
        });
        let kept = segment(&recording, "b-part1.flv", 1000);
        let plan = primary.plan(&unit).unwrap();
        plan.run(futures::stream::iter(vec![kept.clone()]), &ctx, &config)
            .await
            .unwrap();
        assert!(kept.prev_file_path.exists());
        assert_eq!(double.entries().len(), 6, "没有碰上传端");

        // 提交被拒：UploadFailed
        std::fs::write(control.join("fail-submit"), "").unwrap();
        let ctx = context(url, "2026-01-01T04:00:00Z");
        let unit = Unit::of(&ctx);
        primary.unit_started(&unit);
        drain(&mut frames);
        let plan = primary.plan(&unit).unwrap();
        let result = plan
            .run(
                futures::stream::iter(vec![segment(&recording, "c.flv", 10)]),
                &ctx,
                &config,
            )
            .await;
        assert!(result.is_err());
        let messages = drain(&mut frames);
        assert!(matches!(
            messages.last(),
            Some(HaMessage::UploadFailed { reason, .. }) if reason.contains("submit rejected")
        ));
        primary.stop();
        double::install(None);
    }

    #[tokio::test]
    async fn a_restarted_primary_marks_unfinished_rows_interrupted_in_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let (_db, pool) = store::tests::pool().await;
        let now = now_ms();
        store::save_session(&pool, &row("7:1", 7, now - MIN, PrimaryState::Uploading))
            .await
            .unwrap();
        let primary = Primary::start(
            pool.clone(),
            services(dir.path()).await,
            HaMode::DualRecord,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        primary.flush().await;
        let stored = store::session(&pool, "7:1").await.unwrap().unwrap();
        assert_eq!(stored.primary_state, PrimaryState::Interrupted);
        let mut gate = primary.gate.subscribe();
        assert!(!*gate.borrow_and_update());
        primary.standby_message(report(Vec::new()));
        assert!(
            *gate.borrow_and_update(),
            "没连着也可以先收到上报（测试直接喂）"
        );
        primary.stop();
    }

    const RESUME_URL: &str = "https://resume.example/";

    /// 只认 [`RESUME_URL`] 开头的平台；检测一直不返回，不向任何真实平台发请求
    struct Idle;

    #[async_trait::async_trait]
    impl biliup::downloader::live::LivePlugin for Idle {
        fn name(&self) -> &'static str {
            "idle"
        }

        fn matches(&self, url: &str) -> bool {
            url.starts_with(RESUME_URL)
        }

        async fn check_stream(
            &self,
            _request: biliup::downloader::live::LiveRequest,
        ) -> biliup::downloader::live::LiveResult<biliup::downloader::live::LiveStatus> {
            std::future::pending().await
        }
    }

    /// 本机的主库、上传池与房间表（还没有房间）
    async fn services(dir: &std::path::Path) -> ServiceRegister {
        use crate::server::config::Config;
        use crate::server::core::download_manager::DownloadManager;
        use crate::server::infrastructure::connection_pool::ConnectionManager;
        use tracing_subscriber::{EnvFilter, reload};
        let db = dir.join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        managers.add_plugin(Arc::new(Idle)).await;
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await
    }

    /// 本机加一个带模板的房间 `{RESUME_URL}{room}`（备注「房间{room}」）
    async fn add_room(services: &ServiceRegister, room: i64) -> String {
        let url = format!("{RESUME_URL}{room}");
        sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (?, ?, ?)")
            .bind(room)
            .bind(&url)
            .bind(format!("房间{room}"))
            .execute(&services.pool)
            .await
            .unwrap();
        let streamer = serde_json::from_value(
            serde_json::json!({ "id": room, "url": url, "remark": format!("房间{room}") }),
        )
        .unwrap();
        let upload = serde_json::from_value(serde_json::json!({
            "id": room, "template_name": "模板", "title": "{streamer}：{title}", "tags": ["t"],
            "uploader": "bili_web",
        }))
        .unwrap();
        services
            .managers
            .add_room(services.worker(streamer, Some(upload)))
            .await
            .unwrap();
        url
    }

    /// 切片工作台里一场录制的分段：`(文件名, 字节数, 写盘时刻)`，按顺序接在时间轴上；
    /// 像上次异常退出那样都还是 `recording`，再跑一遍启动收尾。返回场次 id
    async fn recorded(
        services: &ServiceRegister,
        recording: &std::path::Path,
        streamer: i64,
        started_at: i64,
        files: &[(&str, u64, i64)],
    ) -> i64 {
        use crate::server::infrastructure::models::StreamerInfo;
        use crate::server::workbench::{self, store as bench};
        let pool = &services.pool;
        let date = chrono::DateTime::from_timestamp_millis(started_at).unwrap();
        let url = format!("{RESUME_URL}{streamer}");
        let info = StreamerInfo::new(&format!("房间{streamer}"), &url, "直播标题", date, "");
        let session = bench::open_session(pool, streamer, &info, started_at, 0)
            .await
            .unwrap()
            .id;
        bench::set_started_at(pool, session, started_at)
            .await
            .unwrap();
        for (n, (name, size, written)) in files.iter().enumerate() {
            let path = recording.join(name);
            let file = std::fs::File::create(&path).unwrap();
            file.set_len(*size).unwrap();
            let at = std::time::UNIX_EPOCH + Duration::from_millis(*written as u64);
            file.set_modified(at).unwrap();
            let start_ms = n as i64 * 5 * MIN;
            bench::insert_segment(pool, session, path.to_str().unwrap(), "flv", start_ms, 0)
                .await
                .unwrap();
        }
        workbench::recover(pool).await.unwrap();
        session
    }

    fn ops(double: &Double) -> Vec<String> {
        double
            .entries()
            .iter()
            .map(|entry| {
                let op = entry["op"].as_str().unwrap_or_default();
                match entry["file"].as_str() {
                    Some(file) => format!("{op} {file}"),
                    None => op.to_string(),
                }
            })
            .collect()
    }

    /// 等主机发出 `kind` 这一条，返回到它为止发出的场次消息
    async fn until(
        frames: &mut mpsc::UnboundedReceiver<ControllerMessage>,
        kind: &str,
    ) -> Vec<HaMessage> {
        let mut out = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(frame) = frames.recv().await {
                if let ControllerMessage::Ha(message) = frame {
                    let done = message.kind() == kind;
                    out.push(message);
                    if done {
                        return;
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("主机应该发出 {kind}，只发了 {out:?}"));
        out
    }

    /// 模式 2 主机重启后按切片工作台补投它那半（经测试替身）：只投这一段、没被过滤的分段，
    /// 中断留下的 `.part` 改回正式文件名；`Uploaded` 让备机追加。再重启一次不再投，只回复稿件号
    #[tokio::test]
    async fn a_restarted_primary_uploads_its_half_from_the_workbench() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let dir = tempfile::tempdir().unwrap();
        let control = dir.path().join("control");
        std::fs::create_dir_all(&control).unwrap();
        let recording = dir.path().join("rec");
        std::fs::create_dir_all(&recording).unwrap();
        let double = Arc::new(Double::new(
            dir.path().join("double.jsonl"),
            control,
            0,
            "P",
        ));
        double::install(Some(double.clone()));

        let services = services(dir.path()).await;
        let url = add_room(&services, 7).await;
        let unit = now_ms() - 60 * MIN;
        const BIG: u64 = 25_000_000;
        // 同一场里上一段录制（已经投过）的分段、这一段录完的、太小的、中断时正在写的
        let session = recorded(
            &services,
            &recording,
            7,
            unit - 30 * MIN,
            &[
                ("old.flv", BIG, unit - MIN),
                ("a-1.flv", BIG, unit + 5 * MIN),
                ("tiny.flv", 1000, unit + 6 * MIN),
                ("a-2.flv.part", BIG, unit + 9 * MIN),
            ],
        )
        .await;
        let key = key::primary_key(7, unit);
        let (_db, pool) = store::tests::pool().await;
        let crashed = SessionRecord {
            local_session_id: Some(session),
            unit_started_at: Some(unit),
            progress_at: Some(unit + 5 * MIN),
            ..row(&key, 7, unit, PrimaryState::Uploading)
        };
        store::save_session(&pool, &crashed).await.unwrap();
        let taken = ReportedSession {
            takeover_of: Some(key.clone()),
            ended_at: Some(unit + 40 * MIN),
            ..reported(
                &key::standby_key(7, unit + 11 * MIN),
                7,
                unit + 11 * MIN,
                ReportedState::AwaitingPrimary,
            )
        };

        let primary = Primary::start(
            pool.clone(),
            services.clone(),
            HaMode::Takeover,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        primary.set_rooms(HashMap::from([(url.clone(), 7)]));
        let (outbox, mut frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        primary.standby_message(report(vec![taken.clone()]));
        let messages = until(&mut frames, "uploaded").await;
        let kinds: Vec<&str> = messages.iter().map(HaMessage::kind).collect();
        assert_eq!(kinds, ["session_ended", "upload_started", "uploaded"]);
        assert!(matches!(
            messages.last(),
            Some(HaMessage::Uploaded { key: k, bvid, to: Some(to), yielded: true, .. })
                if *k == key && bvid == "BVP0001" && *to == unit + 9 * MIN
        ));
        assert_eq!(
            ops(&double),
            [
                "login",
                "upload_start a-1.flv",
                "upload a-1.flv",
                "upload_start a-2.flv",
                "upload a-2.flv",
                "submit",
            ]
        );
        let submit = double.entries().pop().unwrap();
        assert_eq!(submit["parts"], serde_json::json!(["a-1", "a-2"]));
        assert_eq!(submit["title"], "房间7：直播标题");
        assert!(recording.join("a-2.flv").exists());
        assert!(!recording.join("a-2.flv.part").exists());
        assert!(recording.join("old.flv").exists() && recording.join("tiny.flv").exists());
        let paths: Vec<String> =
            crate::server::workbench::store::session_segments(&services.pool, session)
                .await
                .unwrap()
                .into_iter()
                .map(|segment| segment.path)
                .collect();
        assert!(paths.contains(&recording.join("a-2.flv").to_string_lossy().into_owned()));
        primary.flush().await;
        let stored = store::session(&pool, &key).await.unwrap().unwrap();
        assert_eq!(stored.primary_state, PrimaryState::Uploaded);
        assert_eq!(stored.bvid.as_deref(), Some("BVP0001"));
        assert_eq!(stored.uploader, Some(Uploader::Primary));
        primary.stop();

        // 主机又重启：不再投，回复稿件号
        let primary = Primary::start(
            pool.clone(),
            services.clone(),
            HaMode::Takeover,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        primary.set_rooms(HashMap::from([(url, 7)]));
        let (outbox, mut frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        primary.standby_message(report(vec![taken]));
        let messages = until(&mut frames, "uploaded").await;
        assert!(matches!(
            messages.last(),
            Some(HaMessage::Uploaded { key: k, bvid, .. }) if *k == key && bvid == "BVP0001"
        ));
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(double.entries().len(), 6, "只投了一次");
        primary.stop();
        double::install(None);
    }

    /// 主机那半只剩太小的分段，或文件已经不在盘上：`UploadSkipped`，备机投完整的一份（§6 F）
    #[tokio::test]
    async fn a_catch_up_without_usable_files_reports_upload_skipped() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let dir = tempfile::tempdir().unwrap();
        let control = dir.path().join("control");
        std::fs::create_dir_all(&control).unwrap();
        let recording = dir.path().join("rec");
        std::fs::create_dir_all(&recording).unwrap();
        let double = Arc::new(Double::new(
            dir.path().join("double.jsonl"),
            control,
            0,
            "P",
        ));
        double::install(Some(double.clone()));

        let services = services(dir.path()).await;
        let unit = now_ms() - 60 * MIN;
        let mut rooms = HashMap::new();
        let mut rows = Vec::new();
        let mut reports = Vec::new();
        for room in [7, 8, 9] {
            rooms.insert(add_room(&services, room).await, room);
            let files: &[(&str, u64, i64)] = match room {
                7 => &[
                    ("tiny-1.flv", 1000, unit + MIN),
                    ("tiny-2.flv.part", 10, unit + 2 * MIN),
                ],
                8 => &[("gone.flv", 25_000_000, unit + MIN)],
                _ => &[("unlinked.flv", 25_000_000, unit + MIN)],
            };
            let session = recorded(&services, &recording, room, unit, files).await;
            let key = key::primary_key(room, unit);
            rows.push(SessionRecord {
                // 9：场次记录也没有
                local_session_id: (room != 9).then_some(session),
                unit_started_at: Some(unit),
                ..row(&key, room, unit, PrimaryState::Recording)
            });
            reports.push(ReportedSession {
                takeover_of: Some(key),
                ..reported(
                    &key::standby_key(room, unit + 3 * MIN),
                    room,
                    unit + 3 * MIN,
                    ReportedState::Recording,
                )
            });
        }
        std::fs::remove_file(recording.join("gone.flv")).unwrap();
        let (_db, pool) = store::tests::pool().await;
        for row in &rows {
            store::save_session(&pool, row).await.unwrap();
        }

        let primary = Primary::start(
            pool.clone(),
            services.clone(),
            HaMode::Takeover,
            HaParams::default(),
            WINDOW,
        )
        .await
        .unwrap();
        primary.set_rooms(rooms);
        let (outbox, mut frames) = mpsc::unbounded_channel();
        primary.connected(outbox);
        primary.standby_message(report(reports));
        let mut skipped = BTreeMap::new();
        while skipped.len() < 3 {
            for message in until(&mut frames, "upload_skipped").await {
                if let HaMessage::UploadSkipped { room, reason, .. } = message {
                    skipped.insert(room, reason);
                }
            }
        }
        assert_eq!(
            skipped,
            BTreeMap::from([
                (7, SkipReason::Filtered),
                (8, SkipReason::NoFiles),
                (9, SkipReason::NoFiles),
            ])
        );
        assert!(double.entries().is_empty(), "没有碰上传端");
        assert!(recording.join("tiny-1.flv").exists(), "太小的留在盘上");
        primary.flush().await;
        for row in &rows {
            let stored = store::session(&pool, &row.session_key)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.primary_state, PrimaryState::Skipped);
        }
        primary.stop();
        double::install(None);
    }
}
