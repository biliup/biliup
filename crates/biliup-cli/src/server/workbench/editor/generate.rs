//! Provider-backed generation, adapted from Uncut's generate/providers.ts (MIT).
//! Remote calls are never retried automatically. Completed assets await an atomic project save.
use super::{export::generation, imports, keys, text_model};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
mod pricing;

/// A generation is handled only when its registered file is present in the saved document.
/// Reuse may assign another timeline ID without changing the generated asset's identity.
pub(crate) async fn acknowledge_import(
    conn: &mut sqlx::SqliteConnection,
    project: i64,
    document: &super::Document,
    job: &str,
    turn: Option<&str>,
) -> Result<(), super::Error> {
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT agent_turn_id FROM editor_generation_jobs WHERE id=? AND project_id=?",
    )
    .bind(job)
    .bind(project)
    .fetch_optional(&mut *conn)
    .await?
    .flatten();
    if let Some(owner) = owner {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')",
        )
        .bind(&owner)
        .fetch_one(&mut *conn)
        .await?;
        if active && turn != Some(owner.as_str()) {
            return Err(super::invalid(
                "助手正在处理此生成结果，请等待本轮完成或停止后再导入",
            ));
        }
    }
    let raw: Option<String> = sqlx::query_scalar("SELECT result_json FROM editor_generation_jobs WHERE id=? AND project_id=? AND phase='importing'")
        .bind(job).bind(project).fetch_optional(&mut *conn).await?.flatten();
    let mut result: Value = serde_json::from_str(
        &raw.ok_or_else(|| super::invalid("生成任务已处理或已取消，请刷新工程"))?,
    )?;
    let asset_id = result["source"]["assetId"]
        .as_str()
        .ok_or_else(|| super::invalid("生成素材身份无效"))?;
    let media_id = document.project["media"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|media| {
            let id = media["id"].as_str()?;
            let Some(super::MediaSource::File(source)) = document.sources.get(id) else {
                return None;
            };
            (source.asset_id == asset_id
                && media["path"] == result["media"]["path"]
                && media["kind"] == result["media"]["kind"])
                .then(|| id.to_owned())
        })
        .ok_or_else(|| super::invalid("生成结果未加入工程素材"))?;
    result["media"]["id"] = json!(media_id);
    sqlx::query("UPDATE editor_generation_jobs SET phase='done',status='已加入素材',result_json=?,updated_at=? WHERE id=?")
        .bind(serde_json::to_string(&result)?).bind(super::now_ms()).bind(job).execute(&mut *conn).await?;
    Ok(())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    generator: String,
    prompt: String,
    kind: Option<String>,
    size: Option<String>,
    seconds: Option<f64>,
    name: Option<String>,
    content: Option<String>,
    #[serde(default)]
    text_model: Option<text_model::Settings>,
    #[serde(default)]
    pixels: Option<(u32, u32)>,
}

