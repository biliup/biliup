//! Persistent render queue with immutable recipes and cooperative cancellation.
use super::{assets, engine, model::*, store};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::{clips, recorder, session_groups, store as recordings};
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

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryMarker {
    token: String,
    root: PathBuf,
}

fn recovery_marker(file: &mut std::fs::File) -> std::io::Result<Option<RecoveryMarker>> {
    use std::io::{Read, Seek};
    file.rewind()?;
    let mut bytes = Vec::new();
    (&mut *file)
        .take(crate::danmaku_worker::ROOT_LEASE_MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > crate::danmaku_worker::ROOT_LEASE_MAX_BYTES {
        return Ok(None);
    }
    let marker: RecoveryMarker = match serde_json::from_slice(&bytes) {
        Ok(marker) => marker,
        Err(_) => return Ok(None),
    };
    if marker.token.is_empty()
        || marker.token == "."
        || marker.token == ".."
        || marker.token.len() > 128
        || !marker
            .token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_'))
    {
        return Ok(None);
    }
    Ok(Some(marker))
}

fn recovery_file(path: &Path) -> std::io::Result<Option<std::fs::File>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
        _ => {}
    }
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(Some)
}

fn recovery_lock(file: &std::fs::File) -> std::io::Result<bool> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

fn recovery_manifest_path(parent: &Path, marker: &RecoveryMarker) -> PathBuf {
    parent.join(format!(".render-recovery-{}.json", marker.token))
}

fn ensure_recovery_manifest(
    parent: &Path,
    private_root: &Path,
    marker: &RecoveryMarker,
) -> std::io::Result<std::fs::File> {
    use std::io::Write;
    let path = recovery_manifest_path(parent, marker);
    if !path.try_exists()? {
        // Publish the external ownership record completely before renaming or
        // deleting any root contents. Partial recursive deletion cannot erase it.
        let mut temp = tempfile::Builder::new()
            .prefix(".render-manifest-")
            .tempfile_in(private_root)?;
        serde_json::to_writer(temp.as_file_mut(), marker).map_err(std::io::Error::other)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        if let Err(error) = temp.persist_noclobber(&path)
            && error.error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(error.error);
        }
    }
    let mut file = recovery_file(&path)?
        .ok_or_else(|| std::io::Error::other("invalid recovery manifest file"))?;
    if recovery_marker(&mut file)?.as_ref() != Some(marker) {
        return Err(std::io::Error::other("recovery manifest identity changed"));
    }
    Ok(file)
}

fn remove_recovery_tombstone(
    tombstone: &Path,
    manifest: &Path,
    manifest_file: std::fs::File,
    remove: &mut impl FnMut(&Path) -> std::io::Result<()>,
) -> std::io::Result<bool> {
    if let Err(error) = remove(tombstone)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(std::io::Error::new(
            error.kind(),
            format!(
                "cleanup residue {}; manifest {}: {error}",
                tombstone.display(),
                manifest.display()
            ),
        ));
    }
    drop(manifest_file); // Windows: close the sibling handle before deleting it.
    match std::fs::remove_file(manifest) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(std::io::Error::new(
            error.kind(),
            format!("cleanup recovery manifest {}: {error}", manifest.display()),
        )),
    }
}

/// Only authenticated managed roots are disposable. The external, locked
/// manifest survives partial deletion even when the internal lease is gone.
pub(crate) fn cleanup_inactive_danmaku_root(path: &Path) -> std::io::Result<bool> {
    cleanup_danmaku_root_with(path, &mut |path| std::fs::remove_dir_all(path))
}

