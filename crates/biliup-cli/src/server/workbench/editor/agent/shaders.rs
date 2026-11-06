//! TypeGPU resolves a WGSL body, then the existing preview/export runner checks it on a sample.
use super::super::{self as editor, Error, text_model::conversation::Image};
use super::{
    create,
    service::{Access, RunContext, Service},
    views,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::Row;
use tokio_util::sync::CancellationToken;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
impl Service {
    pub(super) async fn shader_read(&self, project: i64, args: &Value) -> Result<Value, Error> {
        create::validate("shader_read", args)?;
        let saved = editor::get(&self.pool, project).await?;
        let row=sqlx::query("SELECT revision,definition_json,body,tested FROM editor_shaders WHERE project_id=? AND id=?").bind(project).bind(args["id"].as_str().unwrap()).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        Ok(
            json!({"definition":serde_json::from_str::<Value>(row.try_get::<&str,_>("definition_json")?)?,"body":row.try_get::<String,_>("body")?,"shaderRevision":row.try_get::<i64,_>("revision")?,"tested":row.try_get::<bool,_>("tested")?,"revision":saved.summary.revision}),
        )
    }
    pub(super) async fn shader_tool(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        create::validate("shader_write", args)?;
        if context.access == Access::Look {
            return Err(invalid("本轮为只读模式"));
        }
        let saved = editor::get(&self.pool, project).await?;
        let definition =
            views::eval(&format!("BiliupExport.prepareShader({args})"), cancel).await?;
        let id = definition["id"]
            .as_str()
            .ok_or_else(|| invalid("特效标识无效"))?;
        let previous: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM editor_shaders WHERE project_id=? AND id=?")
                .bind(project)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        let preview = views::eval(&format!("BiliupExport.tryShader({definition})"), cancel).await?;
        let tested = preview["tested"] == true;
        let mut lines = vec![definition["name"].as_str().unwrap_or(id).to_string()];
        if previous.is_some() {
            lines.push("替换特效模板；已应用的片段保留原版本".into());
        }
        if !tested {
            lines.push("服务器没有可用的 WebGPU，尚未验证实际画面".into());
        }
        if context.access == Access::Ask
            && !self
                .confirm(
                    project,
                    turn,
                    "保存 GPU 特效",
                    &lines,
                    saved.summary.revision,
                    cancel,
                )
                .await?
        {
            return Ok(json!({"saved":false,"reason":"用户未批准保存特效；不要自动重试"}));
        }
        let image = if tested {
            let encoded = preview["jpeg"]
                .as_str()
                .filter(|v| v.len() < 2 * 1024 * 1024)
                .ok_or_else(|| invalid("特效预览图无效"))?;
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|e| invalid(e.to_string()))?;
            Some((
                views::keep(&self.pool, project, turn, saved.summary.revision, &bytes).await?,
                encoded.to_string(),
            ))
        } else {
            None
        };
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
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM editor_shaders WHERE project_id=? AND id=?")
                .bind(project)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if current != previous {
            return Err(Error::Conflict);
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM editor_shaders WHERE project_id=?")
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 64 && previous.is_none() {
            return Err(invalid(
                "工程最多保留 64 个自定义 GPU 特效，请复用已有特效名称",
            ));
        }
        let raw = definition.to_string();
        if raw.len() > 64 * 1024 {
            return Err(invalid("特效定义超过 64 KiB"));
        }
        let revision = previous
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| invalid("特效版本超出上限"))?;
        sqlx::query("INSERT INTO editor_shaders(project_id,id,revision,definition_json,body,tested,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(project_id,id) DO UPDATE SET revision=excluded.revision,definition_json=excluded.definition_json,body=excluded.body,tested=excluded.tested,updated_at=excluded.updated_at")
            .bind(project).bind(id).bind(revision).bind(raw).bind(args["body"].as_str().unwrap()).bind(tested).bind(editor::now_ms()).execute(&mut *tx).await?;
        tx.commit().await?;
        if let Some((_, data)) = &image {
            context.images.push(Image {
                name: format!("shader-{id}-version-{revision}-sample.jpg"),
                mime_type: "image/jpeg".into(),
                data: data.clone(),
            });
        }
        Ok(
            json!({"saved":true,"id":id,"shaderRevision":revision,"revision":saved.summary.revision,"tested":tested,"image":image.map(|(id,_)|id),"replaced":previous.is_some(),"params":definition["params"],"note":preview["note"],"next":"特效已出现在现有特效面板。可用 timeline_edit 在片段 effects 中加入 effect.shader，params 为 {shader:返回的id,values:{}}；完整定义随片段保存，之后用 render_frames 查看实际素材上的效果。"}),
        )
    }
}
