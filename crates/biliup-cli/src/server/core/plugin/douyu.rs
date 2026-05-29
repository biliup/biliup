use crate::server::common::util::{danmaku_filename_template, media_ext_from_url};
use crate::server::core::downloader::{DanmakuClient, RustDanmakuClient};
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use danmaku_client::{PlatformContext, RecorderConfig};
use error_stack::{Report, ResultExt, bail};
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const DOUYU_DEFAULT_DID: &str = "10000000000000000000000000001501";
const DOUYU_WEB_DOMAIN: &str = "www.douyu.com";
const DOUYU_MOBILE_DOMAIN: &str = "m.douyu.com";
const DOUYU_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Douyu {
    re: Regex,
}

impl Default for Douyu {
    fn default() -> Self {
        Self::new()
    }
}

impl Douyu {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:(?:www|m)\.)?douyu\.com").unwrap(),
        }
    }
}

impl DownloadPlugin for Douyu {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let config = ctx.config();
        Box::new(DouyuDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            config.douyu_cdn.unwrap_or_else(|| "hw-h5".to_string()),
            config.douyu_rate.unwrap_or(0),
            config.douyu_disable_interactive_game.unwrap_or(false),
            config.douyu_danmaku.unwrap_or(false),
            ctx.live_streamer()
                .filename_prefix
                .clone()
                .or(config.filename_prefix.clone()),
        ))
    }

    fn name(&self) -> &str {
        "Douyu"
    }
}

struct DouyuDownloader {
    client: Client,
    url: String,
    name: String,
    douyu_cdn: String,
    douyu_rate: u32,
    douyu_disable_interactive_game: bool,
    douyu_danmaku: bool,
    filename_prefix: Option<String>,
    room_id: Option<String>,
}

impl DouyuDownloader {
    fn new(
        client: Client,
        url: String,
        name: String,
        douyu_cdn: String,
        douyu_rate: u32,
        douyu_disable_interactive_game: bool,
        douyu_danmaku: bool,
        filename_prefix: Option<String>,
    ) -> Self {
        Self {
            client,
            url,
            name,
            douyu_cdn,
            douyu_rate,
            douyu_disable_interactive_game,
            douyu_danmaku,
            filename_prefix,
            room_id: None,
        }
    }

