//! Persistent render queue with immutable recipes and cooperative cancellation.
use super::{assets, engine, model::*, store};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::{clips, recorder, store as recordings};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use tokio_util::sync::CancellationToken;

pub struct RenderJobs {
    pool: ConnectionPool,
    root: PathBuf,
    tokens: Mutex<HashMap<i64, CancellationToken>>,
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
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
        // Only job-private temp folders are disposable; completed products live outside them.
        let _ = tokio::fs::remove_dir_all(self.root.join("work")).await;
        if let Ok(mut sessions) = tokio::fs::read_dir(&self.root).await {
            while let Ok(Some(session)) = sessions.next_entry().await {
                if !session
                    .file_name()
                    .to_str()
                    .is_some_and(|s| s.parse::<i64>().is_ok())
                {
                    continue;
                }
                if let Ok(mut files) = tokio::fs::read_dir(session.path()).await {
                    while let Ok(Some(file)) = files.next_entry().await {
                        if file
                            .file_name()
                            .to_str()
                            .is_some_and(|name| name.starts_with(".render-"))
                        {
                            let _ = tokio::fs::remove_dir_all(file.path()).await;
                        }
                    }
                }
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
            if asset.session_id != session_id {
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
            sources.push(RenderSource {
                segment_id: s.id,
                path,
                danmaku_path: xml,
                start_ms: s.start_ms,
                end_ms: end,
                source_origin_ms: source_origin_ms
                    .or_else(|| session.started_at.map(|t| t + s.start_ms)),
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
            in_ms: range.0,
            out_ms: range.1,
            recipe,
            sources,
            assets,
            font_path,
        })
    }
    pub async fn enqueue(self: &Arc<Self>, mut spec: RenderSpec) -> Result<JobView, String> {
        let record = store::insert(&self.pool, &spec, recorder::now_ms())
            .await
            .map_err(|e| e.to_string())?;
        lock(&self.tokens).insert(record.view.id, CancellationToken::new());
        if let Err(error) = self.snapshot_support(record.view.id, &mut spec).await {
            let _ = store::terminal(
                &self.pool,
                record.view.id,
                "failed",
                Some(&error),
                recorder::now_ms(),
            )
            .await;
            let _ = tokio::fs::remove_dir_all(
                self.root.join("snapshots").join(record.view.id.to_string()),
            )
            .await;
            lock(&self.tokens).remove(&record.view.id);
            return Err(error);
        }
        match store::update_snapshot(&self.pool, record.view.id, &spec).await {
            Ok(true) => {}
            Ok(false) => {
                lock(&self.tokens).remove(&record.view.id);
                return Err("合成任务已取消".into());
            }
            Err(error) => {
                let _ = store::terminal(
                    &self.pool,
                    record.view.id,
                    "failed",
                    Some("保存合成任务快照失败"),
                    recorder::now_ms(),
                )
                .await;
                lock(&self.tokens).remove(&record.view.id);
                return Err(error.to_string());
            }
        }
        if let Some(cid) = spec.clip_id {
            match clips::begin_render_export(&self.pool, cid, record.view.id, recorder::now_ms())
                .await
            {
                Ok(Some(clip)) if (clip.in_ms, clip.out_ms) == (spec.in_ms, spec.out_ms) => {}
                Ok(_) => {
                    let _ = store::terminal(
                        &self.pool,
                        record.view.id,
                        "failed",
                        Some("切片正在导出、已发布或范围已变化"),
                        recorder::now_ms(),
                    )
                    .await;
                    let _ = clips::fail_render_export(
                        &self.pool,
                        cid,
                        record.view.id,
                        "切片范围已变化",
                        recorder::now_ms(),
                    )
                    .await;
                    lock(&self.tokens).remove(&record.view.id);
                    return Err("切片正在导出、已发布或范围已变化".into());
                }
                Err(error) => {
                    let _ = store::terminal(
                        &self.pool,
                        record.view.id,
                        "failed",
                        Some(&error.to_string()),
                        recorder::now_ms(),
                    )
                    .await;
                    lock(&self.tokens).remove(&record.view.id);
                    return Err(error.to_string());
                }
            }
        }
        self.start(record.view.id, spec);
        Ok(record.view)
    }
    async fn snapshot_support(&self, id: i64, spec: &mut RenderSpec) -> Result<(), String> {
        if spec.assets.is_empty() && spec.font_path.is_none() {
            return Ok(());
        }
        let dir = self.root.join("snapshots").join(id.to_string());
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("保存素材快照失败：{e}"))?;
        for asset in &mut spec.assets {
            let data = tokio::fs::read(&asset.path)
                .await
                .map_err(|e| format!("读取图片 {} 失败：{e}", asset.id))?;
            if data.len() > assets::MAX_ASSET_BYTES
                || format!("{:x}", Sha256::digest(&data)) != asset.sha256
            {
                return Err(format!("图片 {} 内容发生变化，请重新上传", asset.id));
            }
            let extension = asset
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let destination = dir.join(format!("asset-{}.{}", asset.id, extension));
            tokio::fs::write(&destination, &data)
                .await
                .map_err(|e| format!("保存图片快照失败：{e}"))?;
            asset.path = destination;
        }
        if let Some(font) = &spec.font_path {
            let destination = dir.join("font.otf");
            tokio::fs::copy(font, &destination)
                .await
                .map_err(|e| format!("保存字体快照失败：{e}"))?;
            spec.font_path = Some(destination);
        }
        Ok(())
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
                let path = done.output_path.to_string_lossy().into_owned();
                match store::finish(
                    &self.pool,
                    id,
                    &path,
                    done.output_bytes,
                    done.duration_ms,
                    recorder::now_ms(),
                )
                .await
                {
                    Ok(true) => {
                        if let Some(cid) = spec.clip_id {
                            let _ = clips::finish_render_export(
                                &self.pool,
                                cid,
                                id,
                                &clips::Exported {
                                    cut_in_ms: spec.in_ms,
                                    cut_out_ms: spec.out_ms,
                                    output_path: path,
                                    output_bytes: done.output_bytes,
                                    duration_ms: done.duration_ms,
                                },
                                recorder::now_ms(),
                            )
                            .await;
                        }
                    }
                    _ => {
                        let _ = tokio::fs::remove_file(&done.output_path).await;
                    }
                }
            }
            Err(error) => {
                let state = if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                };
                let _ = store::terminal(&self.pool, id, state, Some(&error), recorder::now_ms())
                    .await
                    .unwrap_or(false);
                if let Some(cid) = spec.clip_id {
                    let _ =
                        clips::fail_render_export(&self.pool, cid, id, &error, recorder::now_ms())
                            .await;
                }
            }
        }
        let _ = tokio::fs::remove_dir_all(self.root.join("work").join(id.to_string())).await;
        lock(&self.tokens).remove(&id);
    }
    async fn execute(
        &self,
        id: i64,
        spec: &RenderSpec,
        cancel: &CancellationToken,
    ) -> Result<engine::EngineOutput, String> {
        // All video encoders share this permit, including precise clips and automatic masks.
        let permit = crate::server::workbench::transcode::acquire(cancel).await?;
        if !store::running(&self.pool, id, recorder::now_ms())
            .await
            .map_err(|e| e.to_string())?
        {
            return Err("合成已取消".into());
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
            output_path: output_dir.join(format!("{id}.mp4")),
            font_path: spec.font_path.clone(),
        };
        let pool = self.pool.clone();
        let done = engine::render(request, cancel.clone(), move |p| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let _ = store::progress(&pool, id, &p.phase, p.ratio, recorder::now_ms()).await;
            });
        })
        .await;
        drop(permit);
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
        if changed && let Some(cid) = record.and_then(|j| j.spec.clip_id) {
            let _ =
                clips::fail_render_export(&self.pool, cid, id, "合成已取消", recorder::now_ms())
                    .await;
        }
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
