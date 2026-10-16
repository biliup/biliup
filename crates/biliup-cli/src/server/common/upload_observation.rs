//! Read-only upload telemetry. Upload scheduling and retries remain with their owners.
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

tokio::task_local! { static ACTIVE_UPLOAD: TaskObservation; }
const CACHED_TERMINAL_TASKS: usize = 500;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn terminal(state: &str) -> bool {
    matches!(
        state,
        "succeeded"
            | "partial"
            | "failed"
            | "cancelled"
            | "interrupted"
            | "unknown"
            | "paused"
            | "skipped"
    )
}
pub fn scrub(error: &str) -> String {
    // Existing scrub removes URL queries, paths and control characters. Also discard
    // credential-bearing response/header diagnostics rather than guessing their format.
    let lower = error.to_ascii_lowercase();
    let sensitive_json_key = serde_json::from_str::<serde_json::Value>(error)
        .ok()
        .is_some_and(|value| contains_secret_key(&value));
    let url_userinfo = error.split("://").skip(1).any(|rest| {
        rest.split(['/', '?', '#', ' ', ')', ']'])
            .next()
            .is_some_and(|authority| authority.contains('@'))
    });
    if sensitive_json_key
        || url_userinfo
        || [
            "access_token",
            "refresh_token",
            "sessdata",
            "bili_jct",
            "authorization",
            "x-upos-auth",
            "cookie:",
            "password",
            "token_info",
            "cookie_info",
            "\"auth\"",
            "auth:",
            "auth=",
            "\"token\"",
            "token=",
            "\"ticket\"",
            "ticket=",
            "\"secret\"",
            "secret=",
        ]
        .iter()
        .any(|key| lower.contains(key))
    {
        "上传失败，详细信息已隐藏；请检查服务日志".into()
    } else {
        crate::server::fleet::events::scrub(error)
    }
}
fn contains_secret_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            key == "auth"
                || key.ends_with("_auth")
                || key.contains("token")
                || key.contains("cookie")
                || key.contains("sessdata")
                || key.contains("bili_jct")
                || key.contains("ticket")
                || key.contains("secret")
                || key.contains("password")
                || key.contains("authorization")
                || contains_secret_key(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(contains_secret_key),
        _ => false,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileView {
    pub id: String,
    pub order: u64,
    pub name: String,
    pub state: String,
    pub bytes_confirmed: u64,
    pub total_bytes: Option<u64>,
    pub retries: u32,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskView {
    pub id: String,
    pub source: String,
    pub scheduler_group: String,
    pub order: u64,
    pub queue_position: Option<u64>,
    pub title: String,
    pub streamer_name: Option<String>,
    pub session_id: Option<i64>,
    pub template_name: Option<String>,
    pub state: String,
    pub phase: String,
    pub current_file: Option<String>,
    pub bytes_confirmed: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: f64,
    pub files_succeeded: u32,
    pub files_failed: u32,
    pub retries: u32,
    pub next_retry_at: Option<i64>,
    pub error: Option<String>,
    pub bvid: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDetail {
    #[serde(flatten)]
    pub task: TaskView,
    pub files: Vec<FileView>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub uploading: usize,
    pub queued: usize,
    pub attention: usize,
    pub speed_bytes_per_sec: f64,
}
#[derive(Serialize)]
pub struct QueueView {
    pub tasks: Vec<TaskView>,
    pub total: usize,
    pub summary: Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
pub struct NewTask<'a> {
    pub source: &'a str,
    pub scheduler_group: &'a str,
    pub title: &'a str,
    pub streamer_name: Option<&'a str>,
    pub session_id: Option<i64>,
    pub template_name: Option<&'a str>,
    pub fifo: bool,
}
struct LiveTask {
    detail: TaskDetail,
    samples: VecDeque<(Instant, u64)>,
    confirmed_parts: HashMap<String, HashSet<usize>>,
    confirmed_ranges: HashMap<String, Vec<(u64, u64)>>,
    open_ended: bool,
    fifo: bool,
    pinned: u32,
    revision: u64,
}
#[derive(Default)]
struct Inner {
    tasks: HashMap<String, LiveTask>,
    dirty: HashSet<String>,
    next_order: HashMap<String, u64>,
    error: Option<String>,
}
pub struct UploadObservations {
    pool: ConnectionPool,
    inner: Mutex<Inner>,
    wake: Notify,
    flush: tokio::sync::Mutex<()>,
}
impl std::fmt::Debug for UploadObservations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadObservations").finish_non_exhaustive()
    }
}
impl UploadObservations {
    pub fn new(pool: ConnectionPool) -> Arc<Self> {
        let this = Arc::new(Self {
            pool,
            inner: Mutex::new(Inner::default()),
            wake: Notify::new(),
            flush: tokio::sync::Mutex::new(()),
        });
        let weak = Arc::downgrade(&this);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let Some(this) = weak.upgrade() else { break };
                if let Err(error) = this.flush_dirty().await {
                    this.monitor_error(&error.to_string());
                }
            }
        });
        this
    }

    /// Called once during service initialization, before upload producers start.
    pub async fn initialize(&self) -> Result<(), sqlx::Error> {
        let mut rows: Vec<String> = sqlx::query_scalar("SELECT snapshot FROM upload_observation_tasks WHERE state IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped') ORDER BY updated_at DESC LIMIT ?")
            .bind(CACHED_TERMINAL_TASKS as i64).fetch_all(&self.pool).await?;
        let active: Vec<String> = sqlx::query_scalar("SELECT snapshot FROM upload_observation_tasks WHERE state NOT IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped') ORDER BY created_at")
            .fetch_all(&self.pool).await?;
        rows.extend(active);
        let orders: Vec<(String, i64)> = sqlx::query_as("SELECT scheduler_group,MAX(task_order) FROM upload_observation_tasks GROUP BY scheduler_group")
            .fetch_all(&self.pool).await?;
        {
            let mut inner = lock(&self.inner);
            inner.next_order.extend(
                orders
                    .into_iter()
                    .map(|(group, order)| (group, order.max(0) as u64)),
            );
            for row in rows {
                let Ok(mut detail) = serde_json::from_str::<TaskDetail>(&row) else {
                    inner.error = Some("部分上传历史无法读取".into());
                    continue;
                };
                if !terminal(&detail.task.state) {
                    detail.task.state = if detail.task.state == "submitting" {
                        "unknown"
                    } else {
                        "interrupted"
                    }
                    .into();
                    detail.task.phase = "finished".into();
                    detail.task.error = Some(
                        if detail.task.state == "unknown" {
                            "服务在提交时中断，请核实 B 站投稿结果；监控不会重复投稿"
                        } else {
                            "服务重启，原上传任务已中断"
                        }
                        .into(),
                    );
                    detail.task.updated_at = now();
                    for file in &mut detail.files {
                        if !terminal(&file.state) {
                            file.state = "interrupted".into();
                        }
                    }
                    inner.dirty.insert(detail.task.id.clone());
                }
                detail.task.speed_bytes_per_sec = 0.0;
                detail.task.queue_position = None;
                detail.task.next_retry_at = None;
                inner.tasks.insert(
                    detail.task.id.clone(),
                    LiveTask {
                        detail,
                        samples: VecDeque::new(),
                        confirmed_parts: HashMap::new(),
                        confirmed_ranges: HashMap::new(),
                        open_ended: false,
                        fifo: false,
                        pinned: 0,
                        revision: 0,
                    },
                );
            }
        }
        self.flush_dirty().await
    }
    pub fn register(self: &Arc<Self>, new: NewTask<'_>) -> TaskGuard {
        let mut inner = lock(&self.inner);
        let order = inner
            .next_order
            .entry(new.scheduler_group.into())
            .or_default();
        *order += 1;
        let order = *order;
        let time = now();
        let id = format!("upload-{time}-{}", rand::random::<u64>());
        let task = TaskView {
            id: id.clone(),
            source: new.source.into(),
            scheduler_group: new.scheduler_group.into(),
            order,
            queue_position: None,
            title: new.title.into(),
            streamer_name: new.streamer_name.map(str::to_owned),
            session_id: new.session_id,
            template_name: new.template_name.map(str::to_owned),
            state: "queued".into(),
            phase: "waiting_slot".into(),
            current_file: None,
            bytes_confirmed: 0,
            total_bytes: None,
            speed_bytes_per_sec: 0.0,
            files_succeeded: 0,
            files_failed: 0,
            retries: 0,
            next_retry_at: None,
            error: None,
            bvid: None,
            created_at: time,
            updated_at: time,
        };
        inner.tasks.insert(
            id.clone(),
            LiveTask {
                detail: TaskDetail {
                    task,
                    files: Vec::new(),
                },
                samples: VecDeque::new(),
                confirmed_parts: HashMap::new(),
                confirmed_ranges: HashMap::new(),
                open_ended: matches!(
                    new.source,
                    "automatic" | "sync" | "ha" | "ha_append" | "ha_resume"
                ),
                fifo: new.fifo,
                pinned: 0,
                revision: 0,
            },
        );
        inner.dirty.insert(id.clone());
        self.wake.notify_one();
        TaskGuard {
            observation: TaskObservation {
                registry: Arc::downgrade(self),
                id,
            },
            finished: false,
            pinned: false,
        }
    }
    pub fn monitor_error(&self, error: &str) {
        let first = {
            let mut inner = lock(&self.inner);
            let first = inner.error.is_none();
            inner.error = Some("上传监控历史保存失败，实时数据仍可查看".into());
            first
        };
        if first {
            tracing::warn!(error = %scrub(error), "上传监控保存失败；上传继续执行");
        }
    }
    fn update(&self, id: &str, edit: impl FnOnce(&mut LiveTask)) {
        let mut inner = lock(&self.inner);
        if let Some(task) = inner.tasks.get_mut(id) {
            edit(task);
            task.detail.task.updated_at = now();
            task.revision += 1;
            inner.dirty.insert(id.into());
        }
        self.wake.notify_one();
    }
    fn requeue(&self, id: &str) {
        let mut inner = lock(&self.inner);
        let Some(group) = inner
            .tasks
            .get(id)
            .map(|task| task.detail.task.scheduler_group.clone())
        else {
            return;
        };
        let order = inner.next_order.entry(group).or_default();
        *order += 1;
        let next_order = *order;
        if let Some(task) = inner.tasks.get_mut(id) {
            task.detail.task.order = next_order;
            task.detail.task.state = "queued".into();
            task.detail.task.phase = "waiting_slot".into();
            task.detail.task.error = None;
            task.detail.task.next_retry_at = None;
            task.detail.task.updated_at = now();
            task.revision += 1;
            inner.dirty.insert(id.into());
        }
        self.wake.notify_one();
    }
    fn pin(&self, id: &str, delta: i32) {
        let mut inner = lock(&self.inner);
        if let Some(task) = inner.tasks.get_mut(id) {
            task.pinned = task.pinned.saturating_add_signed(delta);
        }
    }
    pub async fn detail(&self, id: &str) -> Result<Option<TaskDetail>, sqlx::Error> {
        if let Some(detail) = self.detail_cached(id) {
            return Ok(Some(detail));
        }
        let row: Option<String> =
            match sqlx::query_scalar("SELECT snapshot FROM upload_observation_tasks WHERE id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
            {
                Ok(row) => row,
                Err(error) => {
                    self.monitor_error(&error.to_string());
                    return Err(error);
                }
            };
        row.map(|snapshot| {
            serde_json::from_str(&snapshot).map_err(|error| sqlx::Error::Decode(Box::new(error)))
        })
        .transpose()
    }
    pub(crate) fn detail_cached(&self, id: &str) -> Option<TaskDetail> {
        let inner = lock(&self.inner);
        let positions = queue_positions(&inner.tasks);
        inner.tasks.get(id).map(|task| TaskDetail {
            task: snapshot_task(task, positions.get(id).copied()),
            files: task.detail.files.clone(),
        })
    }
    pub async fn list(
        &self,
        source: Option<&str>,
        state: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> QueueView {
        let filter = "(? IS NULL OR source=?) AND (? IS NULL OR state=?)";
        let total = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM upload_observation_tasks WHERE {filter}"
        ))
        .bind(source)
        .bind(source)
        .bind(state)
        .bind(state)
        .fetch_one(&self.pool)
        .await;
        let summary = sqlx::query_as::<_, (i64,i64,i64)>(&format!(
            "SELECT COALESCE(SUM(CASE WHEN state NOT IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped') AND json_extract(snapshot,'$.phase')='uploading' THEN 1 ELSE 0 END),0), COALESCE(SUM(state='queued'),0), COALESCE(SUM(state IN ('partial','failed','unknown','interrupted','paused')),0) FROM upload_observation_tasks"
        )).fetch_one(&self.pool).await;
        let snapshots: Result<Vec<String>, sqlx::Error> = sqlx::query_scalar(&format!(
            "SELECT snapshot FROM upload_observation_tasks WHERE {filter} ORDER BY CASE WHEN state IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped') THEN 1 ELSE 0 END, CASE WHEN state IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped') THEN updated_at END DESC, scheduler_group,task_order LIMIT ? OFFSET ?"
        )).bind(source).bind(source).bind(state).bind(state).bind(limit as i64).bind(offset as i64).fetch_all(&self.pool).await;
        let (total, (uploading, queued, attention), snapshots) = match (total, summary, snapshots) {
            (Ok(total), Ok(summary), Ok(snapshots)) => (total.max(0) as usize, summary, snapshots),
            (_, _, Err(error)) | (Err(error), _, _) | (_, Err(error), _) => {
                self.monitor_error(&error.to_string());
                let inner = lock(&self.inner);
                return cached_queue(&inner, source, state, limit, offset);
            }
        };
        let inner = lock(&self.inner);
        let positions = queue_positions(&inner.tasks);
        let mut speed = 0.0;
        for live in inner.tasks.values() {
            speed += rolling_speed(live, &live.detail.task.state);
        }
        let tasks: Vec<_> = snapshots
            .into_iter()
            .filter_map(|snapshot| serde_json::from_str::<TaskDetail>(&snapshot).ok())
            .map(|detail| {
                inner
                    .tasks
                    .get(&detail.task.id)
                    .map(|live| snapshot_task(live, positions.get(&detail.task.id).copied()))
                    .unwrap_or(detail.task)
            })
            .filter(|task| {
                source.is_none_or(|source| task.source == source)
                    && state.is_none_or(|state| task.state == state)
            })
            .collect();
        QueueView {
            tasks,
            total,
            summary: Summary {
                uploading: uploading.max(0) as usize,
                queued: queued.max(0) as usize,
                attention: attention.max(0) as usize,
                speed_bytes_per_sec: speed,
            },
            error: inner.error.clone(),
        }
    }
    async fn flush_dirty(&self) -> Result<(), sqlx::Error> {
        let _flush = self.flush.lock().await;
        let snapshots: Vec<_> = {
            let inner = lock(&self.inner);
            inner
                .dirty
                .iter()
                .filter_map(|id| {
                    inner
                        .tasks
                        .get(id)
                        .map(|task| (task.detail.clone(), task.revision))
                })
                .collect()
        };
        if snapshots.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for (detail, _) in &snapshots {
            let task = &detail.task;
            sqlx::query("INSERT INTO upload_observation_tasks(id,source,scheduler_group,task_order,state,created_at,updated_at,snapshot) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET source=excluded.source,scheduler_group=excluded.scheduler_group,task_order=excluded.task_order,state=excluded.state,updated_at=excluded.updated_at,snapshot=excluded.snapshot")
                .bind(&task.id).bind(&task.source).bind(&task.scheduler_group).bind(task.order as i64).bind(&task.state).bind(task.created_at).bind(task.updated_at).bind(serde_json::to_string(detail).expect("upload snapshot is serializable")).execute(&mut *tx).await?;
            for file in &detail.files {
                sqlx::query("INSERT INTO upload_observation_files(id,task_id,file_order,snapshot) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET snapshot=excluded.snapshot")
                    .bind(&file.id).bind(&task.id).bind(file.order as i64).bind(serde_json::to_string(file).expect("upload file is serializable")).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        let mut inner = lock(&self.inner);
        for (detail, revision) in snapshots {
            if inner
                .tasks
                .get(&detail.task.id)
                .is_some_and(|task| task.revision == revision)
            {
                inner.dirty.remove(&detail.task.id);
            }
        }
        inner.error = None;
        drop(inner);
        self.prune_terminal_cache();
        Ok(())
    }
    fn prune_terminal_cache(&self) {
        let mut inner = lock(&self.inner);
        let mut terminal: Vec<_> = inner
            .tasks
            .values()
            .filter(|task| {
                terminal(&task.detail.task.state)
                    && task.pinned == 0
                    && !inner.dirty.contains(&task.detail.task.id)
            })
            .map(|task| (task.detail.task.updated_at, task.detail.task.id.clone()))
            .collect();
        if terminal.len() <= CACHED_TERMINAL_TASKS {
            return;
        }
        terminal.sort_unstable_by(|a, b| b.cmp(a));
        for (_, id) in terminal.into_iter().skip(CACHED_TERMINAL_TASKS) {
            inner.tasks.remove(&id);
        }
    }
    pub async fn flush(&self) {
        match tokio::time::timeout(Duration::from_secs(2), self.flush_dirty()).await {
            Ok(Ok(())) => (),
            Ok(Err(error)) => self.monitor_error(&error.to_string()),
            Err(_) => self.monitor_error("监控保存超时"),
        }
    }
}
fn snapshot_task(task: &LiveTask, queue_position: Option<u64>) -> TaskView {
    let mut view = task.detail.task.clone();
    view.speed_bytes_per_sec = rolling_speed(task, &view.state);
    view.queue_position = queue_position.filter(|_| task.fifo && view.state == "queued");
    view
}
fn queue_positions(tasks: &HashMap<String, LiveTask>) -> HashMap<String, u64> {
    let mut fifo: HashMap<&str, Vec<(&str, u64)>> = HashMap::new();
    for live in tasks.values() {
        let task = &live.detail.task;
        if live.fifo && task.state == "queued" {
            fifo.entry(&task.scheduler_group)
                .or_default()
                .push((&task.id, task.order));
        }
    }
    let mut positions = HashMap::new();
    for entries in fifo.values_mut() {
        entries.sort_unstable_by_key(|(_, order)| *order);
        for (index, (id, _)) in entries.iter().enumerate() {
            positions.insert((*id).to_owned(), index as u64 + 1);
        }
    }
    positions
}
fn cached_queue(
    inner: &Inner,
    source: Option<&str>,
    state: Option<&str>,
    limit: usize,
    offset: usize,
) -> QueueView {
    let positions = queue_positions(&inner.tasks);
    let mut tasks: Vec<_> = inner
        .tasks
        .values()
        .map(|live| snapshot_task(live, positions.get(&live.detail.task.id).copied()))
        .filter(|task| {
            source.is_none_or(|source| task.source == source)
                && state.is_none_or(|state| task.state == state)
        })
        .collect();
    tasks.sort_by(|a, b| {
        let a_terminal = terminal(&a.state);
        let b_terminal = terminal(&b.state);
        a_terminal.cmp(&b_terminal).then_with(|| {
            if a_terminal {
                b.updated_at.cmp(&a.updated_at)
            } else {
                a.scheduler_group
                    .cmp(&b.scheduler_group)
                    .then(a.order.cmp(&b.order))
            }
        })
    });
    let total = tasks.len();
    let mut summary = Summary::default();
    for live in inner.tasks.values() {
        let task = &live.detail.task;
        summary.uploading += usize::from(task.phase == "uploading" && !terminal(&task.state));
        summary.queued += usize::from(task.state == "queued");
        summary.attention += usize::from(matches!(
            task.state.as_str(),
            "partial" | "failed" | "unknown" | "interrupted" | "paused"
        ));
        summary.speed_bytes_per_sec += rolling_speed(live, &task.state);
    }
    QueueView {
        tasks: tasks.into_iter().skip(offset).take(limit).collect(),
        total,
        summary,
        error: inner.error.clone(),
    }
}
fn rolling_speed(task: &LiveTask, state: &str) -> f64 {
    if terminal(state) {
        return 0.0;
    }
    task.samples
        .iter()
        .filter(|(at, _)| at.elapsed() < Duration::from_secs(5))
        .map(|(_, bytes)| *bytes as f64)
        .sum::<f64>()
        / 5.0
}

#[derive(Clone, Debug)]
pub struct TaskObservation {
    registry: Weak<UploadObservations>,
    id: String,
}
impl TaskObservation {
    pub fn current() -> Option<Self> {
        ACTIVE_UPLOAD.try_with(Clone::clone).ok()
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn is_submitting(&self) -> bool {
        self.registry
            .upgrade()
            .and_then(|registry| registry.detail_cached(&self.id))
            .is_some_and(|detail| detail.task.state == "submitting")
    }
    pub async fn scope<F: Future>(&self, future: F) -> F::Output {
        ACTIVE_UPLOAD.scope(self.clone(), future).await
    }
    pub fn phase(&self, phase: &str) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                if !terminal(&task.detail.task.state) {
                    task.detail.task.state = "running".into();
                    task.detail.task.phase = phase.into();
                    task.detail.task.next_retry_at = None;
                }
            });
        }
    }
    pub fn title(&self, title: &str) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| task.detail.task.title = title.into());
        }
    }
    pub fn template_name(&self, template_name: Option<&str>) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                task.detail.task.template_name = template_name.map(str::to_owned)
            });
        }
    }
    pub fn source(&self, source: &str) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| task.detail.task.source = source.into());
        }
    }
    pub fn retry(&self, delay: Duration, error: &str) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                let view = &mut task.detail.task;
                view.state = "retrying".into();
                view.phase = "retrying".into();
                view.next_retry_at = Some(now() + delay.as_millis() as i64);
                view.error = Some(scrub(error));
            });
        }
    }
    pub fn queued(&self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                task.detail.task.state = "queued".into();
                task.detail.task.phase = "waiting_slot".into();
                task.detail.task.error = None;
                task.detail.task.next_retry_at = None;
            });
        }
    }
    pub fn requeue(&self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.requeue(&self.id);
        }
    }
    pub async fn submitting(&self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                task.detail.task.state = "submitting".into();
                task.detail.task.phase = "submitting".into();
            });
            registry.flush().await;
        }
    }
    pub fn finish(&self, state: &str, error: Option<&str>, bvid: Option<&str>) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                task.open_ended = false;
                recalculate(task);
                let final_state = if (state == "succeeded" && task.detail.task.files_failed > 0)
                    || (state == "failed"
                        && task.detail.task.files_failed > 0
                        && task.detail.task.files_succeeded > 0)
                {
                    "partial"
                } else {
                    state
                };
                let file_state = match final_state {
                    "unknown" => Some("unknown"),
                    "interrupted" => Some("interrupted"),
                    "cancelled" | "skipped" => Some("cancelled"),
                    "failed" => Some("failed"),
                    _ => None,
                };
                if let Some(file_state) = file_state {
                    for file in &mut task.detail.files {
                        if !terminal(&file.state) {
                            file.state = file_state.into();
                            file.error = error.map(scrub);
                        }
                    }
                }
                recalculate(task);
                let view = &mut task.detail.task;
                view.state = final_state.into();
                view.phase = "finished".into();
                view.error = error.map(scrub);
                view.bvid = bvid.map(str::to_owned).or_else(|| view.bvid.clone());
                view.next_retry_at = None;
            });
        }
    }
    pub fn bvid(&self, bvid: &str) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| task.detail.task.bvid = Some(bvid.into()));
        }
    }
    pub fn prepare_files(&self, files: impl IntoIterator<Item = (String, Option<u64>)>) {
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                for (name, total_bytes) in files {
                    let id = format!("{}-{}", self.id, task.detail.files.len() + 1);
                    task.detail.files.push(FileView {
                        id,
                        order: task.detail.files.len() as u64 + 1,
                        name,
                        state: "queued".into(),
                        bytes_confirmed: 0,
                        total_bytes,
                        retries: 0,
                        error: None,
                    });
                }
                recalculate(task);
            });
        }
    }
    pub fn file(&self, name: &str, total_bytes: Option<u64>) -> FileObservation {
        let mut id = String::new();
        if let Some(registry) = self.registry.upgrade() {
            registry.update(&self.id, |task| {
                // Whole-file retry updates the same P, preserving deduplication of confirmed parts.
                let mut retried = false;
                if let Some(file) = task
                    .detail
                    .files
                    .iter_mut()
                    .find(|file| file.name == name && file.state != "succeeded")
                {
                    if file.state != "queued" {
                        file.retries += 1;
                        retried = true;
                    }
                    file.state = "running".into();
                    file.total_bytes = total_bytes;
                    file.error = None;
                    id = file.id.clone();
                } else {
                    id = format!("{}-{}", self.id, task.detail.files.len() + 1);
                    task.detail.files.push(FileView {
                        id: id.clone(),
                        order: task.detail.files.len() as u64 + 1,
                        name: name.into(),
                        state: "running".into(),
                        bytes_confirmed: 0,
                        total_bytes,
                        retries: 0,
                        error: None,
                    });
                }
                if retried {
                    task.detail.task.retries += 1;
                }
                task.detail.task.current_file = Some(name.into());
                task.detail.task.state = "running".into();
                task.detail.task.phase = "uploading".into();
                recalculate(task);
            });
        }
        FileObservation {
            task: self.clone(),
            id,
        }
    }
}
fn recalculate(task: &mut LiveTask) {
    let files = &task.detail.files;
    let view = &mut task.detail.task;
    view.bytes_confirmed = files.iter().map(|file| file.bytes_confirmed).sum();
    view.total_bytes = if task.open_ended || files.is_empty() {
        None
    } else {
        files
            .iter()
            .map(|file| file.total_bytes)
            .collect::<Option<Vec<_>>>()
            .map(|bytes| bytes.into_iter().sum())
    };
    view.files_succeeded = files
        .iter()
        .filter(|file| file.state == "succeeded")
        .count() as u32;
    view.files_failed = files.iter().filter(|file| file.state == "failed").count() as u32;
}
#[derive(Clone)]
pub struct FileObservation {
    task: TaskObservation,
    id: String,
}
impl FileObservation {
    pub fn cancel(&self) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    file.state = "cancelled".into();
                    file.error = None;
                }
                recalculate(task);
            });
        }
    }
    pub fn retry(&self) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    file.retries += 1;
                }
                task.detail.task.retries += 1;
            });
        }
    }
    pub fn confirmed_range(&self, start: u64, bytes: usize) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                let ranges = task.confirmed_ranges.entry(self.id.clone()).or_default();
                ranges.push((start, start.saturating_add(bytes as u64)));
                ranges.sort_unstable();
                let mut merged: Vec<(u64, u64)> = Vec::new();
                for &(start, end) in ranges.iter() {
                    if let Some(last) = merged.last_mut()
                        && start <= last.1
                    {
                        last.1 = last.1.max(end);
                    } else {
                        merged.push((start, end));
                    }
                }
                *ranges = merged;
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    let previous = file.bytes_confirmed;
                    file.bytes_confirmed = ranges
                        .iter()
                        .map(|(start, end)| end - start)
                        .sum::<u64>()
                        .min(file.total_bytes.unwrap_or(u64::MAX));
                    let confirmed = file.bytes_confirmed.saturating_sub(previous);
                    if confirmed > 0 {
                        task.samples.push_back((Instant::now(), confirmed));
                    }
                    while task
                        .samples
                        .front()
                        .is_some_and(|(at, _)| at.elapsed() >= Duration::from_secs(5))
                    {
                        task.samples.pop_front();
                    }
                }
                recalculate(task);
            });
        }
    }
    pub fn confirmed(&self, part: usize, bytes: usize) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                if !task
                    .confirmed_parts
                    .entry(self.id.clone())
                    .or_default()
                    .insert(part)
                {
                    return;
                }
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    let previous = file.bytes_confirmed;
                    file.bytes_confirmed = file
                        .bytes_confirmed
                        .saturating_add(bytes as u64)
                        .min(file.total_bytes.unwrap_or(u64::MAX));
                    task.samples
                        .push_back((Instant::now(), file.bytes_confirmed - previous));
                    while task
                        .samples
                        .front()
                        .is_some_and(|(at, _)| at.elapsed() >= Duration::from_secs(5))
                    {
                        task.samples.pop_front();
                    }
                }
                recalculate(task);
            });
        }
    }
    pub fn finish(&self, error: Option<&str>) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    file.state = if error.is_some() {
                        "failed"
                    } else {
                        "succeeded"
                    }
                    .into();
                    file.error = error.map(scrub);
                }
                recalculate(task);
            });
        }
    }
    pub fn total_bytes(&self, bytes: u64) {
        if let Some(registry) = self.task.registry.upgrade() {
            registry.update(&self.task.id, |task| {
                if let Some(file) = task.detail.files.iter_mut().find(|file| file.id == self.id) {
                    file.total_bytes = Some(bytes);
                }
                recalculate(task);
            });
        }
    }
}
pub struct TaskGuard {
    observation: TaskObservation,
    finished: bool,
    pinned: bool,
}
impl std::fmt::Debug for TaskGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TaskGuard")
            .field(&self.observation.id)
            .finish()
    }
}
impl TaskGuard {
    pub fn observation(&self) -> TaskObservation {
        self.observation.clone()
    }
    pub fn pin(&mut self) {
        if self.pinned {
            return;
        }
        if let Some(registry) = self.observation.registry.upgrade() {
            registry.pin(&self.observation.id, 1);
            self.pinned = true;
        }
    }
    pub fn unpin(&mut self) {
        if !self.pinned {
            return;
        }
        if let Some(registry) = self.observation.registry.upgrade() {
            registry.pin(&self.observation.id, -1);
        }
        self.pinned = false;
    }
    pub fn finish(mut self, state: &str, error: Option<&str>, bvid: Option<&str>) {
        self.observation.finish(state, error, bvid);
        self.finished = true;
    }
    pub fn finish_and_keep(&mut self, state: &str, error: Option<&str>, bvid: Option<&str>) {
        self.observation.finish(state, error, bvid);
        self.finished = true;
    }
    pub fn reopen(&mut self) {
        self.finished = false;
    }
    pub async fn scope<F: Future>(&self, future: F) -> F::Output {
        self.observation.scope(future).await
    }
    pub fn finish_result<E: std::fmt::Display>(self, result: &Result<(), E>) {
        match result {
            Ok(()) => {
                let existing = self
                    .observation
                    .registry
                    .upgrade()
                    .and_then(|registry| registry.detail_cached(&self.observation.id));
                if let Some(detail) = existing.filter(|detail| terminal(&detail.task.state)) {
                    self.finish(
                        &detail.task.state,
                        detail.task.error.as_deref(),
                        detail.task.bvid.as_deref(),
                    );
                } else {
                    self.finish("succeeded", None, None);
                }
            }
            Err(error) => {
                let existing = self
                    .observation
                    .registry
                    .upgrade()
                    .and_then(|registry| registry.detail_cached(&self.observation.id));
                match existing {
                    Some(detail) if terminal(&detail.task.state) => self.finish(
                        &detail.task.state,
                        detail.task.error.as_deref(),
                        detail.task.bvid.as_deref(),
                    ),
                    Some(detail) => self.finish(
                        if detail.task.state == "submitting" {
                            "unknown"
                        } else {
                            "failed"
                        },
                        Some(&format!("{error:#}")),
                        None,
                    ),
                    None => self.finish("failed", Some(&format!("{error:#}")), None),
                }
            }
        }
    }
}
impl Drop for TaskGuard {
    fn drop(&mut self) {
        if !self.finished {
            let existing = self
                .observation
                .registry
                .upgrade()
                .and_then(|registry| registry.detail_cached(&self.observation.id));
            if !existing
                .as_ref()
                .is_some_and(|detail| terminal(&detail.task.state))
            {
                let submitting = existing.is_some_and(|detail| detail.task.state == "submitting");
                self.observation.finish(
                    if submitting { "unknown" } else { "interrupted" },
                    Some(if submitting {
                        "提交过程异常退出，请核实 B 站投稿结果"
                    } else {
                        "上传任务被取消或异常退出"
                    }),
                    None,
                );
            }
        }
        self.unpin();
    }
}

#[cfg(test)]
#[path = "upload_observation_tests.rs"]
mod tests;
