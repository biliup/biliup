//! 节点进程里的备机：把 [`StandbyCore`] 接到录制钩子、`data/ha-state.json` 与控制通道上。
//!
//! 节点代理收到带 `ha` 的期望状态才成为备机（[`NodeHa::assign`]），这时才建 `ha-state.json`；
//! 之后重启时按它恢复，控制面连不上也照常录、照常按规则投。没被指定为备机的节点这里什么都不做。
//!
//! 备机录的场次只把分段收下（[`super::upload::Plan::Collect`]），要投时另起任务、占上传池的一个槽位，
//! 按场次里记下的开播信息重建上下文再投（[`super::upload::run_standby`]）。

use super::key;
use super::standby::{Out, PrimaryView, SavedSegment, Session, StandbyCore, UnitData};
use super::upload::{self, Plan, Submitted};
use super::wire::{HaAssignment, HaMessage};
use super::{Hold, Role, Unit, UnitOutput, set_role};
use crate::server::common::upload::execute_postprocessor;
use crate::server::core::downloader::SegmentInfo;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::events;
use crate::server::fleet::now_ms;
use crate::server::fleet::reconcile::FleetState;
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::service_register::ServiceRegister;
use biliup::downloader::live::LiveStream;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

pub const STATE_FILE_NAME: &str = "ha-state.json";
const STATE_FILE_VERSION: u32 = 1;
const TICK: Duration = Duration::from_secs(1);

