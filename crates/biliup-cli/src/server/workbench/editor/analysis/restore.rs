//! Restore the analyses previously used for replaced media, including after a server restart.
//! Adapted from Uncut capture/applyDeclared's cachedFor enumeration. No analysis runs in saves.
use super::*;
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeSet;

pub(crate) async fn record_changes(
    conn: &mut SqliteConnection,
    project: i64,
    next: &super::super::Document,
    now: i64,
) -> Result<(), Error> {
    let raw: String = sqlx::query_scalar("SELECT document_json FROM editor_projects WHERE id=?")
        .bind(project)
        .fetch_one(&mut *conn)
        .await?;
    let old: super::super::Document = serde_json::from_str(&raw)?;
    for media in next.project["media"].as_array().into_iter().flatten() {
        let Some(id) = media["id"].as_str() else {
            continue;
        };
        let Some(version) = media["path"].as_str() else {
            continue;
        };
        let Some(previous) = old.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["id"] == id)
        else {
            continue;
        };
        let Some(previous_version) = previous["path"].as_str().filter(|p| *p != version) else {
            continue;
        };
        let Some(source) = old.sources.get(id) else {
            continue;
        };
        sqlx::query("INSERT INTO editor_analysis_restores(project_id,media_id,version,previous_version,previous_source_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?)").bind(project).bind(id).bind(version).bind(previous_version).bind(serde_json::to_string(source)?).bind(now).bind(now).execute(&mut *conn).await?;
    }
    sqlx::query("DELETE FROM editor_analysis_restores WHERE project_id=? AND phase IN ('done','cancelled') AND id NOT IN (SELECT id FROM editor_analysis_restores WHERE project_id=? ORDER BY id DESC LIMIT 80)").bind(project).bind(project).execute(&mut *conn).await?;
    Ok(())
}

pub(super) struct Profile<'a> {
    pub(super) analyzer: &'a str,
    pub(super) model: Option<&'a str>,
    pub(super) language: &'a str,
}

