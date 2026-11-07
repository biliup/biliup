//! Source creation and generation reuse the editor's storage, queue and atomic edit journal.
use super::super::{self as editor, Error, text_model::conversation::Tool};
use super::{
    plan, planner,
    service::{Access, RunContext, Service},
    store,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
use tokio_util::sync::CancellationToken;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-create-tools.json"
    )))
    .expect("bundled creation tools")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&tool["inputSchema"]).expect("valid creation schema"),
            )
        })
        .collect()
});
pub(super) fn validate(name: &str, args: &Value) -> Result<(), Error> {
    if serde_json::to_vec(args)?.len() > 2 * 1024 * 1024 {
        return Err(invalid("创建请求过大"));
    }
    VALIDATORS
        .get(name)
        .ok_or_else(|| invalid("未知创建工具"))?
        .validate(args)
        .map_err(|e| invalid(format!("创建参数无效：{e}")))
}
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
    pub(super) async fn create_tool(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        name: &str,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        if serde_json::to_vec(args)?.len() > 2 * 1024 * 1024 {
            return Err(invalid("创建请求过大"));
        }
        VALIDATORS
            .get(name)
            .ok_or_else(|| invalid("未知创建工具"))?
            .validate(args)
            .map_err(|e| invalid(format!("创建参数无效：{e}")))?;
        if name == "generate_media" && args.as_object().is_some_and(|v| v.is_empty()) {
            return self.generation.catalog().await.map_err(invalid);
        }
        if name == "graphic_read" {
            let saved = editor::get(&self.pool, project).await?;
            let value =
                editor::graphics::read(&self.pool, project, args["src"].as_str().unwrap()).await?;
            let html = value["html"].as_str().unwrap();
            let count = html.chars().count();
            let from = args["from"].as_u64().unwrap_or(0) as usize;
            if from > count {
                return Err(invalid("图形源码起点超出长度"));
            }
            let page = html.chars().skip(from).take(24_000).collect::<String>();
            let end = from + page.chars().count();
            return Ok(
                json!({"src":value["src"],"name":value["name"],"html":page,"from":from,"characters":count,"next":if end<count{Some(end)}else{None},"revision":saved.summary.revision}),
            );
        }
        if context.access == Access::Look {
            return Err(invalid("本轮为只读模式"));
        }
        if name == "graphic_write" {
            return self
                .graphic_tool(project, turn, context, args, cancel)
                .await;
        }
        let (job, approved) = if let Some(job) = args["job"].as_str() {
            let state = self
                .generation
                .status(project, job)
                .await
                .map_err(invalid)?;
            if let Some(owner) = state["agentTurnId"].as_str().filter(|owner| *owner != turn) {
                let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND status='running')").bind(owner).fetch_one(&self.pool).await?;
                if active {
                    return Err(invalid("另一个助手轮次正在处理此任务"));
                }
            }
            (job.to_string(), false)
        } else {
            let request: editor::generate::Request = serde_json::from_value(args.clone())?;
            let proposed = self
                .generation
                .preflight(project, request.clone())
                .await
                .map_err(invalid)?;
            let stamp = proposed["stamp"].as_str().unwrap();
            let needs_confirm = context.access == Access::Ask || proposed["paid"] == true;
            if needs_confirm {
                let allowed = if let Some(allowed) = context.create_decisions.get(stamp) {
                    *allowed
                } else {
                    let saved = editor::get(&self.pool, project).await?;
                    let label = if proposed["paid"] == true {
                        format!(
                            "使用 {} 生成素材",
                            proposed["model"].as_str().unwrap_or("模型服务")
                        )
                    } else {
                        "生成素材".into()
                    };
                    let mut lines =
                        vec![proposed["prompt"].as_str().unwrap_or_default().to_string()];
                    if let Some(size) = proposed["size"].as_str() {
                        lines.push(format!("画幅：{size}"));
                    }
                    if let Some(seconds) = proposed["seconds"].as_f64() {
                        lines.push(format!("时长：{seconds} 秒"));
                    }
                    lines.push(
                        proposed["cost"]
                            .as_str()
                            .ok_or_else(|| invalid("生成费用信息缺失"))?
                            .to_string(),
                    );
                    if proposed["paid"] == true {
                        lines.push(
                            proposed["pricingSource"]
                                .as_str()
                                .ok_or_else(|| invalid("生成费率来源缺失"))?
                                .to_string(),
                        );
                    }
                    let allowed = self
                        .confirm(
                            project,
                            turn,
                            &label,
                            &lines,
                            saved.summary.revision,
                            cancel,
                        )
                        .await?;
                    context.create_decisions.insert(stamp.into(), allowed);
                    allowed
                };
                if !allowed {
                    return Ok(
                        json!({"generated":false,"reason":"用户未批准此生成请求；不要自动重试"}),
                    );
                }
            }
            let job = self
                .generation
                .start_agent(project, request, turn, stamp, cancel)
                .await
                .map_err(invalid)?;
            (job, needs_confirm)
        };
        let until = tokio::time::Instant::now() + Duration::from_secs(15 * 60 + 15);
        let state = loop {
            let state = self
                .generation
                .status(project, &job)
                .await
                .map_err(invalid)?;
            if state["phase"] != "running" {
                break state;
            }
            tokio::select! {
                _=cancel.cancelled()=>return Err(invalid("助手已停止等待；任务进度可在生成素材中查看，已提交的供应商请求仍可能计费")),
                _=tokio::time::sleep_until(until)=>return Ok(json!({"job":job,"phase":"running","imported":false,"note":"任务仍在运行，可用 job 参数继续等待；不要重新提交生成请求。"})),
                _=tokio::time::sleep(Duration::from_millis(500))=>{}
            }
        };
        if !matches!(state["phase"].as_str(), Some("importing" | "done")) {
            return Ok(
                json!({"job":job,"phase":state["phase"],"error":state["error"],"imported":false,"note":"没有加入素材；未自动重试供应商请求。"}),
            );
        }
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止；生成结果保留在生成素材中，可手动加入"));
        }
        let saved = editor::get(&self.pool, project).await?;
        let generated = &state["result"]["media"];
        let generated_id = generated["id"]
            .as_str()
            .ok_or_else(|| invalid("生成素材身份无效"))?
            .to_string();
        if state["phase"] == "done" {
            let included = saved.document.project["media"]
                .as_array()
                .is_some_and(|all| {
                    all.iter()
                        .any(|m| m["id"] == generated_id && m["path"] == generated["path"])
                });
            return Ok(
                json!({"job":job,"phase":"done","imported":included,"id":generated_id,"revision":saved.summary.revision,"note":if included{"素材已经加入；未重复生成或修改工程。"}else{"此任务已处理，其素材之后被删除或替换；没有重新加入。"}}),
            );
        }
        let (mut media, existing) =
            editor::imports::prepare_media(&saved.document.project, generated)?;
        let id = media["id"]
            .as_str()
            .ok_or_else(|| invalid("生成素材身份无效"))?
            .to_owned();
        if existing {
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_projects p JOIN editor_agent_turns t ON t.project_id=p.id WHERE p.id=? AND p.revision=? AND t.id=? AND t.status='running')")
                .bind(project).bind(saved.summary.revision).bind(turn).fetch_one(&mut *tx).await?;
            if !current || cancel.is_cancelled() {
                return Err(invalid("工程版本已变化或助手已停止"));
            }
            editor::generate::acknowledge_import(
                &mut tx,
                project,
                &saved.document,
                &job,
                Some(turn),
            )
            .await?;
            tx.commit().await?;
            return Ok(
                json!({"job":job,"phase":"done","imported":true,"id":id,"media":media,"revision":saved.summary.revision,"note":"相同文件已在工程素材中，未重复添加或创建撤销步骤。"}),
            );
        }
        if let Some(duration) = state["result"]["source"]["durationMs"].as_f64() {
            let fps = saved.document.project["canvas"]["fps"]
                .as_f64()
                .ok_or_else(|| invalid("工程帧率无效"))?;
            media["probe"]["frames"] = json!(((duration * fps / 1000.).floor() as i64).max(1));
        }
        let edit = json!({"summary":format!("生成素材：{}",media["name"].as_str().unwrap_or(&id)),"ops":[{"op":"add_media","media":media}]});
        let current_plan = plan::read(&self.pool, project).await?;
        plan::check_build(&current_plan, context.planning, &edit["ops"])?;
        let plan_revision = current_plan["revision"].as_i64().unwrap();
        let prepared = match planner::edit_with_sources(
            &self.pool,
            project,
            saved.summary.revision,
            turn,
            &edit,
            &json!({&id:state["result"]["source"]}),
            cancel,
        )
        .await
        {
            Ok(prepared) => prepared,
            Err(error) => {
                return Ok(
                    json!({"job":job,"phase":"importing","imported":false,"error":error.to_string(),"note":"生成结果保留；用 job 参数重试导入，不会重新调用供应商。"}),
                );
            }
        };
        if context.access == Access::Ask
            && !approved
            && !self
                .confirm(
                    project,
                    turn,
                    &prepared.summary,
                    &prepared.lines,
                    saved.summary.revision,
                    cancel,
                )
                .await?
        {
            return Ok(
                json!({"job":job,"phase":"importing","imported":false,"reason":"用户未批准加入素材；结果保留，不要自动重试"}),
            );
        }
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止；生成结果仍可手动加入素材"));
        }
        let committed = editor::save_agent_generated(
            &self.pool,
            (project, prepared.base_revision),
            prepared.document,
            std::slice::from_ref(&job),
            store::Journal::Edit {
                turn,
                summary: &prepared.summary,
                lines: &prepared.lines,
                plan_revision,
            },
        )
        .await;
        match committed {
            Ok(saved) => {
                let imported = saved.document.project["media"]
                    .as_array()
                    .and_then(|all| all.iter().find(|media| media["id"] == id))
                    .ok_or(Error::NotFound)?;
                Ok(
                    json!({"job":job,"phase":"done","applied":true,"imported":true,"id":id,"media":imported,"canvas":saved.document.project["canvas"],"revision":saved.summary.revision,"note":"已加入统一素材库；可通过 timeline_edit 放入时间轴，整轮撤销会移除本次导入。"}),
                )
            }
            Err(error) => Ok(
                json!({"job":job,"phase":"importing","imported":false,"error":error.to_string(),"note":"生成结果已保留；用 job 参数重试导入，不会重新调用供应商。"}),
            ),
        }
    }
    async fn graphic_tool(
        &self,
        project: i64,
        turn: &str,
        context: &RunContext,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        let saved = editor::get(&self.pool, project).await?;
        let prepared =
            editor::graphics::prepare(&self.pool, project, serde_json::from_value(args.clone())?)
                .await?;
        if context.access == Access::Ask
            && !self
                .confirm(
                    project,
                    turn,
                    "保存动态图形",
                    std::slice::from_ref(&prepared.name),
                    saved.summary.revision,
                    cancel,
                )
                .await?
        {
            return Ok(json!({"saved":false,"reason":"用户未批准保存图形；不要自动重试"}));
        }
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        if revision != Some(saved.summary.revision) {
            return Err(Error::Conflict);
        }
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
        if !active || cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        sqlx::query("INSERT OR IGNORE INTO editor_graphics(project_id,src,name,html,created_at) VALUES(?,?,?,?,?)").bind(project).bind(&prepared.src).bind(&prepared.name).bind(&prepared.html).bind(editor::now_ms()).execute(&mut *tx).await?;
        tx.commit().await?;
        let has_render = regex::Regex::new(r"\brender\s*(=|\(|:)|function\s+render\b")
            .unwrap()
            .is_match(&prepared.html);
        Ok(
            json!({"saved":true,"src":prepared.src,"name":prepared.name,"revision":saved.summary.revision,"hasRender":has_render,"next":"使用返回的 src 添加 graphic 时间轴片段，再用 render_frames 检查实际画面。文件不可变，之前的工程版本不会受影响。", "note":if has_render{"源码已保存，尚未执行画面验证。"}else{"未发现 render 函数；静态图形可以不需要它，动画需要定义 render(t, info)。"}}),
        )
    }
}
