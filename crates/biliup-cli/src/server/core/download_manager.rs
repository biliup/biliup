use crate::server::common::util::Recorder;
use crate::server::core::downloader::{Downloader, SegmentEvent};
use crate::server::core::monitor::{Monitor, RoomsHandle};
use crate::server::core::plugin::{DownloadPlugin, StreamInfo};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, Worker, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process_video;
use async_channel::{Receiver, Sender, bounded};
use biliup::bilibili::{BiliBili, Studio, Video};
use biliup::client::StatelessClient;
use biliup::credential::login_by_cookies;
use biliup::uploader::line::{Line, Probe};
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, line};
use core::fmt;
use error_stack::ResultExt;
use futures::StreamExt;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 监控器实例（可选，使用Mutex保护）
    pub monitor: Mutex<Option<Arc<Monitor>>>,
    /// 下载插件
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    /// Actor处理器
    actor_handle: Arc<ActorHandle>,
}

impl DownloadManager {
    /// 创建新的下载管理器实例
    /// 
    /// # 参数
    /// * `plugin` - 下载插件实现
    /// * `actor_handle` - Actor处理器
    pub fn new(
        plugin: impl DownloadPlugin + Send + Sync + 'static,
        actor_handle: Arc<ActorHandle>,
    ) -> Self {
        Self {
            monitor: Mutex::new(None),
            plugin: Arc::new(plugin),
            actor_handle,
        }
    }

    /// 确保监控器存在，如果不存在则创建新的
    /// 
    /// # 返回
    /// 返回监控器的Arc引用
    pub fn ensure_monitor(&self) -> Arc<Monitor> {
        self.monitor
            .lock()
            .unwrap()
            .get_or_insert_with(|| {
                Arc::new(Monitor::new(self.plugin.clone(), self.actor_handle.clone()))
            })
            .clone()
    }

    /// 检查URL是否匹配此下载管理器的插件
    /// 
    /// # 参数
    /// * `url` - 要检查的URL
    /// 
    /// # 返回
    /// 如果URL匹配返回true，否则返回false
    pub fn matches(&self, url: &str) -> bool {
        self.plugin.matches(url)
    }
}

impl fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DownloadManager [{:?}]", self.monitor)
    }
}

/// 下载Actor
/// 负责处理下载相关的消息和任务
pub struct DActor {
    /// 下载消息接收器
    receiver: Receiver<DownloaderMessage>,
    /// 上传消息发送器
    sender: Sender<UploaderMessage>,
}

impl DActor {
    /// 创建新的下载Actor实例
    pub fn new(receiver: Receiver<DownloaderMessage>, sender: Sender<UploaderMessage>) -> Self {
        Self { receiver, sender }
    }

    /// 运行Actor主循环，处理接收到的消息
    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            if let Err(e) = self.handle_message(msg).await {
                error!("Error handling message: {}", e);
            }
        }
    }

    /// 处理下载消息
    /// 
    /// # 参数
    /// * `msg` - 要处理的下载消息
    async fn handle_message(&mut self, msg: DownloaderMessage) -> AppResult<()> {
        match msg {
            DownloaderMessage::Start(plugin, stream_info, ctx, rooms_handle) => {
                // 获取配置和主播信息
                let config = ctx.worker.get_config();
                let streamer = ctx.worker.get_streamer();
                // 确定文件格式后缀
                let suffix = streamer
                    .format
                    .unwrap_or_else(|| stream_info.suffix.clone());
                // 创建录制器
                let recorder = Recorder::new(
                    streamer.filename_prefix,
                    &streamer.remark,
                    &stream_info.title,
                    &suffix,
                );
                // 创建下载器实例
                let downloader = plugin
                    .create_downloader(&stream_info, config, recorder)
                    .await;

                // 更新工作器状态为工作中
                ctx.worker
                    .change_status(Stage::Download, WorkerStatus::Working);
                // 初始化弹幕客户端（如果存在）
                let mut danmaku_client = None;
                if let Some(danmaku) = ctx.extension.get::<Arc<dyn Downloader>>() {
                    let _ = danmaku
                        .download(Box::new(|_| {}))
                        .await
                        .inspect_err(|e| error!(e=?e));
                    danmaku_client = Some(danmaku.clone())
                }

                // 创建分段事件处理通道
                let (tx, rx) = bounded(16);
                let hook = {
                    let room = ctx.worker.clone();
                    let sender = self.sender.clone();
                    let danmaku_client = danmaku_client.clone();
                    move |event: SegmentEvent| {
                        // 如果有弹幕客户端，触发滚动保存
                        if let Some(danmaku) = &danmaku_client {
                            let _ = danmaku
                                .rolling(&event.file_path.display().to_string())
                                .inspect_err(|e| error!(e));
                        }
                        // 处理分段事件
                        if event.segment_index == 0 {
                            // 第一个分段，发送到上传器
                            match sender.force_send(UploaderMessage::SegmentEvent(
                                event,
                                rx.clone(),
                                room.clone(),
                            )) {
                                Ok(Some(ret)) => {
                                    warn!(SegmentEvent = ?ret, "replace an existing message in the channel");
                                }
                                Err(_) => {}
                                Ok(None) => {}
                            };
                        } else {
                            // 后续分段，发送到内部通道
                            match tx.clone().force_send(event) {
                                Ok(Some(ret)) => {
                                    warn!(SegmentEvent = ?ret, "replace an existing message in the channel");
                                }
                                Err(_) => {}
                                Ok(None) => {}
                            }
                        }
                    }
                };
                // 开始下载
                match downloader.download(Box::new(hook)).await {
                    Ok(status) => {
                        println!("Download completed with status: {:?}", status);
                    }
                    Err(err) => {
                        error!("download error: {:?}", err);
                    }
                };
                // 停止弹幕客户端
                if let Some(danmaku) = &danmaku_client {
                    let _ = danmaku.stop().await.inspect_err(|e| error!(e));
                }
                // 更新工作器状态为空闲
                ctx.worker
                    .change_status(Stage::Download, WorkerStatus::Idle);
                // 切换房间状态
                rooms_handle.toggle(ctx.worker).await;
                Ok(())
            }
        }
    }
}
/// 上传Actor
/// 负责处理上传相关的消息和任务
pub struct UActor {
    /// 上传消息接收器
    receiver: Receiver<UploaderMessage>,
}

