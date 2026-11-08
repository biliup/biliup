//! Browser uploads are streamed into managed storage. Client names are labels, never paths.
//! Reuses the configured FFmpeg, including desktop bundles without a separate ffprobe.
use super::{Error, FileSource, invalid};
use crate::server::{
    common::processing::{engine, transcode},
    infrastructure::connection_pool::ConnectionPool,
};
use futures::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqliteConnection};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

const MAX_BYTES: u64 = 20 * 1024 * 1024 * 1024;
// No playlist/container may resolve external files or URLs during probing/decoding.
pub(super) const FORMATS: &str = "mov,matroska,webm,avi,flv,mpegts,mpeg,mp3,wav,flac,ogg,aac,ac3,aiff,gif,avif,png_pipe,jpeg_pipe,webp_pipe,bmp_pipe,tiff_pipe";

/// Timeline IDs survive file replacement; reuse files by their immutable registered path.
pub(crate) fn prepare_media(project: &Value, incoming: &Value) -> Result<(Value, bool), Error> {
    let mut media = incoming.clone();
    let base = media["id"].as_str().ok_or_else(|| invalid("素材标识无效"))?.to_owned();
    let all = project["media"].as_array().ok_or_else(|| invalid("工程素材无效"))?;
    if let Some(existing) = all.iter().find(|m| m["path"] == media["path"] && m["kind"] == media["kind"]) {
        media["id"] = existing["id"].clone();
        return Ok((media, true));
    }
    let mut taken = std::collections::HashSet::new();
    for entry in all.iter().chain(project["scenes"].as_array().into_iter().flatten()) {
        if let Some(id) = entry["id"].as_str() { taken.insert(id); }
    }
    for track in project["tracks"].as_array().into_iter().flatten() {
        if let Some(id) = track["id"].as_str() { taken.insert(id); }
        for item in track["items"].as_array().into_iter().flatten() {
            if let Some(id) = item["id"].as_str() { taken.insert(id); }
        }
    }
    if taken.contains(base.as_str()) {
        let mut suffix = 1;
        loop {
            let id = format!("{base}-{suffix}");
            if !taken.contains(id.as_str()) { media["id"] = json!(id); break; }
            suffix += 1;
        }
    }
    Ok((media, false))
}

pub async fn validate_source(
    conn: &mut SqliteConnection,
    _id: &str,
    media: &Value,
    source: &mut FileSource,
) -> Result<(), Error> {
    let row = sqlx::query("SELECT kind,duration_ms,file_name,path FROM editor_assets WHERE id=?")
        .bind(&source.asset_id)
        .fetch_optional(conn)
        .await?
        .ok_or(Error::NotFound)?;
    let duration: Option<i64> = row.try_get("duration_ms")?;
    if source.kind != "file"
        // A media identity survives re-shoots; the immutable file identity lives in path/source.
        || media["path"] != format!("imports/file-{}{}",source.asset_id,if row.try_get::<String,_>("path")?.ends_with(".svg"){".svg"}else{""})
        || media["kind"] != row.try_get::<String, _>("kind")?
        || source.duration_ms != duration
    {
        return Err(invalid("导入素材的身份、类型或时长不匹配"));
    }
    source.file_name = row.try_get("file_name")?;
    Ok(())
}

