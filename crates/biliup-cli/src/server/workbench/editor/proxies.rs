//! Uncut's automatic preview proxies, adapted to the shared server (MIT).
//! Cache files survive restart and cover the recorded window saved by the project. Export and
//! analysis never use this service. No platform encoder or client filesystem path is used.
use super::{Error, MediaSource, media};
use crate::server::{
    common::processing::{engine, transcode},
    infrastructure::connection_pool::ConnectionPool,
    workbench::retention::MEDIA_LIFECYCLE,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    media_id: String,
    path: String,
    phase: &'static str,
    key: String,
    progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    waiting: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}
struct Entry {
    project: i64,
    id: String,
    path: PathBuf,
    key: String,
    seconds: f64,
    state: Mutex<State>,
    cancel: CancellationToken,
}
pub struct Jobs {
    pool: ConnectionPool,
    root: PathBuf,
    entries: Mutex<BTreeMap<(i64, String), Arc<Entry>>>,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    sync: AsyncMutex<()>,
    slot: Semaphore,
    stopping: AtomicBool,
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
fn invalid(e: impl ToString) -> Error {
    Error::Invalid(e.to_string())
}
async fn fingerprint(path: &Path, source: &MediaSource) -> Result<String, Error> {
    let stat = tokio::fs::metadata(path)
        .await
        .map_err(|_| Error::NotFound)?;
    if !stat.is_file() {
        return Err(Error::NotFound);
    }
    let mut hash = Sha256::new();
    let duration = source.duration_ms().unwrap_or(0);
    match source {
        // Recording segment IDs identify append-only files. Bytes added after the saved
        // duration do not invalidate an already prepared preview of that window.
        MediaSource::Recording(recording) => hash.update(format!(
            "preview|recording|{}|{}|{}|{duration}",
            recording.session_id,
            recording.segment_id,
            path.to_string_lossy()
        )),
        MediaSource::File(_) => {
            let modified = stat
                .modified()
                .map_err(invalid)?
                .duration_since(UNIX_EPOCH)
                .map_err(invalid)?
                .as_nanos();
            hash.update(format!(
                "preview|file|{}|{}|{modified}|{duration}",
                path.to_string_lossy(),
                stat.len()
            ));
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
impl Jobs {
    pub async fn initialize(&self) -> Result<(), sqlx::Error> {
        let mut folders = match tokio::fs::read_dir(&self.root).await {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        while let Some(folder) = folders.next_entry().await? {
            if !folder.file_type().await?.is_dir() {
                continue;
            }
            let Some(project) = folder
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<i64>().ok())
                .filter(|id| *id > 0)
            else {
                continue;
            };
            // Work only in canonical project directories, never an arbitrary filesystem path.
            if folder.file_name() != project.to_string().as_str() {
                continue;
            }
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?)")
                    .bind(project)
                    .fetch_one(&self.pool)
                    .await?;
            if !exists {
                self.clean(project, &BTreeSet::new()).await;
            }
            let mut files = tokio::fs::read_dir(folder.path()).await?;
            while let Some(file) = files.next_entry().await? {
                let name = file.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(".proxy-")
                    && name.ends_with(".tmp")
                    && file.file_type().await?.is_file()
                {
                    tokio::fs::remove_file(file.path()).await?;
                }
            }
        }
        Ok(())
    }
    pub fn new(pool: ConnectionPool, root: PathBuf) -> Self {
        Self {
            pool,
            root: root.join("editor-proxies"),
            entries: Mutex::new(BTreeMap::new()),
            tasks: Mutex::new(Vec::new()),
            sync: AsyncMutex::new(()),
            slot: Semaphore::new(1),
            stopping: AtomicBool::new(false),
        }
    }
    fn output(&self, project: i64, key: &str) -> PathBuf {
        self.root
            .join(project.to_string())
            .join(format!("{key}.mp4"))
    }
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        for entry in lock(&self.entries).values() {
            entry.cancel.cancel();
        }
    }
    pub async fn shutdown(&self) {
        self.stop();
        let tasks = {
            let _sync = self.sync.lock().await;
            std::mem::take(&mut *lock(&self.tasks))
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        for mut task in tasks {
            if tokio::time::timeout_at(deadline, &mut task).await.is_err() {
                task.abort();
                let _ = task.await;
            }
        }
    }
    pub async fn list(&self, project: i64) -> Result<Vec<State>, Error> {
        super::get(&self.pool, project).await?;
        let busy = self.busy().await?;
        Ok(lock(&self.entries)
            .values()
            .filter(|e| e.project == project)
            .filter_map(|e| {
                let mut state = lock(&e.state).clone();
                if state.phase == "queued" {
                    state.waiting = busy.then_some("export");
                }
                (state.phase != "skip").then_some(state)
            })
            .collect())
    }
    /// Serial reconciliation prevents parallel saves/tabs from scheduling duplicate encodes.
    pub async fn sync(self: &Arc<Self>, project: i64) -> Result<Vec<State>, Error> {
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let _sync = self.sync.lock().await;
        if self.stopping.load(Ordering::Acquire) {
            return Err(invalid("代理媒体服务已停止"));
        }
        let saved = super::get(&self.pool, project).await?;
        let mut keep = BTreeSet::new();
        for item in saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["kind"] == "video")
        {
            let Some(id) = item["id"].as_str() else {
                continue;
            };
            let Some(source) = saved.document.sources.get(id) else {
                continue;
            };
            let seconds = source.duration_ms().unwrap_or(0) as f64 / 1000.;
            if seconds <= 0. {
                continue;
            }
            let Ok(path) = media::source_path(&self.pool, source).await else {
                continue;
            };
            let Ok(key) = fingerprint(&path, source).await else {
                continue;
            };
            let identity = (project, id.to_string());
            keep.insert(identity.clone());
            let existing = lock(&self.entries).get(&identity).cloned();
            let cached = tokio::fs::metadata(self.output(project, &key))
                .await
                .is_ok_and(|m| m.len() > 0);
            if existing
                .as_ref()
                .is_some_and(|e| e.key == key && (lock(&e.state).phase != "done" || cached))
            {
                continue;
            }
            let entry = Arc::new(Entry {
                project,
                id: id.into(),
                path,
                key: key.clone(),
                seconds,
                cancel: CancellationToken::new(),
                state: Mutex::new(State {
                    media_id: id.into(),
                    path: item["path"].as_str().unwrap_or("").into(),
                    phase: if cached { "done" } else { "queued" },
                    key,
                    progress: if cached { 1. } else { 0. },
                    waiting: None,
                    error: None,
                }),
            });
            if let Some(old) = lock(&self.entries).insert(identity, entry.clone()) {
                old.cancel.cancel()
            }
            if !cached {
                // Register and spawn without an await between them: disconnecting this
                // HTTP request must not leave a queued entry with no worker to resume it.
                let jobs = self.clone();
                let task = tokio::spawn(async move {
                    jobs.run(entry).await;
                });
                let mut tasks = lock(&self.tasks);
                tasks.retain(|task| !task.is_finished());
                tasks.push(task);
            }
        }
        {
            let mut entries = lock(&self.entries);
            entries.retain(|identity, e| {
                if e.project == project && !keep.contains(identity) {
                    e.cancel.cancel();
                    false
                } else {
                    true
                }
            });
        }
        let keys: BTreeSet<String> = lock(&self.entries)
            .values()
            .filter(|e| e.project == project)
            .map(|e| e.key.clone())
            .collect();
        self.clean(project, &keys).await;
        self.list(project).await
    }
    async fn clean(&self, project: i64, keep: &BTreeSet<String>) {
        let Ok(mut dir) = tokio::fs::read_dir(self.root.join(project.to_string())).await else {
            return;
        };
        while let Ok(Some(file)) = dir.next_entry().await {
            let name = file.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(key) = name.strip_suffix(".mp4") else {
                continue;
            };
            if key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()) && !keep.contains(key)
            {
                let _ = tokio::fs::remove_file(file.path()).await;
            }
        }
    }
    pub async fn forget(&self, project: i64) {
        let _sync = self.sync.lock().await;
        lock(&self.entries).retain(|_, e| {
            if e.project == project {
                e.cancel.cancel();
                false
            } else {
                true
            }
        });
        self.clean(project, &BTreeSet::new()).await;
    }
    pub async fn cancel(&self, project: i64, id: &str) -> Result<(), Error> {
        super::get(&self.pool, project).await?;
        let _sync = self.sync.lock().await;
        let entry = lock(&self.entries)
            .get(&(project, id.into()))
            .cloned()
            .ok_or(Error::NotFound)?;
        let mut state = lock(&entry.state);
        if matches!(state.phase, "queued" | "running") {
            entry.cancel.cancel();
            state.phase = "cancelled";
            state.waiting = None;
        }
        Ok(())
    }
    pub async fn retry(self: &Arc<Self>, project: i64, id: &str) -> Result<Vec<State>, Error> {
        {
            let _sync = self.sync.lock().await;
            let mut entries = lock(&self.entries);
            if let Some(entry) = entries.get(&(project, id.into())) {
                if !matches!(lock(&entry.state).phase, "failed" | "cancelled") {
                    return Err(invalid("此代理无需重试"));
                }
                entry.cancel.cancel();
                entries.remove(&(project, id.into()));
            }
        }
        self.sync(project).await
    }
    /// Validate against current source metadata, including after restart or an external relink.
    pub async fn file(&self, project: i64, id: &str, key: Option<&str>) -> Result<PathBuf, Error> {
        let saved = super::get(&self.pool, project).await?;
        if !saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|m| m["id"] == id && m["kind"] == "video")
        {
            return Err(Error::NotFound);
        }
        let source = saved.document.sources.get(id).ok_or(Error::NotFound)?;
        let path = media::source_path(&self.pool, source).await?;
        self.cached(project, &path, source, key).await
    }
    /// Use a proxy only when it was made from these registered source bytes and duration.
    /// Historical graphic frames may use a surviving cache, never the current ID's new file.
    pub async fn cached(
        &self,
        project: i64,
        path: &Path,
        source: &MediaSource,
        key: Option<&str>,
    ) -> Result<PathBuf, Error> {
        let current = fingerprint(path, source).await?;
        if key.is_some_and(|key| key != current) {
            return Err(Error::NotFound);
        }
        let output = self.output(project, &current);
        if !tokio::fs::metadata(&output)
            .await
            .is_ok_and(|m| m.len() > 0)
        {
            return Err(Error::NotFound);
        }
        Ok(output)
    }
    async fn busy(&self) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_exports WHERE phase IN ('queued','preparing','rendering')) OR EXISTS(SELECT 1 FROM render_jobs WHERE state IN ('queued','running'))").fetch_one(&self.pool).await?)
    }
    async fn run(self: Arc<Self>, entry: Arc<Entry>) {
        let result = self.make(&entry).await;
        let mut state = lock(&entry.state);
        state.waiting = None;
        if entry.cancel.is_cancelled() {
            state.phase = "cancelled";
            return;
        }
        match result {
            Ok(true) => {
                state.phase = "done";
                state.progress = 1.;
            }
            Ok(false) => state.phase = "skip",
            Err(error) => {
                state.phase = "failed";
                state.error = Some(error.to_string());
            }
        }
    }
    async fn make(&self, entry: &Entry) -> Result<bool, Error> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(invalid("代理媒体服务已停止"));
        }
        let cancel = &entry.cancel;
        let _slot = tokio::select! {_ = cancel.cancelled()=>return Err(invalid("代理已取消")),slot=self.slot.acquire()=>slot.map_err(invalid)?};
        loop {
            // Do not enqueue behind the foreground jobs: recheck their state before claiming CPU.
            let (_lifecycle, _permit) = loop {
                if cancel.is_cancelled() {
                    return Err(invalid("代理已取消"));
                }
                let busy = self.busy().await?;
                lock(&entry.state).waiting = busy.then_some("export");
                if !busy {
                    // Match foreground lock order, and never hold the CPU while waiting for
                    // cleanup's write lock (it may be waiting on a foreground reader).
                    if let Ok(lifecycle) = MEDIA_LIFECYCLE.try_read()
                        && let Some(permit) = transcode::try_acquire()
                    {
                        break (lifecycle, permit);
                    }
                }
                tokio::select! {_ = cancel.cancelled()=>return Err(invalid("代理已取消")),_ = tokio::time::sleep(Duration::from_millis(750))=>{}}
            };
            // A long cache encode also yields when a foreground import/probe/export arrives.
            // Incomplete output is discarded; resume from the original when CPU is free again.
            let interrupt = cancel.child_token();
            {
                let work = self.encode(entry, &interrupt);
                tokio::pin!(work);
                loop {
                    tokio::select! {
                        biased;
                        result=&mut work=>return result,
                        _=tokio::time::sleep(Duration::from_millis(750))=>{
                            if transcode::has_waiters()||self.busy().await.unwrap_or(true){
                                interrupt.cancel();
                                let result=work.await;
                                if result.is_ok()||cancel.is_cancelled(){return result}
                                break;
                            }
                        }
                    }
                }
            }
            drop(_permit);
            drop(_lifecycle);
            let mut state = lock(&entry.state);
            state.phase = "queued";
            state.progress = 0.;
        }
    }
    async fn encode(&self, entry: &Entry, cancel: &CancellationToken) -> Result<bool, Error> {
        self.validate(entry).await?;
        let mut probe = crate::tools::ffmpeg_command();
        probe
            .args([
                "-nostdin",
                "-hide_banner",
                "-protocol_whitelist",
                "file,pipe",
                "-format_whitelist",
                super::imports::FORMATS,
                "-i",
            ])
            .arg(&entry.path)
            .args(["-map", "0:v:0", "-frames:v", "1", "-an", "-f", "null", "-"])
            .stdin(Stdio::null());
        let text = engine::run_process(probe, cancel, Duration::from_secs(30), |_, _| {})
            .await
            .map_err(|_| invalid("无法探测代理素材，请检查 FFmpeg 和原片"))?;
        if !needs_proxy(&text) {
            return Ok(false);
        }
        {
            let mut state = lock(&entry.state);
            state.phase = "running";
            state.waiting = None;
        }
        let folder = self.root.join(entry.project.to_string());
        tokio::fs::create_dir_all(&folder).await.map_err(invalid)?;
        let temp = tempfile::Builder::new()
            .prefix(".proxy-")
            .suffix(".tmp")
            .tempfile_in(&folder)
            .map_err(invalid)?
            .into_temp_path();
        let mut command = crate::tools::ffmpeg_command();
        command.args(["-nostdin","-hide_banner","-y","-v","error","-nostats","-progress","pipe:1","-protocol_whitelist","file,pipe","-format_whitelist",super::imports::FORMATS,"-i"]).arg(&entry.path).arg("-t").arg(format!("{:.6}",entry.seconds))
            .args(["-map","0:v:0","-map","0:a:0?","-vf","scale=w='min(1280,iw)':h='min(1280,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2,format=yuv420p","-c:v","libx264","-preset","veryfast","-crf","23","-tune","fastdecode","-g","10","-bf","0","-fps_mode","passthrough","-c:a","aac","-b:a","160k","-tag:v","avc1","-movflags","+faststart","-f","mp4"]).arg(&temp).stdin(Stdio::null());
        engine::run_process(command, cancel, Duration::from_secs(120), |ms, _| {
            lock(&entry.state).progress = (ms as f64 / 1000. / entry.seconds).clamp(0., 0.99);
        })
        .await
        .map_err(|_| invalid("无法生成代理，请检查 FFmpeg、原片和剩余磁盘空间"))?;
        // The source may have changed or the project may have been deleted during encoding.
        let _sync = self.sync.lock().await;
        if cancel.is_cancelled() {
            return Err(invalid("代理已取消"));
        }
        self.validate(entry).await?;
        if tokio::fs::metadata(&temp).await.map_err(invalid)?.len() == 0 {
            return Err(invalid("代理文件为空"));
        }
        let output = self.output(entry.project, &entry.key);
        // Equal cache identities contain identical sources; another project's state is never touched.
        match temp.persist_noclobber(&output) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(invalid(error.error)),
        }
        Ok(true)
    }
    async fn validate(&self, entry: &Entry) -> Result<(), Error> {
        let saved = super::get(&self.pool, entry.project).await?;
        if !saved.document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|m| m["id"] == entry.id && m["kind"] == "video")
        {
            return Err(Error::NotFound);
        }
        let source = saved
            .document
            .sources
            .get(&entry.id)
            .ok_or(Error::NotFound)?;
        let path = media::source_path(&self.pool, source).await?;
        if path != entry.path || fingerprint(&path, source).await? != entry.key {
            return Err(invalid("原片已改变，请刷新代理"));
        }
        Ok(())
    }
}
fn needs_proxy(text: &str) -> bool {
    let input = text.split("Output #").next().unwrap_or(text);
    let Some(video) = input.lines().find(|line| {
        line.contains("Stream #") && line.contains("Video:") && !line.contains("attached pic")
    }) else {
        return false;
    };
    let codec = video
        .split("Video:")
        .nth(1)
        .unwrap_or("")
        .trim()
        .split([',', ' '])
        .next()
        .unwrap_or("");
    let dimensions = regex::Regex::new(r",\s*(\d+)x(\d+)(?:\s|,|\[|$)").unwrap();
    let large = dimensions.captures(video).is_some_and(|c| {
        let w = c[1].parse::<u32>().unwrap_or(0);
        let h = c[2].parse::<u32>().unwrap_or(0);
        w.min(h) >= 1800 || w.max(h) >= 3200
    });
    let bitrate = regex::Regex::new(r"bitrate:\s*(\d+) kb/s")
        .unwrap()
        .captures(input)
        .and_then(|c| c[1].parse::<u64>().ok())
        .unwrap_or(0);
    let container = input
        .lines()
        .find_map(|line| line.trim().strip_prefix("Input #0, "))
        .and_then(|line| line.split(", from ").next())
        .unwrap_or("");
    // Browser video elements cannot seek raw FLV/TS without an index. Even small
    // recordings need a seekable MP4; the demuxed original remains the immediate preview.
    let native_container = container
        .split(',')
        .any(|name| matches!(name.trim(), "mov" | "mp4" | "webm"));
    let native_codec = matches!(codec, "h264" | "vp8" | "vp9" | "av1");
    let audio = input
        .lines()
        .find_map(|line| line.split("Audio:").nth(1))
        .and_then(|line| line.trim().split([',', ' ']).next());
    let native_audio = audio.is_none_or(|codec| matches!(codec, "aac" | "mp3" | "opus" | "vorbis"));
    large
        || !native_container
        || !native_codec
        || !native_audio
        || (matches!(
            codec,
            "h264" | "hevc" | "mpeg2video" | "mpeg4" | "vp8" | "vp9" | "av1"
        ) && bitrate > 30_000)
}
