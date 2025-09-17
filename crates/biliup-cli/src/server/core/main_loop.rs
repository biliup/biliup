use crate::server::core::StreamStatus;
use crate::server::core::live_streamers::{DynLiveStreamersService, LiveStreamerDto, Videos};
use crate::server::core::upload_actor::UploadActorHandle;
use crate::server::core::util::{Cycle, logging_spawn};
use anyhow::Result;
use biliup::client::StatelessClient;
use biliup::downloader::extractor::Site;
use biliup::downloader::util::Segmentable;
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::{Receiver, channel};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// This struct is used by client actors to send messages to the main loop. The
/// message type is `ToServer`.
#[derive(Clone, Debug)]
pub struct ServerHandle {
    chan: Sender<ToMain>,
}
impl ServerHandle {
    pub async fn send(&mut self, msg: ToMain) {
        if self.chan.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }
}

/// The message type used when a client actor sends messages to the main loop.
pub enum ToMain {
    NewRecording(Site, Cycle<StreamStatus>),
    FileClosed(Videos),
    // FatalError(io::Error),
}

pub fn spawn_main_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);

    let handle = ServerHandle { chan: send };

    let join = tokio::spawn(async move {
        let res = main_loop(recv).await;
        match res {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Oops {}.", err);
            }
        }
    });

    (handle, join)
}

async fn main_loop(mut recv: Receiver<ToMain>) -> Result<()> {
    while let Some(msg) = recv.recv().await {
        match msg {
            ToMain::NewRecording(_site, _task) => {}
            ToMain::FileClosed(_) => {}
        }
    }

    Ok(())
}

// async fn recording(
//     url: &str,
//     mut site: Site,
//     task: Cycle<StreamStatus>,
//     client: StatelessClient,
//     live_streamers_service: DynLiveStreamersService,
// ) {
//     println!("Idle\n {url} \n{site}");
//     let (filename, split_size, split_time) = if let Ok(LiveStreamerDto {
//         filename,
//         split_size,
//         split_time,
//         ..
//     }) =
//         live_streamers_service.get_streamer_by_url(url).await
//     {
//         (filename, split_size, split_time.map(Duration::from_secs))
//     } else {
//         ("./video/%Y-%m-%d/%H_%M_%S{title}".to_string(), None, None)
//     };
//
//     logging_spawn({
//         // let client = client.clone();
//         let url = url.to_string();
//         let task = task.clone();
//         // let live_streamers_service = live_streamers_service.clone();
//         async move {
//             let hook = live_streamers_service
//                 .get_studio_by_url(&url)
//                 .await
//                 .unwrap_or_default()
//                 .map(|studio| -> Box<dyn Fn(&str) + Send + Sync> {
//                     let handle = UploadActorHandle::new(client, studio);
//                     Box::new(move |file_name| {
//                         if let Ok(metadata) =
//                             std::fs::metadata(file_name).map_err(|err| error!("{}", err))
//                         {
//                             if metadata.len() > 10 * 1024 * 1024 {
//                                 info!("开始上传: {}", file_name);
//                                 handle.send_file_path(file_name);
//                             }
//                         }
//                     })
//                 });
//             if hook.is_none() {
//                 debug!(url = %url, "upload template not set.");
//             }
//             let segmentable = Segmentable::new(split_time, split_size);
//             // let segmentable = Segmentable::new( None, Some(16*1024*1024));
//             site.download(&filename, segmentable, hook).await?;
//             task.change(&url, StreamStatus::Idle);
//             Ok::<_, Box<dyn Error + Send + Sync>>(())
//         }
//     });
//     task.change(url, StreamStatus::Working);
// }
