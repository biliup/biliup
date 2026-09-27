//! 主机这一侧（ha-pair 方案 §2、§3、§4 里主机的部分）。
//!
//! 主机就是控制面进程，「本机」节点录的每一段都经钩子直接调到这里：开录、录完、开始投、进度、投成 / 失败，
//! 记进 `ha_sessions` 并发给备机。备机（重）连上后、以及主机进程刚启动时，先等备机的 `StandbyReport`
//! （最多 [`REPORT_WAIT_MS`]）：这期间不开始任何投稿、不开录配对里的房间。收到上报后把备机在录 / 在投的场次
//! 交给备机、按上报逐场回复主机这边的结果，再重发近 24 小时投成的场次。
//!
//! [`PrimaryCore`] 不碰时钟、不做 I/O，测试用虚拟时间驱动；[`Primary`] 把它接到钩子、数据库与备机连接上。

use super::key::{self, Span};
use super::params::{HaMode, HaParams};
use super::store::{self, PrimaryState, SessionRecord, Uploader};
use super::upload::Plan;
use super::wire::{HaMessage, ReportedSession, ReportedState, SkipReason};
use super::{Hold, Unit, UnitOutput};
use crate::server::errors::AppResult;
use crate::server::fleet::now_ms;
use crate::server::fleet::protocol::ControllerMessage;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// 备机（重）连上后、主机进程启动后最多等 `StandbyReport` 这么久（§2 硬规则）
pub const REPORT_WAIT_MS: i64 = 10_000;
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
    /// 备机连着的起始时刻
    connected: Option<i64>,
    /// 备机最近一次断开（或主机进程启动）的时刻
    disconnected_at: i64,
    /// 等备机上报的截止时刻；`None` 表示闸门开着
    gate_until: Option<i64>,
    /// 备机正在录、主机暂不开录的房间 → 备机那一场的键
    held: BTreeMap<i64, String>,
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
            connected: None,
            disconnected_at: now,
            gate_until: Some(now + REPORT_WAIT_MS),
            held: BTreeMap::new(),
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
                reason: "等备机上报它手里的场次（最多 10 秒）".into(),
                quick: true,
            });
        }
        let key = self.held.get(&room)?;
        Some(Hold {
            reason: format!("备机正在录这一场（{key}），等它下播再开录"),
            quick: false,
        })
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
            info!("HA：{REPORT_WAIT_MS} 毫秒内没等到备机的场次上报，主机照常开录、开投");
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
                if self.held.get(&room) == Some(&key) {
                    self.held.remove(&room);
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
            HaMessage::UploadProgress { .. }
            | HaMessage::UploadSkipped { .. }
            | HaMessage::Manual { .. } => {}
        }
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
                    if matches.iter().any(|key| self.active.contains_key(key)) {
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
                                    self.answer(key, now);
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
                        self.answer(key, now);
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
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Primary {
    pub(crate) async fn start(
        pool: ConnectionPool,
        mode: HaMode,
        params: HaParams,
        window: i64,
    ) -> AppResult<Arc<Self>> {
        let now = now_ms();
        store::prune_sessions(&pool, now - PRUNE_MS).await?;
        let rows = store::updated_since(&pool, now - KEEP_MS).await?;
        let core = PrimaryCore::new(mode, params, window, rows, now);
        let (writer, mut queue) = mpsc::unbounded_channel();
        let primary = Arc::new(Primary {
            core: Mutex::new(core),
            rooms: RwLock::default(),
            keys: Mutex::default(),
            link: Mutex::default(),
            writer,
            gate: watch::channel(false).0,
            tasks: Mutex::default(),
        });
        let writing = tokio::spawn(async move {
            while let Some(item) = queue.recv().await {
                match item {
                    Write::Save(record) => {
                        if let Err(e) = store::save_session(&pool, &record).await {
                            warn!(key = record.session_key, error = ?e, "HA：没能写入 ha_sessions");
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
        primary.tasks.lock().unwrap().extend([writing, ticking]);
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
        self.update(|core, now| core.unit_ended(now, &key, output));
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
                Out::Save(_) => None,
            })
            .collect()
    }

    fn kinds(outs: &[Out]) -> Vec<&'static str> {
        outs.iter()
            .filter_map(|out| match out {
                Out::Send(message) => Some(message.kind()),
                Out::Save(_) => None,
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
        // 备机没连上：10 秒内不开录不开投，之后照常
        assert!(!core.gate_open());
        assert!(core.hold(1).is_some_and(|hold| hold.quick));
        assert!(core.hold(99).is_some(), "等上报期间配对里的房间都不开录");
        core.tick(REPORT_WAIT_MS - 1);
        assert!(!core.gate_open());
        core.tick(REPORT_WAIT_MS);
        assert!(core.gate_open());
        assert_eq!(core.hold(1), None);
    }

    #[test]
    fn a_reconnect_closes_the_gate_until_the_report_or_ten_seconds() {
        let mut core = core(HaMode::DualRecord, Vec::new());
        core.tick(REPORT_WAIT_MS);
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
        core.tick(REPORT_WAIT_MS);
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
        assert!(messages.iter().any(|message| matches!(
            message,
            HaMessage::UploadSkipped { key, reason: SkipReason::Handover, .. } if key == "7:60000"
        )));
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
        let (_db, pool) = store::tests::pool().await;
        let now = now_ms();
        store::save_session(&pool, &row("7:1", 7, now - MIN, PrimaryState::Uploading))
            .await
            .unwrap();
        let primary = Primary::start(
            pool.clone(),
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
}
