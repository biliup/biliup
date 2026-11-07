//! Assistant tools share the editor's analysis queue, fingerprints and cancellation state.
use super::super::{self as editor, Error, text_model::conversation::Tool};
use super::{service::Service, views};
use futures::{FutureExt, StreamExt, stream};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::LazyLock,
    time::Duration,
};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-analysis-tools.json"
    )))
    .expect("bundled analysis tools")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&tool["inputSchema"]).expect("valid analysis schema"),
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

fn analyzers(args: &Value) -> Result<Vec<String>, Error> {
    let Some(names) = args["analyzers"].as_array() else {
        return Ok(vec!["loudness".into()]);
    };
    let mut result = BTreeSet::new();
    for name in names {
        result.insert(
            match name.as_str().unwrap_or("") {
                "video" | "shots" | "motion" | "stability" => "video",
                "loudness" | "silence" => "loudness",
                "beats" | "rhythm" | "onsets" | "tempo" => "beats",
                "transcript" => "transcript",
                "description" => {
                    return Err(invalid(
                        "参考实现尚未提供画面描述分析，请使用镜头联系表查看画面",
                    ));
                }
                _ => return Err(invalid("未知分析器")),
            }
            .to_string(),
        );
    }
    Ok(result.into_iter().collect())
}
impl Service {
    /// A stopped assistant only stops waiting: another panel may be using the same job.
    /// The shared analysis queue owns explicit job cancellation and project cleanup.
    pub(super) async fn analysis_result(
        &self,
        project: i64,
        media: &str,
        analyzer: &str,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        self.analysis_result_with_language(project, media, analyzer, deadline, cancel, "auto")
            .await
    }
    pub(super) async fn analysis_result_with_language(
        &self,
        project: i64,
        media: &str,
        analyzer: &str,
        deadline: Instant,
        cancel: &CancellationToken,
        language: &str,
    ) -> Result<Value, Error> {
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        if Instant::now() >= deadline {
            let cached = self
                .analysis
                .cached_with_language(project, media, analyzer, &[], language)
                .await?;
            return Ok(if cached["result"].is_null() {
                json!({"state":"pending","note":"本次等待时间已用完；请再次调用以继续或复用此素材的分析任务。"})
            } else {
                json!({"state":"done","cached":true,"result":cached["result"]})
            });
        }
        let starts = self
            .analysis
            .start_with_language(project, media.into(), Some(vec![analyzer.into()]), language)
            .await?;
        let start = starts.first().ok_or_else(|| invalid("没有返回分析任务"))?;
        let cached = start["state"] == "done";
        let mut state = start.clone();
        loop {
            if cancel.is_cancelled() {
                return Err(invalid("助手已停止"));
            }
            if state["state"] == "done" || state["phase"] == "done" {
                let result = if let Some(id) = start["jobId"].as_str() {
                    self.analysis.job_result(project, id, media).await?
                } else {
                    self.analysis
                        .cached_with_language(project, media, analyzer, &[], language)
                        .await?
                };
                if result["result"].is_null() {
                    return Err(invalid("素材已改变，分析缓存已失效，请重新分析"));
                }
                return Ok(json!({"state":"done","cached":cached,"result":result["result"]}));
            }
            if matches!(state["phase"].as_str(), Some("failed" | "cancelled")) {
                return Ok(
                    json!({"state":state["phase"],"jobId":start["jobId"],"error":state["error"]}),
                );
            }
            if Instant::now() >= deadline {
                return Ok(
                    json!({"state":state["phase"].as_str().unwrap_or(start["state"].as_str().unwrap_or("queued")),"jobId":start["jobId"],"progress":state["progress"],"stage":state["stage"],"note":"分析继续在共享队列运行；再次调用会复用任务和缓存。"}),
                );
            }
            tokio::select! { _=cancel.cancelled()=>return Err(invalid("助手已停止")), _=tokio::time::sleep_until((Instant::now()+Duration::from_millis(400)).min(deadline))=>{} }
            if let Some(job) = self
                .analysis
                .status(project, start["jobId"].as_str().unwrap_or(""))
                .await?
            {
                state = job;
            } else {
                let result = self
                    .analysis
                    .cached_with_language(project, media, analyzer, &[], language)
                    .await?;
                if !result["result"].is_null() {
                    return Ok(json!({"state":"done","cached":false,"result":result["result"]}));
                }
                return Err(invalid("分析任务已不在队列中，请重试"));
            }
        }
    }
    pub(super) async fn analysis_tool(
        &self,
        project: i64,
        name: &str,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        if serde_json::to_vec(args)?.len() > 32 * 1024 {
            return Err(invalid("分析请求过大"));
        }
        VALIDATORS
            .get(name)
            .ok_or_else(|| invalid("未知分析工具"))?
            .validate(args)
            .map_err(|e| invalid(format!("分析参数无效：{e}")))?;
        let saved = editor::get(&self.pool, project).await?;
        let doc = &saved.document.project;
        let media = doc["media"]
            .as_array()
            .ok_or_else(|| invalid("工程没有素材列表"))?;
        let deadline = Instant::now() + Duration::from_secs(50);
        let expression = if name == "media_analyze" {
            let id = args["media"].as_str().unwrap();
            let item = media
                .iter()
                .find(|m| m["id"] == id)
                .ok_or_else(|| invalid("素材不在当前工程中"))?;
            if let Some(range) = args["range"].as_object() {
                let start = range["start"].as_u64().unwrap_or(u64::MAX);
                let end = range["end"].as_u64().unwrap_or(0);
                if start >= end
                    || item["probe"]["frames"]
                        .as_u64()
                        .is_some_and(|length| end > length)
                {
                    return Err(invalid("分析范围为空或超出素材时长"));
                }
            }
            let mut outcomes = serde_json::Map::new();
            for analyzer in analyzers(args)? {
                let outcome = self
                    .analysis_result(project, id, &analyzer, deadline, cancel)
                    .await
                    .unwrap_or_else(|e| json!({"state":"failed","error":e.to_string()}));
                outcomes.insert(analyzer, outcome);
            }
            let sound = if outcomes.contains_key("transcript") {
                self.analysis
                    .cached(project, id, "loudness", &[])
                    .await
                    .ok()
                    .map(|v| v["result"].clone())
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            format!(
                "BiliupExport.agentAnalysis({doc},{},{},{},{sound})",
                json!(id),
                args["range"],
                Value::Object(outcomes)
            )
        } else {
            if !args["media"].is_null() && !args["folder"].is_null() {
                return Err(invalid("请指定素材或文件夹中的一种"));
            }
            let mut chosen = vec![];
            if let Some(ids) = args["media"].as_array() {
                for id in ids {
                    let item = media
                        .iter()
                        .find(|m| m["id"] == *id)
                        .ok_or_else(|| invalid(format!("素材 {id} 不在当前工程中")))?;
                    chosen.push(item);
                }
            } else if let Some(folder) = args["folder"].as_str() {
                if !Path::new(folder).is_absolute() {
                    return Err(invalid("请指定绝对文件夹路径"));
                }
                let folder = tokio::fs::canonicalize(folder)
                    .await
                    .map_err(|e| invalid(e.to_string()))?;
                for item in media {
                    let Some(id) = item["id"].as_str() else {
                        continue;
                    };
                    if let Ok((path, _)) = editor::media::source(&self.pool, project, id).await
                        && tokio::fs::canonicalize(path)
                            .await
                            .is_ok_and(|p| p.starts_with(&folder))
                    {
                        chosen.push(item);
                    }
                }
                if chosen.is_empty() {
                    return Err(invalid("工程中没有来自这个文件夹的素材"));
                }
            } else {
                chosen.extend(media.iter().filter(|m| m["kind"] == "video"));
            }
            let mut skipped = vec![];
            chosen.retain(|m| {
                if m["kind"] == "video" {
                    true
                } else {
                    skipped.push(json!({"media":m["id"],"reason":"仅视频素材支持画面质量排序"}));
                    false
                }
            });
            if chosen.len() > 50 {
                return Err(invalid("一次最多排序 50 个视频素材"));
            }
            let mut requests = vec![];
            for item in chosen {
                let id = item["id"].as_str().unwrap_or("").to_string();
                requests.push(
                    async move {
                        let outcome = self
                            .analysis_result(project, &id, "video", deadline, cancel)
                            .await
                            .unwrap_or_else(|e| json!({"state":"failed","error":e.to_string()}));
                        (id, outcome)
                    }
                    .boxed(),
                );
            }
            let outcomes: Vec<(String, Value)> =
                stream::iter(requests).buffer_unordered(8).collect().await;
            format!(
                "BiliupExport.agentRank({doc},{},{},{},{})",
                args["by"].as_str().map_or(json!("clean"), |s| json!(s)),
                args["window"].as_u64().unwrap_or(5),
                Value::Object(outcomes.into_iter().collect()),
                json!(skipped)
            )
        };
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        if expression.len() > 32 * 1024 * 1024 {
            return Err(invalid("分析数据超过单次处理上限，请减少本次素材数量"));
        }
        let mut result = views::eval(&expression, cancel).await?;
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(invalid("分析期间工程已改变，请重新读取工程后使用结果"));
        }
        result["revision"] = json!(saved.summary.revision);
        Ok(result)
    }
}