impl UActor {
    /// 创建新的上传Actor实例
    pub fn new(receiver: Receiver<UploaderMessage>) -> Self {
        Self { receiver }
    }

    /// 运行Actor主循环，处理接收到的消息
    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    /// 处理上传消息
    /// 
    /// # 参数
    /// * `msg` - 要处理的上传消息
    async fn handle_message(&mut self, msg: UploaderMessage) {
        match msg {
            UploaderMessage::SegmentEvent(event, rx, worker) => {
                let Some(upload_config) = worker.get_upload_config() else {
                    return;
                };
                let cookie_file = upload_config
                    .user_cookie
                    .unwrap_or("cookies.json".to_string());
                let bilibili = login_by_cookies(&cookie_file, None).await;
                let Ok(bilibili) = bilibili.inspect_err(|e| error!(e=?e)) else {
                    return;
                };
                let config = worker.get_config();
                let client = worker.client.clone();
                let line = config.lines;
                let line = match line.as_str() {
                    "bda2" => line::bda2(),
                    "bda" => line::bda(),
                    "tx" => line::tx(),
                    "txa" => line::txa(),
                    "bldsa" => line::bldsa(),
                    "alia" => line::alia(),
                    _ => Probe::probe(&client.client).await.unwrap_or_default(),
                };
                let mut video_paths = Vec::new();
                let mut videos = Vec::new();
                if let Ok(video) = UActor::upload_file(
                    &bilibili,
                    config.threads as usize,
                    &client,
                    &line,
                    &event.file_path,
                )
                .await
                .inspect_err(|e| error!(e=?e))
                {
                    videos.push(video);
                }
                video_paths.push(event.file_path);

                while let Ok(se) = rx.recv().await {
                    if let Ok(video) = UActor::upload_file(
                        &bilibili,
                        config.threads as usize,
                        &client,
                        &line,
                        &se.file_path,
                    )
                    .await
                    .map_err(|e| error!(e=?e))
                    {
                        videos.push(video);
                    }
                    video_paths.push(se.file_path);
                }

                // let mut desc_v2 = Vec::new();
                // for credit in desc_v2_credit {
                //     desc_v2.push(Credit {
                //         type_id: credit.type_id,
                //         raw_text: credit.raw_text,
                //         biz_id: credit.biz_id,
                //     });
                // }

                let mut studio: Studio = Studio::builder()
                    .desc(upload_config.description.unwrap_or_default())
                    .dtime(upload_config.dtime)
                    .copyright(upload_config.copyright.unwrap_or(2))
                    .cover(upload_config.cover_path.unwrap_or_default())
                    .dynamic(upload_config.dynamic.unwrap_or_default())
                    .source(upload_config.copyright_source.unwrap_or_default())
                    .tag(upload_config.tags.join(","))
                    .tid(upload_config.tid.unwrap_or(171))
                    .title(upload_config.title.unwrap_or_default())
                    .videos(videos)
                    .dolby(upload_config.dolby.unwrap_or_default())
                    // .lossless_music(upload_config.)
                    .no_reprint(upload_config.no_reprint.unwrap_or_default())
                    .charging_pay(upload_config.charging_pay.unwrap_or_default())
                    .up_close_reply(upload_config.up_close_reply.unwrap_or_default())
                    .up_selection_reply(upload_config.up_selection_reply.unwrap_or_default())
                    .up_close_danmu(upload_config.up_close_danmu.unwrap_or_default())
                    .desc_v2(None)
                    .extra_fields(
                        serde_json::from_str(&upload_config.extra_fields.unwrap_or_default())
                            .unwrap_or_default(),
                    )
                    .build();

                if !studio.cover.is_empty()
                    && let Ok(c) = &std::fs::read(&studio.cover).map_err(|e| error!(e=?e))
                    && let Ok(url) = bilibili.cover_up(c).await.map_err(|e| error!(e=?e))
                {
                    studio.cover = url;
                };

                let submit = match config.submit_api {
                    Some(submit) => SubmitOption::from_str(&submit).unwrap_or(SubmitOption::App),
                    _ => SubmitOption::App,
                };

                let submit_result = match submit {
                    SubmitOption::BCutAndroid => {
                        bilibili.submit_by_bcut_android(&studio, None).await
                    }
                    _ => bilibili.submit_by_app(&studio, None).await,
                };
                info!(submit_result=?submit_result);
                if submit_result.is_ok() {
                    info!("Submit successful");
                    let streamer = worker.get_streamer();
                    if let Some(post_processor) = streamer.postprocessor {

                        let _ = process_video(&(video_paths.iter().map(|p|p.as_path()).collect::<Vec<_>>()), &post_processor)
                            .await
                            .map_err(|e| error!(e=?e));

                    }
                }
            }
        }
    }

