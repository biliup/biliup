//! 节点端的对账：把控制面下发的期望状态落进本机 `livestreamers` / `uploadstreamers`。
//!
//! 只动自己按控制面建的那些行（映射记在 `data/fleet-state.json`），本机自己加的主播与模板一概不碰。
//! 控制面连不上时照常按本机数据库录；重启后按 `fleet-state.json` 认回托管行、恢复暂停状态。
//! 离开或被移除时删掉 `fleet-state.json`，托管行就地变成本地行，录制不中断。

use super::accounts;
use super::guard::{Managed, ManagedHandle};
use super::model::{DesiredRoom, RoomSpec, TemplateSpec};
use super::protocol::{Ack, DesiredState, FailedRoom, HeldRoom};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::models::upload_streamer::InsertUploadStreamer;
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::services::streamers::{
    AddStreamerError, add_streamer, delete_streamer, toggle_pause, update_streamer,
};
use error_stack::ResultExt;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub const STATE_FILE_NAME: &str = "fleet-state.json";
const STATE_FILE_VERSION: u32 = 1;

/// `data/fleet-state.json`：托管行的映射与最近一次落地的期望状态
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FleetState {
    pub version: u32,
    /// 控制面 EndpointId；与 `node.json` 对不上时整份作废
    pub controller: String,
    /// 最近一次落地的期望状态版本号
    #[serde(default)]
    pub state_version: Option<u64>,
    /// 控制面房间 id → 本地主播
    #[serde(default)]
    pub rooms: BTreeMap<i64, ManagedRoom>,
    /// 控制面模板 id → 本地模板
    #[serde(default)]
    pub templates: BTreeMap<i64, ManagedTemplate>,
    /// 正在新增、还没记下本地 id 的房间（控制面房间 id → 地址）。进程在这一步中途退出时，下次按地址认领
    #[serde(default)]
    pub adding: BTreeMap<i64, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagedRoom {
    pub local_id: i64,
    pub epoch: i64,
    pub url: String,
    #[serde(default)]
    pub paused: bool,
    /// 最近一次落地的设置（含本地模板 id），用来判断要不要重建监控
    #[serde(default)]
    pub applied: Value,
    /// 最近一次落地失败的原因；失败的房间不算持有
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagedTemplate {
    pub local_id: i64,
    #[serde(default)]
    pub applied: Value,
}

impl FleetState {
    fn new(controller: &str) -> Self {
        FleetState {
            version: STATE_FILE_VERSION,
            controller: controller.to_string(),
            ..FleetState::default()
        }
    }

    pub fn held(&self) -> Vec<HeldRoom> {
        self.rooms
            .iter()
            .filter(|(_, room)| room.error.is_none())
            .map(|(id, room)| HeldRoom {
                id: *id,
                epoch: room.epoch,
            })
            .collect()
    }
}

pub fn state_path(node_file: &Path) -> PathBuf {
    node_file.with_file_name(STATE_FILE_NAME)
}

fn load(path: &Path) -> Option<FleetState> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<FleetState>(&text) {
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

fn save(path: &Path, state: &FleetState) -> AppResult<()> {
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

/// 删掉 `fleet-state.json`：托管行从此是本地行
pub fn forget(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => info!(
            "{} removed, rooms that were managed by the fleet controller are local rooms now",
            path.display()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(error = %e, "could not remove {}", path.display()),
    }
}

fn local_streamer(spec: &RoomSpec, template: Option<i64>, id: Option<i64>) -> Option<Value> {
    let mut value = serde_json::to_value(spec).ok()?;
    let object = value.as_object_mut()?;
    object.insert("upload_streamers_id".into(), json!(template));
    if let Some(id) = id {
        object.insert("id".into(), json!(id));
    }
    Some(value)
}

fn local_template(spec: &TemplateSpec, cookie: Option<&str>, id: Option<i64>) -> Option<Value> {
    let mut value = serde_json::to_value(spec).ok()?;
    let object = value.as_object_mut()?;
    object.remove("account_mid");
    object.insert("user_cookie".into(), json!(cookie));
    object.insert("id".into(), json!(id));
    Some(value)
}

async fn local_id_by_url(services: &ServiceRegister, url: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT id FROM livestreamers WHERE url = ?")
        .bind(url)
        .fetch_optional(&services.pool)
        .await
        .ok()
        .flatten()
}

async fn row_exists(services: &ServiceRegister, table: &str, id: i64) -> bool {
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?");
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(id)
        .fetch_optional(&services.pool)
        .await
        .ok()
        .flatten()
        .is_some()
}

/// 让本地主播的暂停状态与期望一致。暂停只在内存里，重启后要重新套一遍。
async fn apply_paused(services: &ServiceRegister, local_id: i64, paused: bool) {
    let Some(worker) = services.managers.get_room_by_id(local_id).await else {
        return;
    };
    let is_paused = matches!(
        *worker.downloader_status.read().unwrap(),
        WorkerStatus::Pause
    );
    if is_paused != paused {
        toggle_pause(&services.managers, local_id).await;
    }
}

/// 节点代理持有的对账器
pub struct Reconciler {
    path: PathBuf,
    services: ServiceRegister,
    allow_hooks: bool,
    label: String,
    managed: ManagedHandle,
    state: FleetState,
}

impl Reconciler {
    /// 节点代理启动时：读 `fleet-state.json`，认回还在本机库里的托管行并恢复暂停状态。
    /// 文件属于别的控制面（重新 join 过）时作废，那些行留作本地行。
    pub async fn resume(
        path: PathBuf,
        controller: &str,
        label: String,
        allow_hooks: bool,
        services: ServiceRegister,
        managed: ManagedHandle,
    ) -> Self {
        let state = match load(&path) {
            Some(state) if state.controller == controller => state,
            Some(_) => {
                warn!("{} belongs to another controller", path.display());
                forget(&path);
                FleetState::new(controller)
            }
            None => FleetState::new(controller),
        };
        let mut reconciler = Reconciler {
            path,
            services,
            allow_hooks,
            label,
            managed,
            state,
        };
        reconciler.recover().await;
        reconciler
    }

    async fn recover(&mut self) {
        let mut changed = false;
        let rooms: Vec<(i64, ManagedRoom)> = self
            .state
            .rooms
            .iter()
            .map(|(id, room)| (*id, room.clone()))
            .collect();
        for (id, room) in rooms {
            if !row_exists(&self.services, "livestreamers", room.local_id).await {
                warn!(room = id, local = room.local_id, "managed streamer is gone");
                self.state.rooms.remove(&id);
                changed = true;
                continue;
            }
            if room.paused {
                apply_paused(&self.services, room.local_id, true).await;
            }
        }
        let templates: Vec<(i64, i64)> = self
            .state
            .templates
            .iter()
            .map(|(id, template)| (*id, template.local_id))
            .collect();
        for (id, local_id) in templates {
            if !row_exists(&self.services, "uploadstreamers", local_id).await {
                self.state.templates.remove(&id);
                changed = true;
            }
        }
        if changed {
            self.persist();
        }
        self.publish();
    }

    pub fn held(&self) -> Vec<HeldRoom> {
        self.state.held()
    }

    pub fn state_version(&self) -> Option<u64> {
        self.state.state_version
    }

    pub fn state(&self) -> &FleetState {
        &self.state
    }

    fn persist(&self) {
        if let Err(e) = save(&self.path, &self.state) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    fn publish(&self) {
        let managed = Managed {
            controller: self.label.clone(),
            streamers: self
                .state
                .rooms
                .values()
                .map(|room| (room.local_id, room.url.clone()))
                .collect(),
            templates: self
                .state
                .templates
                .values()
                .map(|template| template.local_id)
                .collect(),
        };
        *self.managed.write().unwrap() = Some(managed);
    }

    /// 离开或被移除：托管行转成本地行，接着录
    pub fn release(&mut self) {
        forget(&self.path);
        self.state = FleetState::new(&self.state.controller);
        *self.managed.write().unwrap() = None;
    }

    /// 落地一份期望状态，返回给控制面的 `Ack`
    pub async fn apply(&mut self, desired: DesiredState) -> Ack {
        let local_accounts = accounts::scan(&self.services.pool).await;
        let mut errors: BTreeMap<i64, String> = BTreeMap::new();

        // 模板：按 mid 在本机凭据里找 user_cookie
        let mut template_errors: HashMap<i64, String> = HashMap::new();
        let mut resolved: HashMap<i64, (TemplateSpec, Option<String>)> = HashMap::new();
        for template in desired.templates {
            match template.spec.account_mid {
                Some(mid) => match local_accounts.iter().find(|account| account.mid == mid) {
                    Some(account) => {
                        resolved.insert(template.id, (template.spec, Some(account.path.clone())));
                    }
                    None => {
                        template_errors.insert(
                            template.id,
                            format!(
                                "本机没有登记 B 站账号 {mid}，投稿模板「{}」用不了",
                                template.spec.template_name
                            ),
                        );
                    }
                },
                None => {
                    resolved.insert(template.id, (template.spec, None));
                }
            }
        }

        let mut wanted: Vec<DesiredRoom> = Vec::new();
        for room in desired.rooms {
            if room.spec.has_hooks() && !self.allow_hooks {
                errors.insert(
                    room.id,
                    "这台节点加入时没有带 --allow-hooks，不接收带钩子（override 或处理器命令）的房间".into(),
                );
                continue;
            }
            if let Some(template) = room.template_id {
                if let Some(error) = template_errors.get(&template) {
                    errors.insert(room.id, error.clone());
                    continue;
                }
                if !resolved.contains_key(&template) {
                    errors.insert(room.id, format!("期望状态里缺少投稿模板 {template}"));
                    continue;
                }
            }
            wanted.push(room);
        }

        // 用得到的模板先落地（新建或更新），落不了的连带房间一起算失败
        let needed: BTreeSet<i64> = wanted.iter().filter_map(|room| room.template_id).collect();
        for template in &needed {
            let (spec, cookie) = &resolved[template];
            if let Err(error) = self
                .upsert_template(*template, spec, cookie.as_deref())
                .await
            {
                template_errors.insert(*template, error);
            }
        }
        wanted.retain(
            |room| match room.template_id.and_then(|t| template_errors.get(&t)) {
                Some(error) => {
                    errors.insert(room.id, error.clone());
                    false
                }
                None => true,
            },
        );

        // 先停掉不该再录的，再加新的、改变了的
        let keep: BTreeSet<i64> = wanted.iter().map(|room| room.id).collect();
        let stale: Vec<i64> = self
            .state
            .rooms
            .keys()
            .filter(|id| !keep.contains(id))
            .copied()
            .collect();
        for id in stale {
            self.remove_room(id).await;
        }
        for room in wanted {
            if let Err(error) = self.upsert_room(&room).await {
                errors.insert(room.id, error);
            }
        }

        // 不再被引用的托管模板：没有任何本地主播在用就删掉；有本地主播在用就留下当本地模板
        let stale: Vec<(i64, i64)> = self
            .state
            .templates
            .iter()
            .filter(|(id, _)| !needed.contains(id))
            .map(|(id, template)| (*id, template.local_id))
            .collect();
        for (id, local_id) in stale {
            self.drop_template(id, local_id).await;
        }

        self.state.state_version = Some(desired.version);
        self.persist();
        self.publish();
        let failed = errors
            .into_iter()
            .map(|(id, error)| FailedRoom { id, error })
            .collect::<Vec<_>>();
        if !failed.is_empty() {
            warn!(?failed, "some fleet rooms could not be applied");
        }
        Ack {
            version: desired.version,
            held: self.held(),
            failed,
        }
    }

    async fn upsert_template(
        &mut self,
        id: i64,
        spec: &TemplateSpec,
        cookie: Option<&str>,
    ) -> Result<(), String> {
        let applied = json!({ "spec": spec, "user_cookie": cookie });
        let existing = self.state.templates.get(&id).cloned();
        if let Some(existing) = &existing {
            if existing.applied == applied {
                return Ok(());
            }
            if row_exists(&self.services, "uploadstreamers", existing.local_id).await {
                let row: InsertUploadStreamer =
                    local_template(spec, cookie, Some(existing.local_id))
                        .and_then(|value| serde_json::from_value(value).ok())
                        .ok_or("投稿模板的格式不对")?;
                row.update_all_fields(&self.services.pool)
                    .await
                    .map_err(|e| format!("保存投稿模板失败：{e}"))?;
                self.state.templates.insert(
                    id,
                    ManagedTemplate {
                        local_id: existing.local_id,
                        applied,
                    },
                );
                self.persist();
                return Ok(());
            }
        }
        let row: InsertUploadStreamer = local_template(spec, cookie, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or("投稿模板的格式不对")?;
        let inserted = ormlite::Insert::insert(row, &self.services.pool)
            .await
            .map_err(|e| format!("保存投稿模板失败：{e}"))?;
        info!(template = id, local = inserted.id, "fleet template added");
        self.state.templates.insert(
            id,
            ManagedTemplate {
                local_id: inserted.id,
                applied,
            },
        );
        self.persist();
        Ok(())
    }

    async fn drop_template(&mut self, id: i64, local_id: i64) {
        let users: i64 =
            sqlx::query_scalar("SELECT count(*) FROM livestreamers WHERE upload_streamers_id = ?")
                .bind(local_id)
                .fetch_one(&self.services.pool)
                .await
                .unwrap_or(1);
        // 删模板会级联删掉引用它的主播，有人用就只解除托管
        if users == 0 {
            if let Err(e) = sqlx::query("DELETE FROM uploadstreamers WHERE id = ?")
                .bind(local_id)
                .execute(&self.services.pool)
                .await
            {
                warn!(error = %e, template = id, "could not delete managed template");
            }
        } else {
            info!(
                template = id,
                local = local_id,
                "managed template is used by local streamers, keeping it as a local template"
            );
        }
        self.state.templates.remove(&id);
        self.persist();
    }

    async fn remove_room(&mut self, id: i64) {
        let Some(room) = self.state.rooms.get(&id).cloned() else {
            return;
        };
        if let Err(e) =
            delete_streamer(&self.services.pool, &self.services.managers, room.local_id).await
        {
            warn!(error = ?e, room = id, "managed streamer was already gone");
        }
        info!(room = id, local = room.local_id, url = %room.url, "fleet room released");
        self.state.rooms.remove(&id);
        self.persist();
    }

    async fn upsert_room(&mut self, room: &DesiredRoom) -> Result<(), String> {
        let template = match room.template_id {
            Some(template) => Some(
                self.state
                    .templates
                    .get(&template)
                    .map(|t| t.local_id)
                    .ok_or_else(|| format!("投稿模板 {template} 没有落地"))?,
            ),
            None => None,
        };
        let applied = json!({ "spec": room.spec, "template": template });

        if !self.state.rooms.contains_key(&room.id)
            && let Some(local_id) = local_id_by_url(&self.services, &room.spec.url).await
        {
            if self.state.adding.get(&room.id) != Some(&room.spec.url) {
                return Err(format!(
                    "本机已有同一地址的本地主播（id {local_id}），先在本机删掉它再分派"
                ));
            }
            // 上次新增到一半退出了：认领那一行，下面按期望状态重建
            self.state.rooms.insert(
                room.id,
                ManagedRoom {
                    local_id,
                    epoch: room.epoch,
                    url: room.spec.url.clone(),
                    paused: false,
                    applied: Value::Null,
                    error: None,
                },
            );
            self.state.adding.remove(&room.id);
            self.persist();
        }

        match self.state.rooms.get(&room.id).cloned() {
            Some(mut managed) => {
                if managed.applied != applied || managed.error.is_some() {
                    let streamer: LiveStreamer =
                        local_streamer(&room.spec, template, Some(managed.local_id))
                            .and_then(|value| serde_json::from_value(value).ok())
                            .ok_or("房间设置的格式不对")?;
                    let result = update_streamer(&self.services, streamer).await;
                    managed.error = result
                        .as_ref()
                        .err()
                        .map(|e| format!("重建监控失败：{e:?}"));
                    managed.applied = applied;
                    managed.url = room.spec.url.clone();
                    managed.paused = false;
                    info!(
                        room = room.id,
                        local = managed.local_id,
                        "fleet room updated"
                    );
                }
                managed.epoch = room.epoch;
                if managed.error.is_none() {
                    apply_paused(&self.services, managed.local_id, room.paused).await;
                    managed.paused = room.paused;
                }
                let error = managed.error.clone();
                self.state.rooms.insert(room.id, managed);
                self.persist();
                error.map_or(Ok(()), Err)
            }
            None => {
                let row: InsertLiveStreamer = local_streamer(&room.spec, template, None)
                    .and_then(|value| serde_json::from_value(value).ok())
                    .ok_or("房间设置的格式不对")?;
                self.state.adding.insert(room.id, room.spec.url.clone());
                self.persist();
                let result = add_streamer(&self.services, row).await;
                self.state.adding.remove(&room.id);
                let outcome = match result {
                    Ok(streamer) => {
                        info!(room = room.id, local = streamer.id, url = %streamer.url, "fleet room added");
                        self.state.rooms.insert(
                            room.id,
                            ManagedRoom {
                                local_id: streamer.id,
                                epoch: room.epoch,
                                url: streamer.url.clone(),
                                paused: false,
                                applied,
                                error: None,
                            },
                        );
                        if room.paused {
                            apply_paused(&self.services, streamer.id, true).await;
                            if let Some(managed) = self.state.rooms.get_mut(&room.id) {
                                managed.paused = true;
                            }
                        }
                        Ok(())
                    }
                    Err(error) => {
                        // 新增失败时行可能已经写进库里（不支持的地址就是这样）：删掉，不留半截本地行
                        if let Some(local_id) =
                            local_id_by_url(&self.services, &room.spec.url).await
                        {
                            let _ = delete_streamer(
                                &self.services.pool,
                                &self.services.managers,
                                local_id,
                            )
                            .await;
                        }
                        Err(match error {
                            AddStreamerError::UnsupportedUrl => {
                                "没有插件支持这个直播间地址".to_string()
                            }
                            AddStreamerError::Internal(e) => format!("新增主播失败：{e:?}"),
                        })
                    }
                };
                self.persist();
                outcome
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::model::DesiredTemplate;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
    use async_trait::async_trait;
    use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
    use std::sync::{Arc, RwLock};
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::reload;

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
        managed: ManagedHandle,
    }

    impl Fixture {
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
            Fixture {
                dir,
                services,
                managed: ManagedHandle::default(),
            }
        }

        fn path(&self) -> PathBuf {
            self.dir.path().join(STATE_FILE_NAME)
        }

        async fn reconciler(&self, allow_hooks: bool) -> Reconciler {
            Reconciler::resume(
                self.path(),
                "controller",
                "10.0.0.2".into(),
                allow_hooks,
                self.services.clone(),
                self.managed.clone(),
            )
            .await
        }

        async fn credential(&self, name: &str, mid: u64) -> String {
            let path = self.dir.path().join(name);
            std::fs::write(&path, json!({ "token_info": { "mid": mid } }).to_string()).unwrap();
            let path = path.to_string_lossy().into_owned();
            sqlx::query("INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', ?)")
                .bind(&path)
                .execute(&self.services.pool)
                .await
                .unwrap();
            path
        }

        async fn streamers(&self) -> Vec<LiveStreamer> {
            LiveStreamer::select()
                .fetch_all(&self.services.pool)
                .await
                .unwrap()
        }

        async fn templates(&self) -> Vec<UploadStreamer> {
            UploadStreamer::select()
                .fetch_all(&self.services.pool)
                .await
                .unwrap()
        }

        async fn status(&self, local_id: i64) -> Option<WorkerStatus> {
            let worker = self.services.managers.get_room_by_id(local_id).await?;
            let status = worker.downloader_status.read().unwrap().clone();
            Some(status)
        }
    }

    fn room(id: i64, url: &str, template: Option<i64>) -> DesiredRoom {
        serde_json::from_value(json!({
            "id": id,
            "epoch": 1,
            "template_id": template,
            "url": url,
            "remark": format!("房间{id}"),
        }))
        .unwrap()
    }

    fn template(id: i64, mid: Option<u64>) -> DesiredTemplate {
        serde_json::from_value(json!({
            "id": id,
            "template_name": format!("模板{id}"),
            "account_mid": mid,
            "tags": ["t"],
        }))
        .unwrap()
    }

    fn desired(
        version: u64,
        rooms: Vec<DesiredRoom>,
        templates: Vec<DesiredTemplate>,
    ) -> DesiredState {
        DesiredState {
            version,
            rooms,
            templates,
        }
    }

    #[tokio::test]
    async fn managed_rows_follow_the_desired_state_and_local_rows_are_left_alone() {
        let f = Fixture::new().await;
        let cookie = f.credential("cookies-42.json", 42).await;
        let local = add_streamer(
            &f.services,
            serde_json::from_value(
                json!({ "url": "https://stuck.example/local", "remark": "本地" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let mut reconciler = f.reconciler(false).await;
        assert!(
            f.managed
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .streamers
                .is_empty()
        );

        let ack = reconciler
            .apply(desired(
                10,
                vec![
                    room(1, "https://stuck.example/1", Some(7)),
                    room(2, "https://stuck.example/2", None),
                ],
                vec![template(7, Some(42))],
            ))
            .await;
        assert_eq!(ack.version, 10);
        assert_eq!(
            ack.held,
            [HeldRoom { id: 1, epoch: 1 }, HeldRoom { id: 2, epoch: 1 }]
        );
        assert!(ack.failed.is_empty());
        let templates = f.templates().await;
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].user_cookie.as_deref(), Some(cookie.as_str()));
        let streamers = f.streamers().await;
        assert_eq!(streamers.len(), 3);
        let first = streamers.iter().find(|s| s.url.ends_with("/1")).unwrap();
        assert_eq!(first.upload_streamers_id, Some(templates[0].id));
        assert!(f.services.managers.get_room_by_id(first.id).await.is_some());
        {
            let managed = f.managed.read().unwrap().clone().unwrap();
            assert_eq!(managed.controller, "10.0.0.2");
            assert_eq!(managed.streamers.len(), 2);
            assert!(!managed.streamers.contains_key(&local.id));
            assert!(managed.templates.contains(&templates[0].id));
        }

        // 改备注、暂停房间 1，拿掉房间 2：只动托管行
        let mut changed = room(1, "https://stuck.example/1", Some(7));
        changed.spec.remark = "改过".into();
        changed.paused = true;
        changed.epoch = 2;
        let ack = reconciler
            .apply(desired(11, vec![changed], vec![template(7, Some(42))]))
            .await;
        assert_eq!(ack.held, [HeldRoom { id: 1, epoch: 2 }]);
        let streamers = f.streamers().await;
        assert_eq!(streamers.len(), 2);
        assert!(streamers.iter().any(|s| s.id == local.id));
        let first = streamers.iter().find(|s| s.url.ends_with("/1")).unwrap();
        assert_eq!(first.remark, "改过");
        assert!(matches!(
            f.status(first.id).await,
            Some(WorkerStatus::Pause)
        ));

        // 全部拿掉：托管主播与模板删掉，本地主播还在
        let ack = reconciler.apply(desired(12, vec![], vec![])).await;
        assert!(ack.held.is_empty());
        let streamers = f.streamers().await;
        assert_eq!(streamers.len(), 1);
        assert_eq!(streamers[0].id, local.id);
        assert!(f.templates().await.is_empty());
        assert!(f.services.managers.get_room_by_id(local.id).await.is_some());
    }

    #[tokio::test]
    async fn rooms_that_cannot_run_here_are_reported_not_applied() {
        let f = Fixture::new().await;
        add_streamer(
            &f.services,
            serde_json::from_value(
                json!({ "url": "https://stuck.example/taken", "remark": "本地" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let mut reconciler = f.reconciler(false).await;
        let mut hooked = room(3, "https://stuck.example/3", None);
        hooked.spec.postprocessor = serde_json::from_value(json!([{ "run": "echo" }])).unwrap();
        let ack = reconciler
            .apply(desired(
                1,
                vec![
                    room(1, "https://stuck.example/1", Some(7)),
                    room(2, "rtmp://unsupported.example/2", None),
                    hooked,
                    room(4, "https://stuck.example/taken", None),
                    room(5, "https://stuck.example/5", Some(8)),
                ],
                vec![template(7, Some(42))],
            ))
            .await;
        assert!(ack.held.is_empty());
        let reasons: BTreeMap<i64, String> =
            ack.failed.into_iter().map(|f| (f.id, f.error)).collect();
        assert!(reasons[&1].contains("42"), "{reasons:?}");
        assert!(reasons[&2].contains("插件"), "{reasons:?}");
        assert!(reasons[&3].contains("--allow-hooks"), "{reasons:?}");
        assert!(reasons[&4].contains("本地主播"), "{reasons:?}");
        assert!(reasons[&5].contains("模板"), "{reasons:?}");
        // 不支持的地址不留半截行；本地那一行原样在
        let streamers = f.streamers().await;
        assert_eq!(streamers.len(), 1);
        assert_eq!(streamers[0].remark, "本地");
        assert!(f.templates().await.is_empty());
    }

    #[tokio::test]
    async fn a_restart_resumes_from_the_state_file_and_release_turns_rows_local() {
        let f = Fixture::new().await;
        let mut reconciler = f.reconciler(true).await;
        let mut paused = room(1, "https://stuck.example/1", None);
        paused.paused = true;
        reconciler.apply(desired(5, vec![paused], vec![])).await;
        let local_id = reconciler.state().rooms[&1].local_id;
        drop(reconciler);

        // 模拟重启：监控里的房间是按库重新载入的，暂停状态丢了
        f.services.managers.del_room(local_id).await;
        let streamer = f.streamers().await.remove(0);
        f.services
            .managers
            .add_room(f.services.worker(streamer, None))
            .await
            .unwrap();
        assert!(!matches!(
            f.status(local_id).await,
            Some(WorkerStatus::Pause)
        ));
        let mut reconciler = f.reconciler(true).await;
        assert_eq!(reconciler.state_version(), Some(5));
        assert_eq!(reconciler.held(), [HeldRoom { id: 1, epoch: 1 }]);
        assert!(matches!(
            f.status(local_id).await,
            Some(WorkerStatus::Pause)
        ));

        // 别的控制面的缓存作废
        let other = Reconciler::resume(
            f.path(),
            "another",
            "x".into(),
            true,
            f.services.clone(),
            ManagedHandle::default(),
        )
        .await;
        assert!(other.held().is_empty());
        assert!(!f.path().exists());
        reconciler.persist();

        reconciler.release();
        assert!(!f.path().exists());
        assert!(f.managed.read().unwrap().is_none());
        // 行还在、还在监控里
        assert_eq!(f.streamers().await.len(), 1);
        assert!(f.services.managers.get_room_by_id(local_id).await.is_some());
    }

    #[tokio::test]
    async fn a_half_finished_add_is_adopted_by_url() {
        let f = Fixture::new().await;
        let orphan = add_streamer(
            &f.services,
            serde_json::from_value(json!({ "url": "https://stuck.example/1", "remark": "旧" }))
                .unwrap(),
        )
        .await
        .unwrap();
        let mut state = FleetState::new("controller");
        state.adding.insert(1, "https://stuck.example/1".into());
        save(&f.path(), &state).unwrap();
        let mut reconciler = f.reconciler(false).await;
        let ack = reconciler
            .apply(desired(
                2,
                vec![room(1, "https://stuck.example/1", None)],
                vec![],
            ))
            .await;
        assert_eq!(ack.held, [HeldRoom { id: 1, epoch: 1 }]);
        assert_eq!(reconciler.state().rooms[&1].local_id, orphan.id);
        assert!(reconciler.state().adding.is_empty());
        assert_eq!(f.streamers().await[0].remark, "房间1");
    }

    #[tokio::test]
    async fn a_managed_template_used_by_a_local_streamer_is_kept_as_local() {
        let f = Fixture::new().await;
        let mut reconciler = f.reconciler(false).await;
        reconciler
            .apply(desired(
                1,
                vec![room(1, "https://stuck.example/1", Some(7))],
                vec![template(7, None)],
            ))
            .await;
        let template_id = f.templates().await[0].id;
        add_streamer(
            &f.services,
            serde_json::from_value(json!({
                "url": "https://stuck.example/local",
                "remark": "本地",
                "upload_streamers_id": template_id,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
        reconciler.apply(desired(2, vec![], vec![])).await;
        assert_eq!(f.templates().await.len(), 1);
        let streamers = f.streamers().await;
        assert_eq!(streamers.len(), 1);
        assert_eq!(streamers[0].remark, "本地");
        assert!(reconciler.state().templates.is_empty());
    }
}