pub async fn path(pool: &ConnectionPool, id: &str) -> Result<PathBuf, Error> {
    let path: String = sqlx::query_scalar("SELECT path FROM editor_assets WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(PathBuf::from(path))
}

/// Metadata parsing uses input streams only: the null output also prints streams.
pub(crate) fn metadata(text: &str) -> Result<(String, Option<i64>, Value, &'static str), String> {
    metadata_with_image(text,false)
}
fn metadata_with_image(text:&str,avif:bool)->Result<(String,Option<i64>,Value,&'static str),String>{
    let input = text.split("Output #").next().unwrap_or(text);
    let video = input.lines().find(|line| {
        line.contains("Stream #") && line.contains("Video:") && !line.contains("attached pic")
    });
    let audio = input
        .lines()
        .any(|line| line.contains("Stream #") && line.contains("Audio:"));
    let image = avif || input.lines().any(|line| {
        line.starts_with("Input #0,")
            && [
                "png_pipe",
                "jpeg_pipe",
                "webp_pipe",
                "bmp_pipe",
                "tiff_pipe",
                "Input #0, gif,",
                "Input #0, avif,",
            ]
            .iter()
            .any(|format| line.contains(format))
    });
    if video.is_none() && !audio {
        return Err("文件中没有可导入的视频、音频或图片".into());
    }
    let kind = if image {
        "image"
    } else if video.is_some() {
        "video"
    } else {
        "audio"
    };
    let duration = regex::Regex::new(r"Duration: (\d+):(\d+):(\d+(?:\.\d+)?)").unwrap();
    let duration_ms = if image {
        None
    } else {
        let c = duration.captures(input).ok_or("无法读取素材时长")?;
        let ms = ((c[1].parse::<f64>().unwrap_or(0.) * 3600.
            + c[2].parse::<f64>().unwrap_or(0.) * 60.
            + c[3].parse::<f64>().unwrap_or(0.))
            * 1000.)
            .round() as i64;
        if ms <= 0 {
            return Err("素材时长无效".into());
        }
        Some(ms)
    };
    let mut probe = json!({"hasAudio": audio});
    if let Some(video) = video {
        let dimensions = regex::Regex::new(r",\s*(\d+)x(\d+)(?:\s|,|\[|$)").unwrap();
        let c = dimensions.captures(video).ok_or("无法读取素材尺寸")?;
        let mut width: u32 = c[1].parse().map_err(|_| "素材尺寸无效")?;
        let mut height: u32 = c[2].parse().map_err(|_| "素材尺寸无效")?;
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err("素材尺寸超出处理范围".into());
        }
        let rotation = regex::Regex::new(r"rotation of (-?[\d.]+) degrees")
            .unwrap()
            .captures(input)
            .and_then(|c| c[1].parse::<f64>().ok())
            .unwrap_or(0.);
        if (rotation.abs() % 180. - 90.).abs() < 1. {
            std::mem::swap(&mut width, &mut height);
        }
        probe["width"] = json!(width);
        probe["height"] = json!(height);
        if !image {
            let fps = regex::Regex::new(r"([\d.]+) fps")
                .unwrap()
                .captures(video)
                .and_then(|c| c[1].parse::<f64>().ok())
                .filter(|fps| *fps > 0. && *fps <= 240.)
                .unwrap_or(30.);
            probe["fps"] = json!(fps);
        }
    }
    // Serve with a media extension determined from the container, never an uploaded HTML name.
    let extension = if image {
        if avif || input.contains("Input #0, avif,") {
            "avif"
        } else if input.contains("Input #0, gif,") {
            "gif"
        } else if input.contains("png_pipe") {
            "png"
        } else if input.contains("webp_pipe") {
            "webp"
        } else if input.contains("bmp_pipe") {
            "bmp"
        } else if input.contains("tiff_pipe") {
            "tiff"
        } else {
            "jpg"
        }
    } else if input.lines().any(|line| line.starts_with("Input #0, mov,")) {
        "mp4"
    } else if input.contains("Input #0, matroska,webm,") {
        "webm"
    } else if input.contains("Input #0, flv,") {
        "flv"
    } else if input.contains("Input #0, mpegts,") {
        "ts"
    } else if input.contains("Input #0, avi,") {
        "avi"
    } else if input.contains("Input #0, wav,") {
        "wav"
    } else if input.contains("Input #0, flac,") {
        "flac"
    } else if input.contains("Input #0, ogg,") {
        "ogg"
    } else if input.contains("Input #0, aac,") {
        "aac"
    } else if kind == "audio" {
        "mp3"
    } else {
        "mpg"
    };
    Ok((kind.into(), duration_ms, probe, extension))
}

