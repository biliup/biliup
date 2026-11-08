//! Immutable JPEGs shared by the cover preview and the submitted publication.
struct CoverSource<'a> {
    kind: &'a str,
    output: i64,
    stamp: &'a str,
    source: &'a str,
    name: &'a str,
    at_ms: Option<i64>,
}

use super::{ConnectionPool, Error, RenderJobs, Result, invalid, now_ms};
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{io::Cursor, path::Path, sync::LazyLock, time::Duration};
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
const MAX_JPEG_BYTES: usize = 2 * 1024 * 1024;
const MAX_STORED_BYTES: i64 = 512 * 1024 * 1024;
const UNUSED_LIFETIME_MS: i64 = 24 * 60 * 60 * 1000;
static IMAGE_SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

#[derive(Default, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase", deny_unknown_fields)]
pub enum Selection {
    #[default]
    Template,
    Default,
    Custom {
        id: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub at_ms: i64,
}
#[derive(Serialize)]
pub struct Cover {
    pub id: String,
    pub name: String,
    pub source: String,
    pub at_ms: Option<i64>,
    pub width: i64,
    pub height: i64,
    pub url: String,
}
struct Image {
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
}

pub async fn cleanup(pool: &ConnectionPool) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM output_covers WHERE updated_at<? AND NOT EXISTS(SELECT 1 FROM output_publications p WHERE p.cover_id=output_covers.id)")
        .bind(now_ms()-UNUSED_LIFETIME_MS).execute(pool).await?;
    Ok(())
}
fn view(row: &sqlx::sqlite::SqliteRow, kind: &str, output: i64) -> Result<Cover> {
    let id: String = row.try_get("id")?;
    Ok(Cover {
        url: format!("/v1/outputs/{kind}/{output}/publish/covers/{id}"),
        id,
        name: row.try_get("name")?,
        source: row.try_get("source")?,
        at_ms: row.try_get("at_ms")?,
        width: row.try_get("width")?,
        height: row.try_get("height")?,
    })
}
async fn normalize(bytes: Vec<u8>) -> Result<Image> {
    if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
        return Err(invalid("封面图片最大 5 MiB"));
    }
    let permit = IMAGE_SLOTS
        .acquire()
        .await
        .map_err(|_| invalid("图片处理暂不可用"))?;

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let format = image::guess_format(&bytes).map_err(|_| invalid("无法识别封面图片"))?;
        if !matches!(
            format,
            image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
        ) {
            return Err(invalid("封面只支持 JPEG、PNG 或 WebP 图片"));
        }
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|_| invalid("封面图片无法解码或尺寸过大"))?;
        let orientation = decoder
            .orientation()
            .map_err(|_| invalid("封面图片方向信息无效"))?;
        let mut image = image::DynamicImage::from_decoder(decoder)
            .map_err(|_| invalid("封面图片无法解码或尺寸过大"))?;
        image.apply_orientation(orientation);
        let rgba = image.thumbnail(1920, 1080).to_rgba8();
        let (width, height) = rgba.dimensions();
        if width == 0 || height == 0 {
            return Err(invalid("封面图片尺寸无效"));
        }
        // Composite transparency onto white; JPEG never interprets transparent pixels as black.
        let rgb = image::RgbImage::from_fn(width, height, |x, y| {
            let pixel = rgba.get_pixel(x, y).0;
            let alpha = u32::from(pixel[3]);
            image::Rgb(std::array::from_fn(|i| {
                ((u32::from(pixel[i]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
            }))
        });
        for quality in [90, 80, 70] {
            let mut jpeg = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)
                .encode_image(&rgb)
                .map_err(|_| invalid("封面编码失败"))?;
            if jpeg.len() <= MAX_JPEG_BYTES {
                return Ok(Image {
                    jpeg,
                    width,
                    height,
                });
            }
        }
        Err(invalid("封面编码后超过 2 MiB，请裁小图片"))
    })
    .await
    .map_err(|_| invalid("封面图片处理失败"))?
}
async fn read_image(path: &Path) -> Result<Vec<u8>> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| invalid("封面文件不可用，请上传图片或从成片截帧"))?;
    let meta = file.metadata().await.map_err(|_| invalid("读取封面失败"))?;
    if !meta.is_file() || meta.len() > MAX_INPUT_BYTES as u64 {
        return Err(invalid("封面必须为不超过 5 MiB 的图片"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| invalid("读取封面失败"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(invalid("封面图片最大 5 MiB"));
    }
    Ok(bytes)
}
async fn store(
    pool: &ConnectionPool,
    CoverSource {
        kind,
        output,
        stamp,
        source,
        name,
        at_ms,
    }: CoverSource<'_>,
    image: Image,
) -> Result<Cover> {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&(
        kind, output, stamp, source, name, at_ms,
    ))?);
    hasher.update(&image.jpeg);
    let id = format!("{:x}", hasher.finalize());
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("DELETE FROM output_covers WHERE updated_at<? AND NOT EXISTS(SELECT 1 FROM output_publications p WHERE p.cover_id=output_covers.id)")
        .bind(now_ms()-UNUSED_LIFETIME_MS).execute(&mut *tx).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM output_covers WHERE id=?)")
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        let bytes: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(length(jpeg)),0) FROM output_covers")
                .fetch_one(&mut *tx)
                .await?;
        if bytes + image.jpeg.len() as i64 > MAX_STORED_BYTES {
            return Err(invalid("投稿封面存储空间已满"));
        }
        let pending:i64=sqlx::query_scalar("SELECT COUNT(*) FROM output_covers c WHERE kind=? AND output_id=? AND NOT EXISTS(SELECT 1 FROM output_publications p WHERE p.cover_id=c.id)")
            .bind(kind).bind(output).fetch_one(&mut *tx).await?;
        if pending >= 64 {
            return Err(invalid("此成片的待用封面过多，请稍后再添加"));
        }
    }
    sqlx::query("INSERT INTO output_covers(id,kind,output_id,source_stamp,source,name,at_ms,jpeg,width,height,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET updated_at=excluded.updated_at")
        .bind(&id).bind(kind).bind(output).bind(stamp).bind(source).bind(name).bind(at_ms).bind(image.jpeg).bind(image.width as i64).bind(image.height as i64).bind(now_ms()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Cover {
        url: format!("/v1/outputs/{kind}/{output}/publish/covers/{id}"),
        id,
        name: name.into(),
        source: source.into(),
        at_ms,
        width: image.width as i64,
        height: image.height as i64,
    })
}
pub async fn bytes(pool: &ConnectionPool, kind: &str, output: i64, id: &str) -> Result<Vec<u8>> {
    sqlx::query_scalar("SELECT jpeg FROM output_covers WHERE id=? AND kind=? AND output_id=?")
        .bind(id)
        .bind(kind)
        .bind(output)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
}
pub(super) async fn select(
    pool: &ConnectionPool,
    kind: &str,
    output: i64,
    stamp: &str,
    template: &str,
    choice: Selection,
) -> Result<Option<Cover>> {
    match choice {
        Selection::Default => Ok(None),
        Selection::Template if template.is_empty() => Ok(None),
        Selection::Template => {
            let path = Path::new(template);
            let name = path
                .file_name()
                .ok_or_else(|| invalid("模板封面文件名无效"))?
                .to_string_lossy()
                .into_owned();
            let image = normalize(read_image(path).await?).await?;
            Ok(Some(
                store(
                    pool,
                    CoverSource {
                        kind,
                        output,
                        stamp,
                        source: "template",
                        name: &name,
                        at_ms: None,
                    },
                    image,
                )
                .await?,
            ))
        }
        Selection::Custom { id } => {
            let row=sqlx::query("UPDATE output_covers SET updated_at=? WHERE id=? AND kind=? AND output_id=? AND source_stamp=? RETURNING id,name,source,at_ms,width,height")
                .bind(now_ms()).bind(&id).bind(kind).bind(output).bind(stamp).fetch_optional(pool).await?.ok_or_else(||invalid("所选封面已过期或成片已变化，请重新选择"))?;
            Ok(Some(view(&row, kind, output)?))
        }
    }
}
pub async fn upload(
    pool: &ConnectionPool,
    jobs: &RenderJobs,
    kind: &str,
    output_id: i64,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Cover> {
    let _lifecycle = super::MEDIA_LIFECYCLE.read().await;
    let output = super::output(pool, jobs, kind, output_id).await?;
    let stamp = super::file_stamp(&output.path).await?;
    let name = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("cover")
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>();
    let image = normalize(bytes).await?;
    store(
        pool,
        CoverSource {
            kind,
            output: output_id,
            stamp: &stamp,
            source: "upload",
            name: if name.is_empty() { "cover" } else { &name },
            at_ms: None,
        },
        image,
    )
    .await
}
pub async fn frame(
    pool: &ConnectionPool,
    jobs: &RenderJobs,
    kind: &str,
    output_id: i64,
    at_ms: i64,
) -> Result<Cover> {
    let _lifecycle = super::MEDIA_LIFECYCLE.read().await;
    let output = super::output(pool, jobs, kind, output_id).await?;
    if at_ms < 0 || at_ms >= output.duration_ms {
        return Err(invalid("截帧位置超出成片时长"));
    }
    let stamp = super::file_stamp(&output.path).await?;
    let cancel = CancellationToken::new();
    let _cancel = cancel.clone().drop_guard();
    let _permit = tokio::time::timeout(
        Duration::from_secs(30),
        crate::server::workbench::transcode::acquire(&cancel),
    )
    .await
    .map_err(|_| invalid("视频处理繁忙，请稍后再截帧"))?
    .map_err(invalid)?;
    let temp = tempfile::Builder::new()
        .prefix("biliup-cover-frame-")
        .suffix(".jpg")
        .tempfile()
        .map_err(|_| invalid("无法准备截帧文件"))?
        .into_temp_path();
    let mut command = crate::tools::ffmpeg_command();
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-ss",
            &format!("{:.3}", at_ms as f64 / 1000.),
            "-threads",
            "1",
            "-i",
        ])
        .arg(&output.path)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-an",
            "-sn",
            "-vf",
            "scale=w='min(1920,iw)':h='min(1080,ih)':force_original_aspect_ratio=decrease,setsar=1",
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
    crate::server::common::processing::engine::run_process(
        command,
        &cancel,
        Duration::from_secs(30),
        |_, _| {},
    )
    .await
    .map_err(|_| invalid("成片截帧失败，请检查 FFmpeg 或更换时间点"))?;
    drop(_permit);
    let image = normalize(read_image(&temp).await?).await?;
    if stamp != super::file_stamp(&output.path).await? {
        return Err(Error::Conflict("截帧期间成片已变化，请重新选择".into()));
    }
    store(
        pool,
        CoverSource {
            kind,
            output: output_id,
            stamp: &stamp,
            source: "frame",
            name: &format!("{} @ {:.3}s", output.name, at_ms as f64 / 1000.),
            at_ms: Some(at_ms),
        },
        image,
    )
    .await
}
