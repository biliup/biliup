use crate::server::core::downloader::SegmentEvent;
use crate::server::core::monitor::{Monitor, RoomsHandle};
use crate::server::core::plugin::{DownloadPlugin, StreamInfo};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use crate::uploader::UploadLine;
use anyhow::Context;
use async_channel::{Receiver, Sender, bounded};
use biliup::bilibili::{BiliBili, Credit, Studio, Video};
use biliup::client::StatelessClient;
use biliup::credential::login_by_cookies;
use biliup::error::Kind;
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
use tracing::{error, info};

pub struct DownloadManager {
    pub monitor: Mutex<Option<Arc<Monitor>>>,
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    actor_handle: Arc<ActorHandle>,
}

impl DownloadManager {
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

    pub(crate) fn ensure_monitor(&self) -> Arc<Monitor> {
        // Monitor::new(self.plugin.name(), url)
        self.monitor
            .lock()
            .unwrap()
            .get_or_insert_with(|| {
                Arc::new(Monitor::new(self.plugin.clone(), self.actor_handle.clone()))
            })
            .clone()
    }

    pub fn matches(&self, url: &str) -> bool {
        self.plugin.matches(url)
    }
}

impl fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DownloadManager [{:?}]", self.monitor)
    }
}

pub struct DActor {
    receiver: Receiver<DownloaderMessage>,
    sender: Sender<UploaderMessage>,
}

impl DActor {
    pub fn new(receiver: Receiver<DownloaderMessage>, sender: Sender<UploaderMessage>) -> Self {
        Self { receiver, sender }
    }

    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: DownloaderMessage) {
        match msg {
            DownloaderMessage::Start(plugin, stream_info, room, rooms_handle) => {
                let downloader = plugin.create_downloader(&stream_info, &room).await.unwrap();

                *room.downloader_status.write().unwrap() = WorkerStatus::Working;

                match downloader.download(self.sender.clone(), room.clone()).await {
                    Ok(status) => {
                        println!("Download completed with status: {:?}", status);
                    }
                    Err(err) => {
                        error!("download error: {:?}", err);
                    }
                };
                *room.downloader_status.write().unwrap() = WorkerStatus::Idle;
                rooms_handle.toggle(room).await;
            }
        }
    }
}
pub struct UActor {
    receiver: Receiver<UploaderMessage>,
}

impl UActor {
    pub fn new(receiver: Receiver<UploaderMessage>) -> Self {
        Self { receiver }
    }

    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    async fn handle_message(&mut self, msg: UploaderMessage) {
        match msg {
            UploaderMessage::SegmentEvent(event, rx, worker) => {
                let Some(upload_config) = worker.get_upload_config().await.unwrap() else {
                    return;
                };
                let cookie_file = upload_config
                    .user_cookie
                    .unwrap_or("cookies.json".to_string());
                let bilibili = login_by_cookies(&cookie_file, None).await;
                let Ok(bilibili) = bilibili.map_err(|e| error!(e=?e)) else {
                    return;
                };
                let config = worker.get_config().await.unwrap();
                let line = config.lines;
                let client = StatelessClient::default();
                let line = match line.as_str() {
                    "bda2" => line::bda2(),
                    "qn" => line::qn(),
                    "bda" => line::bda(),
                    "tx" => line::tx(),
                    "txa" => line::txa(),
                    "bldsa" => line::bldsa(),
                    "alia" => line::alia(),
                    _ => Probe::probe(&client.client).await.unwrap_or_default(),
                };
                let mut videos = Vec::new();
                if let Ok(video) = UActor::upload_file(
                    &bilibili,
                    config.threads as usize,
                    &client,
                    &line,
                    &*event.file_path,
                )
                .await
                {
                    videos.push(video);
                }
                while let Ok(se) = rx.recv().await {
                    if let Ok(video) = UActor::upload_file(
                        &bilibili,
                        config.threads as usize,
                        &client,
                        &line,
                        &*event.file_path,
                    )
                    .await
                    {
                        videos.push(video);
                    }
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

                if !studio.cover.is_empty() {
                    if let Ok(c) = &std::fs::read(&studio.cover).map_err(|e| error!(e=?e)) {
                        if let Ok(url) = bilibili.cover_up(c).await.map_err(|e| error!(e=?e)) {
                            studio.cover = url;
                        }
                    }
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
        let video_file = VideoFile::new(&video_path).change_context(AppError::Unknown)?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let uploader = line
            .pre_upload(&bilibili, video_file)
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

pub struct ActorHandle {
    download_semaphore: u32,
    update_semaphore: u32,
    pub up_sender: Sender<UploaderMessage>,
    pub down_sender: Sender<DownloaderMessage>,
    d_kills: Vec<JoinHandle<()>>,
    u_kills: Vec<JoinHandle<()>>,
}

impl ActorHandle {
    pub fn new(download_semaphore: u32, update_semaphore: u32) -> Self {
        let (up_tx, up_rx) = bounded(16);
        let (down_tx, down_rx) = bounded(1);
        let mut d_kills = Vec::new();
        let mut u_kills = Vec::new();
        for _ in 0..download_semaphore {
            let mut d_actor = DActor::new(down_rx.clone(), up_tx.clone());
            let d_kill = tokio::spawn(async move { d_actor.run().await });
            d_kills.push(d_kill)
        }
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

#[derive(Debug)]
pub enum UploaderMessage {
    SegmentEvent(SegmentEvent, Receiver<SegmentEvent>, Arc<Worker>),
}

pub enum DownloaderMessage {
    Start(
        Arc<dyn DownloadPlugin + Send + Sync>,
        StreamInfo,
        Arc<Worker>,
        Arc<RoomsHandle>,
    ),
}