impl Jobs {
    pub async fn retry_restore(
        &self,
        project: i64,
        media_id: &str,
        version: &str,
        job: &str,
    ) -> Result<Vec<Value>, Error> {
        if self.stop.is_cancelled() {
            return Err(invalid("服务正在停止"));
        }
        let id = job
            .strip_prefix("restore-")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| invalid("分析恢复任务标识无效"))?;
        let updated=sqlx::query("UPDATE editor_analysis_restores SET phase='queued',error=NULL,updated_at=? WHERE id=? AND project_id=? AND media_id=? AND version=? AND phase IN ('failed','cancelled') AND EXISTS(SELECT 1 FROM editor_projects p,json_each(json_extract(p.document_json,'$.project.media')) m WHERE p.id=editor_analysis_restores.project_id AND json_extract(m.value,'$.id')=editor_analysis_restores.media_id AND json_extract(m.value,'$.path')=editor_analysis_restores.version)").bind(super::super::now_ms()).bind(id).bind(project).bind(media_id).bind(version).execute(&self.pool).await?;
        if updated.rows_affected() == 0 {
            return Err(invalid("此分析恢复任务已处理或素材已改变"));
        }
        Ok(vec![])
    }
    pub async fn initialize(self: &Arc<Self>) -> sqlx::Result<()> {
        if self
            .restore_started
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }
        if let Err(error) =
            sqlx::query("UPDATE editor_analysis_restores SET phase='queued' WHERE phase='running'")
                .execute(&self.pool)
                .await
        {
            self.restore_started
                .store(false, std::sync::atomic::Ordering::SeqCst);
            return Err(error);
        }
        let service = self.clone();
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {_=service.stop.cancelled()=>break,_=timer.tick()=>{}}
                if lock(&service.restoring).len() >= 2 {
                    continue;
                }
                if lock(&service.entries)
                    .iter()
                    .filter(|entry| active(&lock(&entry.job)))
                    .count()
                    > 27
                {
                    continue;
                }
                let next=sqlx::query_scalar::<_,i64>("SELECT q.id FROM editor_analysis_restores q WHERE q.phase='queued' AND NOT EXISTS(SELECT 1 FROM editor_analysis_restores r WHERE r.phase='running' AND r.project_id=q.project_id AND r.media_id=q.media_id) ORDER BY q.id LIMIT 1").fetch_optional(&service.pool).await;
                let id = match next {
                    Ok(Some(id)) => id,
                    Ok(None) => continue,
                    Err(error) => {
                        tracing::warn!(%error,"cannot read analysis restoration queue");
                        continue;
                    }
                };
                let claimed=sqlx::query("UPDATE editor_analysis_restores SET phase='running',updated_at=? WHERE id=? AND phase='queued'").bind(super::super::now_ms()).bind(id).execute(&service.pool).await;
                if !claimed.is_ok_and(|r| r.rows_affected() == 1) {
                    continue;
                }
                let cancel = service.stop.child_token();
                lock(&service.restoring).insert(id, cancel.clone());
                let task = service.clone();
                tokio::spawn(async move {
                    use futures::FutureExt;
                    let result = std::panic::AssertUnwindSafe(task.restore(id, &cancel))
                        .catch_unwind()
                        .await
                        .unwrap_or_else(|_| Err(invalid("分析恢复中断")));
                    let (phase, error) = if task.stop.is_cancelled() {
                        ("queued", None)
                    } else if cancel.is_cancelled() {
                        ("cancelled", None)
                    } else {
                        match result {
                            Ok(()) => ("done", None),
                            Err(Error::Conflict | Error::NotFound) => ("cancelled", None),
                            Err(error) => ("failed", Some(error.to_string())),
                        }
                    };
                    let _=sqlx::query("UPDATE editor_analysis_restores SET phase=?,error=?,updated_at=? WHERE id=? AND phase='running'").bind(phase).bind(error).bind(super::super::now_ms()).bind(id).execute(&task.pool).await;
                    lock(&task.restoring).remove(&id);
                });
            }
        });
        Ok(())
    }
    pub(super) async fn remember_profile(
        &self,
        project: i64,
        media: &str,
        version: &str,
        Profile {
            analyzer,
            model,
            language,
        }: Profile<'_>,

        key: &str,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO editor_analysis_profiles(project_id,media_id,version,analyzer,model,language,cache_key,updated_at) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(project_id,media_id,version,analyzer,model,language) DO UPDATE SET cache_key=excluded.cache_key,updated_at=excluded.updated_at").bind(project).bind(media).bind(version).bind(analyzer).bind(model.unwrap_or("")).bind(language).bind(key).bind(super::super::now_ms()).execute(&self.pool).await?;
        Ok(())
    }
    pub(super) async fn restore_jobs(&self, project: i64) -> Result<Vec<Value>, Error> {
        let rows=sqlx::query("SELECT id,media_id,version,phase,error,created_at,updated_at FROM editor_analysis_restores WHERE project_id=? AND phase IN ('queued','running','failed') ORDER BY id DESC LIMIT 40").bind(project).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|row|json!({"id":format!("restore-{}",row.get::<i64,_>("id")),"projectDir":format!("biliup:{project}"),"mediaId":row.get::<String,_>("media_id"),"mediaIds":[row.get::<String,_>("media_id")],"version":row.get::<String,_>("version"),"analyzer":"restore","phase":row.get::<String,_>("phase"),"progress":0,"error":row.get::<Option<String>,_>("error"),"startedAt":chrono::DateTime::from_timestamp_millis(row.get("created_at")).map(|time|time.to_rfc3339())})).collect())
    }
    pub(super) async fn cancel_restore(&self, project: i64, id: i64) -> Result<(), Error> {
        let result=sqlx::query("UPDATE editor_analysis_restores SET phase='cancelled',updated_at=? WHERE project_id=? AND id=? AND phase IN ('queued','running','failed')").bind(super::super::now_ms()).bind(project).bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        if let Some(cancel) = lock(&self.restoring).get(&id) {
            cancel.cancel();
        }
        Ok(())
    }
    async fn restore(self: &Arc<Self>, id: i64, cancel: &CancellationToken) -> Result<(), Error> {
        let row=sqlx::query("SELECT project_id,media_id,version,previous_version,previous_source_json FROM editor_analysis_restores WHERE id=? AND phase='running'").bind(id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let project: i64 = row.get("project_id");
        let media_id: String = row.get("media_id");
        let version: String = row.get("version");
        let previous_version: String = row.get("previous_version");
        let source: super::super::MediaSource =
            serde_json::from_str(row.get("previous_source_json"))?;
        // Lazily discover caches made before the profile index existed. This also preserves
        // requested analysis kinds through several replacements queued before this worker runs.
        let _guard = MEDIA_LIFECYCLE.read().await;
        if let Ok(path) = media::source_path(&self.pool, &source).await {
            let seconds = source.duration_ms().unwrap_or(0) as f64 / 1000.;
            for analyzer in ["loudness", "beats", "video", "transcript"] {
                let model = if analyzer == "transcript" {
                    Some(self.models.selected().await)
                } else {
                    None
                };
                let cache_analyzer = model
                    .as_ref()
                    .map(|m| format!("transcript-v1-{m}-auto"))
                    .unwrap_or(analyzer.into());
                if let Ok(key) = fingerprint_for(&path, seconds, &cache_analyzer).await
                    && read_for(&self.root, &key, analyzer).await.is_some()
                {
                    self.remember_profile(
                        project,
                        &media_id,
                        &previous_version,
                        Profile {
                            analyzer,
                            model: model.as_deref(),
                            language: "auto",
                        },
                        &key,
                    )
                    .await?;
                }
            }
        }
        drop(_guard);
        let profiles=sqlx::query("SELECT analyzer,model,language FROM editor_analysis_profiles WHERE project_id=? AND media_id=? GROUP BY analyzer,model,language ORDER BY MAX(updated_at) DESC LIMIT 32").bind(project).bind(&media_id).fetch_all(&self.pool).await?;
        let saved = super::super::get(&self.pool, project).await?;
        let current = saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["id"] == media_id && m["path"] == version)
            .ok_or(Error::Conflict)?;
        let kind = current["kind"].as_str().unwrap_or_default();
        let silent = current["probe"]["hasAudio"] == false;
        let mut waiting = BTreeSet::new();
        let mut owned = BTreeSet::new();
        let mut problems = Vec::new();
        for profile in profiles {
            if cancel.is_cancelled() {
                break;
            }
            let analyzer: String = profile.get("analyzer");
            let model: String = profile.get("model");
            let language: String = profile.get("language");
            if kind == "image"
                || (analyzer == "video" && kind != "video")
                || (analyzer != "video" && silent)
            {
                continue;
            }
            match self
                .start_configured(
                    project,
                    media_id.clone(),
                    Some(vec![analyzer.clone()]),
                    &language,
                    (!model.is_empty()).then_some(model.as_str()),
                    Some(&version),
                )
                .await
            {
                Ok(started) => {
                    for job in started {
                        if let Some(id) = job["jobId"].as_str() {
                            waiting.insert(id.to_string());
                            if job["created"] == true {
                                owned.insert(id.to_string());
                            }
                        }
                    }
                }
                Err(Error::Conflict) => break,
                Err(error) => problems.push(format!("{analyzer}: {error}")),
            }
        }
        while !waiting.is_empty() && !cancel.is_cancelled() {
            let current = super::super::get(&self.pool, project).await;
            if !current.is_ok_and(|saved| {
                saved.document.project["media"]
                    .as_array()
                    .is_some_and(|all| {
                        all.iter()
                            .any(|m| m["id"] == media_id && m["path"] == version)
                    })
            }) {
                for job in &owned {
                    let _ = self.cancel(project, job).await;
                }
                return Err(Error::Conflict);
            }
            let mut completed = Vec::new();
            for job in &waiting {
                match self.status(project, job).await? {
                    Some(value) if active(&value) => {}
                    Some(value) => {
                        if value["phase"] == "failed" {
                            problems.push(value["error"].as_str().unwrap_or("分析失败").into());
                        }
                        completed.push(job.clone());
                    }
                    None => {
                        problems.push("分析任务已丢失，可重新分析素材".into());
                        completed.push(job.clone());
                    }
                }
            }
            for job in completed {
                waiting.remove(&job);
            }
            tokio::select! {_=cancel.cancelled()=>{},_=tokio::time::sleep(Duration::from_millis(700))=>{}}
        }
        if cancel.is_cancelled() {
            for job in owned {
                let _ = self.cancel(project, &job).await;
            }
            return Ok(());
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(invalid(format!(
                "素材分析恢复未完成：{}",
                problems.join("；")
            )))
        }
    }
}