struct Provider {
    id: &'static str,
    name: &'static str,
    key: &'static str,
    model: &'static str,
    kind: &'static str,
    sizes: &'static [&'static str],
    seconds: &'static [f64],
}
const PROVIDERS: &[Provider] = &[
    Provider {
        id: "generate.svg",
        name: "SVG / HTML",
        key: "",
        model: "",
        kind: "graphic",
        sizes: generation::SIZES,
        seconds: &[3., 5., 8., 10.],
    },
    Provider {
        id: "generate.gpt-image",
        name: "GPT Image",
        key: "openai",
        model: "gpt-image-2",
        kind: "image",
        sizes: &["1536x1024", "1024x1024", "1024x1536"],
        seconds: &[],
    },
    // Veo 3.1 preview was retired on 2026-10-22. Use the supported Gemini replacement.
    Provider {
        id: "generate.omni",
        name: "Gemini Omni Flash",
        key: "google",
        model: "gemini-omni-1.1-flash",
        kind: "video",
        sizes: &["16:9", "9:16"],
        seconds: &[],
    },
    Provider {
        id: "generate.sound",
        name: "Sound effects",
        key: "elevenlabs",
        model: "eleven_text_to_sound_v2",
        kind: "audio",
        sizes: &[],
        seconds: &[2., 5., 10., 20., 30.],
    },
];
fn provider(id: &str) -> Result<&'static Provider, String> {
    PROVIDERS
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "生成服务不存在".into())
}
fn request_stamp(req: &Request) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(req).expect("serializable generation request"))
    )
}
pub struct Jobs {
    pool: ConnectionPool,
    root: PathBuf,
    tokens: Mutex<HashMap<String, CancellationToken>>,
    stop: CancellationToken,
}
impl Jobs {
    pub fn new(pool: ConnectionPool, root: PathBuf) -> Self {
        Self {
            pool,
            root,
            tokens: Mutex::new(HashMap::new()),
            stop: CancellationToken::new(),
        }
    }
    pub async fn initialize(&self) -> sqlx::Result<()> {
        sqlx::query("UPDATE editor_generation_jobs SET phase='failed',status='生成中断',error='服务已重启，请检查供应商记录后重新生成；已提交的请求可能仍会计费',updated_at=? WHERE phase='running'")
            .bind(super::now_ms()).execute(&self.pool).await?;
        Ok(())
    }
    pub fn stop(&self) {
        self.stop.cancel();
        for token in self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
        {
            token.cancel();
        }
    }
    pub async fn shutdown(&self) {
        self.stop();
        for _ in 0..100 {
            if self
                .tokens
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub async fn catalog(&self) -> Result<Value, String> {
        let stored = keys::list(&self.pool).await?;
        let model = text_model::get(&self.pool).await?;
        let browser = super::export::available();
        Ok(Value::Array(PROVIDERS.iter().map(|p| {
            if p.kind=="graphic" {
                let keyed=stored.iter().any(|k|k["provider"]==model.provider && k["set"]==true);
                let ready=keyed&&browser.is_ok();
                return without_nulls(json!({"id":p.id,"name":p.name,"provider":model.provider,"model":model.model,"kinds":["image","video"],"ready":ready,"needsKey":!keyed,"needs":if !keyed{Some("请先设置模型服务的 API 密钥")}else{browser.as_ref().err().map(String::as_str)},"sizes":p.sizes,"seconds":p.seconds,"pricing":pricing::info(p.id,&model.model,&model.provider)}))
            }
            let ready=stored.iter().any(|k|k["provider"]==p.key && k["set"]==true);
            json!({"id":p.id,"name":p.name,"provider":p.key,"model":p.model,"kinds":[p.kind],"ready":ready,"needs":if ready{None}else{Some("请先设置 API 密钥")},"sizes":p.sizes,"seconds":p.seconds,"pricing":pricing::info(p.id,p.model,p.key)})
        }).collect()))
    }
    pub async fn list(&self, project: i64) -> Result<Value, String> {
        super::get(&self.pool, project)
            .await
            .map_err(|e| e.to_string())?;
        let rows=sqlx::query("SELECT id,request_json,phase,status,error,result_json,created_at,agent_turn_id FROM editor_generation_jobs WHERE project_id=? ORDER BY CASE WHEN phase IN ('running','importing') THEN 0 ELSE 1 END,created_at DESC LIMIT 100")
            .bind(project).fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
        rows.into_iter().map(|row| {
            let req:Request=serde_json::from_str(row.get("request_json")).map_err(|e|e.to_string())?;
            let result:Option<Value>=row.get::<Option<String>,_>("result_json").map(|s|serde_json::from_str(&s)).transpose().map_err(|e|e.to_string())?;
            Ok(without_nulls(json!({"id":row.get::<String,_>("id"),"projectDir":format!("biliup:{project}"),"generator":req.generator,"kind":req.kind,"prompt":req.prompt,"size":req.size,"seconds":req.seconds,
                "phase":row.get::<String,_>("phase"),"status":row.get::<String,_>("status"),"error":row.get::<Option<String>,_>("error"),"name":req.name,"agentTurnId":row.get::<Option<String>,_>("agent_turn_id"),
                "mediaId":result.as_ref().map(|r|&r["media"]["id"]),"startedAt":chrono::DateTime::from_timestamp_millis(row.get("created_at")).map(|t|t.to_rfc3339())})))
        }).collect::<Result<Vec<_>,String>>().map(Value::Array)
    }
    async fn prepare(&self, project: i64, mut req: Request) -> Result<(Request, String), String> {
        if req.text_model.is_some() || req.pixels.is_some() {
            return Err("生成设置由服务器确定".into());
        }
        let p = provider(&req.generator)?;
        let graphic = p.kind == "graphic";
        req.prompt = req.prompt.trim().to_string();
        if req.prompt.is_empty() || req.prompt.chars().count() > 4000 {
            return Err("请填写不超过 4000 字的生成描述".into());
        }
        if req.kind.as_deref().is_some_and(|k| {
            if graphic {
                !matches!(k, "image" | "video")
            } else {
                k != p.kind
            }
        }) {
            return Err("素材类型与生成服务不匹配".into());
        }
        req.kind = Some(
            if graphic {
                req.kind.as_deref().unwrap_or("image")
            } else {
                p.kind
            }
            .into(),
        );
        if p.sizes.is_empty() {
            if req.size.is_some() {
                return Err("此服务不接受画幅参数".into());
            }
        } else {
            let size = req.size.get_or_insert_with(|| p.sizes[0].into());
            if !p.sizes.contains(&size.as_str()) {
                return Err("生成尺寸无效".into());
            }
        }
        if p.kind == "audio" {
            let n = req.seconds.get_or_insert(5.);
            if !n.is_finite() || !(0.5..=30.).contains(n) {
                return Err("音效时长必须在 0.5 到 30 秒之间".into());
            }
        } else if graphic && req.kind.as_deref() == Some("video") {
            let seconds = req.seconds.get_or_insert(5.);
            if !seconds.is_finite() || !(0.1..=12.).contains(seconds) {
                return Err("动画时长必须在 0.1 到 12 秒之间".into());
            }
        } else if req.seconds.is_some() {
            return Err("此模型不接受固定时长，请在描述中说明视频节奏".into());
        }
        if req
            .name
            .as_ref()
            .is_some_and(|n| n.chars().count() > 120 || n.chars().any(char::is_control))
        {
            return Err("素材名称无效".into());
        }
        if !graphic && req.content.is_some() {
            return Err("此生成服务不接受图形源码".into());
        }
        if graphic {
            super::export::available()?;
            req.text_model = Some(text_model::get(&self.pool).await?);
            let project = super::get(&self.pool, project)
                .await
                .map_err(|e| e.to_string())?;
            req.pixels = Some(generation::pixels(
                req.size.as_deref().unwrap_or("canvas"),
                &project.document.project["canvas"],
            )?);
            if let Some(content) = &req.content {
                req.content = Some(generation::extract(content)?)
            }
        }
        let key = if graphic && req.content.is_some() {
            String::new()
        } else {
            keys::get(
                &self.pool,
                &self.root,
                req.text_model
                    .as_ref()
                    .map(|s| s.provider.as_str())
                    .unwrap_or(p.key),
            )
            .await?
            .ok_or("请先设置生成服务的 API 密钥")?
        };
        Ok((req, key))
    }
    pub(crate) async fn preflight(&self, project: i64, req: Request) -> Result<Value, String> {
        let (req, key) = self.prepare(project, req).await?;
        let p = provider(&req.generator)?;
        let provider = req
            .text_model
            .as_ref()
            .map(|m| m.provider.as_str())
            .unwrap_or(p.key);
        let model = req
            .text_model
            .as_ref()
            .map(|m| m.model.as_str())
            .unwrap_or(p.model);
        let pricing = pricing::info(p.id, model, provider);
        let cost = if key.is_empty() {
            "直接渲染提供的源码，不调用生成模型".into()
        } else {
            pricing.describe(req.size.as_deref(), req.seconds)
        };
        Ok(
            json!({"stamp":request_stamp(&req),"paid":!key.is_empty(),"provider":provider,"model":model,"generator":req.generator,"name":req.name,"kind":req.kind,"prompt":req.prompt,"size":req.size,"seconds":req.seconds,"cost":cost,"pricingSource":pricing.source(),"pricing":pricing}),
        )
    }
    pub(crate) async fn status(&self, project: i64, id: &str) -> Result<Value, String> {
        let row=sqlx::query("SELECT phase,status,error,result_json,agent_turn_id FROM editor_generation_jobs WHERE project_id=? AND id=?").bind(project).bind(id).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?.ok_or("生成任务不存在")?;
        let result: Option<Value> = row
            .get::<Option<String>, _>("result_json")
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| e.to_string())?;
        Ok(
            json!({"id":id,"phase":row.get::<String,_>("phase"),"status":row.get::<String,_>("status"),"error":row.get::<Option<String>,_>("error"),"result":result,"agentTurnId":row.get::<Option<String>,_>("agent_turn_id")}),
        )
    }
    pub async fn start(self: &Arc<Self>, project: i64, req: Request) -> Result<String, String> {
        self.start_owned(project, req, None, None, self.stop.child_token())
            .await
    }
    pub(crate) async fn start_agent(
        self: &Arc<Self>,
        project: i64,
        req: Request,
        turn: &str,
        stamp: &str,
        cancel: &CancellationToken,
    ) -> Result<String, String> {
        self.start_owned(project, req, Some(turn), Some(stamp), cancel.child_token())
            .await
    }
    async fn start_owned(
        self: &Arc<Self>,
        project: i64,
        req: Request,
        turn: Option<&str>,
        expected: Option<&str>,
        token: CancellationToken,
    ) -> Result<String, String> {
        if token.is_cancelled() || self.stop.is_cancelled() {
            return Err("助手已停止".into());
        }
        let (req, key) = self.prepare(project, req).await?;
        if expected.is_some_and(|stamp| stamp != request_stamp(&req)) {
            return Err("生成配置发生变化，请重新确认".into());
        }
        let request = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let id = random_id()?;
        let now = super::now_ms();
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| e.to_string())?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?)")
                .bind(project)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if !exists {
            return Err("工程不存在".into());
        }
        if let Some(turn) = turn {
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
            if !active || token.is_cancelled() {
                return Err("助手已停止".into());
            }
        }
        // An assistant retry reuses even a failed/completed request from this turn; never bills twice.
        if let Some(existing)=sqlx::query_scalar::<_,String>("SELECT id FROM editor_generation_jobs WHERE project_id=? AND request_json=? AND agent_turn_id IS ? AND (agent_turn_id IS NOT NULL OR phase IN ('running','importing')) ORDER BY created_at DESC LIMIT 1")
            .bind(project).bind(&request).bind(turn).fetch_optional(&mut *tx).await.map_err(|e|e.to_string())? {return Ok(existing)}
        let running: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM editor_generation_jobs WHERE phase='running'")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if running >= 4 {
            return Err("生成任务已满，请等待正在执行的任务完成".into());
        }
        let pending:i64=sqlx::query_scalar("SELECT COUNT(*) FROM editor_generation_jobs WHERE project_id=? AND phase IN ('running','importing')").bind(project).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
        if pending >= 10 {
            return Err("请先处理当前工程等待导入的生成结果".into());
        }
        sqlx::query("INSERT INTO editor_generation_jobs(id,project_id,request_json,phase,status,created_at,updated_at,agent_turn_id) VALUES(?,?,?,'running','正在生成',?,?,?)")
            .bind(&id).bind(project).bind(request).bind(now).bind(now).bind(turn).execute(&mut *tx).await.map_err(|e|e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), token.clone());
        if self.stop.is_cancelled() {
            token.cancel();
        }
        let jobs = self.clone();
        let job = id.clone();
        tokio::spawn(async move {
            // Cancellation can race with the response that publishes the new task ID.
            let active = sqlx::query_scalar::<_, String>(
                "SELECT phase FROM editor_generation_jobs WHERE id=?",
            )
            .bind(&job)
            .fetch_optional(&jobs.pool)
            .await;
            if !matches!(active,Ok(Some(ref phase)) if phase=="running") {
                jobs.tokens
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&job);
                return;
            }
            let result = tokio::select! {_ = token.cancelled()=>Err("已取消".to_string()), result=tokio::time::timeout(Duration::from_secs(15*60),jobs.generate(project,&job,&req,&key,token.clone()))=>result.unwrap_or_else(|_|Err("生成超时；已提交的请求可能仍会计费，请检查供应商记录".into()))};
            let now = super::now_ms();
            let update=match result {
                Ok(result)=>sqlx::query("UPDATE editor_generation_jobs SET phase='importing',status='等待加入素材',result_json=?,updated_at=? WHERE id=? AND phase='running'").bind(result.to_string()).bind(now).bind(&job).execute(&jobs.pool).await,
                Err(error)=>sqlx::query("UPDATE editor_generation_jobs SET phase=?,status=?,error=?,updated_at=? WHERE id=? AND phase='running'").bind(if token.is_cancelled(){"cancelled"}else{"failed"}).bind(if token.is_cancelled(){"已取消"}else{"生成失败"}).bind(error).bind(now).bind(&job).execute(&jobs.pool).await,
            };
            if let Err(error) = update {
                tracing::error!(%error,job_id=%job,"无法保存生成任务状态");
            }
            jobs.tokens
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&job);
        });
        Ok(id)
    }
    pub async fn cancel(&self, project: i64, id: &str) -> Result<(), String> {
        let changed=sqlx::query("UPDATE editor_generation_jobs SET phase='cancelled',status='已取消',updated_at=? WHERE project_id=? AND id=? AND phase IN ('running','importing')")
            .bind(super::now_ms()).bind(project).bind(id).execute(&self.pool).await.map_err(|e|e.to_string())?;
        if changed.rows_affected() == 0 {
            return Err("任务已结束或不存在".into());
        }
        if let Some(token) = self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
        {
            token.cancel()
        }
        Ok(())
    }
    pub async fn result(&self, project: i64, id: &str) -> Result<Value, String> {
        let result:Option<String>=sqlx::query_scalar("SELECT result_json FROM editor_generation_jobs WHERE project_id=? AND id=? AND phase='importing'")
            .bind(project).bind(id).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?.flatten();
        serde_json::from_str(&result.ok_or("没有等待导入的生成结果")?).map_err(|e| e.to_string())
    }
    async fn graphic(
        &self,
        project: i64,
        job: &str,
        req: &Request,
        key: &str,
        cancel: CancellationToken,
    ) -> Result<Value, String> {
        let (width, height) = req.pixels.ok_or("缺少图形尺寸")?;
        let video = req.kind.as_deref() == Some("video");
        let seconds = req.seconds.unwrap_or(5.);
        let model = req.text_model.as_ref().ok_or("模型设置不存在")?;
        let markup = if let Some(content) = &req.content {
            content.clone()
        } else {
            let kind = if video {
                format!("Animation, {seconds} seconds")
            } else {
                "Still picture".into()
            };
            let answer = text_model::ask(
                model,
                key,
                generation::SYSTEM,
                &format!(
                    "{kind}, {width}x{height} pixels.\nWhat to draw: {}",
                    req.prompt
                ),
            )
            .await?;
            generation::extract(&answer)?
        };
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(32);
        let pool = self.pool.clone();
        let job = job.to_string();
        let progress = tokio::spawn(async move {
            while let Some(status) = receiver.recv().await {
                let _=sqlx::query("UPDATE editor_generation_jobs SET status=?,updated_at=? WHERE id=? AND phase='running'").bind(status).bind(super::now_ms()).bind(&job).execute(&pool).await;
            }
        });
        let rendered = generation::render(
            &markup,
            video,
            (width, height),
            seconds,
            &self.root,
            &cancel,
            move |status| {
                let _ = sender.try_send(status);
            },
        )
        .await;
        let _ = progress.await;
        let (_temp, path) = rendered?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("生成文件名无效")?;
        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| e.to_string())?;
        let mut result = imports::upload(
            &self.pool,
            &self.root,
            project,
            filename,
            None,
            axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(file)),
        )
        .await?;
        result["media"]["name"] = json!(
            req.name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| req.prompt.chars().take(64).collect())
        );
        result["media"]["source"] = without_nulls(
            json!({"module":"generate.svg","prompt":req.prompt,"kind":req.kind,"size":req.size,"seconds":req.seconds,"model":if req.content.is_some(){"markup"}else{&model.model}}),
        );
        Ok(result)
    }
    async fn generate(
        &self,
        project: i64,
        job: &str,
        req: &Request,
        key: &str,
        cancel: CancellationToken,
    ) -> Result<Value, String> {
        if req.generator == "generate.svg" {
            return self.graphic(project, job, req, key, cancel).await;
        }
        let p = provider(&req.generator)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(12 * 60))
            .build()
            .map_err(|_| "无法创建生成客户端")?;
        let (url, header, credential, body, ext) = match p.key {
            "openai" => (
                "https://api.openai.com/v1/images/generations",
                "Authorization",
                format!("Bearer {key}"),
                json!({"model":p.model,"prompt":req.prompt,"size":req.size,"quality":pricing::IMAGE_QUALITY,"output_format":"png","n":1}),
                "png",
            ),
            "google" => (
                "https://generativelanguage.googleapis.com/v1beta/interactions",
                "x-goog-api-key",
                key.into(),
                json!({"model":p.model,"input":req.prompt,"background":false,"store":false,"stream":false,"response_format":{"type":"video","aspect_ratio":req.size,"resolution":"720p","delivery":"inline"}}),
                "mp4",
            ),
            _ => (
                "https://api.elevenlabs.io/v1/sound-generation?output_format=mp3_44100_128",
                "xi-api-key",
                key.into(),
                json!({"model_id":p.model,"text":req.prompt,"duration_seconds":req.seconds}),
                "mp3",
            ),
        };
        // Only the fixed provider endpoint receives the secret; no redirects or automatic POST retries.
        let response = client
            .post(url)
            .header(header, credential)
            .json(&body)
            .send()
            .await
            .map_err(|_| "生成请求失败；请求可能已提交，请检查供应商记录后重试")?;
        if !response.status().is_success() {
            return Err(format!(
                "{} 请求失败（HTTP {}），请检查密钥、额度和供应商服务状态",
                p.name,
                response.status().as_u16()
            ));
        }
        let raw = bounded(response).await?;
        let bytes = if p.kind == "audio" {
            raw
        } else {
            let response: Value = serde_json::from_slice(&raw).map_err(|_| "生成响应格式无效")?;
            let encoded = if p.kind == "image" {
                response["data"][0]["b64_json"].as_str()
            } else {
                response["steps"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|s| s["type"] == "model_output")
                    .flat_map(|s| s["content"].as_array().into_iter().flatten())
                    .find(|c| c["type"] == "video" && c["mime_type"] == "video/mp4")
                    .and_then(|c| c["data"].as_str())
            }
            .ok_or("服务没有返回生成文件，内容可能被供应商拦截")?;
            STANDARD.decode(encoded).map_err(|_| "生成文件编码无效")?
        };
        if cancel.is_cancelled() {
            return Err("已取消".into());
        }
        let filename = format!("generated-{}.{}", random_id()?, ext);
        let mut result = imports::upload(
            &self.pool,
            &self.root,
            project,
            &filename,
            None,
            axum::body::Body::from(bytes),
        )
        .await?;
        if result["media"]["kind"] != p.kind {
            return Err("服务返回的文件类型不符合请求".into());
        }
        let name = req
            .name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| req.prompt.chars().take(64).collect());
        result["media"]["name"] = json!(name);
        result["media"]["source"] = without_nulls(
            json!({"module":p.id,"prompt":req.prompt,"kind":p.kind,"size":req.size,"seconds":req.seconds,"model":p.model}),
        );
        Ok(result)
    }
}
fn without_nulls(mut value: Value) -> Value {
    if let Some(map) = value.as_object_mut() {
        map.retain(|_, v| !v.is_null());
    }
    value
}
pub(super) fn random_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "无法生成任务标识")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
async fn bounded(mut response: reqwest::Response) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "生成结果下载中断")? {
        if bytes.len() + chunk.len() > 64 * 1024 * 1024 {
            return Err("生成结果超过 64 MiB 限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err("生成服务返回空文件".into());
    }
    Ok(bytes)
}