    async fn upload_file(
        bilibili: &BiliBili,
        limit: usize,
        client: &StatelessClient,
        line: &Line,
        video_path: &Path,
    ) -> AppResult<Video> {
        println!(
            "{:?}",
            video_path
                .canonicalize()
                .change_context(AppError::Unknown)?
                .to_str()
        );
        info!("{line:?}");
        let video_file = VideoFile::new(video_path).change_context(AppError::Unknown)?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let uploader = line
            .pre_upload(bilibili, video_file)
            .await
            .change_context(AppError::Unknown)?;

        let instant = Instant::now();

        let video = uploader
            .upload(client.clone(), limit, |vs| {
                vs.map(|vs| {
                    let chunk = vs?;
                    let len = chunk.len();
                    Ok((chunk, len))
                })
            })
            .await
            .change_context(AppError::Unknown)?;
        let t = instant.elapsed().as_millis();
        info!(
            "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
            t as f64 / 1000.,
            total_size as f64 / 1000. / t as f64
        );
        Ok(video)
    }
}

/// Actor处理器
/// 管理下载和上传Actor的生命周期
pub struct ActorHandle {
    /// 下载信号量数量
    download_semaphore: u32,
    /// 上传信号量数量
    update_semaphore: u32,
    /// 上传消息发送器
    pub up_sender: Sender<UploaderMessage>,
    /// 下载消息发送器
    pub down_sender: Sender<DownloaderMessage>,
    /// 下载Actor任务句柄列表
    d_kills: Vec<JoinHandle<()>>,
    /// 上传Actor任务句柄列表
    u_kills: Vec<JoinHandle<()>>,
}

impl ActorHandle {
    /// 创建新的Actor处理器实例
    /// 
    /// # 参数
    /// * `download_semaphore` - 下载Actor数量
    /// * `update_semaphore` - 上传Actor数量
    pub fn new(download_semaphore: u32, update_semaphore: u32) -> Self {
        // 创建消息通道
        let (up_tx, up_rx) = bounded(16);
        let (down_tx, down_rx) = bounded(1);
        let mut d_kills = Vec::new();
        let mut u_kills = Vec::new();
        // 创建下载Actor
        for _ in 0..download_semaphore {
            let mut d_actor = DActor::new(down_rx.clone(), up_tx.clone());
            let d_kill = tokio::spawn(async move { d_actor.run().await });
            d_kills.push(d_kill)
        }
        // 创建上传Actor
        for _ in 0..update_semaphore {
            let mut u_actor = UActor::new(up_rx.clone());
            let u_kill = tokio::spawn(async move { u_actor.run().await });
            u_kills.push(u_kill)
        }

        Self {
            download_semaphore,
            update_semaphore,
            up_sender: up_tx,
            down_sender: down_tx,
            d_kills,
            u_kills,
        }
    }
}

/// 上传消息枚举
/// 定义上传Actor可以处理的消息类型
#[derive(Debug)]
pub enum UploaderMessage {
    /// 分段事件消息，包含事件、接收器和工作器
    SegmentEvent(SegmentEvent, Receiver<SegmentEvent>, Arc<Worker>),
}

/// 下载消息枚举
/// 定义下载Actor可以处理的消息类型
pub enum DownloaderMessage {
    /// 开始下载消息，包含插件、流信息、上下文和房间句柄
    Start(
        Arc<dyn DownloadPlugin + Send + Sync>,
        StreamInfo,
        Context,
        Arc<RoomsHandle>,
    ),
}

impl Drop for ActorHandle {
    fn drop(&mut self) {
        // 发送端随 ActorHandle 一起被 drop，会关闭通道（如果没有其他 sender 克隆）。
        // 为避免 tokio 任务在后台“挂着”，这里直接 abort。
        for h in &self.d_kills {
            h.abort();
        }
        for h in &self.u_kills {
            h.abort();
        }
    }
}
