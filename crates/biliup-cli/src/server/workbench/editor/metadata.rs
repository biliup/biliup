//! Probe newly admitted recordings once and save their metadata with the edit.
//! Opening a project and restoring its history read the same stored media facts.
use super::{ConnectionPool, Document, Error, MediaSource, Result, invalid};
use futures::{StreamExt, TryStreamExt, stream};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};

fn recording_facts(probe: &Value) -> bool {
    ["width", "height"].iter().all(|key| probe[key].as_u64().is_some_and(|value| value > 0))
        && probe["hasAudio"].is_boolean()
        && probe["fps"].as_f64().is_some_and(|value| value.is_finite() && value > 0.)
}

pub(super) async fn prepare(
    pool: &ConnectionPool,
    previous: Option<(i64, i64)>,
    document: &mut Document,
) -> Result<()> {
    // Reject a stale edit before any expensive work. The write transaction checks again.
    let saved = if let Some((project, revision)) = previous {
        let saved = super::get(pool, project).await?;
        if saved.summary.revision != revision {
            return Err(Error::Conflict);
        }
        Some(saved.document)
    } else {
        None
    };
    // Resolve registry identities and imported ranges before touching source files.
    // No FFmpeg work or second pool acquisition while this read transaction is open.
    let mut tx = pool.begin().await?;
    super::resolve_sources(&mut tx, document).await?;
    tx.commit().await?;
    let fps = document.project["canvas"]["fps"].as_f64().unwrap();
    let known: BTreeMap<String, Value> = saved.as_ref().into_iter()
        .flat_map(|saved| saved.project["media"].as_array().into_iter().flatten())
        .filter(|media| recording_facts(&media["probe"]))
        .map(|media| (media["path"].as_str().unwrap().to_string(), media["probe"].clone()))
        .collect();
    // A recording can be referenced by several media IDs. Its source range is kept
    // per identity; the width, frame rate and audio presence belong to the file.
    let sources: BTreeMap<String, MediaSource> = document.project["media"].as_array().unwrap().iter()
        .map(|media| (media["path"].as_str().unwrap().to_string(), document.sources[media["id"].as_str().unwrap()].clone()))
        .collect();
    let facts: BTreeMap<String, Value> = stream::iter(sources)
        .map(|(path, source)| {
            let known = known.get(&path).cloned();
            async move {
                let mut probe = match &source {
                    MediaSource::File(file) => super::imports::probe(pool, file, fps).await.map_err(Error::Invalid)?,
                    MediaSource::Recording(recording) => {
                        let cached = if known.is_some() {
                            known
                        } else if let Some((project, _)) = previous {
                            // Re-import/undo can reuse facts from this project's own saved versions.
                            let raw: Option<String> = sqlx::query_scalar(
                                "SELECT json_extract(m.value,'$.probe') FROM editor_revisions r, json_each(json_extract(r.document_json,'$.project.media')) m \
                                 WHERE r.project_id=? AND json_extract(m.value,'$.path')=? AND json_type(m.value,'$.probe.width')='integer' \
                                 ORDER BY r.revision DESC LIMIT 1",
                            ).bind(project).bind(&path).fetch_optional(pool).await?;
                            raw.map(|raw| serde_json::from_str::<Value>(&raw)).transpose()?
                                .filter(recording_facts)
                        } else {
                            None
                        };
                        if let Some(cached) = cached {
                            cached
                        } else if matches!(recording.state.as_str(), "missing" | "deleted") {
                            // Keep unavailable footage in place; its timeline range is still known.
                            json!({})
                        } else {
                            let work = async {
                                let file = super::media::source_path(pool, &source).await?;
                                super::media::probe(pool, &file, &source, fps).await.map_err(Error::Invalid)
                            };
                            tokio::time::timeout(Duration::from_secs(90), work).await
                                .map_err(|_| invalid("录像信息读取超时，请稍后重试导入"))?
                                .map_err(|error| {
                                    tracing::warn!(segment=recording.segment_id, %error, "读取剪辑录像信息失败");
                                    invalid("无法读取录像信息，请检查文件与 FFmpeg 后重试导入")
                                })?
                        }
                    }
                };
                // Duration is the admitted source snapshot, not a later growing recording.
                // The frame count is recomputed per identity below at this project's fps.
                probe.as_object_mut().unwrap().remove("frames");
                Ok::<_, Error>((path, probe))
            }
        })
        .buffer_unordered(2)
        .try_collect()
        .await?;
    for media in document.project["media"].as_array_mut().unwrap() {
        let path = media["path"].as_str().unwrap();
        let source = &document.sources[media["id"].as_str().unwrap()];
        let mut probe = facts[path].clone();
        if let Some(ms) = source.duration_ms() {
            probe["frames"] = json!((ms as f64 * fps / 1000.).floor().max(1.) as i64);
        }
        media["probe"] = probe;
    }
    super::validate(document)
}
