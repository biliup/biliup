use crate::server::core::downloader::{DownloadStatus, Downloader, SegmentEvent};
use crate::server::errors::{AppError, AppResult};
use async_trait::async_trait;
use axum::http::HeaderMap;
use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use error_stack::ResultExt;
use nom::Err;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tracing::{debug, error, info};

/// Stream-gears下载器实现
/// 使用stream-gears库进行直播流下载
pub struct StreamGears {
    /// 流URL
    url: String,
    /// HTTP请求头
    header_map: HeaderMap,
    /// 文件名
    file_name: String,
    /// 分段配置
    segment: Segmentable,
    /// 代理设置（可选）
    proxy: Option<String>,
    /// 下载状态
    status: Arc<RwLock<DownloadStatus>>,
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
    pub fn new(
        url: &str,
        header_map: HeaderMap,
        file_name: String,
        segment: Segmentable,
        proxy: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            header_map,
            file_name,
            segment,
            proxy,
            status: Arc::new(RwLock::new(DownloadStatus::Downloading)),
        }
    }
}

#[async_trait]
impl Downloader for StreamGears {
    /// 开始下载流
    /// 
    /// # 参数
    /// * `callback` - 分段完成时的回调函数
    async fn download(
        &self,
        callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus> {
        let url = self.url.clone();
        let file_name = self.file_name.clone();
        let _headers_in = self.header_map.clone();
        let proxy = self.proxy.clone();
        let _status = Arc::clone(&self.status);
        
        // 创建HTTP客户端
        let client = StatelessClient::new(self.header_map.clone(), proxy.as_deref());
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
        let mut i = 0;
        // 创建分段回调钩子
        let hook = move |s: &str| {
            i += 1;
            let event = SegmentEvent {
                file_path: PathBuf::from(s),
                segment_index: i,
                start_time: SystemTime::now(),
                end_time: SystemTime::now(),
            };
            callback(event);
        };

        // 解析流头部，判断流类型
        match header(&bytes) {
            Ok((_i, header)) => {
                debug!("header: {header:#?}");
                info!("Downloading {}...", url);
                // FLV流下载
                let file = LifecycleFile::with_hook(&file_name, "flv", hook);
                httpflv::download(connection, file, self.segment.clone()).await;
            }
            Err(Err::Incomplete(needed)) => {
                error!("needed: {needed:?}")
            }
            Err(e) => {
                error!("{e}");
                // HLS流下载
                let file = LifecycleFile::with_hook(&file_name, "ts", hook);
                hls::download(&url, &client, file, self.segment.clone())
                    .await
                    .change_context(AppError::Unknown)?;
            }
        }
        Ok(DownloadStatus::StreamEnded)
    }

    /// 停止下载
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 仅发出取消信号并更新状态
        // 如果底层下载函数不支持取消，这里不能真正中断正在进行的下载
        todo!()
    }
}
