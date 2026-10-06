//! 节点进程里的备机：把 [`StandbyCore`] 接到录制钩子、`data/ha-state.json` 与控制通道上。
//!
//! 节点代理收到带 `ha` 的期望状态才成为备机（[`NodeHa::assign`]），这时才建 `ha-state.json`；
//! 之后重启时按它恢复，控制面连不上也照常录、照常按规则投。没被指定为备机的节点这里什么都不做。
//!
//! 备机录的场次只把分段收下（[`super::upload::Plan::Collect`]），要投时另起任务、占上传池的一个槽位，
//! 按场次里记下的开播信息重建上下文再投（[`super::upload::run_standby`]）。

use super::adopt;
use super::handback::NodeHolds;
use super::key;
use super::member::{Member, member_for};
use super::outbox::{self, PairFile};
use super::params::{HaMode, HaParams};
use super::primary::Primary;
use super::standby::{Out, PrimaryView, SavedSegment, Session, StandbyCore, State, UnitData};
use super::sync::{Inventory, PairMessage, Side};
use super::upload::{self, Plan, Submitted};
use super::wire::{HaAssignment, HaMessage, ManualAction};
use super::{Hold, Link, Role, Unit, UnitOutput, set_role, sync_downloader};
use crate::server::common::upload::execute_postprocessor;
use crate::server::config::ConfigPatch;
use crate::server::core::downloader::SegmentInfo;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::events;
use crate::server::fleet::model::DesiredRoom;
use crate::server::fleet::protocol::{Ack, DesiredState};
use crate::server::fleet::reconcile::{FleetState, Reconciler};
use crate::server::fleet::{FLEET_MIGRATOR, now_ms};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::service_register::ServiceRegister;
use biliup::downloader::live::LiveStream;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub const STATE_FILE_NAME: &str = "ha-state.json";
/// 上传主机换到节点时，节点上主机那一侧的场次记录（与控制面的 `ha_sessions` 同一张表）
pub const PRIMARY_DB_NAME: &str = "ha-primary.sqlite3";
const STATE_FILE_VERSION: u32 = 1;
const TICK: Duration = Duration::from_secs(1);
/// 认下的录制段留多久（按开播时刻）
pub(super) const UNIT_KEEP_MS: i64 = 48 * 60 * 60 * 1000;

pub fn state_path(node_file: &Path) -> PathBuf {
    node_file.with_file_name(STATE_FILE_NAME)
}

fn primary_db(state: &Path) -> PathBuf {
    state.with_file_name(PRIMARY_DB_NAME)
}

/// `data/ha-state.json`：配对与备机手里的场次
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HaState {
    pub version: u32,
    /// 控制面 EndpointId；与 `node.json` 对不上时整份作废
    pub controller: String,
    /// 解除配对后为空，场次留着备查
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignment: Option<HaAssignment>,
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub primary: Vec<PrimaryView>,
}

impl HaState {
    fn new(controller: &str) -> Self {
        HaState {
            version: STATE_FILE_VERSION,
            controller: controller.to_string(),
            ..HaState::default()
        }
    }
}

pub fn load(path: &Path) -> Option<HaState> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<HaState>(&text) {
        Ok(state) if state.version == STATE_FILE_VERSION => Some(state),
        Ok(state) => {
            warn!(
                version = state.version,
                "{} has an unsupported version, ignoring it",
                path.display()
            );
            None
        }
        Err(e) => {
            warn!(error = %e, "{} is not valid, ignoring it", path.display());
            None
        }
    }
}

fn save(path: &Path, state: &HaState) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).change_context(AppError::Unknown)?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(state).change_context(AppError::Unknown)?;
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not write {}", tmp.display()))?;
        file.write_all(&body).change_context(AppError::Unknown)?;
        file.sync_all().change_context(AppError::Unknown)?;
    }
    std::fs::rename(&tmp, path)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("could not write {}", path.display()))
}

