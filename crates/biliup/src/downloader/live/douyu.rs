use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
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

#[async_trait]
impl LivePlugin for Douyu {
    fn name(&self) -> &'static str {
        "Douyu"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        DouyuLive::new(request).check_stream().await
    }
}

struct DouyuLive {
    client: Client,
    url: String,
    name: String,
    douyu_cdn: String,
    douyu_rate: u32,
    douyu_disable_interactive_game: bool,
    douyu_danmaku: bool,
    room_id: Option<String>,
}

impl DouyuLive {
    fn new(request: LiveRequest) -> Self {
        let options = request.options.douyu;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            douyu_cdn: options.cdn,
            douyu_rate: options.rate,
            douyu_disable_interactive_game: options.disable_interactive_game,
            douyu_danmaku: options.danmaku,
            room_id: None,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        let room_id = self.resolve_room_id().await?;
        self.room_id = Some(room_id.clone());
        let Some(room_info) = self.get_room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let play_info = self.get_web_play_info(&room_id).await?;
        let raw_stream_url = format!("{}/{}", play_info.rtmp_url, play_info.rtmp_live);

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: room_info.room_name,
                date: Utc::now(),
                live_cover_url: String::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "douyu".to_string(),
                stream_headers: HashMap::new(),
                danmaku: self.danmaku_source(),
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    async fn resolve_room_id(&self) -> LiveResult<String> {
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
            return Err(LiveError::custom("直播间地址错误"));
        };

        let mobile_url = format!("https://{DOUYU_MOBILE_DOMAIN}/{short_id}");
        let text = self
            .client
            .get(mobile_url)
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼真实房间号失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取斗鱼房间页面失败: {err}")))?;

        if let Some(caps) = Regex::new(r#"roomInfo":\{"rid":(\d+)"#)
            .unwrap()
            .captures(&text)
        {
            return Ok(caps[1].to_string());
        }

        if short_id.chars().all(|ch| ch.is_ascii_digit()) {
            return Ok(short_id.to_string());
        }

        Err(LiveError::custom("获取斗鱼房间号错误"))
    }

    async fn get_room_info(&self, room_id: &str) -> LiveResult<Option<RoomInfo>> {
        let resp: BetardResponse = self
            .client
            .get(format!("https://{DOUYU_WEB_DOMAIN}/betard/{room_id}"))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .send()
            .await
            .map_err(|err| {
                LiveError::custom(format!("获取斗鱼直播间信息失败 room_id: {room_id}: {err}"))
            })?
            .json()
            .await
            .map_err(|err| {
                LiveError::custom(format!("解析斗鱼直播间信息失败 room_id: {room_id}: {err}"))
            })?;

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

    async fn has_interactive_game(&self, room_id: &str) -> LiveResult<bool> {
        let data: Value = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/api/interactive/web/v2/list?rid={room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼互动游戏信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析斗鱼互动游戏信息失败: {err}")))?;

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

    async fn get_web_play_info(&self, room_id: &str) -> LiveResult<PlayInfo> {
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

        Err(last_error.unwrap_or_else(|| LiveError::custom("获取斗鱼播放信息失败")))
    }

    async fn request_web_play_info(&self, room_id: &str, cdn: &str) -> LiveResult<PlayInfo> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| LiveError::custom(format!("获取系统时间失败: {err}")))?
            .as_secs();
        let encrypt_key = self.update_key().await?;
        let auth = sign_stream(&encrypt_key, room_id, now);

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
            .map_err(|err| {
                LiveError::custom(format!("请求斗鱼播放信息失败 room_id: {room_id}: {err}"))
            })?
            .json()
            .await
            .map_err(|err| {
                LiveError::custom(format!("解析斗鱼播放信息失败 room_id: {room_id}: {err}"))
            })?;

        if rsp.error == 0
            && let Some(data) = rsp.data
        {
            return Ok(data);
        }

        if rsp.error == -5 {
            return Err(LiveError::custom("[closeRoom] 主播未开播"));
        }
        if rsp.error == -9 {
            return Err(LiveError::custom(
                "[room_bus_checksevertime] 用户本机时间戳不对",
            ));
        }
        if rsp.error == 126 {
            return Err(LiveError::custom(format!(
                "版权原因，该地域不允许播放：{}",
                rsp.msg.unwrap_or_default()
            )));
        }
        Err(LiveError::custom(format!(
            "获取斗鱼播放信息错误: code={}, msg={}",
            rsp.error,
            rsp.msg.unwrap_or_default()
        )))
    }

    async fn update_key(&self) -> LiveResult<WhiteEncryptKey> {
        let rsp: EncryptionResponse = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/wgapi/livenc/liveweb/websec/getEncryption"
            ))
            .query(&[("did", DOUYU_DEFAULT_DID)])
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼加密密钥失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析斗鱼加密密钥失败: {err}")))?;

        if rsp.error != 0 {
            return Err(LiveError::custom(format!(
                "getEncryption error: code={}, msg={}",
                rsp.error,
                rsp.msg.unwrap_or_default()
            )));
        }

        rsp.data
            .ok_or_else(|| LiveError::custom("斗鱼加密密钥为空"))
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.douyu_danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "douyu".to_string(),
            url: self.url.clone(),
            room_id: self.room_id.clone(),
            cookie: None,
            raw: false,
            detail: false,
            extra: HashMap::new(),
            movie_id: None,
            password: None,
        })
    }
}

fn sign_stream(encrypt_key: &WhiteEncryptKey, room_id: &str, ts: u64) -> String {
    let salt = if encrypt_key.is_special == 1 {
        String::new()
    } else {
        format!("{room_id}{ts}")
    };

    let mut secret = encrypt_key.rand_str.clone();
    for _ in 0..encrypt_key.enc_time {
        secret = md5_hex(format!("{}{}", secret, encrypt_key.key));
    }
    md5_hex(format!("{}{}{}", secret, encrypt_key.key, salt))
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
