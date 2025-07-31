use biliup::client::StatelessClient;

use crate::server::core::StreamStatus;
use crate::server::core::live_streamers::{DynLiveStreamersService, LiveStreamerDto};
use crate::server::core::upload_actor::UploadActorHandle;
use crate::server::core::util::{AnyMap, Cycle, logging_spawn};
use biliup::downloader::extractor::{SiteDefinition, find_extractor};
use biliup::downloader::util::Segmentable;

use indexmap::indexmap;

use std::collections::HashMap;
use std::error::Error;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::log::info;
use tracing::{debug, error};

async fn start_monitor(
    task: Cycle<StreamStatus>,
    extractor: &(dyn SiteDefinition + Send + Sync),
    client: StatelessClient,
    live_streamers_service: DynLiveStreamersService,
) {
    let n = &mut 0;
    loop {
        let (url, status) = task.get(n);
        if status != StreamStatus::Working {
            task.change(&url, StreamStatus::Inspecting);
        }
        match (extractor.get_site(&url, client.clone()).await, status) {
            (Ok(mut site), StreamStatus::Idle | StreamStatus::Inspecting) => {
                println!("Idle\n {url} \n{site}");
                let (filename, split_size, split_time) = if let Ok(LiveStreamerDto {
                    filename,
                    split_size,
                    split_time,
                    ..
                }) =
                    live_streamers_service.get_streamer_by_url(&url).await
                {
                    (filename, split_size, split_time.map(Duration::from_secs))
                } else {
                    ("./video/%Y-%m-%d/%H_%M_%S{title}".to_string(), None, None)
                };
                let live_streamers_service = live_streamers_service.clone();
                {
                    let client = client.clone();
                    let url = url.clone();
                    let task = task.clone();
                    logging_spawn(async move {
                        let hook = live_streamers_service
                            .get_studio_by_url(&url)
                            .await
                            .unwrap_or_default()
                            .map(|studio| -> Box<dyn Fn(&str) + Send> {
                                let handle = UploadActorHandle::new(client, studio);
                                Box::new(move |file_name| {
                                    if let Ok(metadata) = std::fs::metadata(file_name)
                                        .map_err(|err| error!("{}", err))
                                    {
                                        if metadata.len() > 10 * 1024 * 1024 {
                                            info!("开始上传: {}", file_name);
                                            handle.send_file_path(file_name);
                                        }
                                    }
                                })
                            });
                        if hook.is_none() {
                            debug!(url = %url, "upload template not set.");
                        }

                        let segmentable = Segmentable::new(split_time, split_size);
                        // let segmentable = Segmentable::new( None, Some(16*1024*1024));
                        site.download(&filename, segmentable, hook).await?;
                        task.change(&url, StreamStatus::Idle);
                        Ok::<_, Box<dyn Error + Send + Sync>>(())
                    });
                }
                task.change(&url, StreamStatus::Working);
            }
            (Ok(_site), StreamStatus::Pending) => {
                println!("Pending");
            }
            (Ok(_site), StreamStatus::Working) => {
                debug!("Working");
            }
            (Err(e), _) => {
                task.change(&url, StreamStatus::Idle);
                debug!(url, "{e}")
            }
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

#[derive(Clone)]
struct DownloadActor {
    live_streamers_service: DynLiveStreamersService,
    client: StatelessClient,
}

impl DownloadActor {
    fn new(live_streamers_service: DynLiveStreamersService, client: StatelessClient) -> Self {
        Self {
            live_streamers_service,
            client,
        }
    }

    fn run(
        &mut self,
        list: Vec<LiveStreamerDto>,
        extensions: StreamActorMap,
        // client: StatelessClient,
    ) {
        for streamer in list {
            // let Some(extractor) = find_extractor(&streamer.url) else { continue; };
            let mut guard = extensions.write().unwrap();
            self.add_streamer(guard.deref_mut(), streamer.url)
        }
        println!("{:?}", extensions);
    }

    fn add_streamer(
        &self,
        map: &mut AnyMap<(Cycle<StreamStatus>, JoinHandle<()>)>,
        url: String,
        // client: StatelessClient,
    ) {
        let Some(extractor) = find_extractor(&url) else {
            return;
        };
        let _entry = map
            .entry(extractor.as_any().type_id())
            .and_modify(|(cy, _)| cy.insert(url.clone(), StreamStatus::Idle))
            .or_insert_with(|| {
                let cycle = Cycle::new(indexmap![url => StreamStatus::Idle]);
                let task = cycle.clone();
                let client = self.client.clone();
                let live_streamers_service = self.live_streamers_service.clone();
                let handle = tokio::spawn(async move {
                    start_monitor(task, extractor, client, live_streamers_service).await
                });
                (cycle, handle)
            });
    }
}

type StreamActorMap = Arc<RwLock<AnyMap<(Cycle<StreamStatus>, JoinHandle<()>)>>>;

#[derive(Clone)]
pub struct DownloadActorHandle {
    platform_map: StreamActorMap,
    // client: StatelessClient,
    actor: DownloadActor,
}

impl DownloadActorHandle {
    pub fn new(
        list: Vec<LiveStreamerDto>,
        client: StatelessClient,
        live_streamers_service: DynLiveStreamersService,
    ) -> Self {
        let mut actor = DownloadActor::new(live_streamers_service, client);
        let platform_map = Arc::new(RwLock::new(HashMap::default()));
        let platform = Arc::clone(&platform_map);
        // let client_c = client.clone();
        actor.run(list, platform);
        Self {
            platform_map,
            actor,
        }
    }

    pub fn add_streamer(&self, url: &str) {
        self.actor.add_streamer(
            self.platform_map.write().unwrap().deref_mut(),
            url.to_string(),
            // self.client.clone(),
        );
    }

    pub fn get_streamers(&self) -> HashMap<String, StreamStatus> {
        let read_guard = self.platform_map.read().unwrap();
        let mut map = HashMap::new();
        for (key, val) in read_guard.iter() {
            map.extend(val.0.get_all());
        }
        map
    }

    pub fn update_streamer(&self, url: &str) {
        self.remove_streamer(url);
        self.add_streamer(url);
    }

    pub fn remove_streamer(&self, url: &str) {
        find_extractor(url).and_then(|extractor| {
            self.platform_map
                .read()
                .unwrap()
                .get(&extractor.as_any().type_id())
                .and_then(|(cy, join_handle)| {
                    let mut guard = cy.write();
                    if guard.len() <= 1 {
                        join_handle.abort()
                    }
                    guard.remove(url)
                })
        });
    }
}