fn cleanup_danmaku_root_with(
    path: &Path,
    remove: &mut impl FnMut(&Path) -> std::io::Result<()>,
) -> std::io::Result<bool> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let canonical = std::fs::canonicalize(path)?;
    let parent = canonical
        .parent()
        .ok_or_else(|| std::io::Error::other("managed root has no parent"))?;
    if metadata.is_file() {
        let Some(mut manifest_file) = recovery_file(&canonical)? else {
            return Ok(false);
        };
        if !recovery_lock(&manifest_file)? {
            return Ok(false);
        }
        let Some(marker) = recovery_marker(&mut manifest_file)? else {
            return Ok(false);
        };
        if !marker.root.is_absolute()
            || marker.root.parent() != Some(parent)
            || recovery_manifest_path(parent, &marker) != canonical
        {
            return Ok(false);
        }
        let tombstone = parent.join(format!(".render-recovery-{}", marker.token));
        match std::fs::symlink_metadata(&tombstone) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if marker.root.try_exists()? {
                    // Crash between manifest publication and rename. Retry the
                    // original root only after acquiring its own exclusive lease.
                    drop(manifest_file);
                    return cleanup_danmaku_root_with(&marker.root, remove);
                }
            }
            Err(error) => return Err(error),
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Ok(false);
            }
            Ok(_) => {
                let lease_path = tombstone.join(crate::danmaku_worker::ROOT_LEASE_FILE);
                if std::fs::symlink_metadata(&lease_path).is_ok_and(|m| !m.file_type().is_file()) {
                    return Ok(false);
                }
                if let Some(mut lease) = recovery_file(&lease_path)? {
                    if !recovery_lock(&lease)?
                        || recovery_marker(&mut lease)?.as_ref() != Some(&marker)
                    {
                        return Ok(false);
                    }
                    drop(lease);
                }
            }
        }
        return remove_recovery_tombstone(&tombstone, &canonical, manifest_file, remove);
    }
    if !metadata.is_dir() {
        return Ok(false);
    }
    let lease_path = canonical.join(crate::danmaku_worker::ROOT_LEASE_FILE);
    if std::fs::symlink_metadata(&lease_path).is_ok_and(|m| !m.file_type().is_file()) {
        return Ok(false);
    }
    let Some(mut lease) = recovery_file(&lease_path)? else {
        // The internal marker may already have been removed by a failed cleanup.
        // Only an independently published sibling manifest can prove ownership.
        if canonical
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".render-recovery-"))
        {
            let mut name = canonical.as_os_str().to_owned();
            name.push(".json");
            let manifest = PathBuf::from(name);
            if !manifest.try_exists()? {
                return Ok(false);
            }
            return cleanup_danmaku_root_with(&manifest, remove);
        }
        return Ok(false);
    };
    if !recovery_lock(&lease)? {
        return Ok(false);
    }
    let Some(marker) = recovery_marker(&mut lease)? else {
        return Ok(false);
    };
    let tombstone = parent.join(format!(".render-recovery-{}", marker.token));
    if !marker.root.is_absolute()
        || marker.root.parent() != Some(parent)
        || (marker.root != canonical && tombstone != canonical)
    {
        return Ok(false);
    }
    let manifest_path = recovery_manifest_path(parent, &marker);
    let manifest_file = ensure_recovery_manifest(parent, &canonical, &marker)?;
    if !recovery_lock(&manifest_file)? {
        return Ok(false);
    }
    // Retire the root while its lease is exclusive. Late parents/workers must
    // reject it after this point, including a crash before the rename. Close
    // every handle inside the directory before renaming it on Windows; the
    // external manifest stays locked throughout rename and recursive removal.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(canonical.join(crate::danmaku_worker::ROOT_RETIRED_FILE))
    {
        Ok(file) => file.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    drop(lease);
    if canonical != tombstone {
        // The retired marker prevents new workers; the unpredictable original
        // token and the locked create-new manifest bind the destination.
        if std::fs::symlink_metadata(&tombstone).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "recovery tombstone already exists",
            ));
        }
        std::fs::rename(&canonical, &tombstone)?;
    }
    remove_recovery_tombstone(&tombstone, &manifest_path, manifest_file, remove)
}

