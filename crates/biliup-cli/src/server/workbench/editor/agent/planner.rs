use super::super::{self as editor, ConnectionPool, Document, Error};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

pub struct PlannedEdit {
    pub document: Document,
    pub base_revision: i64,
    pub summary: String,
    pub operations: usize,
    pub lines: Vec<String>,
}

/// Plan in the same isolated pure runtime as export; no model-generated JavaScript is evaluated.
/// The caller may present this result for approval, then commit against this exact revision.
pub async fn edit(
    pool: &ConnectionPool,
    project: i64,
    revision: i64,
    turn: &str,
    input: &Value,
    cancel: &CancellationToken,
) -> Result<PlannedEdit, Error> {
    edit_with_sources(pool, project, revision, turn, input, &Value::Null, cancel).await
}
pub async fn edit_with_sources(
    pool: &ConnectionPool,
    project: i64,
    revision: i64,
    turn: &str,
    input: &Value,
    sources: &Value,
    cancel: &CancellationToken,
) -> Result<PlannedEdit, Error> {
    let saved = editor::get(pool, project).await?;
    if revision != saved.summary.revision {
        return Err(Error::Conflict);
    }
    let ops = input["ops"]
        .as_array()
        .filter(|ops| !ops.is_empty() && ops.len() <= 500)
        .ok_or_else(|| Error::Invalid("编辑操作数量无效".into()))?;
    if serde_json::to_vec(input)?.len() > 2 * 1024 * 1024 {
        return Err(Error::Invalid("编辑请求过大".into()));
    }
    let summary = input["summary"]
        .as_str()
        .unwrap_or("编辑时间轴")
        .chars()
        .take(500)
        .collect::<String>();
    let shaders = editor::shaders::definitions(pool, project).await?;
    let mut arguments = json!({"ops":ops,"summary":summary,"turnId":turn,"shaders":shaders});
    if let Some(incoming) = sources.as_object() {
        let mut durations: serde_json::Map<String,Value> = saved.document.sources.iter()
            .map(|(id,source)|(id.clone(),json!(source.duration_ms()))).collect();
        for (id, source) in incoming {
            durations.insert(id.clone(), source["durationMs"].clone());
        }
        arguments["importDurations"] = Value::Object(durations);
    }
    let mut browser = editor::export::browser::Browser::launch(cancel)
        .await
        .map_err(Error::Invalid)?;
    let result = browser
        .eval(&format!(
            "BiliupExport.agentEdit(JSON.parse({}),JSON.parse({}))",
            serde_json::to_string(&saved.document.project.to_string())?,
            serde_json::to_string(&arguments.to_string())?
        ))
        .await;
    browser.close().await;
    let result = result.map_err(Error::Invalid)?;
    let mut document = saved.document;
    if let Some(sources) = sources.as_object() {
        for (id, source) in sources {
            document
                .sources
                .insert(id.clone(), serde_json::from_value(source.clone())?);
        }
    }
    document.project = result["project"].clone();
    Ok(PlannedEdit {
        document,
        base_revision: revision,
        summary,
        operations: ops.len(),
        lines: serde_json::from_value(result["lines"].clone())?,
    })
}
pub async fn modules(
    pool: &ConnectionPool,
    project: i64,
    cancel: &CancellationToken,
) -> Result<Value, String> {
    let shaders = editor::shaders::definitions(pool, project)
        .await
        .map_err(|e| e.to_string())?;
    let mut browser = editor::export::browser::Browser::launch(cancel).await?;
    let value = browser
        .eval(&format!("BiliupExport.agentModules({shaders})"))
        .await;
    browser.close().await;
    value
}
pub fn tools() -> Vec<editor::text_model::conversation::Tool> {
    let ops: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/editor/edit-ops.schema.json"
    )))
    .expect("bundled edit ops schema");
    vec![
        editor::text_model::conversation::Tool{name:"project_read".into(),description:"Read the current project, its revision, media, tracks and item IDs. Read again if an edit conflicts; do not invent media paths or IDs.".into(),input_schema:json!({"type":"object","properties":{},"additionalProperties":false})},
        editor::text_model::conversation::Tool{name:"modules_list".into(),description:"List the editor's built-in components, effects, transitions and audio modules. Use these IDs for editing.".into(),input_schema:json!({"type":"object","properties":{},"additionalProperties":false})},
        editor::text_model::conversation::Tool{name:"timeline_edit".into(),description:"Atomically edit the current project using editor operations. All times are frames at the project's fps. Supply the revision last read from project_read. Keep original media; adding a new media item requires importing it through the media tool first. The editor validates every operation and preserves linked audio.".into(),input_schema:json!({"type":"object","properties":{"revision":{"type":"integer","minimum":0},"summary":{"type":"string","maxLength":500},"ops":{"type":"array","minItems":1,"maxItems":500,"items":ops["items"]}},"required":["revision","summary","ops"],"additionalProperties":false,"$defs":ops["$defs"]})},
    ]
}