/// 离开控制面（`biliup node leave`）时删掉，本机当过上传主机时的场次记录一起删
pub fn forget(path: &Path) {
    let db = primary_db(path);
    let wal = db.with_extension("sqlite3-wal");
    let shm = db.with_extension("sqlite3-shm");
    for file in [path, db.as_path(), wal.as_path(), shm.as_path()] {
        match std::fs::remove_file(file) {
            Ok(()) => info!("{} removed", file.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!(error = %e, "could not remove {}", file.display()),
        }
    }
}

/// 场次里只留投稿用得到的开播信息：直链、请求头与弹幕 cookie 不落盘
fn stream_snapshot(stream: &LiveStream) -> Option<serde_json::Value> {
    let kept = LiveStream {
        raw_stream_url: String::new(),
        stream_headers: HashMap::new(),
        danmaku: None,
        runtime_options: None,
        ..stream.clone()
    };
    serde_json::to_value(kept).ok()
}

fn window(services: &ServiceRegister) -> i64 {
    let minutes = services.config.read().unwrap().live_merge_minutes;
    i64::try_from(minutes.saturating_mul(60_000)).unwrap_or(i64::MAX)
}

/// 镜像过来的一个房间
#[derive(Debug, Clone)]
pub(crate) struct Mirrored {
    /// 控制面房间 id
    id: i64,
    /// 房间覆写（判断在本机是不是边录边传）
    override_cfg: Option<ConfigPatch>,
}

impl Mirrored {
    pub(crate) fn new(id: i64, override_cfg: Option<ConfigPatch>) -> Self {
        Mirrored { id, override_cfg }
    }
}

/// 备机：被指定为备机的节点进程里，或上传主机换到节点之后的控制面进程里
pub struct Standby {
    core: Mutex<StandbyCore>,
    path: PathBuf,
    controller: String,
    assignment: Mutex<HaAssignment>,
    services: ServiceRegister,
    /// 镜像过来的房间：主播地址 → 房间
    rooms: RwLock<HashMap<String, Mirrored>>,
    /// 开录时认下的录制段（地址, 开播时刻）→（房间, 备机场次键）。
    /// 录到一半房间不再镜像（解除配对、改派走）时，这一段照样按场次走，不落到普通投稿流程
    units: Mutex<HashMap<(String, i64), (i64, String)>>,
    /// 配对已解除：手里的场次不再投，录到一半的段只留在本地
    retired: AtomicBool,
    link: Mutex<Option<Link>>,
    /// 正在投的场次
    uploading: Mutex<HashSet<String>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Standby {
    /// `controller` 是 `ha-state.json` 的归属：节点上是控制面的 EndpointId，控制面上是配对的对端
    pub(crate) fn start(
        path: PathBuf,
        controller: &str,
        assignment: HaAssignment,
        previous: Option<HaState>,
        services: ServiceRegister,
    ) -> Arc<Self> {
        let (sessions, primary) = previous
            .map(|state| (state.sessions, state.primary))
            .unwrap_or_default();
        let core = StandbyCore::new(&assignment, window(&services), sessions, primary, now_ms());
        info!(mode = %assignment.mode, primary = assignment.primary, rooms = assignment.rooms.len(), "HA：本机是备机");
        let standby = Arc::new(Standby {
            core: Mutex::new(core),
            path,
            controller: controller.to_string(),
            assignment: Mutex::new(assignment),
            services,
            rooms: RwLock::default(),
            units: Mutex::default(),
            retired: AtomicBool::new(false),
            link: Mutex::default(),
            uploading: Mutex::default(),
            tasks: Mutex::default(),
        });
        let ticking = tokio::spawn(tick_loop(Arc::downgrade(&standby)));
        standby.tasks.lock().unwrap().push(ticking);
        standby.update(|_, _| ());
        standby.persist(&standby.core.lock().unwrap(), true);
        standby
    }

    /// 节点代理停止、换成主机：停掉计时与在投的任务，场次留在 `ha-state.json`，重启后接着来
    pub(crate) fn stop(&self) {
        for task in self.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        *self.link.lock().unwrap() = None;
    }

    /// 控制面解除了配对：停下，`ha-state.json` 去掉配对、场次留着备查。
    /// 录到一半的镜像房间只把录像留在本地（[`Plan::Keep`]），主机那边照常投
    pub(crate) fn retire(&self) {
        self.retired.store(true, Ordering::Release);
        self.stop();
        let core = self.core.lock().unwrap();
        if let Err(e) = save(&self.path, &self.state(&core)) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    pub(crate) fn retired(&self) -> bool {
        self.retired.load(Ordering::Acquire)
    }

    fn state(&self, core: &StandbyCore) -> HaState {
        HaState {
            version: STATE_FILE_VERSION,
            controller: self.controller.clone(),
            assignment: (!self.retired()).then(|| self.assignment.lock().unwrap().clone()),
            sessions: core.sessions().cloned().collect(),
            primary: core.primary_views().cloned().collect(),
        }
    }

    fn persist(&self, core: &StandbyCore, force: bool) {
        if !force {
            return;
        }
        if let Err(e) = save(&self.path, &self.state(core)) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    /// 在核心上做一步，然后落盘（有变化时，在锁里写，提交前的「正在提交」标记一定先落盘）、执行它要做的事
    fn update<R>(self: &Arc<Self>, f: impl FnOnce(&mut StandbyCore, i64) -> R) -> R {
        let mut core = self.core.lock().unwrap();
        let result = f(&mut core, now_ms());
        let dirty = core.take_dirty();
        self.persist(&core, dirty);
        let outs = core.take();
        if self.retired() {
            return result;
        }
        for out in outs {
            match out {
                Out::Send(message) => {
                    if let Some(link) = self.link.lock().unwrap().as_ref() {
                        link.ha(message);
                    }
                }
                Out::Upload(id) => self.spawn_upload(id),
                Out::Append { id, bvid } => self.spawn_append(id, bvid),
                Out::Discard(id) => {
                    if let Some(session) = core.session(&id) {
                        discard(session);
                    }
                }
                Out::Attention { id, message } => {
                    if let Some(session) = core.session(&id) {
                        events::ha_attention(&session.unit.url, &session.unit.remark, &message);
                    }
                }
            }
        }
        result
    }

    pub(crate) fn assign(self: &Arc<Self>, assignment: HaAssignment) {
        *self.assignment.lock().unwrap() = assignment.clone();
        self.update(|core, _| core.assign(&assignment));
        self.persist(&self.core.lock().unwrap(), true);
    }

    /// 控制面当备机时改了模式或参数
    pub(crate) fn configure(self: &Arc<Self>, mode: HaMode, params: HaParams) {
        let mut assignment = self.assignment.lock().unwrap().clone();
        assignment.mode = mode;
        assignment.params = params;
        self.assign(assignment);
    }

    /// 有没有做到一半的场次（在录、等主机、在投、待人工）：有就不能换上传主机，返回其中一场的说明
    pub(crate) fn busy(&self) -> Option<String> {
        let core = self.core.lock().unwrap();
        core.sessions()
            .find(|session| {
                matches!(
                    session.state,
                    State::Recording
                        | State::Holding
                        | State::AwaitingPrimary
                        | State::Uploading
                        | State::Appending
                        | State::Manual
                )
            })
            .map(|session| {
                format!(
                    "备机上「{}」这一场还没了结（{}）",
                    session.unit.remark,
                    session.state.as_str()
                )
            })
    }

    pub(crate) fn set_rooms(&self, rooms: HashMap<String, Mirrored>) {
        let config = self.services.config.read().unwrap().clone();
        let sync = |rooms: &HashMap<String, Mirrored>| -> BTreeSet<i64> {
            rooms
                .values()
                .filter(|room| sync_downloader(&config, room.override_cfg.clone()))
                .map(|room| room.id)
                .collect()
        };
        let mut current = self.rooms.write().unwrap();
        let blocked = sync(&rooms);
        if !blocked.is_empty() && blocked != sync(&current) {
            warn!(rooms = ?blocked, "HA：这些镜像房间在本机是边录边传（sync-downloader），备机不录它们");
        }
        *current = rooms;
    }

    fn room_of(&self, url: &str) -> Option<i64> {
        self.rooms.read().unwrap().get(url).map(|room| room.id)
    }

    /// 镜像房间在本机的配置下是边录边传：备机不录（§5.1），不然主机投一份、备机边录边投又一份
    fn sync_room(&self, url: &str) -> bool {
        let Some(override_cfg) = self
            .rooms
            .read()
            .unwrap()
            .get(url)
            .map(|room| room.override_cfg.clone())
        else {
            return false;
        };
        sync_downloader(&self.services.config.read().unwrap(), override_cfg)
    }

    /// 这条连接上第一次收到配对：接上连接，返回要先发的 `StandbyReport`
    pub(crate) fn link_up(self: &Arc<Self>, link: Link) -> HaMessage {
        *self.link.lock().unwrap() = Some(link);
        info!("HA：连上主机，先上报手里的场次");
        self.update(|core, now| core.link_up(now))
    }

    pub(crate) fn link_down(self: &Arc<Self>) {
        if self.link.lock().unwrap().take().is_some() {
            warn!("HA：与主机断开");
        }
        self.update(|core, now| core.link_down(now));
    }

    pub(crate) fn linked(&self) -> bool {
        self.link.lock().unwrap().is_some()
    }

    pub(crate) fn primary_message(self: &Arc<Self>, message: HaMessage) {
        debug!(
            kind = message.kind(),
            key = message.key(),
            "HA：主机的场次消息"
        );
        self.update(|core, now| core.primary_message(now, message));
    }

    pub(crate) fn mirrors(&self, url: &str) -> bool {
        !self.retired() && self.room_of(url).is_some()
    }

    pub(crate) fn hold(&self, url: &str) -> Option<Hold> {
        let room = self.room_of(url)?;
        if self.retired() {
            return None;
        }
        if self.sync_room(url) {
            return Some(Hold {
                reason: "这个镜像房间在本机是边录边传（sync-downloader），备机不录".into(),
                quick: false,
            });
        }
        self.core.lock().unwrap().hold(now_ms(), room)
    }

    /// 开录时认下的录制段
    fn id_of(&self, unit: &Unit) -> Option<(i64, String)> {
        self.units
            .lock()
            .unwrap()
            .get(&(unit.url.clone(), unit.started_at))
            .cloned()
    }

    pub(crate) fn unit_started(self: &Arc<Self>, unit: &Unit, ctx: &Context) {
        if self.retired() {
            return;
        }
        let Some(room) = self.room_of(&unit.url) else {
            return;
        };
        let id = key::standby_key(room, unit.started_at);
        {
            let mut units = self.units.lock().unwrap();
            units.retain(|(_, started_at), _| *started_at >= unit.started_at - UNIT_KEEP_MS);
            units.insert((unit.url.clone(), unit.started_at), (room, id.clone()));
        }
        let data = UnitData {
            url: unit.url.clone(),
            remark: ctx.live_streamer().remark.clone(),
            stream_session: unit.session,
            stream: stream_snapshot(ctx.live_stream()),
            segments: Vec::new(),
        };
        self.update(|core, now| core.unit_started(now, &id, room, unit.started_at, data));
    }

    pub(crate) fn unit_ended(self: &Arc<Self>, unit: &Unit, output: UnitOutput) {
        if self.retired() {
            return;
        }
        let Some((_, id)) = self.id_of(unit) else {
            return;
        };
        self.update(|core, now| core.unit_ended(now, &id, output.sent > 0));
    }

    /// 镜像房间录下的段一律不走普通投稿流程：认下的交给场次，认不下的（开录时还不知道它是镜像房间、
    /// 场次已经清掉、配对已解除）只把录像留在本地。备机自己分派到的普通房间照常投，
    /// 开录时还是本地行、之后才加入配对的那一段也照常投
    pub(crate) fn plan(self: &Arc<Self>, unit: &Unit) -> Option<Plan> {
        let tracked = self.id_of(unit);
        if self.retired() {
            return tracked.map(|_| Plan::Keep {
                reason: "配对已解除，这一段由主机投",
            });
        }
        match tracked {
            Some((_, id)) if self.core.lock().unwrap().session(&id).is_some() => {
                Some(Plan::Collect {
                    standby: self.clone(),
                    id,
                })
            }
            Some(_) => Some(Plan::Keep {
                reason: "这一场备机已经处理完",
            }),
            None if self.room_of(&unit.url).is_some()
                && !adopt::joined_after(&self.services, &unit.url, unit.started_at) =>
            {
                Some(Plan::Keep {
                    reason: "开录时还没认下这个镜像房间",
                })
            }
            None => None,
        }
    }

    pub(crate) fn segment(self: &Arc<Self>, id: &str, event: &SegmentInfo) {
        let segment = SavedSegment {
            path: event.prev_file_path.clone(),
            danmaku: event.danmaku_file_path.clone(),
            index: event.segment_index,
        };
        self.update(|core, now| core.segment(now, id, segment));
    }

    pub(crate) fn collected(self: &Arc<Self>, id: &str) {
        self.update(|core, now| core.collected(now, id));
    }

    pub(crate) fn begin_submit(self: &Arc<Self>, id: &str) -> bool {
        self.update(|core, now| core.begin_submit(now, id))
    }

    /// 备机本地节点页上点的人工处理（§6 H）；主机面板上点的经控制面转来，走 [`Self::primary_message`]
    pub(crate) fn manual(self: &Arc<Self>, key: &str, action: ManualAction) -> Result<(), String> {
        if self.retired() {
            return Err("配对已解除".into());
        }
        info!(key, ?action, "HA：备机本地的人工处理");
        self.update(|core, now| core.manual(now, key, action))
    }

    /// `GET /v1/node/ha`：配对、与主机的连接和手里的场次（不带开播信息与分段路径）
    pub(crate) fn view(&self) -> serde_json::Value {
        let assignment = self.assignment.lock().unwrap().clone();
        let core = self.core.lock().unwrap();
        let sessions: Vec<serde_json::Value> = core
            .sessions()
            .map(|session| {
                json!({
                    "id": session.id,
                    "key": session.key,
                    "room": session.room,
                    "url": session.unit.url,
                    "remark": session.unit.remark,
                    "kind": session.kind,
                    "state": session.state.as_str(),
                    "started_at": session.started_at,
                    "ended_at": session.ended_at,
                    "takeover_of": session.takeover_of,
                    "bvid": session.bvid,
                    "reason": session.reason,
                    "since": session.since,
                    "collected": session.collected,
                    "segments": session.unit.segments.len(),
                })
            })
            .collect();
        json!({
            "role": "standby",
            "leader": assignment.leader,
            "mode": assignment.mode,
            "params": assignment.params,
            "primary": assignment.primary,
            "rooms": assignment.rooms,
            "linked": self.linked(),
            "primary_offline": core.primary_offline(now_ms()),
            "sessions": sessions,
        })
    }

    fn spawn(self: &Arc<Self>, id: String, job: impl Future<Output = ()> + Send + 'static) {
        if !self.uploading.lock().unwrap().insert(id.clone()) {
            return;
        }
        let standby = self.clone();
        let task = tokio::spawn(async move {
            job.await;
            standby.uploading.lock().unwrap().remove(&id);
        });
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|task| !task.is_finished());
        tasks.push(task);
    }

    fn spawn_upload(self: &Arc<Self>, id: String) {
        let standby = self.clone();
        let job_id = id.clone();
        self.spawn(id, async move { standby.run(job_id, None).await });
    }

    fn spawn_append(self: &Arc<Self>, id: String, bvid: String) {
        let standby = self.clone();
        let job_id = id.clone();
        self.spawn(id, async move { standby.run(job_id, Some(bvid)).await });
    }

    /// 按场次里记下的开播信息与本机的房间重建上下文
    async fn context(&self, session: &Session) -> Result<Context, String> {
        let worker = self
            .services
            .managers
            .get_rooms()
            .await
            .into_iter()
            .find(|worker| worker.live_streamer.url == session.unit.url)
            .ok_or("这个房间已经不在本机")?;
        let stream: LiveStream = session
            .unit
            .stream
            .clone()
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or("缺少这一场的开播信息")?;
        let ctx = Context::new(
            session.unit.stream_session,
            worker,
            self.services.pool.clone(),
            stream,
        );
        if ctx
            .upload_config()
            .as_ref()
            .is_none_or(|config| config.is_noop_uploader())
        {
            return Err("这个房间没有投稿模板".into());
        }
        Ok(ctx)
    }

    /// 投这一场：`append` 为空时作为完整稿件投，否则追加为这个稿件的后续分 P。占上传池的一个槽位
    async fn run(self: Arc<Self>, id: String, append: Option<String>) {
        let slots = self.services.managers.upload_slots();
        let _slot = slots.acquire().await;
        let Some(session) = self.core.lock().unwrap().session(&id).cloned() else {
            return;
        };
        let ctx = match self.context(&session).await {
            Ok(ctx) => ctx,
            Err(reason) => {
                self.update(|core, now| core.upload_failed(now, &id, &reason));
                return;
            }
        };
        info!(
            id,
            segments = session.unit.segments.len(),
            append = append.as_deref(),
            "HA：备机开始投这一场"
        );
        self.update(|core, now| core.upload_started(now, &id));
        let segments = session
            .unit
            .segments
            .iter()
            .map(|segment| {
                SegmentInfo::new(
                    segment.path.clone(),
                    segment.danmaku.clone(),
                    None,
                    segment.index,
                )
            })
            .collect();
        let result = match &append {
            None => upload::run_standby(&self, &id, &ctx, segments).await,
            Some(bvid) => upload::run_append(&self, &id, &ctx, segments, bvid).await,
        };
        match result {
            Ok(Submitted::Done { bvid, paths }) => {
                match append {
                    None => self.update(|core, now| core.uploaded(now, &id, &bvid)),
                    Some(_) => self.update(|core, now| core.appended(now, &id, &bvid)),
                }
                if let Err(e) = execute_postprocessor(paths, &ctx).await {
                    warn!(id, error = ?e, "HA：备机投成后的后处理失败");
                }
            }
            Ok(Submitted::Fenced) => self.update(|core, now| core.upload_fenced(now, &id)),
            Err(e) => {
                let reason = events::scrub(&format!("{e:#}"));
                self.update(|core, now| core.upload_failed(now, &id, &reason));
            }
        }
    }
}

/// 主机投成了：删掉备机这份录像（`delete_standby_copy`）
fn discard(session: &Session) {
    for segment in &session.unit.segments {
        for path in std::iter::once(&segment.path).chain(segment.danmaku.as_ref()) {
            match std::fs::remove_file(path) {
                Ok(()) => info!(file = %path.display(), "HA：主机投成了，删掉备机这份"),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => warn!(file = %path.display(), error = %e, "HA：删不掉备机这份"),
            }
        }
    }
}

async fn tick_loop(standby: Weak<Standby>) {
    let mut ticker = tokio::time::interval(TICK);
    loop {
        ticker.tick().await;
        let Some(standby) = standby.upgrade() else {
            return;
        };
        standby.update(|core, now| core.tick(now));
    }
}

/// 节点代理持有的一主一备入口。没有被指定进配对时什么都不做，也不建 `ha-state.json`。
///
/// 通常本机是备机（[`Standby`]）；上传主机换到本机之后（[`HaAssignment::leader`]，H2）本机跑主机那一侧
/// （[`Primary`]，场次记在 `data/ha-primary.sqlite3`），控制面进程改跑备机。`ha-state.json` 里的配对记着
/// 此刻谁是主机，控制面连不上时重启也按它恢复
pub struct NodeHa {
    path: PathBuf,
    controller: String,
    services: ServiceRegister,
    /// 控制面进程内嵌的「本机」节点不在这里当主机或备机（由控制面的配对直接接钩子）
    enabled: bool,
    standby: Option<Arc<Standby>>,
    /// 解除配对后留着的备机（角色还在，见 [`Standby::retire`]），重启或再次被指定时换掉
    retired: Option<Arc<Standby>>,
    /// 上传主机换到本机时的主机
    primary: Option<Arc<Primary>>,
    /// 最近一次收到（或重启时恢复）的配对
    assignment: Option<HaAssignment>,
    /// 期望状态里的房间：控制面房间 id → 主播地址与覆写
    managed: HashMap<i64, (String, Option<ConfigPatch>)>,
    /// 与控制面的双向同步（H2）：控制面次版本 ≥ 5、期望状态带 `pair` 时才有
    pair: Option<Arc<Member>>,
    /// 解除配对后正在交还给本机的房间：交接完之前挡着开录（H2，[`super::handback`]）
    holds: Option<NodeHolds>,
}

impl NodeHa {
    /// 节点代理启动：上次在配对里就按 `ha-state.json` 恢复成备机或主机
    pub async fn resume(
        node_file: &Path,
        controller: &str,
        local: bool,
        services: ServiceRegister,
        fleet: &FleetState,
    ) -> Self {
        let mut ha = NodeHa {
            path: state_path(node_file),
            controller: controller.to_string(),
            services,
            enabled: !local,
            standby: None,
            retired: None,
            primary: None,
            assignment: None,
            managed: HashMap::new(),
            pair: None,
            holds: None,
        };
        if !ha.enabled {
            return ha;
        }
        ha.holds = Some(NodeHolds::resume(&ha.data_dir(), controller, &ha.services));
        match load(&ha.path) {
            Some(state) if state.controller != controller => {
                warn!("{} belongs to another controller", ha.path.display());
                forget(&ha.path);
            }
            Some(state) => match state.assignment.clone() {
                Some(assignment) if assignment.leader == Side::Node => {
                    ha.assignment = Some(assignment);
                }
                Some(assignment) => ha.start(assignment, Some(state)),
                None => {}
            },
            None => {}
        }
        ha.set_rooms(fleet);
        if ha
            .assignment
            .as_ref()
            .is_some_and(|a| a.leader == Side::Node)
        {
            ha.lead(None).await;
        }
        ha
    }

    fn start(&mut self, assignment: HaAssignment, previous: Option<HaState>) {
        self.assignment = Some(assignment.clone());
        let standby = Standby::start(
            self.path.clone(),
            &self.controller,
            assignment,
            previous,
            self.services.clone(),
        );
        set_role(Some(Role::Standby(standby.clone())));
        self.standby = Some(standby);
        self.retired = None;
    }

    /// 本机当上传主机：停下备机、起主机（已经在跑时只更新房间），连着控制面时接上连接等它上报
    async fn lead(&mut self, link: Option<&Link>) {
        let Some(assignment) = self.assignment.clone() else {
            return;
        };
        if let Some(standby) = self.standby.take() {
            info!("HA：上传主机换到本机，本机不再当备机");
            standby.stop();
        }
        self.retired = None;
        park(&self.path, &self.controller, Some(&assignment));
        if self.primary.is_none() {
            match self.start_primary(&assignment).await {
                Ok(primary) => {
                    set_role(Some(Role::Primary(primary.clone())));
                    self.primary = Some(primary);
                }
                Err(e) => {
                    error!(error = ?e, "HA：本机没能当上传主机（打不开 {}），这次不接场次", PRIMARY_DB_NAME);
                    set_role(None);
                    return;
                }
            }
        }
        let rooms = self.primary_rooms();
        let Some(primary) = &self.primary else {
            return;
        };
        primary.set_rooms(rooms);
        if let Some(link) = link
            && !primary.linked()
        {
            primary.connected(link.clone());
        }
    }

    async fn start_primary(&self, assignment: &HaAssignment) -> AppResult<Arc<Primary>> {
        let path = primary_db(&self.path);
        let pool = ConnectionManager::new_pool_with(&path.to_string_lossy(), &FLEET_MIGRATOR)
            .await
            .attach_with(|| format!("could not open {}", path.display()))?;
        Primary::start(
            pool,
            self.services.clone(),
            assignment.mode,
            assignment.params,
            window(&self.services),
        )
        .await
    }

    pub fn is_standby(&self) -> bool {
        self.standby.is_some()
    }

    /// 上传主机换到了本机
    pub fn is_primary(&self) -> bool {
        self.primary.is_some()
    }

    /// 有没有做到一半的场次：有就不能换上传主机
    fn busy(&self) -> Option<String> {
        let standby = self.standby.as_ref().and_then(|standby| standby.busy());
        standby.or_else(|| self.primary.as_ref().and_then(|primary| primary.busy()))
    }

    fn data_dir(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    /// 节点代理启动时：上次在与控制面同步（有属于这个控制面的 `pair-outbox.json`）就接着来，
    /// 连不上控制面期间本机的修改照样记账、排队
    pub async fn resume_pair(&mut self) {
        if !self.enabled || self.pair.is_some() {
            return;
        }
        let dir = self.data_dir();
        if PairFile::load(&outbox::path_in(&dir), &self.controller).is_none() {
            return;
        }
        self.pair = Some(
            Member::start(
                Side::Node,
                &dir,
                &self.controller,
                self.services.clone(),
                self.leader(),
            )
            .await,
        );
    }

    fn leader(&self) -> Side {
        self.assignment
            .as_ref()
            .map_or(Side::Controller, |assignment| assignment.leader)
    }

    /// 期望状态里的同步版本（落地房间与上报场次之前）：带了 `pair` 就建好同步端、接上连接，
    /// 先发离线期间排下的修改；不再带（解除配对，或控制面不支持同步）就停下并删掉同步账本。
    /// 交还中的房间先挡住开录（在解除配对、备机退下之前，免得中间开录一场按普通投稿流程投）
    pub async fn pair(&mut self, desired: &DesiredState, link: &Link) {
        if !self.enabled {
            return;
        }
        if let Some(holds) = &mut self.holds {
            holds.apply(desired);
        }
        if desired.ha.is_none() || desired.pair.is_none() {
            if let Some(member) = self.pair.take() {
                member.dissolve();
            }
            return;
        }
        let leader = desired
            .ha
            .as_ref()
            .map_or(Side::Controller, |assignment| assignment.leader);
        if self.pair.is_none() {
            self.pair = Some(
                Member::start(
                    Side::Node,
                    &self.data_dir(),
                    &self.controller,
                    self.services.clone(),
                    leader,
                )
                .await,
            );
        }
        if let Some(member) = &self.pair {
            member.set_primary(leader);
            member.link_up(link.clone()).await;
        }
    }

    /// 控制面发来的同步消息。控制面问能不能换上传主机时按本机有没有做到一半的场次回话；
    /// 问本机还没纳入配对的本地行时（指定备机之前也问）照实回
    pub async fn pair_message(&self, message: PairMessage, link: &Link) {
        if let PairMessage::InventoryAsk(ask) = &message {
            if self.enabled {
                link.pair(PairMessage::Inventory(self.inventory(ask.id).await));
            }
            return;
        }
        let Some(member) = &self.pair else {
            debug!(op = message.op(), "pair frame while not paired");
            return;
        };
        match message {
            PairMessage::Ha(change) => {
                let error = if change.ha.is_some() {
                    Some("配对的模式与参数只在控制面落库".to_string())
                } else {
                    self.busy()
                };
                member.answer(change.id, error);
            }
            other => {
                member.receive(other, None).await;
            }
        }
    }

    async fn inventory(&self, id: u64) -> Inventory {
        let fleet = adopt::fleet_state(&self.data_dir());
        let rows = match &self.pair {
            Some(member) => member.local_rows(&fleet).await,
            None => adopt::local_rows(&self.services, &fleet, None).await,
        };
        Inventory::new(id, rows)
    }

    /// 落地期望状态：与控制面同步时本机版本更新的配对行先不动（[`Member::reconcile`]）。
    /// 控制面的「本机」按控制面的同步端落地（加入配对的本地行原地认下，[`Member::reconcile_local`]）。
    /// 交还给本机的行先认下或转成本地行（[`Reconciler::hand_back`]）
    pub async fn reconcile(&self, desired: DesiredState, reconciler: &mut Reconciler) -> Ack {
        reconciler.hand_back(desired.handback.as_deref());
        match &self.pair {
            Some(member) => member.reconcile(desired, reconciler).await,
            None if !self.enabled => match member_for(&self.services) {
                Some(member) => member.reconcile_local(desired, reconciler).await,
                None => reconciler.apply(desired).await,
            },
            None => reconciler.apply(desired).await,
        }
    }

    /// 离开控制面、被移除：同步账本删掉。控制面的「本机」节点不碰：同一个 `data/` 里的同步账本是控制面的
    pub fn forget_pair(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(holds) = &mut self.holds {
            holds.forget();
        }
        if let Some(member) = self.pair.take() {
            member.dissolve();
        }
        PairFile::forget(&outbox::path_in(&self.data_dir()));
    }

    /// 期望状态里的配对（落地房间之前）。本机是备机、这条连接上第一次收到配对时返回要先发的 `StandbyReport`；
    /// 上传主机换到本机时起主机、接上连接等控制面上报。
    /// 配对里的房间先按期望状态里的地址认下，免得落地之后、认下之前就开录的一段落到普通投稿流程
    pub async fn assign(
        &mut self,
        assignment: Option<HaAssignment>,
        rooms: &[DesiredRoom],
        link: &Link,
    ) -> Option<HaMessage> {
        if !self.enabled {
            return None;
        }
        let Some(assignment) = assignment else {
            self.assignment = None;
            if let Some(standby) = self.standby.take() {
                warn!("HA：控制面解除了配对，本机不再是备机");
                standby.retire();
                self.retired = Some(standby);
            }
            if let Some(primary) = self.primary.take() {
                warn!("HA：控制面解除了配对，本机不再是上传主机");
                primary.stop();
                set_role(None);
                park(&self.path, &self.controller, None);
            }
            return None;
        };
        for room in rooms {
            let entry = self
                .managed
                .entry(room.id)
                .or_insert_with(|| (room.spec.url.clone(), None));
            entry.1 = room.spec.override_cfg.clone();
        }
        let previous = self.assignment.replace(assignment.clone());
        if assignment.leader == Side::Node {
            if let (Some(primary), Some(previous)) = (&self.primary, &previous)
                && (previous.mode, previous.params) != (assignment.mode, assignment.params)
            {
                primary.configure(assignment.mode, assignment.params);
            }
            self.lead(Some(link)).await;
            return None;
        }
        if let Some(primary) = self.primary.take() {
            info!("HA：上传主机换回控制面，本机改当备机");
            primary.stop();
        }
        match &self.standby {
            Some(standby) => standby.assign(assignment),
            None => {
                let previous = load(&self.path).filter(|state| state.controller == self.controller);
                self.start(assignment, previous);
            }
        }
        let standby = self.standby.as_ref()?;
        standby.set_rooms(self.mirrored());
        (!standby.linked()).then(|| standby.link_up(link.clone()))
    }

    /// 配对里的房间：主播地址 → 房间
    fn mirrored(&self) -> HashMap<String, Mirrored> {
        let Some(assignment) = &self.assignment else {
            return HashMap::new();
        };
        assignment
            .rooms
            .iter()
            .filter_map(|id| {
                let (url, override_cfg) = self.managed.get(id)?;
                Some((url.clone(), Mirrored::new(*id, override_cfg.clone())))
            })
            .collect()
    }

    /// 本机当主机时钩子认的房间；在本机的配置下是边录边传的不算（§5.1）
    fn primary_rooms(&self) -> HashMap<String, i64> {
        let config = self.services.config.read().unwrap().clone();
        self.mirrored()
            .into_iter()
            .filter(|(_, room)| !sync_downloader(&config, room.override_cfg.clone()))
            .map(|(url, room)| (url, room.id))
            .collect()
    }

    /// 期望状态落地之后（房间的本地地址才齐）；不再交还的房间撤掉了才放开开录（[`NodeHolds::settle`]）
    pub fn set_rooms(&mut self, fleet: &FleetState) {
        if let Some(holds) = &mut self.holds {
            holds.settle(fleet);
        }
        self.managed = fleet
            .rooms
            .iter()
            .map(|(id, room)| {
                let override_cfg = room
                    .applied
                    .get("spec")
                    .and_then(|spec| spec.get("override"))
                    .filter(|value| !value.is_null())
                    .and_then(|value| serde_json::from_value(value.clone()).ok());
                (*id, (room.url.clone(), override_cfg))
            })
            .collect();
        if let Some(standby) = &self.standby {
            standby.set_rooms(self.mirrored());
        }
        if let Some(primary) = &self.primary {
            primary.set_rooms(self.primary_rooms());
        }
    }

    pub fn link_down(&self) {
        if let Some(standby) = &self.standby {
            standby.link_down();
        }
        if let Some(primary) = &self.primary
            && primary.linked()
        {
            primary.disconnected();
        }
        if let Some(member) = &self.pair {
            member.link_down();
        }
    }

    pub fn message(&self, message: HaMessage) {
        if let Some(standby) = &self.standby {
            standby.primary_message(message);
        } else if let Some(primary) = &self.primary {
            primary.standby_message(message);
        } else {
            debug!(kind = message.kind(), "HA frame while not paired");
        }
    }

    /// 节点代理停止
    pub fn shutdown(&mut self) {
        if let Some(member) = self.pair.take() {
            member.stop();
        }
        if let Some(standby) = self.standby.take() {
            set_role(None);
            standby.stop();
        }
        if let Some(primary) = self.primary.take() {
            set_role(None);
            primary.stop();
        }
        if self.retired.take().is_some() {
            set_role(None);
        }
    }
}

/// `ha-state.json` 里只换掉配对、场次留着（本机当主机时没有备机替它写）
fn park(path: &Path, controller: &str, assignment: Option<&HaAssignment>) {
    let mut state = load(path)
        .filter(|state| state.controller == controller)
        .unwrap_or_else(|| HaState::new(controller));
    if state.assignment.as_ref() == assignment {
        return;
    }
    state.assignment = assignment.cloned();
    if let Err(e) = save(path, &state) {
        warn!(error = ?e, "could not write {}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::guard::ManagedHandle;
    use crate::server::fleet::ha::upload::double::{self, Double};
    use crate::server::fleet::ha::wire::ReportedState;
    use crate::server::fleet::protocol::NodeMessage;
    use crate::server::fleet::reconcile::{self, Reconciler};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use async_trait::async_trait;
    use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
    use serde_json::json;
    use tokio::sync::mpsc;
    use tracing_subscriber::{EnvFilter, reload};

    const URL: &str = "https://stuck.example/7";
    const CONTROLLER: &str = "controller";

    /// 只认 `https://stuck.example/` 的平台；检测一直不返回，不会向任何真实平台发请求
    struct StuckPlatform;

    #[async_trait]
    impl LivePlugin for StuckPlatform {
        fn name(&self) -> &'static str {
            "stuck"
        }

        fn matches(&self, url: &str) -> bool {
            url.starts_with("https://stuck.example/")
        }

        async fn check_stream(&self, _request: LiveRequest) -> LiveResult<LiveStatus> {
            std::future::pending().await
        }
    }

    struct Fixture {
        dir: tempfile::TempDir,
        services: ServiceRegister,
        reconciler: Reconciler,
    }

    impl Fixture {
        /// 一台节点：期望状态里有房间 7（模板 1），还没有配对
        async fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let db = dir.path().join("data.sqlite3");
            let pool = ConnectionManager::new_pool(db.to_str().unwrap())
                .await
                .unwrap();
            let config = Config::default();
            let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
            managers.add_plugin(Arc::new(StuckPlatform)).await;
            let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
            let services =
                ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle)
                    .await;
            let node_file = dir.path().join("node.json");
            let mut reconciler = Reconciler::resume(
                reconcile::state_path(&node_file),
                CONTROLLER,
                "10.0.0.2".into(),
                false,
                false,
                services.clone(),
                ManagedHandle::default(),
            )
            .await;
            let desired: DesiredState = serde_json::from_value(json!({
                "version": 1,
                "rooms": [{ "id": 7, "epoch": 1, "template_id": 1, "url": URL, "remark": "房间7" }],
                "templates": [{ "id": 1, "template_name": "模板1", "title": "{title}", "tags": ["t"] }],
            }))
            .unwrap();
            let ack = reconciler.apply(desired).await;
            assert!(ack.failed.is_empty(), "{:?}", ack.failed);
            Fixture {
                dir,
                services,
                reconciler,
            }
        }

        fn node_file(&self) -> PathBuf {
            self.dir.path().join("node.json")
        }

        fn state_file(&self) -> PathBuf {
            state_path(&self.node_file())
        }

        async fn resume(&self, local: bool) -> NodeHa {
            NodeHa::resume(
                &self.node_file(),
                CONTROLLER,
                local,
                self.services.clone(),
                self.reconciler.state(),
            )
            .await
        }

        /// 本机的房间 7 开播：与投稿流程拿到的是同一种上下文
        async fn context(&self, started_at: i64) -> Context {
            let worker = self
                .services
                .managers
                .get_rooms()
                .await
                .into_iter()
                .find(|worker| worker.live_streamer.url == URL)
                .unwrap();
            let date = chrono::DateTime::from_timestamp_millis(started_at).unwrap();
            let stream = serde_json::from_value(json!({
                "name": "n", "url": URL, "title": "直播标题", "date": date.to_rfc3339(),
                "live_cover_url": "", "raw_stream_url": "http://127.0.0.1:9/x.flv?sign=secret",
                "platform": "stuck", "stream_headers": { "cookie": "secret" }, "suffix": "flv",
                "danmaku": null, "downloader_hint": "StreamGears", "runtime_options": null,
            }))
            .unwrap();
            Context::new(3, worker, self.services.pool.clone(), stream)
        }
    }

    fn assignment(mode: u8) -> HaAssignment {
        serde_json::from_value(json!({ "mode": mode, "primary": 1, "rooms": [7] })).unwrap()
    }

    fn segment(dir: &Path, name: &str, size: usize) -> SegmentInfo {
        let path = dir.join(name);
        std::fs::write(&path, vec![0u8; size]).unwrap();
        SegmentInfo::new(path, None, None, 0)
    }

    fn node_link() -> (Link, mpsc::UnboundedReceiver<NodeMessage>) {
        let (frames, receiver) = mpsc::unbounded_channel();
        (Link::Node(frames), receiver)
    }

    async fn next(frames: &mut mpsc::UnboundedReceiver<NodeMessage>) -> HaMessage {
        match tokio::time::timeout(Duration::from_secs(10), frames.recv())
            .await
            .expect("备机应该发出场次消息")
            .unwrap()
        {
            NodeMessage::Ha(message) => message,
            other => panic!("unexpected frame {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_node_that_is_not_a_standby_keeps_no_state() {
        let f = Fixture::new().await;
        let (link, mut frames) = node_link();

        let mut ha = f.resume(false).await;
        assert!(!ha.is_standby());
        ha.message(HaMessage::UploadFailed {
            key: "7:1".into(),
            room: 7,
            reason: "x".into(),
        });
        ha.link_down();
        assert_eq!(ha.assign(None, &[], &link).await, None);
        ha.set_rooms(f.reconciler.state());
        ha.shutdown();
        assert!(
            !f.state_file().exists(),
            "没被指定为备机就不建 ha-state.json"
        );

        // 控制面进程内嵌的「本机」节点是主机，期望状态里带了配对也不当备机
        let mut local = f.resume(true).await;
        assert_eq!(local.assign(Some(assignment(1)), &[], &link).await, None);
        assert!(!local.is_standby());
        assert!(!f.state_file().exists());
        assert!(frames.try_recv().is_err());
        // 同一个 `data/` 里的同步账本是控制面的，「本机」节点停下时不删
        let controller_outbox = outbox::path_in(f.dir.path());
        std::fs::write(&controller_outbox, "{}").unwrap();
        local.forget_pair();
        assert!(controller_outbox.exists());
    }

    /// 上传主机换到本机：停下备机、起主机（场次记在 `ha-primary.sqlite3`），`ha-state.json` 记着；
    /// 等控制面（这时是备机）上报之前不开录配对里的房间。重启照它恢复成主机；换回来时接着当备机；
    /// 解除配对后不再是主机；离开控制面时连主机的场次记录一起删
    #[tokio::test]
    async fn a_node_leads_after_a_switch_and_keeps_leading_across_restart() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let f = Fixture::new().await;
        let (link, _frames) = node_link();
        let leading: HaAssignment = serde_json::from_value(
            json!({ "mode": 1, "primary": 5, "rooms": [7], "leader": "node" }),
        )
        .unwrap();

        let mut ha = f.resume(false).await;
        assert!(ha.assign(Some(assignment(1)), &[], &link).await.is_some());
        assert!(ha.is_standby());
        assert_eq!(ha.assign(Some(leading.clone()), &[], &link).await, None);
        assert!(ha.is_primary());
        assert!(!ha.is_standby());
        assert!(crate::server::fleet::ha::standby().is_none());
        assert!(crate::server::fleet::ha::primary().is_some());
        assert_eq!(
            load(&f.state_file()).unwrap().assignment,
            Some(leading.clone())
        );
        assert!(f.dir.path().join(PRIMARY_DB_NAME).exists());
        let hold = crate::server::fleet::ha::hold_recording(URL).expect("等上报之前不开录");
        assert!(hold.reason.contains("上报"), "{}", hold.reason);
        ha.message(HaMessage::StandbyReport { sessions: vec![] });
        assert_eq!(crate::server::fleet::ha::hold_recording(URL), None);
        assert_eq!(crate::server::fleet::ha::busy(), None);
        ha.link_down();
        ha.shutdown();
        assert!(crate::server::fleet::ha::primary().is_none());

        // 重启：控制面连不上也照样当主机
        let mut ha = f.resume(false).await;
        assert!(ha.is_primary());
        assert!(crate::server::fleet::ha::primary().is_some());

        // 换回控制面：接着当备机，先上报
        let report = ha.assign(Some(assignment(1)), &[], &link).await;
        assert!(matches!(report, Some(HaMessage::StandbyReport { .. })));
        assert!(ha.is_standby());
        assert!(!ha.is_primary());
        assert_eq!(
            load(&f.state_file()).unwrap().assignment.unwrap().leader,
            Side::Controller
        );

        // 当主机时被解除配对
        assert_eq!(ha.assign(Some(leading), &[], &link).await, None);
        assert!(ha.is_primary());
        assert_eq!(ha.assign(None, &[], &link).await, None);
        assert!(!ha.is_primary());
        assert!(crate::server::fleet::ha::primary().is_none());
        assert_eq!(load(&f.state_file()).unwrap().assignment, None);
        ha.shutdown();

        forget(&f.state_file());
        assert!(!f.state_file().exists());
        assert!(!f.dir.path().join(PRIMARY_DB_NAME).exists());
    }

    #[tokio::test]
    async fn a_state_file_from_another_controller_is_dropped() {
        let f = Fixture::new().await;
        let stale = HaState {
            version: STATE_FILE_VERSION,
            controller: "another".into(),
            assignment: Some(assignment(1)),
            ..HaState::default()
        };
        save(&f.state_file(), &stale).unwrap();
        let ha = f.resume(false).await;
        assert!(!ha.is_standby());
        assert!(!f.state_file().exists());
    }

    /// 本地行录完、在上传池里排队时加入了配对：轮到投时这个地址已经是镜像房间，这一段照本地行投；
    /// 开始加入之后开录、又没认下的段照旧只留在本地
    #[tokio::test]
    async fn a_unit_recorded_before_its_row_joined_uploads_as_a_local_row() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let f = Fixture::new().await;
        let (link, _frames) = node_link();
        let started_at = now_ms() - 10 * 60_000;
        let queued = f.context(started_at).await;
        crate::server::fleet::ha::unit_started(&queued);
        let owner = crate::server::fleet::ha::member::identity(&f.services);
        adopt::hold(owner, URL, started_at + 60_000);

        let mut ha = f.resume(false).await;
        assert!(ha.assign(Some(assignment(1)), &[], &link).await.is_some());
        assert!(
            crate::server::fleet::ha::upload_plan(&queued)
                .await
                .is_none(),
            "开录时还是本地行：照常投"
        );
        let unknown = f.context(started_at + 120_000).await;
        assert!(
            matches!(
                crate::server::fleet::ha::upload_plan(&unknown).await,
                Some(Plan::Keep { .. })
            ),
            "开始加入之后开录、没认下的段只留在本地"
        );

        adopt::release_all(owner);
        assert!(
            matches!(
                crate::server::fleet::ha::upload_plan(&queued).await,
                Some(Plan::Keep { .. })
            ),
            "同步端停下时加入记录一起清掉"
        );
        ha.shutdown();
    }

    /// 备机走真实的录制钩子收下分段；主机投稿失败后按场次记下的开播信息重建上下文、经测试替身投出，
    /// `ha-state.json` 记着结果；重启后照它恢复并先上报；解除配对后不再是备机，场次留着
    #[tokio::test]
    async fn a_standby_collects_uploads_after_the_primary_fails_and_survives_restart() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let f = Fixture::new().await;
        let control = f.dir.path().join("control");
        std::fs::create_dir_all(&control).unwrap();
        let recording = f.dir.path().join("rec");
        std::fs::create_dir_all(&recording).unwrap();
        let double = Arc::new(Double::new(
            f.dir.path().join("double.jsonl"),
            control,
            0,
            "S",
        ));
        double::install(Some(double.clone()));

        let mut ha = f.resume(false).await;
        let (link, mut frames) = node_link();
        let report = ha.assign(Some(assignment(1)), &[], &link).await;
        assert!(
            matches!(&report, Some(HaMessage::StandbyReport { sessions }) if sessions.is_empty())
        );
        assert!(ha.is_standby());
        assert_eq!(
            load(&f.state_file()).unwrap().assignment,
            Some(assignment(1))
        );
        assert_eq!(
            ha.assign(Some(assignment(1)), &[], &link).await,
            None,
            "同一条连接只上报一次"
        );
        assert_eq!(crate::server::fleet::ha::hold_recording(URL), None);

        let started_at = now_ms() - 20 * 60_000;
        let primary = key::primary_key(7, started_at);
        ha.message(HaMessage::SessionStarted {
            key: primary.clone(),
            room: 7,
            started_at,
            at: started_at,
        });
        let ctx = f.context(started_at).await;
        crate::server::fleet::ha::unit_started(&ctx);
        let plan = crate::server::fleet::ha::upload_plan(&ctx)
            .await
            .expect("镜像过来的房间走 HA 投稿");
        let config = ctx.upload_config().clone().unwrap();
        let segments = vec![
            segment(&recording, "a-part1.flv", 3000),
            segment(&recording, "a-part2.flv", 2000),
        ];
        plan.run(futures::stream::iter(segments), &ctx, &config)
            .await
            .unwrap();
        crate::server::fleet::ha::unit_ended(&ctx, UnitOutput { seen: 2, sent: 2 });
        assert!(
            matches!(next(&mut frames).await, HaMessage::SessionStarted { key, .. } if key == primary),
            "开录时就用上主机的键"
        );
        assert!(matches!(
            next(&mut frames).await,
            HaMessage::SessionEnded { key, produced: true, .. } if key == primary
        ));
        assert!(double.entries().is_empty(), "主机还没结果，备机只收着");

        ha.message(HaMessage::UploadFailed {
            key: primary.clone(),
            room: 7,
            reason: "boom".into(),
        });
        assert!(matches!(
            next(&mut frames).await,
            HaMessage::UploadStarted { key, .. } if key == primary
        ));
        assert!(matches!(
            next(&mut frames).await,
            HaMessage::Uploaded { key, bvid, from, .. } if key == primary && bvid == "BVS0001" && from == started_at
        ));
        let submit = double.entries().pop().unwrap();
        assert_eq!(submit["op"], "submit");
        assert_eq!(submit["parts"], json!(["a-part1", "a-part2"]));

        let saved = load(&f.state_file()).unwrap();
        let [session] = saved.sessions.as_slice() else {
            panic!("{:?}", saved.sessions);
        };
        assert_eq!(session.key, primary);
        assert_eq!(session.state, State::Uploaded);
        assert_eq!(session.bvid.as_deref(), Some("BVS0001"));
        assert_eq!(session.unit.segments.len(), 2);
        let stream = session.unit.stream.as_ref().unwrap();
        assert_eq!(stream["raw_stream_url"], "", "直链不落盘");
        assert_eq!(stream["stream_headers"], json!({}));

        // 节点代理重启：按 ha-state.json 恢复，新连接上先上报投成的一场
        ha.shutdown();
        assert!(crate::server::fleet::ha::upload_plan(&ctx).await.is_none());
        let mut ha = f.resume(false).await;
        assert!(ha.is_standby());
        let (link, _frames) = node_link();
        let Some(HaMessage::StandbyReport { sessions }) =
            ha.assign(Some(assignment(1)), &[], &link).await
        else {
            panic!("重连后应先上报");
        };
        let [reported] = sessions.as_slice() else {
            panic!("{sessions:?}");
        };
        assert_eq!(reported.key, primary);
        assert_eq!(reported.state, ReportedState::Uploaded);
        assert_eq!(reported.bvid.as_deref(), Some("BVS0001"));

        // 改成模式 2：主机在线时备机只监控不录
        assert_eq!(ha.assign(Some(assignment(2)), &[], &link).await, None);
        assert_eq!(
            load(&f.state_file()).unwrap().assignment.unwrap().mode,
            HaMode::Takeover
        );
        assert!(crate::server::fleet::ha::hold_recording(URL).is_some());

        // 重启前开录的那一段新进程不认识：镜像房间的段不落到普通投稿流程，只留在本地
        assert!(matches!(
            crate::server::fleet::ha::upload_plan(&ctx).await,
            Some(Plan::Keep { .. })
        ));
        let in_flight = f.context(now_ms() - 60_000).await;
        crate::server::fleet::ha::unit_started(&in_flight);
        assert!(matches!(
            crate::server::fleet::ha::upload_plan(&in_flight).await,
            Some(Plan::Collect { .. })
        ));

        // 解除配对：不再是备机，场次留在文件里备查；录到一半的那段不投（主机投）
        let submitted = double.entries().len();
        assert_eq!(ha.assign(None, &[], &link).await, None);
        assert!(!ha.is_standby());
        assert!(crate::server::fleet::ha::standby().is_none());
        assert_eq!(crate::server::fleet::ha::hold_recording(URL), None);
        assert!(matches!(
            crate::server::fleet::ha::upload_plan(&in_flight).await,
            Some(Plan::Keep { .. })
        ));
        let later = f.context(now_ms()).await;
        crate::server::fleet::ha::unit_started(&later);
        assert!(
            crate::server::fleet::ha::upload_plan(&later)
                .await
                .is_none(),
            "解除之后开录的段照常投"
        );
        let saved = load(&f.state_file()).unwrap();
        assert_eq!(saved.assignment, None);
        assert_eq!(saved.sessions.len(), 2);
        assert_eq!(double.entries().len(), submitted, "解除之后没有再投");
        ha.shutdown();
        assert!(
            crate::server::fleet::ha::upload_plan(&in_flight)
                .await
                .is_none()
        );
        assert!(!f.resume(false).await.is_standby());
        double::install(None);
    }

    /// 模式 2：主机在线时备机只监控；主机断开超过 offline_grace，备机接手它正在录的一场，录完等主机；
    /// 主机回来先投它那半，备机把自己这半追加为那个稿件的后续分 P（经测试替身，不另建稿件）
    #[tokio::test]
    async fn a_mode_two_takeover_is_appended_to_the_primary_half() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let f = Fixture::new().await;
        let control = f.dir.path().join("control");
        std::fs::create_dir_all(&control).unwrap();
        let recording = f.dir.path().join("rec");
        std::fs::create_dir_all(&recording).unwrap();
        let double = Arc::new(Double::new(
            f.dir.path().join("double.jsonl"),
            control,
            0,
            "S",
        ));
        double::install(Some(double.clone()));
        let assignment: HaAssignment = serde_json::from_value(json!({
            "mode": 2, "primary": 1, "rooms": [7], "params": { "offline_grace": 1 },
        }))
        .unwrap();

        let mut ha = f.resume(false).await;
        let (link, _frames) = node_link();
        assert!(
            ha.assign(Some(assignment.clone()), &[], &link)
                .await
                .is_some()
        );
        let primary_start = now_ms() - 30 * 60_000;
        let primary = key::primary_key(7, primary_start);
        ha.message(HaMessage::SessionStarted {
            key: primary.clone(),
            room: 7,
            started_at: primary_start,
            at: primary_start,
        });
        assert!(
            crate::server::fleet::ha::hold_recording(URL).is_some(),
            "主机在线：只监控不录"
        );

        ha.link_down();
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert_eq!(
            crate::server::fleet::ha::hold_recording(URL),
            None,
            "主机离线超过 offline_grace：接手"
        );
        let started_at = now_ms();
        let ctx = f.context(started_at).await;
        crate::server::fleet::ha::unit_started(&ctx);
        let plan = crate::server::fleet::ha::upload_plan(&ctx).await.unwrap();
        assert!(matches!(plan, Plan::Collect { .. }));
        let config = ctx.upload_config().clone().unwrap();
        let segments = vec![
            segment(&recording, "b-part1.flv", 1000),
            segment(&recording, "b-part2.flv", 1000),
        ];
        plan.run(futures::stream::iter(segments), &ctx, &config)
            .await
            .unwrap();
        crate::server::fleet::ha::unit_ended(&ctx, UnitOutput { seen: 2, sent: 2 });
        let standby = crate::server::fleet::ha::standby().unwrap();
        let view = standby.view();
        assert_eq!(view["primary_offline"], true);
        assert_eq!(view["sessions"][0]["kind"], "takeover");
        assert_eq!(view["sessions"][0]["state"], "awaiting_primary");
        assert_eq!(view["sessions"][0]["takeover_of"], primary.as_str());
        assert!(double.entries().is_empty(), "接手的一场先不投");

        // 主机回来：先上报，再收到主机投成它那半
        let (link, mut frames) = node_link();
        let Some(HaMessage::StandbyReport { sessions }) =
            ha.assign(Some(assignment), &[], &link).await
        else {
            panic!("重连后应先上报");
        };
        assert_eq!(sessions[0].state, ReportedState::AwaitingPrimary);
        assert_eq!(sessions[0].takeover_of.as_deref(), Some(primary.as_str()));
        ha.message(HaMessage::Uploaded {
            key: primary.clone(),
            room: 7,
            bvid: "BVP0001".into(),
            from: primary_start,
            to: Some(started_at - 5 * 60_000),
            yielded: false,
        });
        let id = key::standby_key(7, started_at);
        loop {
            match next(&mut frames).await {
                HaMessage::Uploaded { key, bvid, .. } => {
                    assert_eq!((key.as_str(), bvid.as_str()), (id.as_str(), "BVP0001"));
                    break;
                }
                HaMessage::UploadStarted { key, .. } => assert_eq!(key, id),
                other => panic!("{other:?}"),
            }
        }
        let entries = double.entries();
        assert!(
            !entries.iter().any(|entry| entry["op"] == "submit"),
            "不另建稿件"
        );
        let append = entries.last().unwrap();
        assert_eq!(append["op"], "append");
        assert_eq!(append["bvid"], "BVP0001");
        assert_eq!(append["parts"], json!(["b-part1", "b-part2"]));
        let saved = load(&f.state_file()).unwrap();
        assert_eq!(saved.sessions[0].state, State::Appended);
        assert_eq!(saved.sessions[0].bvid.as_deref(), Some("BVP0001"));
        ha.shutdown();
        double::install(None);
    }

    /// 镜像房间在落地之前就按期望状态认下；在本机配置下是边录边传的不录
    #[tokio::test]
    async fn mirrored_rooms_are_known_before_they_land_and_sync_rooms_are_not_recorded() {
        let _guard = crate::server::fleet::ha::test_guard().await;
        let f = Fixture::new().await;
        let mut ha = f.resume(false).await;
        let (link, _frames) = node_link();
        let desired = |id: i64, url: &str, downloader: Option<&str>| -> DesiredRoom {
            serde_json::from_value(json!({
                "id": id, "epoch": 1, "template_id": 1, "url": url, "remark": "r",
                "override": downloader.map(|downloader| json!({ "downloader": downloader })),
            }))
            .unwrap()
        };
        let assignment: HaAssignment =
            serde_json::from_value(json!({ "mode": 1, "primary": 1, "rooms": [7, 8] })).unwrap();
        let rooms = [
            desired(7, URL, None),
            desired(8, "https://stuck.example/8", None),
        ];
        assert!(
            ha.assign(Some(assignment.clone()), &rooms, &link)
                .await
                .is_some()
        );
        let standby = crate::server::fleet::ha::standby().unwrap();
        assert!(
            standby.mirrors("https://stuck.example/8"),
            "还没落地的房间也认下"
        );
        assert_eq!(crate::server::fleet::ha::hold_recording(URL), None);

        let rooms = [
            desired(7, URL, Some("sync-downloader")),
            desired(8, "https://stuck.example/8", None),
        ];
        assert_eq!(ha.assign(Some(assignment), &rooms, &link).await, None);
        let hold = crate::server::fleet::ha::hold_recording(URL).expect("边录边传的镜像房间不录");
        assert!(hold.reason.contains("边录边传"), "{}", hold.reason);
        assert_eq!(
            crate::server::fleet::ha::hold_recording("https://stuck.example/8"),
            None
        );

        let view = standby.view();
        assert_eq!(view["mode"], 1);
        assert_eq!(view["rooms"], json!([7, 8]));
        assert_eq!(view["linked"], true);
        assert_eq!(view["sessions"], json!([]));
        assert!(
            standby
                .manual("7:1", ManualAction::Drop)
                .unwrap_err()
                .contains("没有场次")
        );
        ha.shutdown();
        assert!(crate::server::fleet::ha::standby().is_none());
    }
}
