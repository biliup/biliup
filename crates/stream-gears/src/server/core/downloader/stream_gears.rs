use crate::server::core::download_manager::UploaderMessage;
use crate::server::core::downloader::{DownloadStatus, Downloader, SegmentEvent};
use crate::server::infrastructure::context::Worker;
use async_channel::{Sender, bounded};
use async_trait::async_trait;
use axum::http::HeaderMap;
use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use nom::Err;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

// 纯 Rust 的 Segment（代替 PySegment）
#[derive(Clone, Debug, Default)]
pub struct Segment {
    pub time: Option<u64>, // 秒
    pub size: Option<u64>, // 字节
}

// 纯 Rust 的文件名回调
pub type FileNameCallback = Arc<dyn Fn(&str) + Send + Sync + 'static>;

// 若 biliup::downloader::download 接口里用的是这个别名，可以按需对齐
type BiliupCallbackFn = Box<dyn Fn(&str) + Send + Sync + 'static>;

// 具体下载器实现
pub struct StreamGears {
    url: String,
    header_map: HeaderMap,
    file_name: String,
    segment: Segmentable,
    proxy: Option<String>,
    status: Arc<RwLock<DownloadStatus>>,
}

impl StreamGears {
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
            file_name: file_name,
            segment,
            proxy,
            status: Arc::new(RwLock::new(DownloadStatus::Downloading)),
        }
    }
}

#[async_trait]
impl Downloader for StreamGears {
    async fn download(
        &self,
        sender: Sender<UploaderMessage>,
        worker: Arc<Worker>,
    ) -> Result<DownloadStatus, Box<dyn std::error::Error>> {
        let url = self.url.clone();
        let file_name = self.file_name.clone();
        let headers_in = self.header_map.clone();
        let proxy = self.proxy.clone();
        let status = Arc::clone(&self.status);
        // response.read_exact(buf)?;
        // let out = File::create(format!("{}.flv", file_name)).expect("Unable to create file.");
        // let mut writer = BufWriter::new(out);
        // let mut buf = [0u8; 8 * 1024];
        // response.copy_to(&mut writer)?;
        // io::copy(&mut resp, &mut out).expect("Unable to copy the content.");
        let client = StatelessClient::new(self.header_map.clone(), proxy.as_deref());
        let response = client.retryable(&url).await?;
        let mut connection = Connection::new(response);
        // let buf = &mut [0u8; 9];
        let bytes = connection.read_frame(9).await?;
        let (tx, rx) = bounded(16);
        let mut i = 0;
        let hook = move |s: &str| {
            if i == 0 {
                match sender.force_send(UploaderMessage::SegmentEvent(
                    SegmentEvent {
                        file_path: PathBuf::from(s),
                        segment_index: i,
                        start_time: SystemTime::now(),
                        end_time: SystemTime::now(),
                    },
                    rx.clone(),
                    worker.clone(),
                )) {
                    Ok(Some(ret)) => {
                        warn!(SegmentEvent = ?ret, "replace an existing message in the channel");
                    }
                    Err(_) => {}
                    Ok(None) => {}
                };
            } else {
                match tx.force_send(SegmentEvent {
                    file_path: PathBuf::from(s),
                    segment_index: i,
                    start_time: SystemTime::now(),
                    end_time: SystemTime::now(),
                }) {
                    Ok(Some(ret)) => {
                        warn!(SegmentEvent = ?ret, "replace an existing message in the channel");
                    }
                    Err(_) => {}
                    Ok(None) => {}
                }
            }
            i += 1;
            info!(s = s)
        };
        match header(&bytes) {
            Ok((_i, header)) => {
                debug!("header: {header:#?}");
                info!("Downloading {}...", url);
                let file = LifecycleFile::with_hook(&file_name, "flv", hook);
                httpflv::download(connection, file, self.segment.clone()).await;
            }
            Err(Err::Incomplete(needed)) => {
                error!("needed: {needed:?}")
            }
            Err(e) => {
                error!("{e}");
                let file = LifecycleFile::with_hook(&file_name, "ts", hook);
                hls::download(&url, &client, file, self.segment.clone()).await?;
            }
        }
        Ok(DownloadStatus::StreamEnded)
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 仅发出取消信号并更新状态。
        // 如上所述，如果底层下载函数不支持取消，这里不能真正中断正在进行的下载。
        todo!()
    }

    async fn get_status(&self) -> DownloadStatus {
        todo!()
    }
}
