use crate::server::common::construct_headers;
use crate::server::core::downloader::{DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo};
use crate::server::errors::{AppError, AppResult};
use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup::downloader::live::strip_ws_expire_override;
use biliup::downloader::preview::PreviewFormat;
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use error_stack::{ResultExt, bail};
use nom::Err;
use reqwest::{Response, StatusCode};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// Stream-gears下载器实现
/// 使用stream-gears库进行直播流下载
pub struct StreamGears {
    /// 代理设置（可选）
    proxy: Option<String>,

    token: RwLock<CancellationToken>,
}

impl StreamGears {
    /// 创建新的Stream-gears下载器实例
    ///
    /// # 参数
    /// * `url` - 流URL
    /// * `header_map` - HTTP请求头
    /// * `file_name` - 输出文件名
    /// * `segment` - 分段配置
    /// * `proxy` - 代理设置（可选）
    pub fn new(proxy: Option<String>) -> Self {
        Self {
            proxy,
            token: RwLock::new(CancellationToken::new()),
        }
    }

    async fn start_download<'a>(
        &self,
        callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        // 开段与关段两个钩子共用同一个回调
        let callback = Arc::new(Mutex::new(callback));
        let file_name = download_config.recorder.filename_template();
        let headers_in = construct_headers(&download_config.headers).map_err(AppError::Custom)?;
        let proxy = self.proxy.clone();
        let segment = Segmentable::new(
            // 快到录制时间范围结束时会被裁短，使录制停在窗口边界
            download_config.segment_time_limit(),
            download_config.file_size,
        );

        // 创建HTTP客户端
        let client = StatelessClient::new(headers_in, proxy.as_deref());
        // 获取可重试的响应
        let (url, response) = connect(&client, download_config.url.clone())
            .await
            .change_context(AppError::Unknown)?;
        // 创建连接
        let mut connection = Connection::new(response);
        // 读取帧头
        let bytes = connection
            .read_frame(9)
            .await
            .change_context(AppError::Unknown)?;
        // let mut i = 0;
        // let mut prev_file_path = None;
        // 创建分段回调钩子
        let start_hook = {
            let callback = callback.clone();
            move |s: &str| {
                (callback.lock().unwrap())(SegmentEvent::Start {
                    next_file_path: PathBuf::from(s),
                });
            }
        };
        let hook = {
            let mut i = 0;
            move |s: &str| {
                let file_path = PathBuf::from(s);

                let event = SegmentInfo {
                    prev_file_path: file_path,
                    danmaku_file_path: None,
                    next_file_path: None,
                    segment_index: i,
                    duration_secs: None,
                    size_bytes: None,
                };
                (callback.lock().unwrap())(SegmentEvent::Segment(event));

                i += 1;
            }
        };
        // 解析流头部，判断流类型
        match header(&bytes) {
            Ok((_i, header)) => {
                debug!("header: {header:#?}");
                info!("Downloading {}...", url);
                // FLV流下载
                let file = LifecycleFile::with_hook(&file_name, "flv", hook)
                    .with_start_hook(start_hook)
                    .with_counter(download_config.bytes_written.clone())
                    .with_index_tap(download_config.index_tap.clone());
                // 直播预览：写入端与这一次拉流同寿命，拉流结束即 drop
                let preview = download_config.preview.attach(PreviewFormat::Flv);
                httpflv::download(connection, file, segment.clone(), Some(preview)).await;
            }
            Err(Err::Incomplete(needed)) => {
                error!("needed: {needed:?}")
            }
            Err(e) => {
                error!("{e}");
                // HLS流下载
                let file = LifecycleFile::with_hook(&file_name, "ts", hook)
                    .with_start_hook(start_hook)
                    .with_counter(download_config.bytes_written.clone())
                    .with_index_tap(download_config.index_tap.clone());
                let preview = download_config.preview.attach(PreviewFormat::MpegTs);
                hls::download(&url, &client, file, segment.clone(), Some(preview))
                    .await
                    .change_context(AppError::Unknown)?;
            }
        }
        Ok(DownloadStatus::StreamEnded)
    }
}

/// 首连；斗鱼网宿直链追加的 `expire=0` 被 403 时，本次改用原直链再连一次。
/// 返回实际连上的直链。
async fn connect(client: &StatelessClient, url: String) -> reqwest::Result<(String, Response)> {
    match client.retryable(&url).await {
        Err(e) if e.status() == Some(StatusCode::FORBIDDEN) => {
            let Some(original) = strip_ws_expire_override(&url) else {
                return Err(e);
            };
            warn!(
                "网宿拒绝了追加 expire=0 的直链（403），本次改用原直链，连接仍会按 expire 定时断开"
            );
            let original = original.to_string();
            let response = client.retryable(&original).await?;
            Ok((original, response))
        }
        result => result.map(|response| (url, response)),
    }
}

impl StreamGears {
    /// 开始下载流
    ///
    /// # 参数
    /// * `callback` - 分段完成时的回调函数
    pub(crate) async fn download<'a>(
        &self,
        callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        *self.token.write().unwrap() = CancellationToken::new();
        let token = self.token.read().unwrap().clone();
        tokio::select! {
            _ = token.cancelled() => {
                bail!(AppError::Custom("StreamGears token cancelled".into()))
            }
            res = self.start_download(callback, download_config) => {res}
        }
    }

    /// 停止下载
    pub(crate) async fn stop(&self) -> AppResult<()> {
        // 仅发出取消信号并更新状态
        // 如果底层下载函数不支持取消，这里不能真正中断正在进行的下载
        self.token.read().unwrap().cancel();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::StatusCode as HttpStatus;
    use axum::routing::get;
    use reqwest::header::HeaderMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 模拟网宿修掉重复参数的解析差异：带重复 expire 的一律 403，原直链 200
    async fn wangsu_rejecting_duplicate_expire() -> (String, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let app = Router::new().route(
            "/live/a.flv",
            get(move |RawQuery(query): RawQuery| {
                counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    let expires = query
                        .unwrap_or_default()
                        .split('&')
                        .filter(|pair| pair.starts_with("expire="))
                        .count();
                    if expires > 1 {
                        (HttpStatus::FORBIDDEN, "Invalid Request")
                    } else {
                        (HttpStatus::OK, "FLV")
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}/live/a.flv"), hits)
    }

    #[tokio::test]
    async fn connect_falls_back_to_original_url_once_on_403() {
        let (base, hits) = wangsu_rejecting_duplicate_expire().await;
        let original = format!("{base}?wsAuth=a&token=t&expire=300&fcdn=ws");
        let client = StatelessClient::new(HeaderMap::new(), None);

        let (url, response) = connect(&client, format!("{original}&expire=0"))
            .await
            .unwrap();

        assert_eq!(url, original);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn connect_does_not_retry_403_on_other_urls() {
        let (base, hits) = wangsu_rejecting_duplicate_expire().await;
        let client = StatelessClient::new(HeaderMap::new(), None);

        // 不是网宿直链：403 原样返回，不另发请求
        let err = connect(&client, format!("{base}?expire=300&fcdn=hw&expire=0"))
            .await
            .unwrap_err();

        assert_eq!(err.status(), Some(StatusCode::FORBIDDEN));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }
}
