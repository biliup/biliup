//! Uncut tools/library.ts and generate.ts, using the editor's existing libraries and import journal.
use super::super::{self as editor, Error, invalid, text_model::conversation::Tool};
use super::{
    service::{Access, RunContext, Service},
    views,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
use tokio_util::sync::CancellationToken;
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-library-tools.json"
    )))
    .expect("library schemas")
});
static VALIDATORS: LazyLock<BTreeMap<String, jsonschema::Validator>> = LazyLock::new(|| {
    SPEC["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().into(),
                jsonschema::validator_for(&t["inputSchema"]).expect("library schema"),
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
impl Service {
    pub(super) async fn library_tool(
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
            .ok_or_else(|| invalid("未知素材库工具"))?
            .validate(args)
            .map_err(|e| Error::Invalid(format!("素材库参数无效：{e}")))?;
        match name {
            "library_add" => {
                if context.access == Access::Look {
                    return Err(invalid("只读模式不能导入音源"));
                }
                if let Some(id) = args["id"].as_str() {
                    let saved = editor::get(&self.pool, project).await?;
                    if let Some(media) = saved.document.project["media"]
                        .as_array()
                        .and_then(|all| all.iter().find(|m| m["source"]["libraryId"] == id))
                    {
                        let source = saved
                            .document
                            .sources
                            .get(media["id"].as_str().unwrap_or(""));
                        let present = if let Some(source) = source {
                            match editor::media::source_path(&self.pool, source).await {
                                Ok(path) => {
                                    tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file())
                                }
                                Err(_) => false,
                            }
                        } else {
                            false
                        };
                        if present {
                            return Ok(
                                json!({"id":media["id"],"media":media,"imported":true,"revision":saved.summary.revision,"note":"该音源已在当前工程素材中，未重复导入；可用 timeline_edit 放到音轨。"}),
                            );
                        }
                    }
                }
                let args = if let Some(id) = args["id"].as_str() {
                    json!({"path":format!("library:{id}"),"retry":args["retry"]==true})
                } else {
                    json!({"job":args["job"],"retry":args["retry"]==true})
                };
                self.import_job(project, turn, context, &args, cancel, data)
                    .await
            }
            "media_search" => {
                let result = editor::recent_media::search(
                    &self.pool,
                    project,
                    serde_json::from_value(args.clone())?,
                )
                .await?;
                let found = result["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .cloned()
                    .map(|mut entry| {
                        entry["path"] =
                            json!(format!("recent:{}", entry["id"].as_str().unwrap_or("")));
                        entry
                    })
                    .collect::<Vec<_>>();
                Ok(
                    json!({"found":found,"next":result["next"],"note":"仅包含实际登记且仍可读取的其他工程素材；next 非空时以相同筛选和 before=next 继续读取。用 media_import 的 path 参数传入 recent: 标识，再用 timeline_edit 放入时间轴。没有结果时请用户选择文件或使用编辑器的浏览器采集，不要编造素材。"}),
                )
            }
            "library_search" => {
                let saved = editor::get(&self.pool, project).await?;
                let fps = saved.document.project["canvas"]["fps"]
                    .as_f64()
                    .ok_or_else(|| invalid("工程帧率无效"))?;
                if args["source"] == "included" {
                    let folders = editor::sound_library::list(&self.pool, project).await?;
                    return views::eval(
                        &format!(
                            "BiliupExport.agentLibrarySearch({args},{},{fps},{project})",
                            serde_json::to_string(&folders)?
                        ),
                        cancel,
                    )
                    .await;
                }
                if context.library_searches >= 16 {
                    return Err(invalid(
                        "本轮在线音源搜索已达 16 次，请使用已有结果或内置音效",
                    ));
                }
                if args["category"].is_string() {
                    return Err(invalid(
                        "分类筛选用于内置音效；在线音源请把分类关键词放在 query 中",
                    ));
                }
                context.library_searches += 1;
                let mut query = args.clone();
                query.as_object_mut().unwrap().remove("category");
                if query["query"].is_null() {
                    query["query"] = json!("");
                }
                let mut result = self.online_library_search(&query, cancel).await?;
                if let Some(hits) = result["hits"].as_array_mut() {
                    for hit in hits {
                        let source = hit["source"].as_str().unwrap_or("openverse").to_string();
                        let id = hit["id"].as_str().unwrap_or("").to_string();
                        hit["id"] = json!(format!("{source}:{id}"));
                        hit["preview"] = json!(format!("/v1/editor/library/preview/{source}/{id}"));
                        if let Some(seconds) = hit["seconds"].as_f64() {
                            hit["frames"] = json!((seconds * fps).round());
                        }
                    }
                }
                if let Some(note) = result.get("note").cloned() {
                    result["clip"] = note;
                }
                result["note"] = json!(
                    "搜索结果是来源元数据，不能据此声称已经试听。CC BY 音源需要在成片说明中署名；library_add 会保留真实来源和署名信息。限额命中时请使用已有结果或内置音效，不要循环重试。"
                );
                Ok(result)
            }
            _ => Err(invalid("未知素材库工具")),
        }
    }

    async fn online_library_search(
        &self,
        query: &Value,
        cancel: &CancellationToken,
    ) -> Result<Value, Error> {
        // Reuse the portable search core, including aliases and Chinese terms. Keep
        // one runtime for preparation and ranking, and close it on every outcome.
        let mut runtime = editor::export::browser::Browser::launch(cancel)
            .await
            .map_err(Error::Invalid)?;
        let result = async {
            let prepared = runtime.eval(&format!("BiliupExport.prepareLibrarySearch({query})"))
                .await.map_err(Error::Invalid)?;
            let prepared = serde_json::from_value(prepared)?;
            let response = tokio::select! {
                _=cancel.cancelled()=>return Err(invalid("助手已停止搜索")),
                value=tokio::time::timeout(Duration::from_secs(60),self.library.search(prepared))=>value.map_err(|_|invalid("音源搜索超时"))?.map_err(Error::Invalid)?
            };
            runtime.eval(&format!("BiliupExport.finishLibrarySearch({response},{query})"))
                .await.map_err(Error::Invalid)
        }.await;
        runtime.close().await;
        result
    }
}