async fn recover_danmaku_root(path: PathBuf) {
    let path_text = path.display().to_string();
    match tokio::task::spawn_blocking(move || cleanup_inactive_danmaku_root(&path)).await {
        Ok(Ok(true)) => {}
        Ok(Ok(false)) => tracing::warn!(path=%path_text,"保留仍活动或无法确认归属的弹幕工作目录"),
        Ok(Err(error)) => {
            tracing::warn!(path=%path_text,%error,"弹幕工作目录恢复清理失败，下次启动重试")
        }
        Err(error) => tracing::warn!(path=%path_text,%error,"弹幕工作目录恢复任务失败"),
    }
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
        // Preview/self-check workers use this application-owned cache root,
        // so they participate in the same lease-guarded crash recovery.
        if let Ok(mut entries) =
            tokio::fs::read_dir(crate::danmaku_worker::managed_preview_root_dir()).await
        {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(".render-"))
                {
                    recover_danmaku_root(entry.path()).await;
                }
            }
        }
        // Do not recursively remove an ancestor of a surviving worker. Each
        // managed root is recovered separately under its own exclusive lease.
        if let Ok(mut entries) = tokio::fs::read_dir(self.root.join("work")).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                recover_danmaku_root(entry.path()).await;
            }
        }
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
                            recover_danmaku_root(file.path()).await;
                        }
                    }
                }
            }
        }

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
    pub async fn enqueue(self: &Arc<Self>, mut spec: RenderSpec) -> Result<JobView, String> {
        let record = store::insert(&self.pool, &spec, recorder::now_ms())
            .await
            .map_err(|e| e.to_string())?;
        lock(&self.tokens).insert(record.view.id, CancellationToken::new());
        if let Err(error) = self.snapshot_support(record.view.id, &mut spec).await {
            self.persist_terminal(record.view.id, "failed", &error)
                .await;
            self.remove_snapshot_dir(record.view.id).await;
            lock(&self.tokens).remove(&record.view.id);
            return Err(error);
        }
        match self.register_snapshot(record.view.id, &spec).await {
            Ok(true) => {}
            Ok(false) => {
                lock(&self.tokens).remove(&record.view.id);
                return Err("合成任务已取消".into());
            }
            Err(error) => {
                self.persist_terminal(
                    record.view.id,
                    "failed",
                    &format!("保存合成任务快照失败：{error}"),
                )
                .await;
                // An UPDATE error can be returned after an uncertain commit. Keep the
                // directory if the persisted recipe references it; startup cleanup
                // will collect it later if the write did not commit.
                self.remove_snapshot_dir_if_unreferenced(record.view.id)
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
    async fn remove_snapshot_dir(&self, id: i64) {
        let path = self.root.join("snapshots").join(id.to_string());
        if let Err(error) = tokio::fs::remove_dir_all(&path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, id, path = %path.display(), "清理合成素材快照失败");
        }
    }
    /// Register frozen paths only while the task remains queued. An explicit
    /// zero-row result proves that this enqueue no longer owns the directory.
    pub(super) async fn register_snapshot(&self, id: i64, spec: &RenderSpec) -> sqlx::Result<bool> {
        match store::update_snapshot(&self.pool, id, spec).await {
            Ok(true) => Ok(true),
            Ok(false) => {
                self.remove_snapshot_dir(id).await;
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }
    async fn remove_snapshot_dir_if_unreferenced(&self, id: i64) {
        let path = self.root.join("snapshots").join(id.to_string());
        match store::get(&self.pool, id).await {
            Ok(Some(record)) => {
                let referenced = record
                    .spec
                    .assets
                    .iter()
                    .any(|asset| asset.path.starts_with(&path))
                    || record
                        .spec
                        .font_path
                        .as_ref()
                        .is_some_and(|font| font.starts_with(&path));
                if referenced {
                    return;
                }
                self.remove_snapshot_dir(id).await;
            }
            Ok(None) => self.remove_snapshot_dir(id).await,
            Err(error) => {
                tracing::error!(%error, id, path = %path.display(), "无法确认合成素材快照是否仍被引用，保留目录");
            }
        }
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
        let work = self.root.join("work").join(id.to_string());
        if let Err(error) = tokio::fs::remove_dir_all(&work).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, id, path = %work.display(), "清理合成工作目录失败");
        }
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
        let done = engine::render(request, cancel.clone(), move |p| {
            // Short GPU renders can finish while the writer is saving an
            // earlier update. Retain real encoding samples when the watch
            // channel coalesces them with a preparation/join/final snapshot.
            progress_tx.send_replace(Some(samples.retain(p)));
        })
        .await;
        let _ = writer.await;
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
    fn recovery_preserves_active_and_unmanaged_roots_and_retries_tombstones() {
        let directory = tempfile::tempdir().unwrap();
        let unmanaged = directory.path().join(".render-legacy");
        std::fs::create_dir(&unmanaged).unwrap();
        std::fs::write(unmanaged.join("keep"), b"user or unknown owner").unwrap();
        assert!(!cleanup_inactive_danmaku_root(&unmanaged).unwrap());
        assert!(unmanaged.join("keep").exists());
        let managed = directory.path().join(".render-owned");
        std::fs::create_dir(&managed).unwrap();
        let lease = crate::danmaku_worker::acquire_root_lease(&managed).unwrap();
        std::fs::write(managed.join("out.ass"), b"complete but unconfirmed").unwrap();
        assert!(!cleanup_inactive_danmaku_root(&managed).unwrap());
        assert!(managed.join("out.ass").exists());
        drop(lease);
        assert!(cleanup_inactive_danmaku_root(&managed).unwrap());
        assert!(!managed.exists());

        let managed = directory.path().join(".render-interrupted-cleanup");
        std::fs::create_dir(&managed).unwrap();
        let lease = crate::danmaku_worker::acquire_root_lease(&managed).unwrap();
        drop(lease);
        let marker: serde_json::Value = serde_json::from_slice(
            &std::fs::read(managed.join(crate::danmaku_worker::ROOT_LEASE_FILE)).unwrap(),
        )
        .unwrap();
        let pending = directory.path().join(format!(
            ".render-recovery-{}",
            marker["token"].as_str().unwrap()
        ));
        std::fs::rename(&managed, &pending).unwrap();
        assert!(cleanup_inactive_danmaku_root(&pending).unwrap());
        assert!(!pending.exists());
    }

    #[test]
    fn recovery_rejects_replaced_malformed_and_other_root_markers() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join(".render-first");
        let second = directory.path().join(".render-second");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        drop(crate::danmaku_worker::acquire_root_lease(&first).unwrap());
        std::fs::copy(
            first.join(crate::danmaku_worker::ROOT_LEASE_FILE),
            second.join(crate::danmaku_worker::ROOT_LEASE_FILE),
        )
        .unwrap();
        assert!(!cleanup_inactive_danmaku_root(&second).unwrap());
        std::fs::write(
            second.join(crate::danmaku_worker::ROOT_LEASE_FILE),
            b"broken",
        )
        .unwrap();
        assert!(!cleanup_inactive_danmaku_root(&second).unwrap());
        assert!(first.exists() && second.exists());
    }

    #[test]
    fn recovery_retries_after_partial_deletion_removes_the_internal_lease() {
        let directory = tempfile::tempdir().unwrap();
        let managed = directory.path().join(".render-partially-deleted");
        std::fs::create_dir(&managed).unwrap();
        drop(crate::danmaku_worker::acquire_root_lease(&managed).unwrap());
        std::fs::write(managed.join("unconfirmed.ass"), b"complete but unconfirmed").unwrap();
        let mut tombstone = PathBuf::new();
        let error = cleanup_danmaku_root_with(&managed, &mut |path| {
            tombstone = path.to_owned();
            std::fs::remove_file(path.join(crate::danmaku_worker::ROOT_LEASE_FILE))?;
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected failure after marker deletion",
            ))
        })
        .unwrap_err();
        assert!(error.to_string().contains("cleanup residue"));
        assert!(tombstone.join("unconfirmed.ass").exists());
        assert!(
            !tombstone
                .join(crate::danmaku_worker::ROOT_LEASE_FILE)
                .exists()
        );
        let mut manifest_path = tombstone.as_os_str().to_owned();
        manifest_path.push(".json");
        let manifest_path = PathBuf::from(manifest_path);
        assert!(manifest_path.is_file());
        assert!(cleanup_inactive_danmaku_root(&tombstone).unwrap());
        assert!(!tombstone.exists());
        assert!(!manifest_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn recovery_preserves_its_manifest_after_a_real_permission_failure() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let managed = directory.path().join(".render-permission-failure");
        std::fs::create_dir(&managed).unwrap();
        drop(crate::danmaku_worker::acquire_root_lease(&managed).unwrap());
        let blocked = managed.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("retained"), b"must retry").unwrap();
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let permission_enforced = std::fs::read_dir(&blocked).is_err();
        let mut tombstone = PathBuf::new();
        let result = cleanup_danmaku_root_with(&managed, &mut |path| {
            tombstone = path.to_owned();
            // Force the exact partial-deletion state independently of readdir order.
            std::fs::remove_file(path.join(crate::danmaku_worker::ROOT_LEASE_FILE))?;
            std::fs::remove_dir_all(path)
        });
        let restored = tombstone.join("blocked");
        if restored.exists() {
            std::fs::set_permissions(&restored, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        // Root can bypass Unix mode bits; this environment cannot exercise
        // that failure, while the injected cross-platform test still does.
        if !permission_enforced {
            assert!(result.is_ok(), "privileged cleanup unexpectedly failed");
            return;
        }
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let mut manifest = tombstone.as_os_str().to_owned();
        manifest.push(".json");
        let manifest = PathBuf::from(manifest);
        assert!(manifest.exists());
        assert!(cleanup_inactive_danmaku_root(&manifest).unwrap());
        assert!(!tombstone.exists() && !manifest.exists());
    }

    #[test]
    fn recovery_finishes_a_manifest_written_before_rename_and_after_root_removal() {
        let directory = tempfile::tempdir().unwrap();
        let managed = directory.path().join(".render-manifest-stages");
        std::fs::create_dir(&managed).unwrap();
        let active = crate::danmaku_worker::acquire_root_lease(&managed).unwrap();
        let mut lease = recovery_file(&managed.join(crate::danmaku_worker::ROOT_LEASE_FILE))
            .unwrap()
            .unwrap();
        let marker = recovery_marker(&mut lease).unwrap().unwrap();
        drop(lease);
        drop(ensure_recovery_manifest(directory.path(), &managed, &marker).unwrap());
        let manifest = recovery_manifest_path(directory.path(), &marker);
        assert!(!cleanup_inactive_danmaku_root(&manifest).unwrap());
        assert!(managed.exists() && manifest.exists());
        drop(active);
        // A crash after retirement but before rename must still be recoverable.
        std::fs::File::create_new(managed.join(crate::danmaku_worker::ROOT_RETIRED_FILE))
            .unwrap()
            .sync_all()
            .unwrap();
        assert!(crate::danmaku_worker::acquire_root_lease(&managed).is_err());
        assert!(cleanup_inactive_danmaku_root(&manifest).unwrap());
        assert!(!managed.exists() && !manifest.exists());
        let private = tempfile::tempdir_in(directory.path()).unwrap();
        drop(ensure_recovery_manifest(directory.path(), private.path(), &marker).unwrap());
        assert!(cleanup_inactive_danmaku_root(&manifest).unwrap());
        assert!(!manifest.exists());
        let unknown = directory.path().join(".render-recovery-unknown");
        std::fs::create_dir(&unknown).unwrap();
        assert!(!cleanup_inactive_danmaku_root(&unknown).unwrap());
        assert!(unknown.exists());
    }
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