pub fn state_path(node_file: &Path) -> PathBuf {
    node_file.with_file_name(STATE_FILE_NAME)
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

/// 离开控制面（`biliup node leave`）时删掉
pub fn forget(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => info!("{} removed", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(error = %e, "could not remove {}", path.display()),
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

/// 被指定为备机的节点进程里的备机
pub struct Standby {
    core: Mutex<StandbyCore>,
    path: PathBuf,
    controller: String,
    assignment: Mutex<HaAssignment>,
    services: ServiceRegister,
    /// 镜像过来的房间：主播地址 → 控制面房间 id
    rooms: RwLock<HashMap<String, i64>>,
    link: Mutex<Option<mpsc::UnboundedSender<HaMessage>>>,
    /// 正在投的场次
    uploading: Mutex<HashSet<String>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Standby {
    fn start(
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

    /// 解除配对或节点代理停止：停掉计时与在投的任务，场次留在 `ha-state.json`
    fn stop(&self, dissolved: bool) {
        for task in self.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        *self.link.lock().unwrap() = None;
        if dissolved {
            let core = self.core.lock().unwrap();
            let state = HaState {
                assignment: None,
                ..self.state(&core)
            };
            if let Err(e) = save(&self.path, &state) {
                warn!(error = ?e, "could not write {}", self.path.display());
            }
        }
    }

    fn state(&self, core: &StandbyCore) -> HaState {
        HaState {
            version: STATE_FILE_VERSION,
            controller: self.controller.clone(),
            assignment: Some(self.assignment.lock().unwrap().clone()),
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
        for out in core.take() {
            match out {
                Out::Send(message) => {
                    if let Some(link) = self.link.lock().unwrap().as_ref() {
                        let _ = link.send(message);
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

    fn assign(self: &Arc<Self>, assignment: HaAssignment) {
        *self.assignment.lock().unwrap() = assignment.clone();
        self.update(|core, _| core.assign(&assignment));
        self.persist(&self.core.lock().unwrap(), true);
    }

    fn set_rooms(&self, rooms: HashMap<String, i64>) {
        *self.rooms.write().unwrap() = rooms;
    }

    fn room_of(&self, url: &str) -> Option<i64> {
        self.rooms.read().unwrap().get(url).copied()
    }

    /// 这条连接上第一次收到配对：接上连接，返回要先发的 `StandbyReport`
    fn link_up(self: &Arc<Self>, link: mpsc::UnboundedSender<HaMessage>) -> HaMessage {
        *self.link.lock().unwrap() = Some(link);
        info!("HA：连上主机，先上报手里的场次");
        self.update(|core, now| core.link_up(now))
    }

    fn link_down(self: &Arc<Self>) {
        if self.link.lock().unwrap().take().is_some() {
            warn!("HA：与主机（控制面）断开");
        }
        self.update(|core, now| core.link_down(now));
    }

    fn linked(&self) -> bool {
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
        self.room_of(url).is_some()
    }

    pub(crate) fn hold(&self, url: &str) -> Option<Hold> {
        let room = self.room_of(url)?;
        self.core.lock().unwrap().hold(now_ms(), room)
    }

    fn id_of(&self, unit: &Unit) -> Option<(i64, String)> {
        let room = self.room_of(&unit.url)?;
        Some((room, key::standby_key(room, unit.started_at)))
    }

    pub(crate) fn unit_started(self: &Arc<Self>, unit: &Unit, ctx: &Context) {
        let Some((room, id)) = self.id_of(unit) else {
            return;
        };
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
        let Some((_, id)) = self.id_of(unit) else {
            return;
        };
        self.update(|core, now| core.unit_ended(now, &id, output.sent > 0));
    }

    pub(crate) fn plan(self: &Arc<Self>, unit: &Unit) -> Option<Plan> {
        let (_, id) = self.id_of(unit)?;
        self.core
            .lock()
            .unwrap()
            .session(&id)
            .is_some()
            .then(|| Plan::Collect {
                standby: self.clone(),
                id,
            })
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
        self.spawn(id, async move { standby.run_upload(job_id).await });
    }

    fn spawn_append(self: &Arc<Self>, id: String, bvid: String) {
        let standby = self.clone();
        let job_id = id.clone();
        self.spawn(id, async move {
            standby.update(|core, now| {
                core.require_manual(
                    now,
                    &job_id,
                    format!("主机已投成它那半（{bvid}），本版本的备机还不能追加分 P"),
                )
            });
        });
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

    async fn run_upload(self: Arc<Self>, id: String) {
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
        match upload::run_standby(&self, &id, &ctx, segments).await {
            Ok(Submitted::Done { bvid, paths }) => {
                self.update(|core, now| core.uploaded(now, &id, &bvid));
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

/// 节点代理持有的一主一备入口。没有被指定为备机时什么都不做，也不建 `ha-state.json`
pub struct NodeHa {
    path: PathBuf,
    controller: String,
    services: ServiceRegister,
    /// 控制面进程内嵌的「本机」节点就是主机，不会是备机
    enabled: bool,
    standby: Option<Arc<Standby>>,
    /// 期望状态里的房间：控制面房间 id → 主播地址
    managed: HashMap<i64, String>,
}

impl NodeHa {
    /// 节点代理启动：上次是备机就按 `ha-state.json` 恢复
    pub fn resume(
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
            managed: HashMap::new(),
        };
        if !ha.enabled {
            return ha;
        }
        match load(&ha.path) {
            Some(state) if state.controller != controller => {
                warn!("{} belongs to another controller", ha.path.display());
                forget(&ha.path);
            }
            Some(state) => {
                if let Some(assignment) = state.assignment.clone() {
                    ha.start(assignment, Some(state));
                }
            }
            None => {}
        }
        ha.set_rooms(fleet);
        ha
    }

    fn start(&mut self, assignment: HaAssignment, previous: Option<HaState>) {
        let standby = Standby::start(
            self.path.clone(),
            &self.controller,
            assignment,
            previous,
            self.services.clone(),
        );
        set_role(Some(Role::Standby(standby.clone())));
        self.standby = Some(standby);
    }

    pub fn is_standby(&self) -> bool {
        self.standby.is_some()
    }

    /// 期望状态里的配对。这条连接上第一次收到配对时返回要先发的 `StandbyReport`
    pub fn assign(
        &mut self,
        assignment: Option<HaAssignment>,
        link: &mpsc::UnboundedSender<HaMessage>,
    ) -> Option<HaMessage> {
        if !self.enabled {
            return None;
        }
        let Some(assignment) = assignment else {
            if let Some(standby) = self.standby.take() {
                warn!("HA：控制面解除了配对，本机不再是备机");
                set_role(None);
                standby.stop(true);
            }
            return None;
        };
        match &self.standby {
            Some(standby) => standby.assign(assignment),
            None => {
                let previous = load(&self.path).filter(|state| state.controller == self.controller);
                self.start(assignment, previous);
            }
        }
        let standby = self.standby.as_ref()?;
        let rooms = self.mirrored();
        standby.set_rooms(rooms);
        (!standby.linked()).then(|| standby.link_up(link.clone()))
    }

    fn mirrored(&self) -> HashMap<String, i64> {
        let Some(standby) = &self.standby else {
            return HashMap::new();
        };
        let wanted = standby.assignment.lock().unwrap().rooms.clone();
        wanted
            .into_iter()
            .filter_map(|id| self.managed.get(&id).map(|url| (url.clone(), id)))
            .collect()
    }

    /// 期望状态落地之后（房间的本地地址才齐）
    pub fn set_rooms(&mut self, fleet: &FleetState) {
        self.managed = fleet
            .rooms
            .iter()
            .map(|(id, room)| (*id, room.url.clone()))
            .collect();
        let rooms = self.mirrored();
        if let Some(standby) = &self.standby {
            standby.set_rooms(rooms);
        }
    }

    pub fn link_down(&self) {
        if let Some(standby) = &self.standby {
            standby.link_down();
        }
    }

    pub fn message(&self, message: HaMessage) {
        match &self.standby {
            Some(standby) => standby.primary_message(message),
            None => debug!(kind = message.kind(), "HA frame while not a standby"),
        }
    }

    /// 节点代理停止
    pub fn shutdown(&mut self) {
        if let Some(standby) = self.standby.take() {
            set_role(None);
            standby.stop(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::guard::ManagedHandle;
    use crate::server::fleet::ha::params::HaMode;
    use crate::server::fleet::ha::standby::State;
    use crate::server::fleet::ha::upload::double::{self, Double};
    use crate::server::fleet::ha::wire::ReportedState;
    use crate::server::fleet::protocol::DesiredState;
    use crate::server::fleet::reconcile::{self, Reconciler};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use async_trait::async_trait;
    use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
    use serde_json::json;
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

        fn resume(&self, local: bool) -> NodeHa {
            NodeHa::resume(
                &self.node_file(),
                CONTROLLER,
                local,
                self.services.clone(),
                self.reconciler.state(),
            )
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

    async fn next(frames: &mut mpsc::UnboundedReceiver<HaMessage>) -> HaMessage {
        tokio::time::timeout(Duration::from_secs(10), frames.recv())
            .await
            .expect("备机应该发出场次消息")
            .unwrap()
    }

    #[tokio::test]
    async fn a_node_that_is_not_a_standby_keeps_no_state() {
        let f = Fixture::new().await;
        let (link, mut frames) = mpsc::unbounded_channel();

        let mut ha = f.resume(false);
        assert!(!ha.is_standby());
        ha.message(HaMessage::UploadFailed {
            key: "7:1".into(),
            room: 7,
            reason: "x".into(),
        });
        ha.link_down();
        assert_eq!(ha.assign(None, &link), None);
        ha.set_rooms(f.reconciler.state());
        ha.shutdown();
        assert!(
            !f.state_file().exists(),
            "没被指定为备机就不建 ha-state.json"
        );

        // 控制面进程内嵌的「本机」节点是主机，期望状态里带了配对也不当备机
        let mut local = f.resume(true);
        assert_eq!(local.assign(Some(assignment(1)), &link), None);
        assert!(!local.is_standby());
        assert!(!f.state_file().exists());
        assert!(frames.try_recv().is_err());
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
        let ha = f.resume(false);
        assert!(!ha.is_standby());
        assert!(!f.state_file().exists());
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

        let mut ha = f.resume(false);
        let (link, mut frames) = mpsc::unbounded_channel();
        let report = ha.assign(Some(assignment(1)), &link);
        assert!(
            matches!(&report, Some(HaMessage::StandbyReport { sessions }) if sessions.is_empty())
        );
        assert!(ha.is_standby());
        assert_eq!(
            load(&f.state_file()).unwrap().assignment,
            Some(assignment(1))
        );
        assert_eq!(
            ha.assign(Some(assignment(1)), &link),
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
        let mut ha = f.resume(false);
        assert!(ha.is_standby());
        let (link, _frames) = mpsc::unbounded_channel();
        let Some(HaMessage::StandbyReport { sessions }) = ha.assign(Some(assignment(1)), &link)
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
        assert_eq!(ha.assign(Some(assignment(2)), &link), None);
        assert_eq!(
            load(&f.state_file()).unwrap().assignment.unwrap().mode,
            HaMode::Takeover
        );
        assert!(crate::server::fleet::ha::hold_recording(URL).is_some());

        // 解除配对：不再是备机，场次留在文件里备查
        assert_eq!(ha.assign(None, &link), None);
        assert!(!ha.is_standby());
        assert_eq!(crate::server::fleet::ha::hold_recording(URL), None);
        let saved = load(&f.state_file()).unwrap();
        assert_eq!(saved.assignment, None);
        assert_eq!(saved.sessions.len(), 1);
        assert!(!f.resume(false).is_standby());
        double::install(None);
    }
}
