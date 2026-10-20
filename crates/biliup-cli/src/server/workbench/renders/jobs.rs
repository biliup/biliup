//! Persistent render queue with immutable recipes and cooperative cancellation.
use super::{assets, engine, model::*, store};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::{clips, recorder, session_groups, store as recordings};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use tokio_util::sync::CancellationToken;

pub struct RenderJobs {
    pool: ConnectionPool,
    root: PathBuf,
    tokens: Mutex<HashMap<i64, CancellationToken>>,
}
#[derive(Default)]
struct ProgressSamples {
    fps: Option<f64>,
    speed: Option<f64>,
}
impl ProgressSamples {
    fn retain(&mut self, mut progress: engine::EngineProgress) -> engine::EngineProgress {
        self.fps = progress.fps.or(self.fps);
        self.speed = progress.speed.or(self.speed);
        progress.fps = self.fps;
        progress.speed = self.speed;
        progress
    }
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A bounded filename stem works on both Unix and Windows. The task ID added
/// by the caller provides uniqueness even after unsafe characters are removed.
fn output_file_name(name: Option<&str>, id: i64) -> String {
    let mut stem = String::new();
    for c in name.unwrap_or("合成").chars() {
        let c = if c.is_control()
            || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        {
            '_'
        } else {
            c
        };
        if stem.len() + c.len_utf8() > 120 {
            break;
        }
        stem.push(c);
    }
    let stem = stem.trim_matches(|c: char| c.is_whitespace() || c == '.');
    let stem = if stem.is_empty() { "合成" } else { stem };
    let base = stem
        .split('.')
        .next()
        .unwrap_or(stem)
        .trim()
        .to_ascii_uppercase();
    let reserved = matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        base.strip_prefix(*prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if reserved {
        return format!("clip-{stem}-{id}.mp4");
    }
    format!("{stem}-{id}.mp4")
}
impl RenderJobs {
    pub fn new(pool: ConnectionPool, root: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            root: root.into(),
            tokens: Mutex::new(HashMap::new()),
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub async fn initialize(&self) -> sqlx::Result<()> {
        store::recover(&self.pool, recorder::now_ms()).await?;
        self.cleanup_unreferenced_snapshots().await?;
        Ok(())
    }
    async fn cleanup_unreferenced_snapshots(&self) -> sqlx::Result<()> {
        let root = self.root.join("snapshots");
        let mut entries = match tokio::fs::read_dir(&root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let Some(id) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<i64>().ok())
            else {
                continue;
            };
            let Some(record) = store::get(&self.pool, id).await? else {
                tokio::fs::remove_dir_all(entry.path()).await?;
                continue;
            };
            let still_retryable = record.view.state != "ready";
            let referenced = still_retryable
                && (record
                    .spec
                    .assets
                    .iter()
                    .any(|asset| asset.path.starts_with(entry.path()))
                    || record
                        .spec
                        .font_path
                        .as_ref()
                        .is_some_and(|font| font.starts_with(entry.path())));
            if !referenced {
                tokio::fs::remove_dir_all(entry.path()).await?;
            }
        }
        Ok(())
    }
    pub async fn validate_assets(
        &self,
        session_id: i64,
        recipe: &RenderRecipe,
    ) -> Result<Vec<RenderAsset>, String> {
        recipe.validate()?;
        let mut result = vec![];
        for id in store::asset_ids(recipe) {
            let asset = assets::get(&self.pool, id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("图片 {id} 不存在"))?;
            if !store::asset_authorized(&self.pool, session_id, id)
                .await
                .map_err(|e| e.to_string())?
            {
                return Err("不能使用其他场次的遮挡图片".into());
            }
            if !tokio::fs::try_exists(&asset.render.path)
                .await
                .unwrap_or(false)
            {
                return Err(format!("图片 {id} 文件已丢失，请重新上传"));
            }
            result.push(asset.render);
        }
        Ok(result)
    }
    pub async fn make_spec(
        &self,
        session_id: i64,
        clip_id: Option<i64>,
        range: Option<(i64, i64)>,
        recipe: RenderRecipe,
    ) -> Result<RenderSpec, String> {
        let session = recordings::session(&self.pool, session_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("场次不存在")?;
        let segments = recordings::session_segments(&self.pool, session_id)
            .await
            .map_err(|e| e.to_string())?;
        let members = session_groups::timeline(&self.pool, session_id)
            .await
            .map_err(|e| e.to_string())?;
        let mut output_name = Some(if session.title.trim().is_empty() {
            format!("场次-{session_id}")
        } else {
            session.title.clone()
        });
        let range = if let Some(cid) = clip_id {
            if range.is_some() {
                return Err("指定切片时不能同时指定范围".into());
            }
            let c = clips::get(&self.pool, cid)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("切片不存在")?;
            if c.session_id != session_id {
                return Err("切片不属于这个场次".into());
            }
            output_name = Some(c.title);
            (c.in_ms, c.out_ms)
        } else if let Some(range) = range {
            range
        } else {
            if session.ended_at.is_none() {
                return Err("整场合成只支持已结束场次".into());
            }
            let start = segments
                .iter()
                .map(|s| s.start_ms)
                .min()
                .ok_or("场次没有录像分段")?;
            let end = segments
                .iter()
                .filter_map(|s| s.end_ms)
                .max()
                .ok_or("场次没有已结束录像分段")?;
            (start, end)
        };
        if range.0 < 0 || range.1 <= range.0 {
            return Err("导出范围无效".into());
        }
        let mut sources = vec![];
        for s in segments {
            if s.start_ms >= range.1 || s.end_ms.is_some_and(|end| end <= range.0) {
                continue;
            }
            if !matches!(
                s.state,
                recordings::SegmentState::Finished | recordings::SegmentState::PendingDelete
            ) {
                return Err(format!("分段 {} 尚未结束、损坏或已被删除", s.id));
            }
            let end = s
                .end_ms
                .ok_or_else(|| format!("分段 {} 没有结束时间", s.id))?;
            if end <= s.start_ms {
                return Err(format!("分段 {} 时长无效", s.id));
            }
            let path = PathBuf::from(&s.path);
            let meta = tokio::fs::metadata(&path)
                .await
                .map_err(|_| format!("分段 {} 的录像文件已丢失", s.id))?;
            if !meta.is_file() || meta.len() == 0 {
                return Err(format!("分段 {} 的录像文件为空", s.id));
            }
            let xml = s.danmaku_path.map(PathBuf::from);
            if recipe.danmaku.enabled
                && !match &xml {
                    Some(p) => tokio::fs::try_exists(p).await.unwrap_or(false),
                    None => false,
                }
            {
                return Err(format!(
                    "分段 {} 缺少弹幕 XML；关闭弹幕或补齐文件后重试",
                    s.id
                ));
            }
            let source_origin_ms: Option<i64> =
                sqlx::query_scalar("SELECT recorded_at_ms FROM segments WHERE id=?")
                    .bind(s.id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            let member = members
                .iter()
                .find(|member| member.source_session_id == s.session_id)
                .ok_or_else(|| format!("分段 {} 不属于当前场次", s.id))?;
            let source_started_at: Option<i64> =
                sqlx::query_scalar("SELECT started_at FROM stream_sessions WHERE id=?")
                    .bind(s.session_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            sources.push(RenderSource {
                segment_id: s.id,
                source_session_id: s.session_id,
                path,
                danmaku_path: xml,
                start_ms: s.start_ms,
                end_ms: end,
                source_origin_ms: source_origin_ms
                    .or_else(|| source_started_at.map(|t| t + s.start_ms - member.offset_ms)),
                trim_start_ms: (range.0 - s.start_ms).max(0),
                trim_end_ms: (range.1.min(end) - s.start_ms),
            });
        }
        if sources.is_empty() {
            return Err("所选范围没有可用录像分段".into());
        }
        let assets = self.validate_assets(session_id, &recipe).await?;
        let font_path = recipe
            .danmaku
            .enabled
            .then(crate::tools::render_font)
            .flatten();
        Ok(RenderSpec {
            session_id,
            clip_id,
            output_name,
            in_ms: range.0,
            out_ms: range.1,
            recipe,
            sources,
            assets,
            font_path,
        })
    }
    pub async fn enqueue(self: &Arc<Self>, spec: RenderSpec) -> Result<JobView, String> {
        for asset in &spec.assets {
            assets::verify(asset).await?;
        }
        let record = store::insert(&self.pool, &spec, recorder::now_ms())
            .await
            .map_err(|e| e.to_string())?;
        lock(&self.tokens).insert(record.view.id, CancellationToken::new());
        if let Some(cid) = spec.clip_id {
            match clips::begin_render_export(&self.pool, cid, record.view.id, recorder::now_ms())
                .await
            {
                Ok(Some(clip)) if (clip.in_ms, clip.out_ms) == (spec.in_ms, spec.out_ms) => {}
                Ok(_) => {
                    self.persist_terminal(
                        record.view.id,
                        "failed",
                        "切片正在导出、已发布或范围已变化",
                    )
                    .await;
                    lock(&self.tokens).remove(&record.view.id);
                    return Err("切片正在导出、已发布或范围已变化".into());
                }
                Err(error) => {
                    self.persist_terminal(record.view.id, "failed", &error.to_string())
                        .await;
                    lock(&self.tokens).remove(&record.view.id);
                    return Err(error.to_string());
                }
            }
        }
        self.start(record.view.id, spec);
        Ok(record.view)
    }
    /// Keep the job token alive until a terminal transition is committed. A transient
    /// database failure therefore cannot turn a running row into an unowned task.
    async fn persist_terminal(&self, id: i64, state: &str, error: &str) -> Option<String> {
        let mut delay = std::time::Duration::from_millis(100);
        loop {
            match store::terminal(&self.pool, id, state, Some(error), recorder::now_ms()).await {
                Ok(true) => return Some(state.to_owned()),
                Ok(false) => match store::get(&self.pool, id).await {
                    Ok(Some(record))
                        if !matches!(record.view.state.as_str(), "queued" | "running") =>
                    {
                        return Some(record.view.state);
                    }
                    Ok(None) => return None,
                    Ok(Some(_)) => {
                        tracing::error!(id, state, "任务仍处于活动状态但终态更新未生效，将重试");
                    }
                    Err(read_error) => {
                        tracing::error!(%read_error, id, state, "读取合成任务终态失败，将重试");
                    }
                },
                Err(write_error) => {
                    tracing::error!(%write_error, id, state, "保存合成任务终态失败，将重试");
                }
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(std::time::Duration::from_secs(5));
        }
    }
    /// Persist the completed output before releasing the in-memory owner. Retry a
    /// failed transaction while retaining its source pin and the generated product.
    pub(super) async fn finish_output(
        &self,
        id: i64,
        output_path: &Path,
        output_bytes: i64,
        duration_ms: i64,
        cancel: &CancellationToken,
    ) -> bool {
        let path = output_path.to_string_lossy().into_owned();
        let mut delay = std::time::Duration::from_millis(100);
        loop {
            if cancel.is_cancelled() {
                let state = self.persist_terminal(id, "cancelled", "合成已取消").await;
                if state.as_deref().is_some_and(|state| state == "ready") {
                    return true;
                }
                return false;
            }
            match store::finish(
                &self.pool,
                id,
                &path,
                output_bytes,
                duration_ms,
                recorder::now_ms(),
            )
            .await
            {
                Ok(true) => return true,
                Ok(false) => match store::get(&self.pool, id).await {
                    Ok(Some(record)) if record.view.state == "ready" => return true,
                    Ok(Some(record))
                        if !matches!(record.view.state.as_str(), "queued" | "running") =>
                    {
                        return false;
                    }
                    Ok(None) => return false,
                    Ok(Some(_)) => {
                        tracing::error!(id, "合成完成事务未更新任务且任务仍活动，将重试");
                    }
                    Err(read_error) => {
                        tracing::error!(%read_error, id, "读取合成任务状态失败，将重试");
                    }
                },
                Err(finish_error) => {
                    tracing::error!(%finish_error, id, output = %path, "提交合成结果失败，将重试");
                    if let Err(record_error) = store::finalization_error(
                        &self.pool,
                        id,
                        &finish_error.to_string(),
                        recorder::now_ms(),
                    )
                    .await
                    {
                        tracing::error!(%record_error, id, "保存合成结果提交错误详情失败");
                    }
                }
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(std::time::Duration::from_secs(5));
        }
    }
    /// Enqueue and guard the clip export without clearing its previous output.
    pub async fn enqueue_clip(
        self: &Arc<Self>,
        clip: &clips::Clip,
        recipe: RenderRecipe,
    ) -> Result<JobView, String> {
        let spec = self
            .make_spec(clip.session_id, Some(clip.id), None, recipe)
            .await?;
        self.enqueue(spec).await
    }
    fn start(self: &Arc<Self>, id: i64, spec: RenderSpec) {
        let token = lock(&self.tokens)
            .get(&id)
            .cloned()
            .unwrap_or_else(CancellationToken::new);
        lock(&self.tokens).insert(id, token.clone());
        let this = self.clone();
        tokio::spawn(async move {
            this.run(id, spec, token).await;
        });
    }
    async fn run(self: Arc<Self>, id: i64, spec: RenderSpec, cancel: CancellationToken) {
        let result = self.execute(id, &spec, &cancel).await;
        match result {
            Ok(done) => {
                let keep_output = self
                    .finish_output(
                        id,
                        &done.output_path,
                        done.output_bytes,
                        done.duration_ms,
                        &cancel,
                    )
                    .await;
                if !keep_output
                    && let Err(error) = tokio::fs::remove_file(&done.output_path).await
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!(%error, id, path = %done.output_path.display(), "清理未提交的合成输出失败");
                }
            }
            Err(error) => {
                let state = if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                };
                self.persist_terminal(id, state, &error).await;
            }
        }
        lock(&self.tokens).remove(&id);
    }
    async fn execute(
        &self,
        id: i64,
        spec: &RenderSpec,
        cancel: &CancellationToken,
    ) -> Result<engine::EngineOutput, String> {
        let permit = crate::server::common::processing::transcode::acquire(cancel).await?;
        if !store::running(&self.pool, id, recorder::now_ms())
            .await
            .map_err(|e| e.to_string())?
        {
            return Err("合成已取消".into());
        }
        for asset in &spec.assets {
            assets::verify(asset).await?;
        }
        let output_dir = self.root.join(spec.session_id.to_string());
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| e.to_string())?;
        let request = engine::EngineRequest {
            sources: spec
                .sources
                .iter()
                .map(|s| engine::SourceSpan {
                    segment_id: s.segment_id,
                    source_session_id: s.source_session_id,
                    path: s.path.clone(),
                    danmaku_path: s.danmaku_path.clone(),
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    source_origin_ms: s.source_origin_ms,
                    trim_start_ms: s.trim_start_ms,
                    trim_end_ms: s.trim_end_ms,
                })
                .collect(),
            settings: spec.recipe.clone(),
            assets: spec
                .assets
                .iter()
                .map(|a| (a.id.to_string(), a.path.clone()))
                .collect(),
            output_path: output_dir.join(output_file_name(spec.output_name.as_deref(), id)),
            font_path: spec.font_path.clone(),
        };
        // Coalesce fast progress updates and write them in order. Await the
        // final update before publishing the terminal state so a late writer
        // cannot lose the encoder information or regress the progress.
        let pool = self.pool.clone();
        let (progress_tx, mut progress_rx) =
            tokio::sync::watch::channel::<Option<engine::EngineProgress>>(None);
        let writer = tokio::spawn(async move {
            while progress_rx.changed().await.is_ok() {
                let progress = progress_rx.borrow_and_update().clone();
                if let Some(progress) = progress {
                    if let Err(error) =
                        store::progress_snapshot(&pool, id, &progress, recorder::now_ms()).await
                    {
                        tracing::warn!(%error, id, "保存合成进度失败");
                    }
                }
            }
        });
        let mut samples = ProgressSamples::default();
        let done = engine::render_with_permit(
            request,
            cancel.clone(),
            move |p| {
                // Short GPU renders can finish while the writer is saving an
                // earlier update. Retain real encoding samples when the watch
                // channel coalesces them with a preparation/join/final snapshot.
                progress_tx.send_replace(Some(samples.retain(p)));
            },
            permit,
        )
        .await;
        let _ = writer.await;
        done
    }
    pub async fn cancel(&self, id: i64) -> Result<bool, String> {
        if let Some(token) = lock(&self.tokens).get(&id) {
            token.cancel();
        }
        let record = store::get(&self.pool, id)
            .await
            .map_err(|e| e.to_string())?;
        // A running worker keeps its source pin until FFmpeg exits. Queued work has no readers.
        if record.as_ref().is_some_and(|j| j.view.state == "running") {
            return Ok(true);
        }
        let changed = store::terminal(
            &self.pool,
            id,
            "cancelled",
            Some("合成已取消"),
            recorder::now_ms(),
        )
        .await
        .map_err(|e| e.to_string())?;
        // The worker can observe the cancelled token and commit its terminal
        // state before this request reaches the UPDATE. That is a successful
        // cancellation, not a conflicting transition.
        if changed {
            return Ok(true);
        }
        Ok(store::get(&self.pool, id)
            .await
            .map_err(|e| e.to_string())?
            .is_some_and(|job| job.view.state == "cancelled"))
    }
    pub async fn retry(self: &Arc<Self>, id: i64) -> Result<JobView, String> {
        let job = store::get(&self.pool, id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("合成任务不存在")?;
        if !matches!(job.view.state.as_str(), "failed" | "cancelled") {
            return Err("只有失败或取消的任务可以重试".into());
        }
        // Reuse the saved source/config snapshot. Validate source rows again transactionally at insert.
        for s in &job.spec.sources {
            if !tokio::fs::try_exists(&s.path).await.unwrap_or(false) {
                return Err(format!("分段 {} 的源文件已丢失", s.segment_id));
            }
        }
        self.enqueue(job.spec).await
    }
    pub async fn shutdown(&self) {
        self.stop();
        for _ in 0..600 {
            if lock(&self.tokens).is_empty() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    pub fn stop(&self) {
        for token in lock(&self.tokens).values() {
            token.cancel();
        }
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn output_names_are_safe_bounded_and_unique() {
        assert_eq!(
            output_file_name(Some(" .剪辑/\\:*?\"<>|\n. "), 42),
            "剪辑__________-42.mp4"
        );
        assert_eq!(output_file_name(Some("..."), 42), "合成-42.mp4");
        assert_eq!(output_file_name(None, 43), "合成-43.mp4");
        for name in [
            "CON.txt",
            "aux.foo",
            "NUL.a",
            "LPT1.x",
            "COM9.txt",
            "COM¹.txt",
            "LPT².txt",
            "CONIN$.txt",
        ] {
            assert_eq!(
                output_file_name(Some(name), 42),
                format!("clip-{name}-42.mp4")
            );
        }
        assert_eq!(output_file_name(Some("COM10.txt"), 42), "COM10.txt-42.mp4");
        let long = output_file_name(Some(&"中".repeat(100)), 42);
        assert!(long.len() <= 127);
        assert!(!long.contains('/'));
    }
    #[tokio::test]
    async fn coalesced_final_snapshot_keeps_last_encoding_speed_for_slow_writer() {
        let mut samples = ProgressSamples::default();
        let (tx, mut rx) = tokio::sync::watch::channel(None);
        let mut progress = engine::EngineProgress {
            phase: "合成分段 1/1".into(),
            ratio: Some(0.95),
            encoder: Some("h264_nvenc".into()),
            encoder_device: Some("0".into()),
            fallback_reason: None,
            fps: Some(120.),
            speed: Some(4.),
            processed_ms: Some(10000),
            total_ms: Some(10000),
        };
        // The writer intentionally has not read while multiple updates arrive.
        tx.send_replace(Some(samples.retain(progress.clone())));
        progress.phase = "拼接并保存".into();
        progress.fps = None;
        progress.speed = None;
        tx.send_replace(Some(samples.retain(progress.clone())));
        progress.phase = "已完成".into();
        progress.ratio = Some(1.);
        tx.send_replace(Some(samples.retain(progress)));
        drop(tx);
        rx.changed().await.unwrap();
        let snapshot = rx.borrow_and_update().clone().unwrap();
        assert_eq!(snapshot.phase, "已完成");
        assert_eq!(snapshot.fps, Some(120.));
        assert_eq!(snapshot.speed, Some(4.));
        assert_eq!(snapshot.encoder.as_deref(), Some("h264_nvenc"));
        assert!(rx.changed().await.is_err());
    }
}
