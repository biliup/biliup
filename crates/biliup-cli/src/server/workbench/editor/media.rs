//! Browser media URLs resolve registered project/segment IDs only. Cache work shares the
//! encoder permit and lifecycle lock with recording cleanup and the existing render queue.
use super::{Error, MediaSource, RecordingSource};
use crate::server::{
    common::processing::{engine, transcode},
    infrastructure::connection_pool::ConnectionPool,
    workbench::store,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, Weak},
    time::{Duration, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;

type PictureLock = tokio::sync::Mutex<()>;
static PICTURES: LazyLock<Mutex<BTreeMap<String, Weak<PictureLock>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn picture_lock(key: &str) -> Arc<PictureLock> {
    let mut entries = PICTURES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    entries.retain(|_, entry| entry.strong_count() > 0);
    if let Some(lock) = entries.get(key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(PictureLock::new(()));
    entries.insert(key.to_string(), Arc::downgrade(&lock));
    lock
}

pub async fn source(
    pool: &ConnectionPool,
    project: i64,
    media: &str,
) -> Result<(PathBuf, MediaSource), Error> {
    version(pool, project, media, None).await
}

pub async fn source_path(pool: &ConnectionPool, source: &MediaSource) -> Result<PathBuf, Error> {
    let MediaSource::Recording(source) = source else {
        let MediaSource::File(source) = source else {
            unreachable!()
        };
        return super::imports::path(pool, &source.asset_id).await;
    };
    let segment = store::segment(pool, source.segment_id)
        .await?
        .ok_or(Error::NotFound)?;
    if segment.session_id != source.session_id {
        return Err(Error::NotFound);
    }
    let path = PathBuf::from(segment.path);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(Error::NotFound);
    }
    Ok(path)
}
/// Resolve only a version that this project actually saved, so undo can preview before its
/// optimistic save finishes. A path is a registry key, never a server filesystem path.
pub async fn version(pool:&ConnectionPool,project:i64,media:&str,version:Option<&str>)->Result<(PathBuf,MediaSource),Error>{
    let (_,source)=registered(pool,project,media,version).await?;
    Ok((source_path(pool,&source).await?,source))
}

/// The timeline media entry and source must come from the same saved file version. Ordinary
/// media, analysis caches and graphic frames share this registry lookup, including after undo.
pub async fn registered(
    pool: &ConnectionPool,
    project: i64,
    media: &str,
    version: Option<&str>,
) -> Result<(Value, MediaSource), Error> {
    if version.is_some_and(|value| value.len() > 512) {
        return Err(super::invalid("素材版本路径无效"));
    }
    let mut document = super::get(pool, project).await?.document;
    if let Some(version) = version {
        let current = document.project["media"].as_array().into_iter().flatten()
            .any(|entry| entry["id"] == media && entry["path"] == version);
        if !current {
            let raw: Option<String> = sqlx::query_scalar("SELECT document_json FROM editor_revisions WHERE project_id=? AND EXISTS(SELECT 1 FROM json_each(json_extract(document_json,'$.project.media')) m WHERE json_extract(m.value,'$.id')=? AND json_extract(m.value,'$.path')=?) ORDER BY revision DESC LIMIT 1")
                .bind(project).bind(media).bind(version).fetch_optional(pool).await?;
            document = serde_json::from_str(&raw.ok_or(Error::NotFound)?)?;
        }
    }
    let entry = document.project["media"].as_array().into_iter().flatten()
        .find(|entry| entry["id"] == media && version.is_none_or(|path| entry["path"] == path))
        .cloned().ok_or(Error::NotFound)?;
    let source = document.sources.remove(media).ok_or(Error::NotFound)?;
    Ok((entry, source))
}

/// Source browsing does not create an editing project. Membership is checked against the
/// source/group timeline, so a segment ID from another session cannot be used here.
pub async fn session_source(
    pool: &ConnectionPool,
    session: i64,
    media: &str,
) -> Result<(PathBuf, RecordingSource), Error> {
    let segment_id = media
        .rsplit('-')
        .next()
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let segment = store::segment(pool, segment_id)
        .await?
        .ok_or(Error::NotFound)?;
    let member = crate::server::workbench::session_groups::timeline(pool, session)
        .await?
        .into_iter()
        .find(|member| member.source_session_id == segment.session_id)
        .ok_or(Error::NotFound)?;
    if media != format!("recording-{}-{}", segment.session_id, segment.id)
        || matches!(
            segment.state,
            store::SegmentState::Missing | store::SegmentState::Deleted
        )
    {
        return Err(Error::NotFound);
    }
    let path = PathBuf::from(&segment.path);
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| Error::NotFound)?;
    let duration = if segment.state == store::SegmentState::Recording {
        let index = crate::server::workbench::segment_index(&segment)
            .await
            .map_err(|_| Error::NotFound)?;
        crate::server::workbench::readable_ms(&index, metadata.len())
    } else {
        segment
            .end_ms
            .unwrap_or(segment.start_ms)
            .saturating_sub(segment.start_ms)
    };
    if duration <= 0 {
        return Err(Error::NotFound);
    }
    let source = RecordingSource {
        kind: "recording".into(),
        session_id: segment.session_id,
        segment_id,
        view_session_id: session,
        view_start_ms: segment.start_ms + member.offset_ms,
        source_start_ms: segment.start_ms,
        duration_ms: duration,
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        state: segment.state.as_str().into(),
        has_danmaku: segment.danmaku_path.is_some(),
    };
    Ok((path, source))
}

