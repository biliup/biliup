//! Download/import tools use the same media registration and edit journal as manual imports.
use super::super::{self as editor, Error, downloads, invalid, text_model::conversation::Tool};
use super::{
    plan, planner,
    service::{Access, RunContext, Service},
    store,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
use tokio_util::sync::CancellationToken;
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-download-tools.json"
    )))
    .expect("bundled download tools")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&t["inputSchema"]).expect("download schema"),
            )
        })
        .collect()
});
static TRANSFERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
pub fn tools() -> Vec<Tool> {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| Tool {
            name: t["name"].as_str().unwrap().into(),
            description: t["description"].as_str().unwrap().into(),
            input_schema: t["inputSchema"].clone(),
        })
        .collect()
}
impl Service {
    pub(super) async fn download_tool(
        &self,
        scope: super::Turn<'_>,
        context: &mut RunContext,
        name: &str,
        args: &Value,
        data: &mut Value,
    ) -> Result<Value, Error> {
        let super::Turn {
            project,
            id: turn,
            cancel,
        } = scope;
        VALIDATORS
            .get(name)
            .ok_or_else(|| invalid("未知素材导入工具"))?
            .validate(args)
            .map_err(|e| Error::Invalid(format!("素材导入参数无效：{e}")))?;
        if name == "media_downloads" {
            if let Some(id) = args["job"].as_str() {
                let mut state = downloads::get(&self.pool, project, id).await?;
                state.as_object_mut().unwrap().remove("owner");
                return Ok(state);
            }
            return Ok(json!({"jobs":downloads::list(&self.pool,project).await?}));
        }
        self.import_job(project, turn, context, args, cancel, data)
            .await
    }
    pub(super) async fn import_job(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        args: &Value,
        cancel: &CancellationToken,
        data: &mut Value,
    ) -> Result<Value, Error> {
        if context.access == Access::Look {
            return Err(invalid("只读模式不能下载或导入素材"));
        }
        let source = args["url"].as_str().or_else(|| args["path"].as_str());
        // Use the same decision cache as other resource writes so a refusal is not retried in this turn.
        let decision = format!(
            "download:{}:{}",
            args["job"].as_str().or(source).unwrap_or(""),
            args["name"].as_str().unwrap_or("")
        );
        if context.create_decisions.get(&decision) == Some(&false) {
            return Ok(json!({"imported":false,"reason":"用户已拒绝本次导入，不要自动重试"}));
        }
        let old = if let Some(job) = args["job"].as_str() {
            Some(downloads::get(&self.pool, project, job).await?)
        } else {
            None
        };
        let saved = editor::get(&self.pool, project).await?;
        if old.as_ref().is_some_and(|old| {
            old["source"]
                .as_str()
                .is_some_and(|source| source.starts_with("capture:"))
                && !matches!(old["phase"].as_str(), Some("ready" | "done"))
        }) {
            return Ok(
                json!({"job":args["job"],"imported":false,"phase":old.as_ref().map(|job|&job["phase"]),"error":old.as_ref().map(|job|&job["error"]),"message":"录制尚未产生可导入的文件，不能作为网络下载重试；可从浏览器录制恢复中保存原文件"}),
            );
        }
        if old.as_ref().is_some_and(|old| old["phase"] == "done") {
            return Ok(handled(old.as_ref().unwrap(), &saved.document));
        }
        if context.access == Access::Ask && !context.create_decisions.contains_key(&decision) {
            let source = source
                .or_else(|| old.as_ref().and_then(|old| old["source"].as_str()))
                .unwrap_or("");
            let summary = if source.starts_with("https://") {
                "下载并导入素材"
            } else {
                "导入素材"
            };
            let approved = self
                .confirm(
                    project,
                    turn,
                    summary,
                    &[source.to_string()],
                    saved.summary.revision,
                    cancel,
                )
                .await?;
            context.create_decisions.insert(decision.clone(), approved);
            if !approved {
                return Ok(json!({"imported":false,"reason":"用户未批准本次导入，不要自动重试"}));
            }
        }
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        let mut state = downloads::prepare(
            &self.pool,
            project,
            turn,
            source,
            args["name"].as_str(),
            args["job"].as_str(),
            args["retry"].as_bool().unwrap_or(false),
        )
        .await?;
        let id = state["id"].as_str().unwrap().to_string();
        if state["phase"] == "done" {
            return Ok(handled(&state, &saved.document));
        }
        if matches!(state["phase"].as_str(), Some("failed" | "cancelled")) {
            return Ok(
                json!({"job":id,"phase":state["phase"],"error":state["error"],"imported":false,"note":"未自动重试；用户要求重试时显式传 retry:true。"}),
            );
        }
        let index = data["items"]
            .as_array()
            .and_then(|items| items.len().checked_sub(1));
        if let Some(i) = index {
            data["items"][i]["downloadJob"] = json!(id);
            store::update(&self.pool, turn, data).await?;
        }
        if state["phase"] == "queued" {
            let _permit = tokio::select! {_=cancel.cancelled()=>return Err(invalid("助手已停止")),p=TRANSFERS.acquire()=>p.map_err(|_|invalid("下载服务已停止"))?};
            let updated=sqlx::query("UPDATE editor_downloads SET phase='running',error=NULL,updated_at=? WHERE id=? AND phase='queued' AND agent_turn_id=? AND EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')").bind(editor::now_ms()).bind(&id).bind(turn).bind(turn).execute(&self.pool).await?;
            if updated.rows_affected() != 1 {
                return Err(invalid("下载任务状态已变化，请重新读取"));
            }
            let (progress, rx) = tokio::sync::watch::channel((0u64, None::<u64>));
            let before = context.download_budget.bytes;
            let result = {
                let transfer = downloads::transfer(
                    editor::Storage {
                        pool: &self.pool,
                        root: &self.root,
                    },
                    project,
                    &state,
                    &mut context.download_budget,
                    cancel,
                    &self.library,
                    move |bytes, total| {
                        let _ = progress.send((bytes, total));
                    },
                );
                tokio::pin!(transfer);
                let timeout = tokio::time::sleep(Duration::from_secs(300));
                tokio::pin!(timeout);
                loop {
                    tokio::select! {
                        result=&mut transfer=>break result,
                        _=cancel.cancelled()=>break Err(invalid("助手已停止下载；可用 job 和 retry:true 重试")),
                        _=&mut timeout=>break Err(invalid("素材下载或解析超时")),
                        _=tokio::time::sleep(Duration::from_millis(500))=>{
                            let (bytes,total)=*rx.borrow();
                            if let Some(i)=index{
                                let text=if total.is_some_and(|n|n==bytes){"正在登记素材".to_string()}else{format!("正在下载素材 · {:.1} MiB{}",bytes as f64/1048576.,total.filter(|n|*n>0).map(|total|format!(" / {:.1} MiB",total as f64/1048576.)).unwrap_or_default())};
                                data["items"][i]["text"]=json!(text);
                                if let Err(error)=store::update(&self.pool,turn,data).await{break Err(error)}
                            }
                        }
                    }
                }
            };
            let bytes = rx.borrow().0 as i64;
            context.download_budget.bytes = context
                .download_budget
                .bytes
                .max(before.saturating_add(bytes.max(0) as u64));
            match result {
                Ok(result) => {
                    sqlx::query("UPDATE editor_downloads SET phase='ready',result_json=?,bytes=?,error=NULL,updated_at=? WHERE id=? AND agent_turn_id=?").bind(serde_json::to_string(&result)?).bind(bytes).bind(editor::now_ms()).bind(&id).bind(turn).execute(&self.pool).await?;
                }
                Err(error) => {
                    let phase = if cancel.is_cancelled() {
                        "cancelled"
                    } else {
                        "failed"
                    };
                    sqlx::query("UPDATE editor_downloads SET phase=?,bytes=?,error=?,updated_at=? WHERE id=? AND agent_turn_id=?").bind(phase).bind(bytes).bind(error.to_string()).bind(editor::now_ms()).bind(&id).bind(turn).execute(&self.pool).await?;
                    return Ok(
                        json!({"job":id,"phase":phase,"error":error.to_string(),"imported":false}),
                    );
                }
            }
            state = downloads::get(&self.pool, project, &id).await?;
        }
        if cancel.is_cancelled() {
            return Ok(
                json!({"job":id,"phase":"ready","imported":false,"note":"助手已停止；下载结果保留，可用 job 继续导入。"}),
            );
        }
        let mut result = state["result"].clone();
        if result["font"] == true {
            let path = downloads::staged_path(
                &self.root,
                &id,
                result["extension"]
                    .as_str()
                    .ok_or_else(|| invalid("字体结果格式无效"))?,
            )?;
            let bytes =
                editor::resources::importer::read(&path, editor::project_fonts::MAX_BYTES).await?;
            let input = editor::project_fonts::Upload {
                name: result["name"].as_str().unwrap().to_string(),
                family: None,
                weight: None,
                style: None,
            };
            match editor::project_fonts::upload_agent(
                &self.pool, &self.root, project, input, &bytes, turn, &id,
            )
            .await
            {
                Ok(fonts) => {
                    let _ = tokio::fs::remove_file(path).await;
                    return Ok(
                        json!({"job":id,"phase":"done","imported":true,"fonts":fonts,"note":"字体已加入工程字体库，可直接用于文字；字体资源不属于时间轴撤销。"}),
                    );
                }
                Err(error) => return Ok(ready(&id, error)),
            }
        }
        let saved = editor::get(&self.pool, project).await?;
        let (mut media, existing) =
            editor::imports::prepare_media(&saved.document.project, &result["media"])?;
        if media["id"] != result["media"]["id"] {
            result["media"]["id"] = media["id"].clone();
            let updated=sqlx::query("UPDATE editor_downloads SET result_json=?,updated_at=? WHERE id=? AND project_id=? AND agent_turn_id=? AND phase='ready' AND EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')").bind(serde_json::to_string(&result)?).bind(editor::now_ms()).bind(&id).bind(project).bind(turn).bind(turn).execute(&self.pool).await?;
            if updated.rows_affected() != 1 {
                return Ok(ready(&id, invalid("助手已停止或下载结果已处理")));
            }
        }
        let media_id = media["id"]
            .as_str()
            .ok_or_else(|| invalid("下载素材标识无效"))?
            .to_string();
        if let Some(duration) = result["source"]["durationMs"].as_f64() {
            let fps = saved.document.project["canvas"]["fps"]
                .as_f64()
                .ok_or_else(|| invalid("工程帧率无效"))?;
            media["probe"]["frames"] = json!(((duration * fps / 1000.).floor() as i64).max(1));
        }
        let previous = existing.then(|| {
            saved.document.project["media"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == media_id)
                .unwrap()
        });
        // Reusing a local file through the library adds its registered attribution once,
        // without replacing the user's name or an existing source's provenance.
        let attach_source = previous.is_some_and(|entry| !entry["source"].is_object())
            && media["source"]["module"]
                .as_str()
                .is_some_and(|module| module.starts_with("library."));
        if existing && !attach_source {
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_projects p JOIN editor_agent_turns t ON t.project_id=p.id WHERE p.id=? AND p.revision=? AND t.id=? AND t.status='running')").bind(project).bind(saved.summary.revision).bind(turn).fetch_one(&mut *tx).await?;
            if !current {
                return Ok(ready(&id, invalid("工程版本已变化或助手已停止")));
            }
            downloads::acknowledge(&mut tx, project, &saved.document, &id, turn).await?;
            tx.commit().await?;
            return Ok(
                json!({"job":id,"phase":"done","imported":true,"id":media_id,"media":previous,"revision":saved.summary.revision,"note":"相同文件已经在工程素材中，未重复添加。"}),
            );
        }
        let edit = if attach_source {
            json!({"summary":format!("补充音源署名：{}",previous.unwrap()["name"].as_str().unwrap_or(&media_id)),"ops":[{"op":"update_media","media":media_id,"set":{"source":media["source"]}}]})
        } else {
            json!({"summary":format!("导入素材：{}",media["name"].as_str().unwrap_or(&media_id)),"ops":[{"op":"add_media","media":media}]})
        };
        let sources = if existing {
            Value::Null
        } else {
            json!({&media_id:result["source"]})
        };
        let current_plan = plan::read(&self.pool, project).await?;
        if let Err(error) = plan::check_build(&current_plan, context.planning, &edit["ops"]) {
            return Ok(ready(&id, error));
        }
        let plan_revision = current_plan["revision"].as_i64().unwrap();
        let prepared = match planner::edit_with_sources(
            &self.pool,
            project,
            saved.summary.revision,
            turn,
            &edit,
            &sources,
            cancel,
        )
        .await
        {
            Ok(p) => p,
            Err(error) => return Ok(ready(&id, error)),
        };
        if cancel.is_cancelled() {
            return Ok(ready(&id, invalid("助手已停止")));
        }
        match editor::save_agent_download(
            &self.pool,
            (project, prepared.base_revision),
            prepared.document,
            &id,
            store::Journal::Edit {
                turn,
                summary: &prepared.summary,
                lines: &prepared.lines,
                plan_revision,
            },
        )
        .await
        {
            Ok(saved) => {
                let imported = saved.document.project["media"]
                    .as_array()
                    .and_then(|all| all.iter().find(|media| media["id"] == media_id))
                    .ok_or(Error::NotFound)?;
                Ok(
                    json!({"job":id,"phase":"done","applied":true,"imported":true,"id":media_id,"media":imported,"canvas":saved.document.project["canvas"],"revision":saved.summary.revision,"note":"素材已加入当前工程，可用 timeline_edit 放入时间轴；导入包含在本轮撤销中。"}),
                )
            }
            Err(error) => Ok(ready(&id, error)),
        }
    }
}
fn ready(id: &str, error: Error) -> Value {
    json!({"job":id,"phase":"ready","imported":false,"error":error.to_string(),"note":"下载结果保留；用 job 参数继续导入，无需重新下载。"})
}
fn handled(state: &Value, document: &editor::Document) -> Value {
    let media = &state["result"]["media"];
    let included = document.project["media"].as_array().is_some_and(|all| {
        all.iter()
            .any(|m| m["id"] == media["id"] && m["path"] == media["path"])
    });
    json!({"job":state["id"],"phase":"done","imported":if state["result"]["font"]==true{Value::Null}else{json!(included)},"id":media["id"],"note":"此任务已经处理；没有重复导入，也不会恢复之后被删除或撤销的素材或字体。"})
}
