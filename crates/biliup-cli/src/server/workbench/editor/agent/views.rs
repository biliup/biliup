//! Grounded visual tools: model-visible images and a persistent evidence record at one revision.
use super::super::{
    self as editor, ConnectionPool, Error,
    text_model::conversation::{Image, Tool},
};
use super::{
    plan,
    service::{RunContext, Service},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock};
use tokio_util::sync::CancellationToken;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-view-tools.json"
    )))
    .expect("bundled view tools")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().unwrap().to_string(),
                jsonschema::validator_for(&tool["inputSchema"]).expect("valid view schema"),
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
#[derive(Default)]
pub(super) struct Evidence {
    pub revision: i64,
    pub stamp: String,
    pub sheet: bool,
    pub phone: bool,
    pub frames: Vec<u64>,
}
pub async fn eval(expression: &str, cancel: &CancellationToken) -> Result<Value, Error> {
    let mut browser = editor::export::browser::Browser::launch(cancel)
        .await
        .map_err(invalid)?;
    let result = browser.eval(expression).await;
    browser.close().await;
    result.map_err(invalid)
}
pub async fn keep(
    pool: &ConnectionPool,
    project: i64,
    turn: &str,
    revision: i64,
    jpeg: &[u8],
) -> Result<String, Error> {
    if jpeg.len() > 5 * 1024 * 1024 {
        return Err(invalid("助手画面超过 5 MiB"));
    }
    let id = editor::generate::random_id().map_err(invalid)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
    if !active {
        return Err(invalid("助手已停止"));
    }
    let size: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(length(jpeg)),0) FROM editor_agent_images WHERE project_id=?",
    )
    .bind(project)
    .fetch_one(&mut *tx)
    .await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM editor_agent_images WHERE turn_id=?")
        .bind(turn)
        .fetch_one(&mut *tx)
        .await?;
    if size + jpeg.len() as i64 > 256 * 1024 * 1024 || count >= 256 {
        return Err(invalid("助手画面已达到工程或本轮上限"));
    }
    sqlx::query("INSERT INTO editor_agent_images(id,project_id,turn_id,revision,jpeg,created_at) VALUES(?,?,?,?,?,?)").bind(&id).bind(project).bind(turn).bind(revision).bind(jpeg).bind(editor::now_ms()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(id)
}
impl Service {
    pub(super) async fn ui_tool(
        &self,
        project: i64,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        if serde_json::to_vec(args)?.len() > 32 * 1024 {
            return Err(invalid("界面定位请求过大"));
        }
        VALIDATORS
            .get("ui_show")
            .unwrap()
            .validate(args)
            .map_err(|error| invalid(format!("界面定位参数无效：{error}")))?;
        let saved = editor::get(&self.pool, project).await?;
        let mut ui = eval(
            &format!("BiliupExport.agentUi({},{args})", saved.document.project),
            cancel,
        )
        .await?;
        if editor::get(&self.pool, project).await?.summary.revision != saved.summary.revision {
            return Err(invalid("工程已改变，请重新读取工程后定位"));
        }
        ui["id"] = json!(editor::generate::random_id().map_err(invalid)?);
        ui["revision"] = json!(saved.summary.revision);
        Ok(ui)
    }
    pub(super) async fn view_tool(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        name: &str,
        args: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        if serde_json::to_vec(args)?.len() > 32 * 1024 {
            return Err(invalid("查看请求过大"));
        }
        VALIDATORS
            .get(name)
            .ok_or_else(|| invalid("未知查看工具"))?
            .validate(args)
            .map_err(|e| invalid(format!("查看参数无效：{e}")))?;
        let saved = editor::get(&self.pool, project).await?;
        let revision = saved.summary.revision;
        if name == "project_check" {
            let plan = plan::read(&self.pool, project).await?;
            let mut result = eval(
                &format!(
                    "BiliupExport.agentChecks({},{args},{plan})",
                    saved.document.project
                ),
                cancel,
            )
            .await?;
            if editor::get(&self.pool, project).await?.summary.revision != revision {
                return Err(Error::Conflict);
            }
            result["revision"] = json!(revision);
            return Ok(result);
        }
        let (stamp, _) =
            super::review::environment(&self.pool, &self.root, project, &saved.document).await?;
        let video = if name == "contact_sheet" && args["at"] == "shots" {
            let media = args["media"]
                .as_str()
                .ok_or_else(|| invalid("按镜头取帧需要指定视频素材"))?;
            if !args["every"].is_null() {
                return Err(invalid("按镜头取帧不能同时设置间隔"));
            }
            if !saved.document.project["media"]
                .as_array()
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|m| m["id"] == media && m["kind"] == "video")
                })
            {
                return Err(invalid("请选择当前工程中的视频素材"));
            }
            let result = self
                .analysis_result(
                    project,
                    media,
                    "video",
                    tokio::time::Instant::now() + std::time::Duration::from_secs(50),
                    cancel,
                )
                .await?;
            if result["state"] != "done" {
                return Ok(
                    json!({"media":media,"video":result,"note":"镜头分析尚未完成，未生成联系表；再次调用会复用任务。"}),
                );
            }
            result["result"].clone()
        } else {
            Value::Null
        };
        let view = eval(
            &format!(
                "BiliupExport.agentView({},{},{args},{video})",
                saved.document.project,
                json!(name)
            ),
            cancel,
        )
        .await?;
        let mut document = saved.document;
        document.project = view["project"].clone();
        if let Some(media) = args["media"].as_str() {
            document.sources.retain(|id, _| id == media);
            document.project["media"] = json!(
                document.project["media"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|m| m["id"] == media)
                    .cloned()
                    .collect::<Vec<_>>()
            );
        }
        let frames: Vec<u64> = serde_json::from_value(view["frames"].clone())?;
        let width = view["width"]
            .as_u64()
            .ok_or_else(|| invalid("取帧尺寸无效"))? as u32;
        let mut renderer = editor::export::frames::Session::new(cancel).map_err(invalid)?;
        let mut pictures = vec![];
        for chunk in frames.chunks(24) {
            pictures.extend(
                renderer
                    .render(
                        &self.pool,
                        &self.root,
                        project,
                        editor::export::frames::FrameRequest {
                            document: document.clone(),
                            frames: chunk,
                            width,
                            motion_blur: false,
                            census: true,
                        },
                    )
                    .await
                    .map_err(invalid)?,
            );
        }
        let mut evidence = vec![];
        let mut attachments = vec![];
        if name == "contact_sheet" {
            let cells = Value::Array(
                pictures
                    .iter()
                    .map(|p| json!({"jpeg":STANDARD.encode(&p.jpeg),"frame":p.evidence["frame"]}))
                    .collect(),
            );
            let jpeg = editor::export::frames::sheet(&cells, &view["fps"], &view["grid"], cancel)
                .await
                .map_err(invalid)?;
            let id = keep(&self.pool, project, turn, revision, &jpeg).await?;
            evidence.push(json!({"image":id,"frames":frames,"fps":view["fps"],"grid":view["grid"],"shots":view["shots"],"totalShots":view["totalShots"],"cells":pictures.iter().map(|p|&p.evidence).collect::<Vec<_>>()}));
            attachments.push(Image {
                name: format!("contact-sheet-revision-{revision}.jpg"),
                mime_type: "image/jpeg".into(),
                data: STANDARD.encode(jpeg),
            });
        } else {
            let total: usize = pictures.iter().map(|p| p.jpeg.len()).sum();
            if total > 20 * 1024 * 1024 {
                return Err(invalid("本次画面总大小超过 20 MiB，请缩小比例或减少帧数"));
            }
            for picture in pictures {
                let id = keep(&self.pool, project, turn, revision, &picture.jpeg).await?;
                let mut item = picture.evidence;
                item["image"] = json!(id);
                attachments.push(Image {
                    name: format!("revision-{revision}-frame-{}.jpg", item["frame"]),
                    mime_type: "image/jpeg".into(),
                    data: STANDARD.encode(picture.jpeg),
                });
                evidence.push(item);
            }
        }
        let current = editor::get(&self.pool, project).await?;
        if current.summary.revision != revision
            || super::review::environment(&self.pool, &self.root, project, &current.document)
                .await?
                .0
                != stamp
        {
            return Err(Error::Conflict);
        }
        if context.looked.revision != revision || context.looked.stamp != stamp {
            context.looked = Evidence {
                revision,
                stamp,
                ..Default::default()
            };
        }
        if args["media"].is_null() {
            if name == "contact_sheet" {
                context.looked.sheet = true;
            } else {
                context.looked.phone |= args["phone"] == true;
                context.looked.frames.extend(frames);
                context.looked.frames.sort_unstable();
                context.looked.frames.dedup();
            }
        }
        context.images.extend(attachments);
        Ok(
            json!({"revision":revision,"images":evidence,"phone":args["phone"]==true,"media":args["media"]}),
        )
    }
}
