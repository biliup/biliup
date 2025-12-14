use crate::server::core::downloader::cover_downloader;
use crate::server::errors::{AppError, AppResult};
use axum::http::HeaderValue;
use axum::http::header::USER_AGENT;
use error_stack::ResultExt;
use reqwest::header::HeaderMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, instrument, warn};
use url::Url;

/// 支持的图片格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpg,
    Png,
    Webp,
}

impl ImageFormat {
    /// 从 URL 路径中检测图片格式
    fn detect(url_path: &str) -> Option<Self> {
        // 转小写后匹配，更健壮
        let path = url_path.to_ascii_lowercase();

        [
            (".jpg", Self::Jpg),
            (".jpeg", Self::Jpg),
            (".png", Self::Png),
            (".webp", Self::Webp),
        ]
        .into_iter()
        .find(|(ext, _)| path.contains(ext))
        .map(|(_, fmt)| fmt)
    }

    const fn as_ext(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    const fn needs_conversion(self) -> bool {
        matches!(self, Self::Webp)
    }
}

/// 封面下载器
/// 下载封面图片
#[instrument(fields(url = %url))]
async fn download(
    url: &str,
    filename: &str,
    base_dir: PathBuf,
    client: reqwest::Client,
) -> AppResult<PathBuf> {
    // 解析并验证 URL
    let parsed =
        Url::parse(url).change_context_lazy(|| AppError::Custom(String::from("无效的 URL")))?;

    let format = ImageFormat::detect(parsed.path())
        .ok_or_else(|| AppError::Custom(format!("不支持的图片格式: {url}")))?;

    // 构建保存目录
    let save_dir = base_dir;
    fs::create_dir_all(&save_dir)
        .change_context_lazy(|| AppError::Custom(String::from("创建目录失败")))?;

    let file_path = save_dir.join(format!("{filename}.{}", format.as_ext()));

    // 下载或使用缓存
    if !file_path.exists() {
        fetch_and_save(url, &file_path, client).await?;
    }

    // webp 需要转换为 jpg
    if format.needs_conversion() {
        return convert_to_jpg(&file_path, &save_dir, filename);
    }

    Ok(file_path)
}

/// 下载并保存文件
async fn fetch_and_save(url: &str, path: &Path, client: reqwest::Client) -> AppResult<()> {
    let bytes = client
        .get(url)
        .send()
        .await
        .change_context_lazy(|| AppError::Custom(String::from("网络请求失败")))?
        .error_for_status()
        .change_context_lazy(|| AppError::Custom(String::from("服务器返回错误状态")))?
        .bytes()
        .await
        .change_context_lazy(|| AppError::Custom(String::from("读取响应数据失败")))?;

    fs::write(path, &bytes)
        .change_context_lazy(|| AppError::Custom(String::from("写入文件失败")))?;
    Ok(())
}

/// 将图片转换为 JPG 格式
fn convert_to_jpg(src: &Path, dir: &Path, filename: &str) -> AppResult<PathBuf> {
    let jpg_path = dir.join(format!("{filename}.jpg"));

    image::open(src)
        .change_context_lazy(|| AppError::Custom(String::from("打开图片失败")))?
        .to_rgb8()
        .save(&jpg_path)
        .change_context_lazy(|| AppError::Custom(String::from("保存 JPG 失败")))?;

    // 删除原文件，忽略错误
    let _ = fs::remove_file(src);

    Ok(jpg_path)
}

pub async fn download_cover_with(
    url: &str,
    enabled: bool,
    fmtname: &str,
    client: reqwest::Client,
) -> Option<PathBuf> {
    // 使用 guard clause 提前返回
    if !enabled {
        return None;
    }

    // 构建请求头
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
    );

    // 创建下载器
    match cover_downloader::download(url, fmtname, PathBuf::from("data/cover"), client).await {
        Ok(path) => {
            let display_path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            info!("封面下载成功，路径：{}", display_path.display());
            Some(path)
        }
        Err(e) => {
            error!("封面下载失败: {e:#}");
            None
        }
    }
}