pub async fn upload(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    name: &str,
    capture: Option<&str>,
    body: axum::body::Body,
) -> Result<Value, String> {
    let saved = super::get(pool, project).await.map_err(|e| e.to_string())?;
    if capture.is_some_and(|kind| !["screen", "camera", "voiceover", "system"].contains(&kind)) {
        return Err("采集类型无效".into());
    }
    let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if name.is_empty() || name.chars().count() > 256 || name.chars().any(char::is_control) {
        return Err("文件名称无效".into());
    }
    let directory = root.join("editor-assets");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| e.to_string())?;
    let mut temp = tempfile::Builder::new()
        .prefix("upload-")
        .tempfile_in(&directory)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(120), stream.next())
        .await
        .map_err(|_| "上传超时")?
    {
        let chunk = chunk.map_err(|_| "上传中断")?;
        size = size.checked_add(chunk.len() as u64).ok_or("文件过大")?;
        if size > MAX_BYTES {
            return Err("单个素材不能超过 20 GB".into());
        }
        hash.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if size == 0 {
        return Err("不能导入空文件".into());
    }
    let cancel = CancellationToken::new();
    let _cancel = cancel.clone().drop_guard();
    let _permit = transcode::acquire(&cancel).await?;
    // MediaRecorder WebM often has no duration and variable frame timing. Normalize before
    // registering it so the regular probe, timeline, preview and exporter share one asset.
    if let Some(kind) = capture {
        let audio = matches!(kind, "voiceover" | "system");
        let converted = tempfile::Builder::new()
            .prefix("capture-")
            .suffix(if audio { ".m4a" } else { ".mp4" })
            .tempfile_in(&directory)
            .map_err(|e| e.to_string())?
            .into_temp_path();
        let mut command = crate::tools::ffmpeg_command();
        command
            .args([
                "-y",
                "-nostdin",
                "-hide_banner",
                "-protocol_whitelist",
                "file,pipe",
                "-format_whitelist",
                "mov,matroska,webm",
                "-i",
            ])
            .arg(&temp);
        if audio {
            command.args([
                "-map", "0:a:0", "-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "48000",
            ]);
        } else {
            let fps = saved.document.project["canvas"]["fps"]
                .as_f64()
                .unwrap_or(30.)
                .clamp(1., 240.);
            command.args([
                "-map",
                "0:v:0",
                "-map",
                "0:a:0?",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "20",
                "-pix_fmt",
                "yuv420p",
                "-vf",
                "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                "-r",
                &fps.to_string(),
                "-fps_mode",
                "cfr",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-ar",
                "48000",
            ]);
        }
        command.args(["-movflags", "+faststart"]).arg(&converted);
        engine::run_process(command, &cancel, Duration::from_secs(1800), |_, _| {}).await?;
        temp = converted;
        hash = Sha256::new();
        let mut input = tokio::fs::File::open(&temp)
            .await
            .map_err(|e| e.to_string())?;
        let mut buffer = vec![0; 128 * 1024];
        loop {
            let length = input.read(&mut buffer).await.map_err(|e| e.to_string())?;
            if length == 0 {
                break;
            }
            hash.update(&buffer[..length]);
        }
    }
    let asset_id = format!("{:x}", hash.finalize());
    let svg=name.to_ascii_lowercase().ends_with(".svg") && capture.is_none();
    let (kind, duration_ms, mut probe, extension) = if svg {
        if size>super::svg::MAX_BYTES as u64{return Err("SVG 文件不能超过 10 MiB".into())}
        let text=tokio::fs::read_to_string(&temp).await.map_err(|_|"SVG 必须是 UTF-8 文本")?;
        let (width,height)=super::svg::size(&text)?;
        ("image".to_string(),None,json!({"width":width,"height":height,"hasAudio":false}),"svg")
    } else {
    let mut command = crate::tools::ffmpeg_command();
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-protocol_whitelist",
            "file,pipe",
            "-format_whitelist",
            FORMATS,
            "-i",
        ])
        .arg(&temp)
        .args([
            "-map",
            "0:v:0?",
            "-map",
            "0:a:0?",
            "-frames:v",
            "1",
            "-frames:a",
            "1",
            "-t",
            "0.05",
            "-f",
            "null",
            "-",
        ]);
    let output = engine::run_process(command, &cancel, Duration::from_secs(30), |_, _| {}).await?;
    let mut header=Vec::new();
    tokio::fs::File::open(&temp).await.map_err(|e|e.to_string())?.take(64).read_to_end(&mut header).await.map_err(|e|e.to_string())?;
    // AVIF uses the MOV/ISO-BMFF demuxer in some FFmpeg builds. Trust the brands, not the label.
    let avif=header.get(4..8)==Some(b"ftyp")&&(header.get(8..12).is_some_and(|brand|matches!(brand,b"avif"|b"avis"))||header.get(16..).is_some_and(|brands|brands.chunks_exact(4).any(|brand|matches!(brand,b"avif"|b"avis"))));
    metadata_with_image(&output,avif)?
    };
    let output = std::path::absolute(directory.join(format!("{asset_id}.{extension}")))
        .map_err(|e| e.to_string())?;
    // Serialized by the shared transcode permit; never replace an existing immutable asset.
    if !tokio::fs::try_exists(&output).await.unwrap_or(false) {
        temp.persist(&output).map_err(|e| e.to_string())?;
    }
    sqlx::query("INSERT OR IGNORE INTO editor_assets(id,path,file_name,kind,duration_ms,probe_json,created_at) VALUES(?,?,?,?,?,?,?)")
        .bind(&asset_id).bind(output.to_string_lossy().as_ref()).bind(name).bind(&kind).bind(duration_ms).bind(probe.to_string()).bind(super::now_ms())
        .execute(pool).await.map_err(|e| e.to_string())?;
    if let Some(ms) = duration_ms {
        probe["frames"] = json!(
            (ms as f64
                * saved.document.project["canvas"]["fps"]
                    .as_f64()
                    .unwrap_or(30.)
                / 1000.)
                .floor()
                .max(1.) as i64
        );
    }
    let id = format!("file-{asset_id}");
    Ok(
        json!({"media":{"id":id,"path":format!("imports/{id}{}",if svg{".svg"}else{""}),"name":name,"kind":kind,"probe":probe},
        "source":FileSource{kind:"file".into(),asset_id,duration_ms,file_name:name.into()}}),
    )
}

pub async fn probe(pool: &ConnectionPool, source: &FileSource, fps: f64) -> Result<Value, String> {
    let stored: String = sqlx::query_scalar("SELECT probe_json FROM editor_assets WHERE id=?")
        .bind(&source.asset_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let mut probe: Value = serde_json::from_str(&stored).map_err(|e| e.to_string())?;
    if let Some(ms) = source.duration_ms {
        probe["frames"] = json!((ms as f64 * fps / 1000.).floor().max(1.) as i64);
    }
    Ok(probe)
}
