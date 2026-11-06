//! At-target preview measures the exact export audio graph (Uncut loudnessGain, MIT).
use super::{Progress, audio, browser::Browser, jobs::Jobs};
use crate::server::{
    common::processing::transcode,
    workbench::{editor, retention::MEDIA_LIFECYCLE},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) async fn measure(
    browser: &mut Browser,
    target: f64,
    directory: &Path,
    cancel: &CancellationToken,
    preview: bool,
    mut progress: impl FnMut(Progress),
) -> Result<Value, String> {
    let mut state = browser
        .eval(&format!(
            "BiliupExport.measure({target},undefined,{preview})"
        ))
        .await?;
    for index in 0..8 {
        if state["done"] == true {
            return Ok(state["result"]
                .as_object()
                .map(|_| state["result"].clone())
                .unwrap_or(json!({})));
        }
        if let Some(error) = state["error"].as_str() {
            return Err(error.to_string());
        }
        let pass: audio::Command = serde_json::from_value(state["pass"].clone())
            .map_err(|e| format!("响度分析失败：{e}"))?;
        let stderr = audio::run(pass, directory, cancel, |processed_ms, total_ms| {
            progress(Progress::Loudness { pass: index + 1, processed_ms, total_ms });
        }).await?;
        state = browser
            .eval(&format!(
                "BiliupExport.measure({target},{},{preview})",
                serde_json::to_string(&stderr).unwrap()
            ))
            .await?;
    }
    if state["done"] == true {
        Ok(state["result"]
            .as_object()
            .map(|_| state["result"].clone())
            .unwrap_or(json!({})))
    } else {
        Err(state["error"].as_str().unwrap_or("响度分析未完成").into())
    }
}
impl Jobs {
    /// The response lifetime owns this preview measurement, including time in the queue.
    /// Separate viewers do not cancel each other's requests or share mutable gain state.
    pub fn estimate_stream(
        self: &Arc<Self>, project: i64, revision: i64, target: f64,
    ) -> impl futures::Stream<Item = Result<String, std::convert::Infallible>> + Send + 'static {
        let (updates, receiver) = tokio::sync::watch::channel(json!({"type":"progress"}));
        let jobs = self.clone();
        tokio::spawn(async move {
            let cancel = CancellationToken::new();
            let _guard = cancel.clone().drop_guard();
            let work = jobs.estimate(project, revision, target, &cancel, |progress| {
                let step = match progress {
                    Progress::Sound { done, total } => json!({"kind":"sound","done":done,"total":total}),
                    Progress::Loudness { pass, processed_ms, total_ms } => json!({"kind":"loudness","pass":pass,"done":processed_ms,"total":total_ms}),
                    _ => return,
                };
                let _ = updates.send_replace(json!({"type":"progress","step":step}));
            });
            tokio::pin!(work);
            let result = tokio::select! {
                result = &mut work => result,
                _ = updates.closed() => { cancel.cancel(); let _ = work.await; return; }
            };
            let event = match result {
                Ok(value) => json!({"type":"result","result":value}),
                Err(error) => json!({"type":"error","message":error}),
            };
            let _ = updates.send_replace(event);
        });
        futures::stream::unfold((receiver, false, false), |(mut receiver, started, finished)| async move {
            if finished { return None; }
            if started {
                tokio::select! {
                    changed = receiver.changed() => { if changed.is_err() { return None; } }
                    _ = tokio::time::sleep(Duration::from_secs(15)) => {
                        // A transport heartbeat is not media progress and never resets FFmpeg's watchdog.
                        return Some((Ok("{\"type\":\"keepalive\"}\n".to_string()), (receiver, true, false)));
                    }
                }
            }
            let event = receiver.borrow_and_update().clone();
            let finished = event["type"] == "result" || event["type"] == "error";
            Some((Ok(format!("{event}\n")), (receiver, true, finished)))
        })
    }

    async fn estimate(
        self: &Arc<Self>,
        project: i64,
        revision: i64,
        target: f64,
        token: &CancellationToken,
        mut report: impl FnMut(Progress),
    ) -> Result<Value, String> {
        if !target.is_finite() || !(-70. ..=-5.).contains(&target) {
            return Err("响度目标无效".into());
        }
            let _lifecycle = tokio::select! {
                _ = token.cancelled() => return Ok(json!({"cancelled":true})),
                guard = MEDIA_LIFECYCLE.read() => guard,
            };
            let mut saved = editor::get(&self.pool, project)
                .await
                .map_err(|e| e.to_string())?;
            if saved.summary.revision != revision {
                return Ok(json!({"cancelled":true}));
            }
            let _permit = transcode::acquire(token).await?;
            report(Progress::Sound { done: 0, total: 0 });
            let (mut browser, paths) = super::inputs::prepare_in_slot(
                &self.pool, &mut saved.document, &Value::Null, "audio", token, &_permit,
            ).await?;
            let result = async {
                let project_data = &saved.document.project;
                let sources = browser
                    .eval(&format!("BiliupExport.envelopeSources({project_data})"))
                    .await?;
                let mut envelopes = BTreeMap::new();
                let sources = sources.as_array().ok_or("音频分析计划无效")?;
                report(Progress::Sound { done: 0, total: sources.len() });
                for (index, source) in sources.iter().enumerate() {
                    let id = source["id"].as_str().ok_or("音频素材标识无效")?;
                    let value = if source["hasAudio"] == false {
                        json!({"hasAudio":false})
                    } else {
                        let path = paths.get(id).ok_or("音频素材不可读取")?;
                        let seconds = source["seconds"]
                            .as_f64()
                            .filter(|v| v.is_finite() && *v > 0.)
                            .ok_or("音频素材时长无效")?;
                        editor::analysis::measure(
                            &self.root,
                            Path::new(path),
                            seconds,
                            token,
                            |_| {},
                        )
                        .await?
                    };
                    envelopes.insert(id.to_string(), value);
                    report(Progress::Sound { done: index + 1, total: sources.len() });
                }
                browser
                    .eval(&format!(
                        "BiliupExport.prepareLoudness({project_data},{},{})",
                        serde_json::to_string(&paths).unwrap(),
                        serde_json::to_string(&envelopes).unwrap()
                    ))
                    .await?;
                let temp = tempfile::Builder::new()
                    .prefix("editor-loudness-")
                    .tempdir()
                    .map_err(|e| e.to_string())?;
                measure(&mut browser, target, temp.path(), token, true, &mut report).await
            }
            .await;
            browser.close().await;
            if token.is_cancelled() { return Ok(json!({"cancelled":true})); }
            let latest: Option<i64> =
                sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
                    .bind(project)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            if latest != Some(revision) || token.is_cancelled() {
                return Ok(json!({"cancelled":true}));
            }
            result
    }
}
