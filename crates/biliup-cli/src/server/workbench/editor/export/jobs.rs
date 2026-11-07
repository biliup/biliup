//! Persistent project exports, using the application's shared processing budget.
use super::{Options, Progress};
use crate::server::{
    infrastructure::connection_pool::ConnectionPool,
    workbench::{editor, recorder::now_ms, retention::MEDIA_LIFECYCLE},
};
use serde_json::{Value, json};
use sqlx::Row;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};
use tokio_util::sync::CancellationToken;

pub struct Jobs {
    pub(super) pool: ConnectionPool,
    pub(super) root: PathBuf,
    live: Mutex<HashMap<i64, (CancellationToken, Value)>>,
}
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
impl Jobs {
    pub fn live_snapshot(&self, id: i64) -> Option<Value> {
        lock(&self.live).get(&id).map(|(_, job)| job.clone())
    }
    pub async fn status(&self, project: i64, id: i64) -> Result<Value, String> {
        let raw: String =
            sqlx::query_scalar("SELECT job_json FROM editor_exports WHERE project_id=? AND id=?")
                .bind(project)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("导出任务不存在")?;
        let mut job = match self.live_snapshot(id) {
            Some(value) => value,
            None => serde_json::from_str(&raw).map_err(|e| e.to_string())?,
        };
        if job["phase"] == "done" {
            job["download"] = json!(format!(
                "/v1/editor/projects/{project}/exports/{id}/download?attachment=true"
            ));
            if job["subtitlesFile"].is_string() {
                job["subtitlesDownload"] = json!(format!(
                    "/v1/editor/projects/{project}/exports/{id}/download?subtitles=true&attachment=true"
                ));
            }
        }
        Ok(job)
    }
    pub(crate) async fn cancel_agent(&self, project: i64, turn: &str) -> Result<(), String> {
        let ids:Vec<i64>=sqlx::query_scalar("SELECT id FROM editor_exports WHERE project_id=? AND agent_turn_id=? AND phase IN ('queued','preparing','rendering')").bind(project).bind(turn).fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
        for id in ids {
            self.cancel(project, id).await?;
        }
        Ok(())
    }
    pub fn new(pool: ConnectionPool, root: PathBuf) -> Self {
        Self {
            pool,
            root,
            live: Mutex::new(HashMap::new()),
        }
    }
    pub async fn initialize(&self) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE editor_exports SET phase='failed',job_json=json_set(job_json,'$.phase','failed','$.error','服务重启中断了导出，请重试') WHERE phase IN ('queued','preparing','rendering')").execute(&self.pool).await?;
        Ok(())
    }
    pub async fn list(&self, project: i64) -> Result<Vec<Value>, String> {
        let rows = sqlx::query(
            "SELECT id,job_json FROM editor_exports WHERE project_id=? AND (phase IN ('queued','preparing','rendering') OR id IN (SELECT id FROM editor_exports WHERE project_id=? ORDER BY id DESC LIMIT 100)) ORDER BY id DESC",
        )
        .bind(project)
        .bind(project)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let live = lock(&self.live);
        rows.iter()
            .map(|row| {
                live.get(&row.get::<i64, _>("id"))
                    .map(|(_, job)| Ok(job.clone()))
                    .unwrap_or_else(|| {
                        serde_json::from_str(row.get::<&str, _>("job_json"))
                            .map_err(|e| e.to_string())
                    })
            })
            .collect()
    }
    pub async fn enqueue(
        self: &Arc<Self>,
        project: i64,
        revision: i64,
        input: Value,
    ) -> Result<String, String> {
        self.enqueue_inner(project, revision, input, None, None)
            .await
    }
    /// Retry the stored export snapshot; later project edits must not change its content or options.
    /// One child per attempt makes retries safe when a response is lost or two pages click together.
    pub async fn retry(self: &Arc<Self>, project: i64, previous: i64) -> Result<String, String> {
        super::available()?;
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| e.to_string())?;
        let row=sqlx::query("SELECT revision,phase,document_json,options_json,brand_json,job_json FROM editor_exports WHERE project_id=? AND id=?")
            .bind(project).bind(previous).fetch_optional(&mut *tx).await.map_err(|e|e.to_string())?.ok_or("导出任务不存在")?;
        if let Some(id) = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM editor_exports WHERE retry_of=? AND project_id=?",
        )
        .bind(previous)
        .bind(project)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        {
            return Ok(format!("editor-export-{id}"));
        }
        if !matches!(row.get::<&str, _>("phase"), "failed" | "cancelled") {
            return Err("只能重试失败或已取消的导出".into());
        }
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?)")
                .bind(project)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if !exists {
            return Err("源工程已删除，无法重试导出".into());
        }
        let document: editor::Document =
            serde_json::from_str(row.get("document_json")).map_err(|e| e.to_string())?;
        let options: Value =
            serde_json::from_str(row.get("options_json")).map_err(|e| e.to_string())?;
        let brand: editor::fonts::Snapshot =
            serde_json::from_str(row.get("brand_json")).map_err(|e| e.to_string())?;
        let mut job: Value =
            serde_json::from_str(row.get("job_json")).map_err(|e| e.to_string())?;
        let object = job.as_object_mut().ok_or("导出任务内容无效")?;
        for key in [
            "error",
            "warnings",
            "finishedAt",
            "elapsedSeconds",
            "encoderName",
            "bytes",
            "subtitlesFile",
            "subtitleTrack",
            "step",
            "etaSeconds",
        ] {
            object.remove(key);
        }
        job["phase"] = json!("queued");
        job["progress"] = json!(0);
        job["startedAt"] = json!(chrono::Utc::now().to_rfc3339());
        job["agent"] = json!(false);
        job["retryOf"] = json!(previous);
        let id:i64=sqlx::query_scalar("INSERT INTO editor_exports(project_id,revision,phase,job_json,document_json,options_json,brand_json,retry_of,created_at) VALUES(?,?,'queued',?,?,?,?,?,?) RETURNING id")
            .bind(project).bind(row.get::<i64,_>("revision")).bind(job.to_string()).bind(row.get::<&str,_>("document_json"))
            .bind(row.get::<&str,_>("options_json")).bind(row.get::<&str,_>("brand_json")).bind(previous).bind(now_ms())
            .fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
        job["id"] = json!(format!("editor-export-{id}"));
        sqlx::query("UPDATE editor_exports SET job_json=? WHERE id=?")
            .bind(job.to_string())
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        let token = CancellationToken::new();
        lock(&self.live).insert(id, (token.clone(), job));
        let jobs = self.clone();
        tokio::spawn(async move {
            jobs.run(id, project, document, brand, options, token).await;
        });
        Ok(format!("editor-export-{id}"))
    }
    pub(crate) async fn enqueue_agent(
        self: &Arc<Self>,
        project: i64,
        revision: i64,
        input: Value,
        turn: &str,
        key: &str,
        cancel: &CancellationToken,
    ) -> Result<String, String> {
        self.enqueue_inner(project, revision, input, Some((turn, key)), Some(cancel))
            .await
    }
    async fn enqueue_inner(
        self: &Arc<Self>,
        project: i64,
        revision: i64,
        input: Value,
        owner: Option<(&str, &str)>,
        parent: Option<&CancellationToken>,
    ) -> Result<String, String> {
        if let Some((turn,key))=owner
            && let Some(id)=sqlx::query_scalar::<_,i64>("SELECT id FROM editor_exports WHERE project_id=? AND agent_turn_id=? AND request_key=?").bind(project).bind(turn).bind(key).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?{return Ok(format!("editor-export-{id}"))}
        super::available()?;
        let saved = editor::get(&self.pool, project)
            .await
            .map_err(|e| e.to_string())?;
        if saved.summary.revision != revision {
            return Err("工程已改变，请等待保存完成后重新导出".into());
        }
        let target = match input["loudness"].as_str().unwrap_or("normal") {
            "off" => None,
            "normal" => Some(-14.),
            "quiet" => Some(-16.),
            "broadcast" => Some(-23.),
            "target" => Some(input["targetLufs"].as_f64().unwrap_or(-14.)),
            _ => return Err("响度选项无效".into()),
        };
        let options = json!({"quality":input["quality"],"video":input["video"],"range":input["range"],
            "burnCaptions":input["burnCaptions"].as_bool().unwrap_or(true),"motionBlur":input["motionBlur"].as_bool().unwrap_or(false),"targetLufs":target,"subtitles":input["subtitles"]});
        let parsed: Options = serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
        let token = parent
            .map(CancellationToken::child_token)
            .unwrap_or_default();
        let _cancel = token.clone().drop_guard();
        let mut browser = super::browser::Browser::launch(&token).await?;
        let preview = browser
            .eval(&format!(
                "BiliupExport.exportRequest({},{options})",
                saved.document.project
            ))
            .await;
        browser.close().await;
        let preview = preview?;
        if parsed.subtitles.as_ref().is_some_and(|s| {
            s.file
                .as_deref()
                .is_some_and(|f| !matches!(f, "srt" | "vtt"))
        }) {
            return Err("字幕格式无效".into());
        }
        let name = super::filename(input["output"].as_str().unwrap_or(&saved.summary.name));
        let mut job = json!({"id":"","projectDir":format!("biliup:{project}"),"name":saved.summary.name,"output":name,
            "quality":input["quality"],"burnCaptions":options["burnCaptions"],"loudness":input["loudness"].as_str().unwrap_or("normal"),"phase":"queued","progress":0,
            "width":saved.document.project["canvas"]["width"],"height":saved.document.project["canvas"]["height"],
            "fps":saved.document.project["canvas"]["fps"],"seconds":0,"startedAt":chrono::Utc::now().to_rfc3339(),"encoder":"software","codec":input["video"]["codec"].as_str().unwrap_or("h264")});
        job["revision"] = json!(revision);
        job["range"] = options["range"].clone();
        job["motionBlur"] = options["motionBlur"].clone();
        job["targetLufs"] = options["targetLufs"].clone();
        job["agent"] = json!(owner.is_some());
        for key in ["width", "height", "fps", "seconds"] {
            job[key] = preview[key].clone();
        }
        let brand = editor::fonts::snapshot(&self.pool, &self.root, project)
            .await
            .map_err(|e| e.to_string())?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| e.to_string())?;
        let current: i64 = sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
            .bind(project)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        if current != revision {
            return Err("工程已改变，请重新导出".into());
        }
        if let Some((turn, key)) = owner {
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
            if !active {
                return Err("助手已停止，未提交导出".into());
            }
            if let Some(id)=sqlx::query_scalar::<_,i64>("SELECT id FROM editor_exports WHERE project_id=? AND agent_turn_id=? AND request_key=?").bind(project).bind(turn).bind(key).fetch_optional(&mut *tx).await.map_err(|e|e.to_string())?{return Ok(format!("editor-export-{id}"))}
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM editor_exports WHERE agent_turn_id=?")
                    .bind(turn)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            if count >= 3 {
                return Err("本轮最多提交 3 次导出，请先查看已有任务".into());
            }
            let busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_exports WHERE project_id=? AND phase IN ('queued','preparing','rendering'))").bind(project).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
            if busy {
                return Err(
                    "工程已有导出任务正在运行，请用 export_status 查看进度，完成后再提交新的导出"
                        .into(),
                );
            }
        }
        let id:i64=sqlx::query_scalar("INSERT INTO editor_exports(project_id,revision,phase,job_json,document_json,options_json,created_at,brand_json,agent_turn_id,request_key) VALUES(?,?,'queued',?,?,?,?,?,?,?) RETURNING id")
            .bind(project).bind(revision).bind(job.to_string()).bind(serde_json::to_string(&saved.document).map_err(|e|e.to_string())?).bind(options.to_string()).bind(now_ms()).bind(serde_json::to_string(&brand).map_err(|e|e.to_string())?).bind(owner.map(|o|o.0)).bind(owner.map(|o|o.1)).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
        job["id"] = json!(format!("editor-export-{id}"));
        sqlx::query("UPDATE editor_exports SET job_json=? WHERE id=?")
            .bind(job.to_string())
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        let token = CancellationToken::new();
        lock(&self.live).insert(id, (token.clone(), job));
        if parent.is_some_and(CancellationToken::is_cancelled) {
            token.cancel();
        }
        if let Some((turn, _)) = owner {
            let active = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')",
            )
            .bind(turn)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(false);
            if !active {
                token.cancel();
            }
        }
        let jobs = self.clone();
        tokio::spawn(async move {
            jobs.run(id, project, saved.document, brand, options, token)
                .await;
        });
        Ok(format!("editor-export-{id}"))
    }
    async fn run(
        self: Arc<Self>,
        id: i64,
        project: i64,
        mut document: editor::Document,
        brand: editor::fonts::Snapshot,
        options: Value,
        cancel: CancellationToken,
    ) {
        let started = std::time::Instant::now();
        let result: Result<super::Output, String> = async {
            sqlx::query("UPDATE editor_exports SET phase='preparing',job_json=json_set(job_json,'$.phase','preparing') WHERE id=? AND phase='queued'")
                .bind(id).execute(&self.pool).await.map_err(|e|e.to_string())?;
            if let Some((_,job))=lock(&self.live).get_mut(&id){job["phase"]=json!("preparing");}
            let _lifecycle = MEDIA_LIFECYCLE.read().await;
            let (mut browser, mut paths) = super::inputs::prepare(
                &self.pool, &mut document, &options["range"], "export", &cancel,
            ).await?;
            // Preserve the original timeline clock through preparation and rendering.
            // The range limits sampled frames; it never creates a retimed project.
            let effects = tempfile::Builder::new()
                .prefix("editor-effects-")
                .tempdir()
                .map_err(|e| e.to_string())?;
            super::recording_effects::prepare(&self.pool, &mut browser, super::recording_effects::Preparation { document: &mut document, range: &options["range"], paths: &mut paths, directory: effects.path() }, &cancel, |done, total| {
                    if let Some((_, job)) = lock(&self.live).get_mut(&id) {
                        job["phase"] = json!("preparing");
                        job["step"] = json!({"kind":"graphics","done":done,"total":total})
                    }
                })
            .await?;
            let picture: std::collections::BTreeSet<String> = serde_json::from_value(browser.eval(&format!(
                "BiliupExport.pictureItemIds({},{})",document.project,options["range"],
            )).await?).map_err(|error|format!("画面素材计划无效：{error}"))?;
            let graphics=editor::graphics::load_for_render(&self.pool,project,&document.project,Some(&picture)).await.map_err(|e|e.to_string())?;
            let assets=editor::fonts::materialize(&self.root,&brand).await;
            super::render_graphics(super::RenderProject { project: document.project, paths, graphics, assets }, serde_json::from_value(options).map_err(|e| e.to_string())?, &self.root, &self.path(id), &cancel, Some(browser), |progress| {
                    if let Some((_, job)) = lock(&self.live).get_mut(&id) {
                        match progress {
                            Progress::Sound { done, total } => {
                                job["phase"] = json!("preparing");
                                job["step"] = json!({"kind":"sound","done":done,"total":total});
                            }
                            Progress::Loudness{pass,processed_ms,total_ms}=>{
                                job["phase"]=json!("preparing");
                                job["step"]=json!({"kind":"loudness","pass":pass,"done":processed_ms,"total":total_ms});
                            },
                            Progress::Audio{processed_ms,total_ms}=>{
                                job["phase"]=json!("preparing");
                                job["step"]=json!({"kind":"mix","done":processed_ms,"total":total_ms});
                            },
                            Progress::Planned {
                                width,
                                height,
                                fps,
                                seconds,
                            } => {
                                job["phase"] = json!("preparing");
                                job["width"] = json!(width);
                                job["height"] = json!(height);
                                job["fps"] = json!(fps);
                                job["seconds"] = json!(seconds);
                            }
                            Progress::Preparing { done, total } => {
                                job["phase"] = json!("preparing");
                                job["step"] = json!({"kind":"graphics","done":done,"total":total});
                            }
                            Progress::Encoder { name } => {
                                job["phase"] = json!("preparing");
                                job["progress"] = json!(0);
                                job["step"] = json!({"kind":"encoder"});
                                job["encoderName"] = json!(name);
                            }
                            Progress::Encoding {
                                processed_ms,
                                total_ms,
                            } => {
                                job["phase"] = json!("rendering");
                                if let Some(object) = job.as_object_mut() { object.remove("step"); }
                                job["progress"] = json!(
                                    (processed_ms as f64 / total_ms.max(1) as f64).clamp(0., 0.999)
                                );
                            }
                        }
                    }
                })
            .await
        }
        .await;
        let mut job = lock(&self.live)
            .get(&id)
            .map(|(_, job)| job.clone())
            .unwrap_or_default();
        match result {
            Ok(output) => {
                job["phase"] = json!("done");
                job["progress"] = json!(1);
                job["width"] = json!(output.width);
                job["height"] = json!(output.height);
                job["fps"] = json!(output.fps);
                job["seconds"] = json!(output.seconds);
                job["warnings"] = json!(output.warnings);
                job["encoder"] = json!(if output.encoder.starts_with("lib") {
                    "software"
                } else {
                    "hardware"
                });
                job["encoderName"] = json!(output.encoder);
                job["bytes"] = json!(output.bytes);
                job["subtitlesFile"] = json!(output.subtitles_file.map(|format| {
                    format!(
                        "{}.{}",
                        job["output"]
                            .as_str()
                            .unwrap_or("export.mp4")
                            .trim_end_matches(".mp4"),
                        format
                    )
                }));
                job["subtitleTrack"] = json!(output.subtitle_track);
            }
            Err(error) => {
                job["phase"] = json!(if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                });
                job["error"] = json!(error);
            }
        }
        job["finishedAt"] = json!(chrono::Utc::now().to_rfc3339());
        job["elapsedSeconds"] = json!(started.elapsed().as_secs());
        match sqlx::query("UPDATE editor_exports SET phase=?,job_json=? WHERE id=?")
            .bind(job["phase"].as_str())
            .bind(job.to_string())
            .bind(id)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                lock(&self.live).remove(&id);
            }
            Err(error) => {
                tracing::error!(%error,id,"保存工程导出状态失败");
                if let Some((_, latest)) = lock(&self.live).get_mut(&id) {
                    *latest = job;
                }
            }
        }
    }
    fn path(&self, id: i64) -> PathBuf {
        self.root.join("editor-exports").join(format!("{id}.mp4"))
    }
    pub async fn download(&self, project: i64, id: i64) -> Result<(PathBuf, String), String> {
        let row =
            sqlx::query("SELECT phase,job_json FROM editor_exports WHERE project_id=? AND id=?")
                .bind(project)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("成片不存在")?;
        if row.get::<&str, _>("phase") != "done" {
            return Err("成片尚未完成".into());
        }
        let job: Value =
            serde_json::from_str(row.get::<&str, _>("job_json")).map_err(|e| e.to_string())?;
        Ok((
            self.path(id),
            job["output"].as_str().unwrap_or("export.mp4").into(),
        ))
    }
    pub async fn download_subtitles(
        &self,
        project: i64,
        id: i64,
    ) -> Result<(PathBuf, String), String> {
        let (video, _) = self.download(project, id).await?;
        let text: String =
            sqlx::query_scalar("SELECT job_json FROM editor_exports WHERE project_id=? AND id=?")
                .bind(project)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        let job: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let name = job["subtitlesFile"]
            .as_str()
            .ok_or("此任务没有独立字幕文件")?;
        let extension = name.rsplit('.').next().ok_or("字幕格式无效")?;
        if !matches!(extension, "srt" | "vtt") {
            return Err("字幕格式无效".into());
        }
        Ok((video.with_extension(extension), name.into()))
    }
    pub async fn cancel(&self, project: i64, id: i64) -> Result<(), String> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM editor_exports WHERE project_id=? AND id=?)",
        )
        .bind(project)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if !exists {
            return Err("导出任务不存在".into());
        }
        if let Some((token, _)) = lock(&self.live).get(&id) {
            token.cancel();
        }
        Ok(())
    }
}