    async fn resolve_room_id(&self) -> Result<String, Report<AppError>> {
        if let Ok(parsed) = Url::parse(&self.url)
            && let Some(rid) = parsed
                .query_pairs()
                .find(|(key, _)| key == "rid")
                .map(|(_, value)| value.to_string())
            && rid.chars().all(|ch| ch.is_ascii_digit())
        {
            return Ok(rid);
        }

        let Some(short_id) = self
            .url
            .split("douyu.com/")
            .nth(1)
            .and_then(|part| part.split('/').next())
            .and_then(|part| part.split('?').next())
            .filter(|part| !part.is_empty())
        else {
            bail!(AppError::Custom("直播间地址错误".to_string()))
        };

        let mobile_url = format!("https://{DOUYU_MOBILE_DOMAIN}/{short_id}");
        let text = self
            .client
            .get(mobile_url)
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取斗鱼真实房间号失败".to_string()))?
            .text()
            .await
            .change_context(AppError::Custom("读取斗鱼房间页面失败".to_string()))?;

        if let Some(caps) = Regex::new(r#"roomInfo":\{"rid":(\d+)"#)
            .unwrap()
            .captures(&text)
        {
            return Ok(caps[1].to_string());
        }

        if short_id.chars().all(|ch| ch.is_ascii_digit()) {
            return Ok(short_id.to_string());
        }

        bail!(AppError::Custom("获取斗鱼房间号错误".to_string()))
    }

    async fn get_room_info(&self, room_id: &str) -> Result<Option<RoomInfo>, Report<AppError>> {
        let resp: BetardResponse = self
            .client
            .get(format!("https://{DOUYU_WEB_DOMAIN}/betard/{room_id}"))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .send()
            .await
            .change_context(AppError::Custom(format!(
                "获取斗鱼直播间信息失败 room_id: {room_id}"
            )))?
            .json()
            .await
            .change_context(AppError::Custom(format!(
                "解析斗鱼直播间信息失败 room_id: {room_id}"
            )))?;

        let Some(room) = resp.room else {
            return Ok(None);
        };
        if room.show_status != 1 || room.video_loop != 0 {
            return Ok(None);
        }
        if self.douyu_disable_interactive_game && self.has_interactive_game(room_id).await? {
            return Ok(None);
        }
        Ok(Some(room))
    }

    async fn has_interactive_game(&self, room_id: &str) -> Result<bool, Report<AppError>> {
        let data: Value = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/api/interactive/web/v2/list?rid={room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取斗鱼互动游戏信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析斗鱼互动游戏信息失败".to_string()))?;

        Ok(data
            .get("data")
            .map(|data| match data {
                Value::Null => false,
                Value::Array(items) => !items.is_empty(),
                Value::Object(map) => !map.is_empty(),
                Value::String(value) => !value.is_empty(),
                Value::Bool(value) => *value,
                Value::Number(_) => true,
            })
            .unwrap_or(false))
    }

    async fn get_web_play_info(&self, room_id: &str) -> Result<PlayInfo, Report<AppError>> {
        let mut cdn = self.douyu_cdn.clone();
        let mut last_error = None;

        for _ in 0..2 {
            match self.request_web_play_info(room_id, &cdn).await {
                Ok(play_info) => {
                    if play_info.rtmp_cdn.starts_with("scdn")
                        && let Some(next_cdn) = play_info
                            .cdns_with_name
                            .iter()
                            .rev()
                            .find_map(|cdn| cdn.cdn.clone())
                    {
                        cdn = next_cdn;
                        continue;
                    }
                    return Ok(play_info);
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error
            .unwrap_or_else(|| Report::new(AppError::Custom("获取斗鱼播放信息失败".to_string()))))
    }

    async fn request_web_play_info(
        &self,
        room_id: &str,
        cdn: &str,
    ) -> Result<PlayInfo, Report<AppError>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .change_context(AppError::Unknown)?
            .as_secs();
        let encrypt_key = self.update_key().await?;
        let auth = sign_stream(&encrypt_key, room_id, now)?;

        let form = vec![
            ("cdn", cdn.to_string()),
            ("rate", self.douyu_rate.to_string()),
            ("ver", "Douyu_new".to_string()),
            ("iar", "0".to_string()),
            ("ive", "0".to_string()),
            ("rid", room_id.to_string()),
            ("hevc", "0".to_string()),
            ("fa", "0".to_string()),
            ("sov", "0".to_string()),
            ("enc_data", encrypt_key.enc_data),
            ("tt", now.to_string()),
            ("did", DOUYU_DEFAULT_DID.to_string()),
            ("auth", auth),
        ];

        let rsp: PlayResponse = self
            .client
            .post(format!(
                "https://{DOUYU_WEB_DOMAIN}/lapi/live/getH5PlayV1/{room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT)
            .form(&form)
            .send()
            .await
            .change_context(AppError::Custom(format!(
                "请求斗鱼播放信息失败 room_id: {room_id}"
            )))?
            .json()
            .await
            .change_context(AppError::Custom(format!(
                "解析斗鱼播放信息失败 room_id: {room_id}"
            )))?;

        if rsp.error == 0
            && let Some(data) = rsp.data
        {
            return Ok(data);
        }

        if rsp.error == -5 {
            bail!(AppError::Custom("[closeRoom] 主播未开播".to_string()))
        }
        if rsp.error == -9 {
            bail!(AppError::Custom(
                "[room_bus_checksevertime] 用户本机时间戳不对".to_string()
            ))
        }
        if rsp.error == 126 {
            bail!(AppError::Custom(format!(
                "版权原因，该地域不允许播放：{}",
                rsp.msg.unwrap_or_default()
            )))
        }
        bail!(AppError::Custom(format!(
            "获取斗鱼播放信息错误: code={}, msg={}",
            rsp.error,
            rsp.msg.unwrap_or_default()
        )))
    }

    async fn update_key(&self) -> Result<WhiteEncryptKey, Report<AppError>> {
        let rsp: EncryptionResponse = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/wgapi/livenc/liveweb/websec/getEncryption"
            ))
            .query(&[("did", DOUYU_DEFAULT_DID)])
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取斗鱼加密密钥失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析斗鱼加密密钥失败".to_string()))?;

        if rsp.error != 0 {
            bail!(AppError::Custom(format!(
                "getEncryption error: code={}, msg={}",
                rsp.error,
                rsp.msg.unwrap_or_default()
            )))
        }

        rsp.data
            .ok_or_else(|| Report::new(AppError::Custom("斗鱼加密密钥为空".to_string())))
    }
}

#[async_trait]
impl DownloadBase for DouyuDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.resolve_room_id().await?;
        self.room_id = Some(room_id.clone());
        let Some(room_info) = self.get_room_info(&room_id).await? else {
            return Ok(StreamStatus::Offline);
        };
        let play_info = self.get_web_play_info(&room_id).await?;
        let raw_stream_url = format!("{}/{}", play_info.rtmp_url, play_info.rtmp_live);

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: room_info.room_name,
                    date: Utc::now(),
                    live_cover_path: "".to_string(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "douyu".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }

    fn danmaku_init(&self) -> Option<Arc<dyn DanmakuClient + Send + Sync>> {
        if !self.douyu_danmaku {
            return None;
        }

        let config = RecorderConfig::new(
            self.url.clone(),
            PathBuf::from(danmaku_filename_template(
                self.filename_prefix.as_deref(),
                &self.name,
            )),
        )
        .with_context(PlatformContext::new().with_room_id(self.room_id.clone()?));

        Some(Arc::new(RustDanmakuClient::new(config)) as Arc<dyn DanmakuClient + Send + Sync>)
    }
}

fn sign_stream(
    encrypt_key: &WhiteEncryptKey,
    room_id: &str,
    ts: u64,
) -> Result<String, Report<AppError>> {
    let salt = if encrypt_key.is_special == 1 {
        String::new()
    } else {
        format!("{room_id}{ts}")
    };

    let mut secret = encrypt_key.rand_str.clone();
    for _ in 0..encrypt_key.enc_time {
        secret = md5_hex(format!("{}{}", secret, encrypt_key.key));
    }
    Ok(md5_hex(format!("{}{}{}", secret, encrypt_key.key, salt)))
}

fn md5_hex(input: String) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Deserialize)]
struct BetardResponse {
    room: Option<RoomInfo>,
}

#[derive(Deserialize)]
struct RoomInfo {
    room_name: String,
    show_status: i64,
    #[serde(rename = "videoLoop")]
    video_loop: i64,
}

#[derive(Deserialize)]
struct EncryptionResponse {
    error: i64,
    msg: Option<String>,
    data: Option<WhiteEncryptKey>,
}

#[derive(Clone, Deserialize)]
struct WhiteEncryptKey {
    rand_str: String,
    enc_time: u32,
    is_special: i64,
    key: String,
    enc_data: String,
}

#[derive(Deserialize)]
struct PlayResponse {
    error: i64,
    msg: Option<String>,
    data: Option<PlayInfo>,
}

#[derive(Deserialize)]
struct PlayInfo {
    rtmp_url: String,
    rtmp_live: String,
    #[serde(default)]
    rtmp_cdn: String,
    #[serde(default, rename = "cdnsWithName")]
    cdns_with_name: Vec<CdnInfo>,
}

#[derive(Deserialize)]
struct CdnInfo {
    cdn: Option<String>,
}
