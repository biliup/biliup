use crate::client::StatelessClient;
use crate::downloader::error::Error;
use crate::downloader::extractor::{Extension, Site, SiteDefinition};
use async_trait::async_trait;
use md5::{Digest, Md5};
use tracing::info;

use std::any::Any;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DouyuLive;

#[async_trait]
impl SiteDefinition for DouyuLive {
    fn can_handle_url(&self, url: &str) -> bool {
        regex::Regex::new(r"(?:https?://)?(?:(?:www|m)\.)?douyu\.com")
            .unwrap()
            .is_match(url)
    }

    async fn get_site(
        &self,
        url: &str,
        client: StatelessClient,
    ) -> crate::downloader::error::Result<Site> {
        let text = client.client.get(url).send().await?.text().await?;
        let patterns = [
            r"\$ROOM\.room_id\s*=\s*(\d+)",
            r"room_id\s*=\s*(\d+)",
            r#""room_id.?":(\d+)"#,
            r"data-onlineid=(\d+)",
        ];

        // Compile each pattern independently.
        let room_id = patterns
            .iter()
            .map(|pat| regex::Regex::new(pat).unwrap())
            .find_map(|pat| pat.captures(&text))
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Error::Custom(format!("Wrong url: {url}")))?;

        let room_info: serde_json::Value = client
            .client
            .get(format!("https://www.douyu.com/betard/{room_id}"))
            .send()
            .await?
            .json()
            .await?;

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let mut hasher = Md5::new();
        hasher.update(format!("{room_id}{time}"));
        let sign = format!("{:x}", hasher.finalize());
        let data = [
            ("did", "10000000000000000000000000001501"),
            ("rid", &room_id),
        ];
        info!("{room_id}");
        let result: serde_json::Value = client
            .client
            .post(format!(
                "https://playweb.douyucdn.cn/lapi/live/hlsH5Preview/{room_id}"
            ))
            .header("rid", &room_id)
            .header("time", time.to_string())
            .header("auth", sign)
            .form(&data)
            .send()
            .await?
            .json()
            .await?;
        if result["error"] == 0 {
            if let Some(key) = regex::Regex::new(r"(\d{1,8}[0-9a-zA-Z]+)_?\d{0,4}(/playlist|.m3u8)")
                .unwrap()
                .captures(&result["data"]["rtmp_live"].to_string())
            {
                return Ok(Site {
                    name: "douyu",
                    title: room_info
                        .get("room")
                        .and_then(|room| room.get("room_name"))
                        .and_then(|name| name.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    direct_url: format!("https://hw-tct.douyucdn.cn/live/{}.flv?uuid=", &key[1]),
                    extension: Extension::Flv,
                    client,
                });
            }
        }
        Err(Error::Custom(result.to_string()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
