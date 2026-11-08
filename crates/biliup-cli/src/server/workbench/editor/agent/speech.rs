//! Transcript and beat tools reuse the editor's registered media and analysis jobs.
use super::super::{self as editor, Error, text_model::conversation::Tool};
use super::{
    service::{Access, RunContext, Service},
    views,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-speech-tools.json"
    )))
    .expect("bundled speech tools")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&t["inputSchema"]).expect("valid speech schema"),
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
pub(super) fn validate(name: &str, args: &Value) -> Result<(), Error> {
    if serde_json::to_vec(args)?.len() > 32 * 1024 {
        return Err(invalid("语音工具请求过大"));
    }
    VALIDATORS
        .get(name)
        .ok_or_else(|| invalid("未知语音工具"))?
        .validate(args)
        .map_err(|e| invalid(format!("语音工具参数无效：{e}")))
}
impl Service {
    pub(super) async fn speech_tool(
        &self,
        project: i64,
        name: &str,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        validate(name, args)?;
        let saved = editor::get(&self.pool, project).await?;
        let target = views::eval(
            &format!(
                "BiliupExport.agentSpeechTarget({},{},{args})",
                saved.document.project,
                json!(name)
            ),
            cancel,
        )
        .await?;
        let id = target["media"]
            .as_str()
            .ok_or_else(|| invalid("语音来源无效"))?;
        let analyzer = if name == "transcript_read" {
            "transcript"
        } else {
            "beats"
        };
        let outcome = self
            .analysis_result_with_language(
                project,
                id,
                analyzer,
                Instant::now() + Duration::from_secs(50),
                cancel,
                args["language"].as_str().unwrap_or("auto"),
            )
            .await?;
        let sound = if name == "transcript_read" {
            self.analysis
                .cached(project, id, "loudness", &[])
                .await
                .ok()
                .map(|v| v["result"].clone())
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        let expression = format!(
            "BiliupExport.agentSpeechResult({},{},{args},{outcome},{sound})",
            saved.document.project,
            json!(name)
        );
        if expression.len() > 32 * 1024 * 1024 {
            return Err(invalid("分析数据超过单次处理上限，请拆分素材"));
        }
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(invalid("分析期间工程已改变，请重新读取工程后使用结果"));
        }
        let mut result = views::eval(&expression, cancel).await?;
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(invalid("工程已改变，请重新读取工程后使用结果"));
        }
        result["revision"] = json!(saved.summary.revision);
        Ok(result)
    }
    pub(super) async fn speech_edit_tool(
        &self,
        project: i64,
        turn: &str,
        context: &RunContext,
        name: &str,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        validate(name, args)?;
        let dry_run = name == "transcript_edit" && args["dryRun"] == true;
        if context.access == Access::Look && !dry_run {
            return Err(invalid("本轮为只读模式"));
        }
        let saved = editor::get(&self.pool, project).await?;
        let target = views::eval(
            &format!(
                "BiliupExport.agentSpeechTarget({},{},{args})",
                saved.document.project,
                json!(name)
            ),
            cancel,
        )
        .await?;
        let id = target["media"]
            .as_str()
            .ok_or_else(|| invalid("语音来源无效"))?;
        let deadline = Instant::now() + Duration::from_secs(50);
        let transcript = self
            .analysis_result_with_language(
                project,
                id,
                "transcript",
                deadline,
                cancel,
                args["language"].as_str().unwrap_or("auto"),
            )
            .await?;
        if transcript["state"] != "done" {
            return Ok(
                json!({"applied":false,"media":id,"transcript":transcript,"note":"尚未完成转写，没有修改工程；用相同参数重试可读取同一个任务。"}),
            );
        }
        let loudness = self
            .analysis_result(project, id, "loudness", deadline, cancel)
            .await?;
        if loudness["state"] != "done" {
            return Ok(
                json!({"applied":false,"media":id,"loudness":loudness,"note":"尚未完成语音边界分析，没有修改工程；用相同参数重试可读取同一个任务。"}),
            );
        }
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(Error::Conflict);
        }
        let expression = format!(
            "BiliupExport.agentSpeechPlan({},{},{args},{},{},{})",
            saved.document.project,
            json!(name),
            transcript["result"],
            loudness["result"],
            json!(turn)
        );
        if expression.len() > 32 * 1024 * 1024 {
            return Err(invalid("语音数据超过单次处理上限，请拆分素材"));
        }
        let planned = views::eval(&expression, cancel).await?;
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(Error::Conflict);
        }
        let mut result = planned["result"].clone();
        if dry_run || planned["ops"].as_array().is_some_and(|v| v.is_empty()) {
            result["applied"] = json!(false);
            result["revision"] = json!(saved.summary.revision);
            return Ok(result);
        }
        let edit = json!({"summary":planned["summary"],"ops":planned["ops"]});
        let committed = self
            .apply_edit(
                project,
                turn,
                context,
                saved.summary.revision,
                &edit,
                cancel,
            )
            .await?;
        if committed["applied"] != true {
            return Ok(committed);
        }
        result["applied"] = json!(true);
        result["revision"] = committed["revision"].clone();
        Ok(result)
    }
}
