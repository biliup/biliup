use crate::server::common::construct_headers;
use crate::server::common::util::parse_time;
use crate::server::core::downloader::{DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo};
use crate::server::errors::{AppError, AppResult};
use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use error_stack::{ResultExt, bail};
use nom::Err;
use std::path::PathBuf;
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

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
        mut callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        let url = download_config.url.clone();
        let file_name = download_config.recorder.filename_template();
        let headers_in = construct_headers(&download_config.headers);
        let proxy = self.proxy.clone();
        let segment = Segmentable::new(
            download_config.segment_time.as_deref().map(parse_time),
            download_config.file_size,
        );

        // 创建HTTP客户端
        let client = StatelessClient::new(headers_in, proxy.as_deref());
        // 获取可重试的响应
        let response = client
            .retryable(&url)
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
        let hook = {
            let mut i = 0;
            let mut prev_file_path = None::<PathBuf>;
            move |s: &str| {
                let file_path = PathBuf::from(s);

                let event = SegmentInfo {
                    prev_file_path: file_path.clone(),
                    next_file_path: None,
                    segment_index: i,
                };
                callback(SegmentEvent::Segment(event));

                i += 1;
                prev_file_path = Some(file_path);
            }
        };
        // 解析流头部，判断流类型
        match header(&bytes) {
            Ok((_i, header)) => {
                debug!("header: {header:#?}");
                info!("Downloading {}...", url);
                // FLV流下载
                let file = LifecycleFile::with_hook(&file_name, "flv", hook);
                httpflv::download(connection, file, segment.clone()).await;
            }
            Err(Err::Incomplete(needed)) => {
                error!("needed: {needed:?}")
            }
            Err(e) => {
                error!("{e}");
                // HLS流下载
                let file = LifecycleFile::with_hook(&file_name, "ts", hook);
                hls::download(&url, &client, file, segment.clone())
                    .await
                    .change_context(AppError::Unknown)?;
            }
        }
        Ok(DownloadStatus::StreamEnded)
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
