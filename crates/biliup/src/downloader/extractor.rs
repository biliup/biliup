use crate::downloader;
use crate::downloader::httpflv::Connection;
use crate::downloader::util::{LifecycleFile, Segmentable};
use crate::downloader::{hls, httpflv};
use async_trait::async_trait;
use reqwest::header::{ACCEPT_ENCODING, HeaderValue};
use std::any::Any;
use std::fmt::{Display, Formatter};
use tracing::info;

use crate::client::StatelessClient;

mod bilibili;
mod douyu;
mod huya;

const EXTRACTORS: [&(dyn SiteDefinition + Send + Sync); 3] = [
    &bilibili::BiliLive {},
    &huya::HuyaLive {},
    &douyu::DouyuLive,
];

#[async_trait]
pub trait SiteDefinition {
    // true, if this site can handle <url>.
    fn can_handle_url(&self, url: &str) -> bool;

    async fn get_site(&self, url: &str, client: StatelessClient) -> super::error::Result<Site>;

    fn as_any(&self) -> &dyn Any;
}

pub struct Site {
    pub name: &'static str,
    pub title: String,
    pub direct_url: String,
    extension: Extension,
    client: StatelessClient,
}

impl Display for Site {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "Title: {}", self.title)?;
        write!(f, "Direct url: {}", self.direct_url)
    }
}

pub enum Extension {
    Flv,
    Ts,
}

pub type CallbackFn<'a> = Box<dyn FnMut(&str) + Send + Sync + 'a>;

impl Site {
    pub async fn download(
        &mut self,
        fmt_file_name: &str,
        segment: Segmentable,
        hook: Option<CallbackFn<'_>>,
    ) -> downloader::error::Result<()> {
        let fmt_file_name = fmt_file_name.replace("{title}", &self.title);
        self.client
            .headers
            .append(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
        info!("{}", self);
        match self.extension {
            Extension::Flv => {
                let file = LifecycleFile::new(&fmt_file_name, "flv");
                let response = self.client.retryable(&self.direct_url).await?;
                let mut connection = Connection::new(response);
                connection.read_frame(9).await?;
                httpflv::parse_flv(connection, file, segment).await?
            }
            Extension::Ts => {
                let file = LifecycleFile::new(&fmt_file_name, "ts");
                hls::download(&self.direct_url, &self.client, file, segment).await?
            }
        }
        Ok(())
    }
}

pub fn find_extractor(url: &str) -> Option<&'static (dyn SiteDefinition + Send + Sync)> {
    EXTRACTORS
        .into_iter()
        .find(|&extractor| extractor.can_handle_url(url))
}
