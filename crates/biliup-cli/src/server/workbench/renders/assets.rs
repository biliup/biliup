//! Uploaded images are addressed by IDs, never by paths supplied by a browser.
use super::model::{AssetView, RenderAsset};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
pub const MAX_ASSET_BYTES: usize = 10 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct Asset {
    pub session_id: i64,
    pub mime: String,
    pub render: RenderAsset,
}
impl Asset {
    pub fn view(&self) -> AssetView {
        AssetView {
            id: self.render.id,
            width: self.render.width,
            height: self.render.height,
            sha256: self.render.sha256.clone(),
            url: format!("/v1/render-assets/{}", self.render.id),
        }
    }
}
pub async fn get(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<Asset>> {
    let row = sqlx::query(
        "SELECT id,session_id,path,mime,width,height,sha256 FROM render_assets WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        Ok(Asset {
            session_id: r.try_get("session_id")?,
            mime: r.try_get("mime")?,
            render: RenderAsset {
                id: r.try_get("id")?,
                path: PathBuf::from(r.try_get::<String, _>("path")?),
                width: r.try_get::<i64, _>("width")? as u32,
                height: r.try_get::<i64, _>("height")? as u32,
                sha256: r.try_get("sha256")?,
            },
        })
    })
    .transpose()
}
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<AssetView>> {
    let ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM render_assets WHERE session_id=? ORDER BY id DESC")
            .bind(session_id)
            .fetch_all(pool)
            .await?;
    let mut out = vec![];
    for id in ids {
        if let Some(a) = get(pool, id).await? {
            out.push(a.view());
        }
    }
    Ok(out)
}
pub async fn upload(
    pool: &ConnectionPool,
    root: &Path,
    session_id: i64,
    mime: &str,
    data: &[u8],
    now: i64,
) -> Result<AssetView, String> {
    if data.is_empty() || data.len() > MAX_ASSET_BYTES {
        return Err("图片最大为 10 MiB".into());
    }
    let format = image::guess_format(data).map_err(|_| "图片格式无效".to_string())?;
    let (extension, expected) = match format {
        image::ImageFormat::Png => ("png", "image/png"),
        image::ImageFormat::Jpeg => ("jpg", "image/jpeg"),
        _ => return Err("只接受静态 PNG 或 JPEG 图片".into()),
    };
    if mime.split(';').next().unwrap_or("").trim() != expected {
        return Err("图片内容与 Content-Type 不一致".into());
    }
    let decode_data = data.to_vec();
    let (width, height) = tokio::task::spawn_blocking(move || {
        if format == image::ImageFormat::Png && animated_png(&decode_data) {
            return Err("图片遮挡只支持静态 PNG，不支持 APNG".to_string());
        }
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(decode_data), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|_| "图片无法解码或大于 8192 像素".to_string())?;
        if image.width() == 0 || image.height() == 0 {
            return Err("图片尺寸无效".into());
        }
        Ok((image.width(), image.height()))
    })
    .await
    .map_err(|e| format!("图片解码失败：{e}"))??;
    let digest = format!("{:x}", Sha256::digest(data));
    let dir = root.join("assets").join(session_id.to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("创建图片目录失败：{e}"))?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let id:i64=sqlx::query_scalar("INSERT INTO render_assets(session_id,path,mime,width,height,sha256,bytes,created_at) VALUES(?,'',?,?,?,?,?,?) RETURNING id")
        .bind(session_id).bind(expected).bind(width as i64).bind(height as i64).bind(&digest).bind(data.len() as i64).bind(now).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    let path = dir.join(format!("{id}-{digest}.{extension}"));
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .map_err(|e| format!("创建图片文件失败：{e}"))?;
    if let Err(e) = file.write_all(data).await {
        drop(file);
        let _ = tokio::fs::remove_file(&path).await;
        return Err(format!("保存图片失败：{e}"));
    }
    drop(file);
    if let Err(e) = sqlx::query("UPDATE render_assets SET path=? WHERE id=?")
        .bind(path.to_string_lossy().as_ref())
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(e.to_string());
    }
    if let Err(e) = tx.commit().await {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(e.to_string());
    }
    Ok(AssetView {
        id,
        width,
        height,
        sha256: digest,
        url: format!("/v1/render-assets/{id}"),
    })
}
pub async fn delete(pool: &ConnectionPool, id: i64) -> Result<bool, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let referenced:i64=sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM render_recipe_assets WHERE asset_id=?)+(SELECT COUNT(*) FROM render_job_assets WHERE asset_id=?)")
        .bind(id).bind(id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    if referenced > 0 {
        return Err("图片正被效果配置或合成任务引用，请先移除引用".into());
    }
    let path: Option<String> =
        sqlx::query_scalar("DELETE FROM render_assets WHERE id=? RETURNING path")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    if let Some(path) = path {
        let _ = tokio::fs::remove_file(path).await;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn animated_png(data: &[u8]) -> bool {
    let mut at = 8usize;
    while at.checked_add(12).is_some_and(|end| end <= data.len()) {
        let len = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
        if &data[at + 4..at + 8] == b"acTL" {
            return true;
        }
        let Some(next) = at.checked_add(12).and_then(|v| v.checked_add(len)) else {
            break;
        };
        if next > data.len() {
            break;
        }
        at = next;
    }
    false
}
