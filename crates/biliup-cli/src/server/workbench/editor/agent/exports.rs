//! Uncut tools/exportVideo.ts adapted to biliup's shared managed export queue.
use super::super::{self as editor, Error, invalid, text_model::conversation::Tool};
use super::{
    service::{Access, RunContext, Service},
    store, views,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-export-tools.json"
    )))
    .expect("export tool schemas")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&t["inputSchema"]).expect("export schema"),
            )
        })
        .collect()
});
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
fn id(value: &str) -> Result<i64, Error> {
    value
        .strip_prefix("editor-export-")
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| invalid("导出任务标识无效"))
}
fn active(value: &Value) -> bool {
    matches!(
        value["phase"].as_str(),
        Some("queued" | "preparing" | "rendering")
    )
}
impl Service {
    pub(super) async fn export_tool(
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
            .ok_or_else(|| invalid("未知导出工具"))?
            .validate(args)
            .map_err(|e| Error::Invalid(format!("导出参数无效：{e}")))?;
        let job = args["job"].as_str().map(id).transpose()?;
        if name == "export_status" {
            if let Some(job) = job {
                return self
                    .exports
                    .status(project, job)
                    .await
                    .map_err(Error::Invalid);
            }
            let list = self.exports.list(project).await.map_err(Error::Invalid)?;
            let mut jobs = Vec::new();
            for item in list.into_iter().take(30) {
                if let Some(key) = item["id"].as_str() {
                    jobs.push(
                        self.exports
                            .status(project, id(key)?)
                            .await
                            .map_err(Error::Invalid)?,
                    );
                }
            }
            return Ok(json!({"jobs":jobs,"view":"/outputs"}));
        }
        if context.access == Access::Look {
            return Err(invalid("只读模式不能提交或取消导出"));
        }
        if name == "export_cancel" {
            let job = job.unwrap();
            let state = self
                .exports
                .status(project, job)
                .await
                .map_err(Error::Invalid)?;
            if !active(&state) {
                return Ok(
                    json!({"job":state,"cancelled":false,"note":"任务已经结束，未修改成片。"}),
                );
            }
            let saved = editor::get(&self.pool, project).await?;
            if context.access == Access::Ask
                && !self
                    .confirm(
                        project,
                        turn,
                        "取消导出",
                        &[state["output"].as_str().unwrap_or("视频").to_string()],
                        saved.summary.revision,
                        cancel,
                    )
                    .await?
            {
                return Ok(json!({"cancelled":false,"reason":"用户未批准取消"}));
            }
            if cancel.is_cancelled() {
                return Err(invalid("助手已停止"));
            }
            self.exports
                .cancel(project, job)
                .await
                .map_err(Error::Invalid)?;
            return Ok(
                json!({"job":self.exports.status(project,job).await.map_err(Error::Invalid)?,"cancelRequested":true}),
            );
        }
        let key = if let Some(job) = job {
            if args["retry"] == true {
                return Err(invalid(
                    "重试导出请传入当前 revision 和完整导出选项；job 仅用于查询已有任务",
                ));
            }
            // Waiting for another turn's/manual task does not transfer its cancellation ownership.
            self.exports
                .status(project, job)
                .await
                .map_err(Error::Invalid)?;
            job
        } else {
            let revision = args["revision"].as_i64().unwrap();
            let saved = editor::get(&self.pool, project).await?;
            if saved.summary.revision != revision {
                return Err(Error::Conflict);
            }
            let input = views::eval(
                &format!(
                    "BiliupExport.agentExportRequest({},{},{})",
                    saved.document.project,
                    args,
                    serde_json::to_string(&context.export_ranges)?
                ),
                cancel,
            )
            .await?;
            let stamp = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(
                    &json!({"revision":revision,"input":input})
                )?)
            );
            let decision = format!("export:{stamp}");
            if context.create_decisions.get(&decision) == Some(&false) {
                return Ok(json!({"started":false,"reason":"用户已拒绝此导出，不要自动重试"}));
            }
            if context.access == Access::Ask && !context.create_decisions.contains_key(&decision) {
                let lines = vec![
                    format!(
                        "{} · {}",
                        input["output"].as_str().unwrap_or("视频"),
                        input["quality"].as_str().unwrap_or("standard")
                    ),
                    format!(
                        "工程版本 {revision} · {}",
                        if input["range"].is_object() {
                            "选定范围"
                        } else {
                            "整个时间轴"
                        }
                    ),
                ];
                let approved = self
                    .confirm(project, turn, "导出视频", &lines, revision, cancel)
                    .await?;
                context.create_decisions.insert(decision, approved);
                if !approved {
                    return Ok(json!({"started":false,"reason":"用户未批准导出"}));
                }
            }
            if cancel.is_cancelled() {
                return Err(invalid("助手已停止，未提交导出"));
            }
            let stamp = if args["retry"] == true {
                format!(
                    "{stamp}:{}",
                    editor::generate::random_id().map_err(Error::Invalid)?
                )
            } else {
                stamp
            };
            let job = self
                .exports
                .enqueue_agent(project, revision, input, turn, &stamp, cancel)
                .await
                .map_err(Error::Invalid)?;
            id(&job)?
        };
        let index = data["items"]
            .as_array()
            .and_then(|items| items.len().checked_sub(1));
        if let Some(i) = index {
            data["items"][i]["exportJob"] = json!(format!("editor-export-{key}"));
        }
        let until = tokio::time::Instant::now()
            + Duration::from_secs(args["waitSeconds"].as_u64().unwrap_or(1500));
        loop {
            let state = self
                .exports
                .status(project, key)
                .await
                .map_err(Error::Invalid)?;
            if !active(&state) {
                if let Some(i) = index {
                    data["items"][i]["text"] = json!(match state["phase"].as_str() {
                        Some("done") => "视频已导出",
                        Some("cancelled") => "导出已取消",
                        _ => "导出失败",
                    });
                }
                return Ok(
                    json!({"job":state,"saved":state["phase"]=="done","error":if state["phase"]=="failed"{state["error"].clone()}else{Value::Null},"view":"/outputs","note":if state["phase"]=="done"{"视频已完成，下载链接指向实际成片；工程没有被修改。"}else{"任务未完成，未自动重试；可在压制与导出查看原因。"}}),
                );
            }
            if tokio::time::Instant::now() >= until {
                if let Some(i) = index {
                    data["items"][i]["text"] = json!("导出仍在运行，可在压制与导出查看");
                }
                return Ok(
                    json!({"job":state,"saved":false,"view":"/outputs","note":"任务仍在运行，可在压制与导出查看进度或用 export_status 查询；不要重复提交。"}),
                );
            }
            if let Some(i) = index {
                let text = match state["phase"].as_str() {
                    Some("queued") => "等待导出".to_string(),
                    Some("preparing") => {
                        if state["step"]["kind"] == "loudness" {
                            "正在测量响度".into()
                        } else {
                            "正在准备导出".into()
                        }
                    }
                    _ => format!(
                        "正在导出 · {:.0}%",
                        state["progress"].as_f64().unwrap_or(0.) * 100.
                    ),
                };
                data["items"][i]["text"] = json!(text);
                store::update(&self.pool, turn, data).await?;
            }
            tokio::select! {
                _=cancel.cancelled()=>{self.exports.cancel_agent(project,turn).await.map_err(Error::Invalid)?;return Ok(json!({"job":format!("editor-export-{key}"),"stopped":true,"note":"已停止助手，并请求取消本轮发起的导出；其他来源的任务保持原状态。"}))},
                _=tokio::time::sleep_until(until)=>{},
                _=tokio::time::sleep(Duration::from_millis(500))=>{}
            }
        }
    }
}
