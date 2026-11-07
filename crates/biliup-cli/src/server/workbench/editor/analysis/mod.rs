//! Project-scoped, cancellable analysis jobs with reusable fingerprinted results.
pub(crate) mod beats;
pub(crate) mod loudness;
pub(crate) mod restore;
pub(crate) mod speech_models;
#[cfg(test)]
mod tests;
mod transcript;
pub(crate) mod video;
use super::{Error, media};
use crate::server::{
    infrastructure::connection_pool::ConnectionPool, workbench::retention::MEDIA_LIFECYCLE,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, UNIX_EPOCH},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::sync::CancellationToken;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
pub struct CacheQuery<'a> {
    pub analyzer: &'a str,
    pub requested_language: &'a str,
    pub version: Option<&'a str>,
    pub selected_model: Option<&'a str>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn active(value: &Value) -> bool {
    matches!(value["phase"].as_str(), Some("queued" | "running"))
}
fn language(value: &str) -> Result<&str, Error> {
    if value == "auto"
        || ((2..=3).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_lowercase()))
    {
        Ok(value)
    } else {
        Err(invalid(
            "转写语言应为 auto 或 2 至 3 个小写英文字母的语言代码",
        ))
    }
}
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
struct Entry {
    project: i64,
    analyzer: String,
    model: Option<String>,
    language: String,
    cache_analyzer: String,
    key: String,
    version: String,
    id: String,
    cancel: CancellationToken,
    job: Mutex<Value>,
}
pub struct Jobs {
    pub models: Arc<speech_models::Models>,
    pool: ConnectionPool,
    root: PathBuf,
    entries: Mutex<Vec<Arc<Entry>>>,
    stop: CancellationToken,
    restoring: Mutex<std::collections::HashMap<i64, CancellationToken>>,
    restore_started: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
pub async fn fingerprint(path: &Path, seconds: f64) -> Result<String, String> {
    fingerprint_for(path, seconds, "loudness").await
}
async fn fingerprint_for(path: &Path, seconds: f64, analyzer: &str) -> Result<String, String> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(format!(
        "{analyzer}-v2|{}|{}|{}|{seconds:.3}",
        path.display(),
        metadata.len(),
        metadata
            .modified()
            .ok()
            .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_nanos())
            .unwrap_or(0)
    ));
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0; 64 * 1024];
    let size = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
    hash.update(&buffer[..size]);
    if metadata.len() > buffer.len() as u64 {
        file.seek(std::io::SeekFrom::End(-(buffer.len() as i64)))
            .await
            .map_err(|e| e.to_string())?;
        let size = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[cfg(test)]
pub async fn read(root: &Path, key: &str) -> Option<Value> {
    read_for(root, key, "loudness").await
}
async fn read_for(root: &Path, key: &str, analyzer: &str) -> Option<Value> {
    let path = root
        .join("editor-cache")
        .join(format!("analysis-{key}.json"));
    if tokio::fs::metadata(&path).await.ok()?.len() > 64 * 1024 * 1024 {
        return None;
    }
    let result: Value = serde_json::from_slice(&tokio::fs::read(path).await.ok()?).ok()?;
    if analyzer == "transcript" {
        return transcript::valid(&result).then_some(result);
    }
    if analyzer == "beats" {
        return beats::valid(&result).then_some(result);
    }
    if analyzer == "video" {
        return video::valid(&result).then_some(result);
    }
    if result["hasAudio"] == false
        || result["hasAudio"] == true
            && result["envelope"]["rate"] == 100
            && result["envelope"]["data"].is_string()
    {
        Some(result)
    } else {
        None
    }
}
async fn write(root: &Path, key: &str, value: &Value) -> Result<(), String> {
    let directory = root.join("editor-cache");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| e.to_string())?;
    let file = tempfile::Builder::new()
        .prefix("analysis-")
        .tempfile_in(&directory)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    tokio::fs::write(&file, value.to_string())
        .await
        .map_err(|e| e.to_string())?;
    file.persist(directory.join(format!("analysis-{key}.json")))
        .map_err(|e| e.to_string())?;
    Ok(())
}
/// Also used by exports while they own the encoder permit, so ducking hears the same envelope.
pub async fn measure(
    root: &Path,
    path: &Path,
    seconds: f64,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    measure_for(root, path, seconds, "loudness", cancel, progress).await
}
async fn measure_for(
    root: &Path,
    path: &Path,
    seconds: f64,
    analyzer: &str,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    let key = fingerprint_for(path, seconds, analyzer).await?;
    if let Some(value) = read_for(root, &key, analyzer).await {
        return Ok(value);
    }
    let result = match analyzer {
        "loudness" => loudness::analyze(path, seconds, cancel, progress).await?,
        "beats" => beats::analyze(path, seconds, cancel, progress).await?,
        "video" => video::analyze(path, seconds, cancel, progress).await?,
        _ => return Err("请求的分析器尚未接入".into()),
    };
    if cancel.is_cancelled() {
        return Err("分析已取消".into());
    }
    if fingerprint_for(path, seconds, analyzer).await? != key {
        return Err("素材在分析过程中发生变化，请重试".into());
    }
    write(root, &key, &result).await?;
    Ok(result)
}
impl Jobs {
    async fn require_project(&self, project: i64) -> Result<(), Error> {
        let exists = sqlx::query_scalar::<_, i64>("SELECT id FROM editor_projects WHERE id=?")
            .bind(project)
            .fetch_optional(&self.pool)
            .await?;
        exists.map(|_| ()).ok_or(Error::NotFound)
    }
    pub fn new(pool: ConnectionPool, root: PathBuf) -> Self {
        Self {
            models: Arc::new(speech_models::Models::new(&root)),
            pool,
            root,
            entries: Mutex::new(Vec::new()),
            stop: CancellationToken::new(),
            restoring: Mutex::new(std::collections::HashMap::new()),
            restore_started: std::sync::atomic::AtomicBool::new(false),
        }
    }
    pub async fn list(&self, project: i64) -> Result<Vec<Value>, Error> {
        let saved = super::get(&self.pool, project).await?;
        let versions = saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| Some((m["id"].as_str()?, m["path"].as_str()?)))
            .collect::<std::collections::HashMap<_, _>>();
        let mut jobs = lock(&self.entries)
            .iter()
            .rev()
            .filter(|entry| entry.project == project)
            .filter_map(|entry| {
                let mut job = lock(&entry.job).clone();
                let ids = job["mediaIds"]
                    .as_array()?
                    .iter()
                    .filter(|id| {
                        id.as_str()
                            .and_then(|id| versions.get(id))
                            .is_some_and(|path| *path == entry.version)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if ids.is_empty() {
                    return None;
                }
                job["mediaId"] = ids[0].clone();
                job["mediaIds"] = json!(ids);
                Some(job)
            })
            .take(40)
            .collect::<Vec<_>>();
        jobs.extend(self.restore_jobs(project).await?);
        jobs.retain(|job| {
            job["mediaId"]
                .as_str()
                .and_then(|id| versions.get(id))
                .is_some_and(|version| Some(*version) == job["version"].as_str())
        });
        Ok(jobs)
    }
    pub fn stop(&self) {
        self.stop.cancel();
        self.models.stop();
    }
    pub(crate) async fn status(&self, project: i64, id: &str) -> Result<Option<Value>, Error> {
        self.require_project(project).await?;
        Ok(lock(&self.entries)
            .iter()
            .find(|entry| entry.project == project && entry.id == id)
            .map(|entry| lock(&entry.job).clone()))
    }
    pub(crate) async fn job_result(
        &self,
        project: i64,
        id: &str,
        media_id: &str,
    ) -> Result<Value, Error> {
        self.require_project(project).await?;
        let entry = lock(&self.entries)
            .iter()
            .find(|entry| entry.project == project && entry.id == id)
            .cloned()
            .ok_or(Error::NotFound)?;
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let (path, source) = media::source(&self.pool, project, media_id).await?;
        let seconds = source.duration_ms().unwrap_or(0) as f64 / 1000.;
        if fingerprint_for(&path, seconds, &entry.cache_analyzer)
            .await
            .map_err(invalid)?
            != entry.key
        {
            return Err(invalid("素材已改变，请重新分析"));
        }
        Ok(json!({"result":read_for(&self.root,&entry.key,&entry.analyzer).await}))
    }
    pub async fn cached(
        &self,
        project: i64,
        media_id: &str,
        analyzer: &str,
        omit: &[String],
    ) -> Result<Value, Error> {
        self.cached_with_language(project, media_id, analyzer, omit, "auto")
            .await
    }
    pub(crate) async fn cached_with_language(
        &self,
        project: i64,
        media_id: &str,
        analyzer: &str,
        omit: &[String],
        requested_language: &str,
    ) -> Result<Value, Error> {
        self.cached_version(
            project,
            media_id,
            CacheQuery {
                analyzer,
                requested_language,
                version: None,
                selected_model: None,
            },
            omit,
        )
        .await
    }
    pub async fn cached_version(
        &self,
        project: i64,
        media_id: &str,
        CacheQuery {
            analyzer,
            requested_language,
            version,
            selected_model,
        }: CacheQuery<'_>,
        omit: &[String],
    ) -> Result<Value, Error> {
        let language = language(requested_language)?;
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let (path, source) = media::version(&self.pool, project, media_id, version).await?;
        if !matches!(analyzer, "loudness" | "beats" | "transcript" | "video") {
            return Ok(json!({"result":null}));
        }
        let seconds = source.duration_ms().unwrap_or(0) as f64 / 1000.;
        let cache_analyzer = if analyzer == "transcript" {
            let model = match selected_model {
                Some(id) => speech_models::model(id).map_err(invalid)?.id.to_owned(),
                None => self.models.selected().await,
            };
            format!("transcript-v1-{model}-{language}")
        } else {
            analyzer.to_owned()
        };
        let key = fingerprint_for(&path, seconds, &cache_analyzer)
            .await
            .map_err(invalid)?;
        let mut result = read_for(&self.root, &key, analyzer)
            .await
            .unwrap_or(Value::Null);
        if let Some(value) = result.as_object_mut() {
            for field in omit {
                value.remove(field);
            }
        }
        Ok(json!({"result":result}))
    }
    pub async fn start(
        self: &Arc<Self>,
        project: i64,
        media_id: String,
        analyzers: Option<Vec<String>>,
    ) -> Result<Vec<Value>, Error> {
        self.start_with_language(project, media_id, analyzers, "auto")
            .await
    }
    pub(crate) async fn start_with_language(
        self: &Arc<Self>,
        project: i64,
        media_id: String,
        analyzers: Option<Vec<String>>,
        requested_language: &str,
    ) -> Result<Vec<Value>, Error> {
        self.start_configured(project, media_id, analyzers, requested_language, None, None)
            .await
    }
    pub(crate) async fn start_configured(
        self: &Arc<Self>,
        project: i64,
        media_id: String,
        analyzers: Option<Vec<String>>,
        requested_language: &str,
        selected_model: Option<&str>,
        expected_version: Option<&str>,
    ) -> Result<Vec<Value>, Error> {
        if self.stop.is_cancelled() {
            return Err(invalid("服务正在停止"));
        }
        let language = language(requested_language)?;
        let mut analyzers = analyzers.unwrap_or_else(|| vec!["loudness".into()]);
        if analyzers.is_empty()
            || analyzers.len() > 8
            || analyzers.iter().any(|analyzer| {
                !matches!(
                    analyzer.as_str(),
                    "loudness" | "beats" | "transcript" | "video"
                )
            })
        {
            return Err(invalid("请求的分析器尚未接入"));
        }
        let mut unique = std::collections::HashSet::new();
        analyzers.retain(|analyzer| unique.insert(analyzer.clone()));
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let saved = super::get(&self.pool, project).await?;
        let version = saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["id"] == media_id)
            .and_then(|m| m["path"].as_str())
            .ok_or(Error::NotFound)?
            .to_string();
        if expected_version.is_some_and(|expected| expected != version) {
            return Err(Error::Conflict);
        }
        let source = saved
            .document
            .sources
            .get(&media_id)
            .ok_or(Error::NotFound)?
            .clone();
        let path = media::source_path(&self.pool, &source).await?;
        let seconds = source.duration_ms().unwrap_or(0) as f64 / 1000.;
        if seconds <= 0. {
            return Err(invalid("此素材没有可分析的时长"));
        }
        let mut targets = vec![];
        for analyzer in analyzers {
            let model = if analyzer == "transcript" {
                Some(if let Some(model) = selected_model {
                    model.to_owned()
                } else {
                    self.models.selected().await
                })
            } else {
                None
            };
            let cache_analyzer = if let Some(model) = &model {
                if !speech_models::model(model).map_err(invalid)?.multilingual
                    && !matches!(language, "auto" | "en")
                {
                    return Err(invalid("所选语音模型只支持英语，请选择多语言模型"));
                }
                format!("transcript-v1-{model}-{language}")
            } else {
                analyzer.clone()
            };
            let key = fingerprint_for(&path, seconds, &cache_analyzer)
                .await
                .map_err(invalid)?;
            let cached = read_for(&self.root, &key, &analyzer).await.is_some();
            if cached {
                self.remember_profile(
                    project,
                    &media_id,
                    &version,
                    restore::Profile {
                        analyzer: &analyzer,
                        model: model.as_deref(),
                        language,
                    },
                    &key,
                )
                .await?;
            }
            if !cached && let Some(model) = &model {
                speech_models::model(model).map_err(invalid)?;
                if speech_models::binary().is_none() {
                    return Err(invalid(
                        "未找到 whisper-cli，请先配置服务器上的 whisper.cpp",
                    ));
                }
            }
            targets.push((analyzer, key, cached, model, cache_analyzer));
        }
        let mut result = vec![];
        let mut pending = vec![];
        {
            let mut entries = lock(&self.entries);
            let count = entries
                .iter()
                .filter(|entry| active(&lock(&entry.job)))
                .count();
            let needed = targets
                .iter()
                .filter(|(_, key, cached, _, _)| {
                    !cached
                        && !entries.iter().any(|entry| {
                            entry.project == project
                                && entry.version == version
                                && entry.key == *key
                                && active(&lock(&entry.job))
                                && !entry.cancel.is_cancelled()
                        })
                })
                .count();
            if count + needed > 32 {
                return Err(invalid("分析队列已满，请等待现有任务完成"));
            }
            if entries.len() >= 200 {
                entries.retain(|entry| active(&lock(&entry.job)));
            }
            for (analyzer, key, cached, model, cache_analyzer) in targets {
                if cached {
                    result.push(json!({"analyzer":analyzer,"state":"done"}));
                    continue;
                }
                if let Some(entry) = entries.iter().find(|entry| {
                    entry.project == project
                        && entry.version == version
                        && entry.key == key
                        && active(&lock(&entry.job))
                        && !entry.cancel.is_cancelled()
                }) {
                    let mut job = lock(&entry.job);
                    let media_ids = job["mediaIds"].as_array_mut().unwrap();
                    if !media_ids.iter().any(|id| id == &media_id) {
                        media_ids.push(json!(media_id));
                    }
                    result.push(json!({"analyzer":analyzer,"state":job["phase"],"jobId":entry.id}));
                    continue;
                }
                let id = format!(
                    "analysis-{}-{}",
                    chrono::Utc::now().timestamp_micros(),
                    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                );
                let entry = Arc::new(Entry {
                    project,
                    analyzer: analyzer.clone(),
                    model,
                    language: language.into(),
                    cache_analyzer,
                    key,
                    version: version.clone(),
                    id: id.clone(),
                    cancel: self.stop.child_token(),
                    job: Mutex::new(
                        json!({"id":id,"projectDir":format!("biliup:{project}"),"mediaId":media_id,"mediaIds":[media_id],"version":version,"analyzer":analyzer,"phase":"queued","progress":0,"startedAt":chrono::Utc::now().to_rfc3339()}),
                    ),
                });
                entries.push(entry.clone());
                result.push(
                    json!({"analyzer":analyzer,"state":"queued","jobId":entry.id,"created":true}),
                );
                pending.push(entry);
            }
        }
        for entry in pending {
            let service = self.clone();
            let path = path.clone();
            tokio::spawn(async move {
                service.run(entry, path, seconds).await;
            });
        }
        Ok(result)
    }
    pub async fn cancel(&self, project: i64, id: &str) -> Result<(), Error> {
        self.require_project(project).await?;
        if let Some(id) = id
            .strip_prefix("restore-")
            .and_then(|id| id.parse::<i64>().ok())
        {
            return self.cancel_restore(project, id).await;
        }
        let entries = lock(&self.entries);
        let entry = entries
            .iter()
            .find(|entry| entry.project == project && entry.id == id)
            .ok_or(Error::NotFound)?;
        if active(&lock(&entry.job)) {
            entry.cancel.cancel();
        }
        Ok(())
    }
    async fn run(&self, entry: Arc<Entry>, path: PathBuf, seconds: f64) {
        let result=async {
            if let Some(model)=&entry.model {
                {let mut job=lock(&entry.job);job["phase"]=json!("running");job["stage"]=json!("model");}
                self.models.ensure(model,&entry.cancel,|fraction|lock(&entry.job)["progress"]=json!(fraction)).await?;
                {let mut job=lock(&entry.job);job["phase"]=json!("queued");job["progress"]=json!(0);job.as_object_mut().unwrap().remove("stage");}
            }
            // Exports take precedence; queued analysis does not consume an encoder slot.
            loop {
                if entry.cancel.is_cancelled(){return Err("分析已取消".into());}
                let exporting:i64=sqlx::query_scalar("SELECT count(*) FROM editor_exports WHERE phase IN ('queued','preparing','rendering')").fetch_one(&self.pool).await.map_err(|e|e.to_string())?;
                if exporting==0{break;}
                lock(&entry.job)["waiting"]=json!("export");
                tokio::select!{_=entry.cancel.cancelled()=>return Err("分析已取消".into()),_=tokio::time::sleep(Duration::from_millis(500))=>{}}
            }
            let _lifecycle=tokio::select!{_=entry.cancel.cancelled()=>return Err("分析已取消".into()),guard=MEDIA_LIFECYCLE.read()=>guard};
            let _permit=crate::server::common::processing::transcode::acquire(&entry.cancel).await?;
            let media_id=lock(&entry.job)["mediaId"].as_str().unwrap().to_owned();
            let (registered,source)=media::source(&self.pool,entry.project,&media_id).await.map_err(|e|e.to_string())?;
            if registered!=path||source.duration_ms().unwrap_or(0) as f64/1000.!=seconds{return Err("工程素材在等待分析时发生变化，请重试".into());}
            if fingerprint_for(&path,seconds,&entry.cache_analyzer).await?!=entry.key{return Err("素材在等待分析时发生变化，请重试".into());}
            {let mut job=lock(&entry.job);job["phase"]=json!("running");job.as_object_mut().unwrap().remove("waiting");}
            if let Some(model)=&entry.model {
                if read_for(&self.root,&entry.key,"transcript").await.is_none(){
                    let result=transcript::analyze(&path,seconds,model,&entry.language,&self.models,&entry.cancel,|fraction|lock(&entry.job)["progress"]=json!(fraction)).await?;
                    if entry.cancel.is_cancelled(){return Err("分析已取消".into());}
                    if fingerprint_for(&path,seconds,&entry.cache_analyzer).await?!=entry.key{return Err("素材在转写时发生变化，请重试".into());}
                    write(&self.root,&entry.key,&result).await?;
                }
            }else{measure_for(&self.root,&path,seconds,&entry.analyzer,&entry.cancel,|fraction|lock(&entry.job)["progress"]=json!(fraction)).await?;}
            let media_ids=lock(&entry.job)["mediaIds"].as_array().cloned().unwrap_or_default();
            for media_id in media_ids.iter().filter_map(Value::as_str){self.remember_profile(entry.project, media_id, &entry.version, restore::Profile { analyzer: &entry.analyzer, model: entry.model.as_deref(), language: &entry.language }, &entry.key).await.map_err(|e|e.to_string())?;}
            Ok::<_,String>(())
        }.await;
        let mut job = lock(&entry.job);
        job["finishedAt"] = json!(chrono::Utc::now().to_rfc3339());
        job.as_object_mut().unwrap().remove("waiting");
        if entry.cancel.is_cancelled() {
            job["phase"] = json!("cancelled");
        } else {
            match result {
                Ok(()) => {
                    job["phase"] = json!("done");
                    job["progress"] = json!(1);
                }
                Err(error) => {
                    job["phase"] = json!("failed");
                    job["error"] = json!(error);
                }
            }
        }
    }
}
