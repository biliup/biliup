//! Downloadable subtitle output is project-owned; model-provided paths never reach the filesystem.
use super::super::{self as editor, Error};
use super::{
    service::{Access, RunContext, Service},
    speech, views,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
impl Service {
    pub(super) async fn subtitles_tool(
        &self,
        project: i64,
        turn: &str,
        context: &RunContext,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        speech::validate("subtitles_export", args)?;
        if context.access == Access::Look {
            return Err(invalid("本轮为只读模式"));
        }
        let saved = editor::get(&self.pool, project).await?;
        let default_name = saved.document.project["name"]
            .as_str()
            .unwrap_or("Subtitles")
            .chars()
            .filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c))
            .take(100)
            .collect::<String>();
        let mut name = args["path"]
            .as_str()
            .unwrap_or(default_name.trim().trim_matches('.'))
            .to_string();
        if name.is_empty() {
            if args["path"].is_string() {
                return Err(invalid("字幕文件名不能为空"));
            }
            name = "Subtitles".into();
        }
        if name.chars().count() > 180
            || name
                .chars()
                .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
            || matches!(name.as_str(), "." | "..")
            || name.trim() != name
        {
            return Err(invalid(
                "请提供文件名，例如 talk.srt；不能使用目录或绝对路径",
            ));
        }
        let extension = args["path"]
            .as_str()
            .and_then(|name| name.rsplit_once('.'))
            .map(|(_, ext)| ext.to_ascii_lowercase());
        let explicit = args["format"].as_str();
        let format = match extension.as_deref() {
            Some(ext @ ("srt" | "vtt")) => {
                if explicit.is_some_and(|f| f != ext) {
                    return Err(invalid("字幕格式与文件扩展名不一致"));
                }
                ext.to_string()
            }
            Some(_) if args["path"].is_string() => {
                return Err(invalid("字幕文件应使用 .srt 或 .vtt 扩展名"));
            }
            _ => {
                let format = explicit.unwrap_or("srt").to_string();
                name.push('.');
                name.push_str(&format);
                format
            }
        };
        let mut output = views::eval(
            &format!(
                "BiliupExport.agentSubtitles({},{})",
                saved.document.project,
                json!(format)
            ),
            cancel,
        )
        .await?;
        let text = output["text"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 8 * 1024 * 1024)
            .ok_or_else(|| invalid("字幕文件超过 8 MiB 或内容为空"))?
            .to_string();
        if context.access == Access::Ask
            && !self
                .confirm(
                    project,
                    turn,
                    "导出字幕",
                    &[format!(
                        "{} {}",
                        if args["overwrite"] == true {
                            "保存并允许替换"
                        } else {
                            "保存"
                        },
                        name
                    )],
                    saved.summary.revision,
                    cancel,
                )
                .await?
        {
            return Ok(json!({"saved":false,"reason":"用户未批准本次导出；不要自动重试"}));
        }
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        if current != Some(saved.summary.revision) {
            return Err(Error::Conflict);
        }
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
        if !active || cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM editor_agent_files WHERE project_id=? AND name=?)",
        )
        .bind(project)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await?;
        if exists && args["overwrite"] != true {
            return Err(invalid(
                "同名字幕文件已存在，请更名或明确指定 overwrite: true",
            ));
        }
        if exists {
            sqlx::query("DELETE FROM editor_agent_files WHERE project_id=? AND name=?")
                .bind(project)
                .bind(&name)
                .execute(&mut *tx)
                .await?;
        }
        let (count,size):(i64,i64)=sqlx::query_as("SELECT COUNT(*),COALESCE(SUM(length(CAST(content AS BLOB))),0) FROM editor_agent_files WHERE project_id=?").bind(project).fetch_one(&mut *tx).await?;
        let turn_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM editor_agent_files WHERE turn_id=?")
                .bind(turn)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 128 || turn_count >= 16 || size + text.len() as i64 > 64 * 1024 * 1024 {
            return Err(invalid("字幕文件已达到工程或本轮保存上限，请替换已有文件"));
        }
        let id = editor::generate::random_id().map_err(invalid)?;
        let cues = output["cues"]
            .as_i64()
            .ok_or_else(|| invalid("字幕条数无效"))?;
        sqlx::query("INSERT INTO editor_agent_files(id,project_id,turn_id,revision,name,format,cues,content,created_at) VALUES(?,?,?,?,?,?,?,?,?)").bind(&id).bind(project).bind(turn).bind(saved.summary.revision).bind(&name).bind(&format).bind(cues).bind(text).bind(editor::now_ms()).execute(&mut *tx).await?;
        tx.commit().await?;
        output.as_object_mut().unwrap().remove("text");
        output["saved"] = json!(true);
        output["revision"] = json!(saved.summary.revision);
        output["path"] = json!(name);
        output["download"] = json!(format!("/v1/editor/projects/{project}/agent/files/{id}"));
        output["note"] = json!(
            "字幕已保存，本轮消息中可下载。时间对应此工程版本剪辑后的字幕；工程后续编辑不会改变已经导出的文件。"
        );
        Ok(output)
    }
}
