//! Browser-owned screen recording; reuse the normal importer, durable receipt and agent journal.
use super::super::{self as editor, Error, text_model::conversation::Tool};
use super::service::{Access, RunContext, Service};
use crate::server::api::access::Caller;
use crate::server::infrastructure::permissions::Role;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{sync::LazyLock, time::Duration};
use tokio_util::sync::CancellationToken;

static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-capture-tools.json"
    )))
    .expect("capture tools")
});
static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    jsonschema::validator_for(&SPEC["tools"][0]["inputSchema"]).expect("capture schema")
});
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
pub fn tools() -> Vec<Tool> {
    vec![Tool {
        name: "screen_record".into(),
        description: SPEC["tools"][0]["description"].as_str().unwrap().into(),
        input_schema: SPEC["tools"][0]["inputSchema"].clone(),
    }]
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub id: String,
    pub path: Option<String>,
    pub source: Option<String>,
    pub problem: Option<String>,
}
impl Service {
    pub(super) async fn screen_record(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        args: &Value,
        cancel: &CancellationToken,
        data: &mut Value,
    ) -> Result<Value, Error> {
        SCHEMA
            .validate(args)
            .map_err(|e| invalid(format!("录屏参数无效：{e}")))?;
        if context.access == Access::Look {
            return Err(invalid("只读模式不能录制或导入素材"));
        }
        if let Some(job) = args["job"].as_str() {
            let saved = editor::downloads::get(&self.pool, project, job).await?;
            if !saved["source"]
                .as_str()
                .is_some_and(|source| source.starts_with("capture:"))
            {
                return Err(invalid("这不是屏幕录制任务"));
            }
            if !matches!(saved["phase"].as_str(), Some("ready" | "done")) {
                return Ok(
                    json!({"job":job,"recorded":false,"phase":saved["phase"],"error":saved["error"],"message":"录制尚未产生可导入的文件；不要把它当作网络下载重试"}),
                );
            }
            return self
                .import_job(project, turn, context, &json!({"job":job}), cancel, data)
                .await;
        }
        let decision = format!("screen:{}", args);
        if context.create_decisions.get(&decision) == Some(&false) {
            return Ok(
                json!({"recorded":false,"message":"用户已拒绝或未完成这次录制，不要重复请求"}),
            );
        }
        let source = format!(
            "capture:{}",
            editor::generate::random_id().map_err(invalid)?
        );
        let job = editor::downloads::prepare(
            &self.pool,
            project,
            turn,
            Some(&source),
            Some("屏幕录制"),
            None,
            false,
        )
        .await?;
        let id = job["id"].as_str().unwrap().to_owned();
        sqlx::query(
            "UPDATE editor_downloads SET phase='capture-waiting' WHERE id=? AND phase='queued'",
        )
        .bind(&id)
        .execute(&self.pool)
        .await?;
        let seconds = args["seconds"].as_f64().unwrap();
        let suggested=args["source"].as_str().map(str::trim).filter(|s|!s.is_empty());
        let label=suggested.map(|s|format!("屏幕（{s}）")).unwrap_or_else(||"屏幕".into());
        let mic=if args["withMic"]==true{"（含麦克风）"}else{""};
        let work=async{
            let approval=json!({"id":id,"text":format!("录制{label} {seconds} 秒{mic}并添加到素材"),"screen":{"source":args["source"],"seconds":seconds,"withMic":args["withMic"]==true}});
            let allowed=self.wait_approval(project,turn,approval,120,cancel).await?;
            if !allowed.is_some_and(|answer|answer.allow){context.create_decisions.insert(decision.clone(),false);return Ok(json!({"recorded":false,"job":id,"message":"用户没有同意录制或确认已超时，未录制；不要自动再次请求"}))}
            // A picker failure may already have delivered a terminal answer while the card was open.
            sqlx::query("UPDATE editor_downloads SET phase='capture-recording',updated_at=? WHERE id=? AND project_id=? AND phase='capture-waiting' AND EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')")
                .bind(editor::now_ms()).bind(&id).bind(project).bind(turn).execute(&self.pool).await?;
            let wait=async{
                loop{
                    let job=editor::downloads::get(&self.pool,project,&id).await?;
                    match job["phase"].as_str(){
                        Some("ready"|"done")=>return Ok::<_,Error>(()),
                        Some("failed"|"cancelled")=>return Err(invalid(job["error"].as_str().unwrap_or("录制已停止"))),
                        _=>{},
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            };
            tokio::select!{_=cancel.cancelled()=>return Err(invalid("助手已停止等待录制")),result=tokio::time::timeout(Duration::from_secs(seconds.ceil() as u64+600),wait)=>result.map_err(|_|invalid("等待录制上传超时，未加入工程；可从录制恢复中保存原文件"))??};
            // This card explicitly approved recording and adding it to Media; don't ask again for the same import.
            context.create_decisions.insert(format!("download:{id}:"),true);
            self.import_job(project,turn,context,&json!({"job":id}),cancel,data).await
        }.await;
        sqlx::query("UPDATE editor_downloads SET phase='cancelled',error=COALESCE(error,'录制请求结束，未收到完整文件'),updated_at=? WHERE id=? AND phase IN ('capture-waiting','capture-recording','capture-uploading')")
            .bind(editor::now_ms()).bind(&id).execute(&self.pool).await?;
        work
    }

    /// Only the active turn's initiating user (or administrator) may send a recording.
    pub async fn upload_screen(
        &self,
        project: i64,
        id: &str,
        name: &str,
        device: Option<&str>,
        caller: &Caller,
        body: axum::body::Body,
    ) -> Result<Value, Error> {
        let (turn, cancel) = self.capture_owner(project, caller)?;
        let device=device.map(str::trim).filter(|name|!name.is_empty());
        if device.is_some_and(|name|name.chars().count()>120 || name.chars().any(char::is_control)){
            return Err(invalid("录制来源名称无效"));
        }
        let mut job = editor::downloads::get(&self.pool, project, id).await?;
        if job["owner"] != turn
            || !job["source"]
                .as_str()
                .is_some_and(|source| source.starts_with("capture:"))
        {
            return Err(invalid("录制不属于本轮助手请求"));
        }
        // Confirmation is synchronous at the API boundary, while the waiting tool commits its phase asynchronously.
        for _ in 0..50 {
            if job["phase"] != "capture-waiting" {
                break;
            }
            tokio::select! {_=cancel.cancelled()=>return Err(invalid("助手已停止")),_=tokio::time::sleep(Duration::from_millis(100))=>{}}
            job = editor::downloads::get(&self.pool, project, id).await?;
        }
        if job["phase"] == "ready" || job["phase"] == "done" {
            return Ok(json!({"path":format!("agent-capture:{id}")}));
        }
        let claimed=sqlx::query("UPDATE editor_downloads SET phase='capture-uploading',updated_at=? WHERE id=? AND project_id=? AND agent_turn_id=? AND phase='capture-recording'")
            .bind(editor::now_ms()).bind(id).bind(project).bind(&turn).execute(&self.pool).await?;
        if claimed.rows_affected() != 1 {
            return Err(invalid("录制请求已结束或文件正在上传"));
        }
        let work = async {
            let mut entry = editor::imports::upload(
                &self.pool,
                &self.root,
                project,
                name,
                Some("screen"),
                body,
            )
            .await
            .map_err(invalid)?;
            if entry["media"]["kind"] != "video" {
                return Err(invalid("录屏必须包含视频画面"));
            }
            let take = format!(
                "{}-{}-{}-{}-{}",
                &id[..8],
                &id[8..12],
                &id[12..16],
                &id[16..20],
                &id[20..]
            );
            entry["media"]["id"] = json!(format!("capture-{take}-screen"));
            entry["media"]["source"] =
                json!({"module":"capture.browser","takeId":take,"kind":"screen"});
            if let Some(device)=device{
                entry["media"]["name"]=json!(device);
                entry["media"]["source"]["device"]=json!(device);
            }
            let updated=sqlx::query("UPDATE editor_downloads SET phase='ready',result_json=?,error=NULL,updated_at=? WHERE id=? AND project_id=? AND agent_turn_id=? AND phase='capture-uploading' AND EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')")
                .bind(entry.to_string()).bind(editor::now_ms()).bind(id).bind(project).bind(&turn).bind(&turn).execute(&self.pool).await?;
            if updated.rows_affected() != 1 {
                return Err(invalid("录制请求已结束，文件未加入工程"));
            }
            Ok(json!({"path":format!("agent-capture:{id}")}))
        };
        let result = tokio::select! {_=cancel.cancelled()=>Err(invalid("助手已停止上传录制")),result=work=>result};
        if result.is_err() {
            sqlx::query("UPDATE editor_downloads SET phase='failed',error='录制上传未完成，原文件保留在浏览器录制恢复中',updated_at=? WHERE id=? AND phase='capture-uploading'").bind(editor::now_ms()).bind(id).execute(&self.pool).await?;
        }
        result
    }
    pub async fn screen_answer(
        &self,
        project: i64,
        answer: Answer,
        caller: &Caller,
    ) -> Result<(), Error> {
        let job = editor::downloads::get(&self.pool, project, &answer.id).await?;
        let turn = job["owner"]
            .as_str()
            .ok_or_else(|| invalid("录制请求无效"))?;
        let owner: Option<Option<i64>> = sqlx::query_scalar(
            "SELECT owner_id FROM editor_agent_turns WHERE id=? AND project_id=?",
        )
        .bind(turn)
        .bind(project)
        .fetch_optional(&self.pool)
        .await?;
        if owner.is_none()
            || (caller.subject.role != Role::Admin && owner.flatten() != caller.subject.user_id)
        {
            return Err(invalid("只有发起者或管理员可以提供录制"));
        }
        if !job["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("capture:"))
        {
            return Err(invalid("录制请求无效"));
        }
        if answer.source.as_ref().is_some_and(|name| name.len() > 1000) {
            return Err(invalid("录制来源名称过长"));
        }
        if let Some(problem) = answer.problem {
            sqlx::query("UPDATE editor_downloads SET phase='failed',error=?,updated_at=? WHERE id=? AND phase IN ('capture-waiting','capture-recording','capture-uploading')")
                .bind(problem.chars().take(1000).collect::<String>()).bind(editor::now_ms()).bind(&answer.id).execute(&self.pool).await?;
            self.capture_failed(project, &answer.id);
            return Ok(());
        }
        if answer.path.as_deref() != Some(format!("agent-capture:{}", answer.id).as_str())
            || !matches!(job["phase"].as_str(), Some("ready" | "done"))
        {
            return Err(invalid("录制文件尚未上传完成"));
        }
        Ok(())
    }
}