pub async fn probe(
    pool: &ConnectionPool,
    path: &Path,
    source: &MediaSource,
    project_fps: f64,
) -> Result<Value, String> {
    let MediaSource::Recording(source) = source else {
        let MediaSource::File(source) = source else {
            unreachable!()
        };
        return super::imports::probe(pool, source, project_fps).await;
    };
    let cancel = CancellationToken::new();
    let _cancel = cancel.clone().drop_guard();
    let _permit = transcode::acquire(&cancel).await?;
    let info = engine::probe(path, &cancel).await?;
    let (num, den) = info.frame_rate.split_once('/').unwrap_or(("30", "1"));
    let fps = num.parse::<f64>().unwrap_or(30.) / den.parse::<f64>().unwrap_or(1.);
    Ok(
        json!({"width":info.width,"height":info.height,"hasAudio":info.has_audio,"fps":fps,
        "frames":(info.duration_ms.unwrap_or(source.duration_ms).min(source.duration_ms) as f64 / 1000. * project_fps).floor().max(1.) as i64}),
    )
}

/// A bounded sprite: twelve individually sought frames, not a full decode of a long recording.
pub async fn picture(
    root: &Path,
    path: &Path,
    source: &MediaSource,
    strip: bool,
) -> Result<(PathBuf, Value), String> {
    if path.extension().is_some_and(|e|e=="svg") {
        return Ok((path.to_path_buf(),json!({"version":1,"count":1,"interval":1,"seconds":1,"cols":1,"rows":1,"tileWidth":160,"tileHeight":90})));
    }
    let seconds = source.duration_ms().unwrap_or(1000) as f64 / 1000.;
    let count = if strip && source.duration_ms().is_some() {
        12
    } else {
        1
    };
    let interval = (seconds / count as f64).max(0.001);
    let info = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
    let key = format!(
        "{:x}",
        Sha256::digest(format!(
            "v1|{}|{}|{}|{}|{count}",
            path.display(),
            info.len(),
            info.modified()
                .ok()
                .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                .map(|v| v.as_nanos())
                .unwrap_or(0),
            source.duration_ms().unwrap_or(0)
        ))
    );
    let directory = root.join("editor-cache");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| e.to_string())?;
    let output = directory.join(format!("{key}.jpg"));
    let layout = json!({"version":1,"count":count,"interval":interval,"seconds":seconds,"cols":count,"rows":1,"tileWidth":160,"tileHeight":90});
    if tokio::fs::try_exists(&output).await.unwrap_or(false) {
        return Ok((output, layout));
    }
    // Different projects or windows can request the same registered file together.
    // Generate its content-keyed picture once, including while waiting for an encoder.
    let picture = picture_lock(&key);
    let _picture = picture.lock().await;
    if tokio::fs::try_exists(&output).await.unwrap_or(false) {
        return Ok((output, layout));
    }
    let cancel = CancellationToken::new();
    let _cancel = cancel.clone().drop_guard();
    let _permit = transcode::acquire(&cancel).await?;
    if tokio::fs::try_exists(&output).await.unwrap_or(false) {
        return Ok((output, layout));
    }
    let temp = tempfile::Builder::new()
        .prefix("editor-")
        .suffix(".jpg")
        .tempfile_in(&directory)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    let mut command = crate::tools::ffmpeg_command();
    command.args([
        "-hide_banner",
        "-nostdin",
        "-y",
        "-filter_complex_threads",
        "1",
    ]);
    let mut filters = Vec::new();
    for index in 0..count {
        command
            .args([
                "-ss",
                &format!("{:.6}", interval * index as f64),
                "-threads",
                "1",
                "-i",
            ])
            .arg(path);
        filters.push(format!("[{index}:v:0]scale=160:90:force_original_aspect_ratio=decrease,pad=160:90:(ow-iw)/2:(oh-ih)/2,setsar=1[v{index}]"));
    }
    let map = if count > 1 {
        filters.push(format!(
            "{}hstack=inputs={count}[out]",
            (0..count).map(|n| format!("[v{n}]")).collect::<String>()
        ));
        "[out]"
    } else {
        "[v0]"
    };
    command
        .args([
            "-filter_complex",
            &filters.join(";"),
            "-map",
            map,
            "-frames:v",
            "1",
            "-an",
            "-c:v",
            "mjpeg",
            "-q:v",
            "3",
            "-threads",
            "1",
            "-update",
            "1",
        ])
        .arg(&temp);
    engine::run_process(command, &cancel, Duration::from_secs(30), |_, _| {}).await?;
    temp.persist(&output).map_err(|e| e.to_string())?;
    Ok((output, layout))
}
