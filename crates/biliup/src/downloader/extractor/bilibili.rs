use crate::client::StatelessClient;
use crate::downloader::error::{Error, Result};
use crate::downloader::extractor::{Extension, Site, SiteDefinition};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, REFERER};
use serde_json::Value;
use std::any::Any;

pub struct BiliLive {}

#[async_trait]
impl SiteDefinition for BiliLive {
    fn can_handle_url(&self, url: &str) -> bool {
        regex::Regex::new(r"(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com")
            .unwrap()
            .is_match(url)
    }

    async fn get_site(&self, url: &str, mut client: StatelessClient) -> Result<Site> {
        let rid: u32 = match regex::Regex::new(r"/(\d+)").unwrap().captures(url) {
            Some(captures) => captures[1].parse().unwrap(),
            _ => {
                return Err(Error::Custom(format!("Wrong url: {url}")));
            }
        };
        let mut room_info: Value = client
            .client
            .get(format!(
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom?room_id={rid}"
            ))
            .send()
            .await?
            .json()
            .await?;

        let vid = if room_info["code"] == 0 {
            room_info["data"]["room_info"]["room_id"].take()
        } else {
            return Err(Error::Custom(format!("{}", room_info["message"])));
        };

        if room_info["data"]["room_info"]["live_status"] != 1 {
            return Err(Error::Custom(format!("Not online: {url}")));
        }

        let params = [
            ("room_id", &*vid.to_string()),
            ("qn", "10000"),
            ("platform", "web"),
            ("codec", "0,1"),
            ("protocol", "0,1"),
            ("format", "0,1,2"),
            ("ptype", "8"),
            ("dolby", "5"),
        ];
        let room_play_info: Value = client
            .client
            .get("https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo")
            .query(&params)
            .send()
            .await?
            .json()
            .await?;

        if room_play_info["code"] != 0 {
            return Err(Error::Custom(room_play_info["msg"].to_string()));
        }
        let direct_url = room_play_info["data"]["playurl_info"]["playurl"]["stream"]
            .as_array()
            .and_then(|v| {
                v.iter()
                    .filter_map(|v| v["format"].as_array())
                    .flatten()
                    .find(|v| v["format_name"] == "flv")
            })
            .and_then(|v| {
                let url_info = v["codec"][0]["url_info"]
                    .as_array()
                    .and_then(|info| {
                        info.iter()
                            .find(|i| !i["host"].to_string().contains(".mcdn."))
                    })
                    .unwrap_or(&v["codec"][0]["url_info"][0]);
                if let (Some(host), Some(base_url), Some(extra)) = (
                    url_info["host"].as_str(),
                    v["codec"][0]["base_url"].as_str(),
                    url_info["extra"].as_str(),
                ) {
                    Some(format!("{host}{base_url}{extra}"))
                } else {
                    None
                }
            })
            .ok_or_else(|| Error::Custom(format!("{}", room_play_info)))?;
        let mut header_map = HeaderMap::new();
        header_map.insert(
            REFERER,
            HeaderValue::from_static("https://live.bilibili.com"),
        );
        client.headers.append(
            REFERER,
            HeaderValue::from_static("https://live.bilibili.com"),
        );
        return Ok(Site {
            name: "bilibili",
            title: room_info["data"]["room_info"]["title"]
                .as_str()
                .unwrap()
                .to_string(),
            direct_url,
            extension: Extension::Flv,
            client,
        });
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
