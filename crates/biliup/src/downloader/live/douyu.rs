use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use md5::{Digest, Md5};
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;
use url::{Url, form_urlencoded};

const DOUYU_DEFAULT_DID: &str = "10000000000000000000000000001501";
const DOUYU_WEB_DOMAIN: &str = "www.douyu.com";
const DOUYU_MOBILE_DOMAIN: &str = "m.douyu.com";
const DOUYU_HUOS_DOMAIN: &str = "openflv-huos.douyucdn2.cn";
const DOUYU_HS_CDN: &str = "hs-h5";
const DOUYU_P2P_DOMAIN_TCT: &str = "hdltctwk.douyucdn.cn";
const DOUYU_P2PSDK_APIS: [&str; 2] = ["https://sdkapiv4.douyucdn.cn", "https://sdkapi.douyucdn.cn"];
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
    douyu_force_hs: bool,
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
            douyu_force_hs: options.force_hs,
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
        let raw_stream_url = self.maybe_build_huos_url(raw_stream_url).await;

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

    async fn maybe_build_huos_url(&self, raw_stream_url: String) -> String {
        if self.should_build_huos_url() {
            match self.build_huos_url(&raw_stream_url).await {
                Ok(huos_url) => huos_url,
                Err(err) => {
                    warn!(
                        error = ?err,
                        url = raw_stream_url,
                        "failed to build Douyu huos URL, falling back to original stream URL"
                    );
                    raw_stream_url
                }
            }
        } else {
            raw_stream_url
        }
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

    fn should_build_huos_url(&self) -> bool {
        self.douyu_force_hs && self.douyu_cdn.eq_ignore_ascii_case(DOUYU_HS_CDN)
    }

    async fn build_huos_url(&self, raw_stream_url: &str) -> LiveResult<String> {
        let (stream_id, params) = parse_stream_url(raw_stream_url)?;
        let tx_secret = self.get_txsecret(&stream_id).await?;
        Ok(build_huos_url(&stream_id, &params, &tx_secret))
    }

    async fn get_txsecret(&self, stream_id: &str) -> LiveResult<XP2PTxSecret> {
        let mut apis = DOUYU_P2PSDK_APIS;
        {
            let mut rng = rand::thread_rng();
            apis.shuffle(&mut rng);
        }

        let mut last_error = None;
        for api in apis {
            match self.request_txsecret(api, stream_id).await {
                Ok(tx_secret) => return Ok(tx_secret),
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LiveError::custom(format!("获取 txSecret 失败: {stream_id}"))
        }))
    }

    async fn request_txsecret(&self, api: &str, stream_id: &str) -> LiveResult<XP2PTxSecret> {
        let tx_secret: XP2PTxSecret = self
            .client
            .get(format!("{api}/p2p/get_txsecret"))
            .query(&[("lid", stream_id)])
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| {
                LiveError::custom(format!(
                    "获取 txSecret 失败 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?
            .error_for_status()
            .map_err(|err| {
                LiveError::custom(format!(
                    "获取 txSecret 响应异常 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?
            .json()
            .await
            .map_err(|err| {
                LiveError::custom(format!(
                    "解析 txSecret 失败 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?;

        if tx_secret.tx_secret.is_empty() || tx_secret.tx_time.is_empty() {
            return Err(LiveError::custom(format!("txSecret 为空: {stream_id}")));
        }

        Ok(tx_secret)
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

fn parse_stream_url(input: &str) -> LiveResult<(String, Vec<(String, String)>)> {
    let parsed = Url::parse(input)
        .map_err(|err| LiveError::custom(format!("解析斗鱼 huos 源链接失败: {err}")))?;
    let stream_name = parsed
        .path()
        .rsplit('/')
        .find(|part| !part.is_empty())
        .ok_or_else(|| LiveError::custom("斗鱼 huos 源链接缺少 stream_id"))?;
    let stream_id = stream_name
        .split_once('.')
        .map(|(stream_id, _)| stream_id)
        .unwrap_or(stream_name);
    if stream_id.is_empty() {
        return Err(LiveError::custom("斗鱼 huos 源链接 stream_id 为空"));
    }
    let params = parsed
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    Ok((stream_id.to_string(), params))
}

fn build_huos_url(
    stream_id: &str,
    params: &[(String, String)],
    tx_secret: &XP2PTxSecret,
) -> String {
    let mut next_params = Vec::with_capacity(params.len() + 3);
    let mut has_fcdn = false;

    for (key, value) in params {
        if matches!(key.as_str(), "txSecret" | "txTime" | "domain") {
            continue;
        }
        if key == "fcdn" {
            if !has_fcdn {
                next_params.push((key.clone(), "hs".to_string()));
                has_fcdn = true;
            }
            continue;
        }
        next_params.push((key.clone(), value.clone()));
    }

    if !has_fcdn {
        next_params.push(("fcdn".to_string(), "hs".to_string()));
    }
    next_params.push(("txSecret".to_string(), tx_secret.tx_secret.clone()));
    next_params.push(("txTime".to_string(), tx_secret.tx_time.clone()));
    next_params.push(("domain".to_string(), DOUYU_P2P_DOMAIN_TCT.to_string()));

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in next_params {
        serializer.append_pair(&key, &value);
    }
    let query = serializer.finish();

    format!("http://{DOUYU_HUOS_DOMAIN}/live/{stream_id}.xs?{query}")
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

#[derive(Deserialize)]
struct XP2PTxSecret {
    #[serde(rename = "xp2p_txSecret")]
    tx_secret: String,
    #[serde(rename = "xp2p_txTime")]
    tx_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_huos_url_rewrites_fcdn_and_appends_secret() {
        let (stream_id, params) = parse_stream_url(
            "https://hw3.douyucdn2.cn/live/6925114rIDrEEuKo.flv?wsAuth=auth&fcdn=hw&isp=",
        )
        .unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "secret".to_string(),
            tx_time: "time".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/6925114rIDrEEuKo.xs?wsAuth=auth&fcdn=hs&isp=&txSecret=secret&txTime=time&domain=hdltctwk.douyucdn.cn"
        );
    }

    #[test]
    fn build_huos_url_removes_existing_secret_params() {
        let (stream_id, params) = parse_stream_url(
            "https://hw3.douyucdn2.cn/live/abc.flv?fcdn=hw&txSecret=old&txTime=old&domain=old.example",
        )
        .unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "new".to_string(),
            tx_time: "next".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/abc.xs?fcdn=hs&txSecret=new&txTime=next&domain=hdltctwk.douyucdn.cn"
        );
    }

    #[test]
    fn build_huos_url_adds_missing_fcdn() {
        let (stream_id, params) =
            parse_stream_url("https://hw3.douyucdn2.cn/live/abc.flv?token=value").unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "secret".to_string(),
            tx_time: "time".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/abc.xs?token=value&fcdn=hs&txSecret=secret&txTime=time&domain=hdltctwk.douyucdn.cn"
        );
    }

    #[tokio::test]
    async fn maybe_build_huos_url_falls_back_when_build_fails() {
        let live = DouyuLive {
            client: Client::new(),
            url: "https://www.douyu.com/10568722".to_string(),
            name: "test".to_string(),
            douyu_cdn: DOUYU_HS_CDN.to_string(),
            douyu_force_hs: true,
            douyu_rate: 0,
            douyu_disable_interactive_game: false,
            douyu_danmaku: false,
            room_id: None,
        };

        let raw_stream_url = "not a url".to_string();

        assert_eq!(
            live.maybe_build_huos_url(raw_stream_url.clone()).await,
            raw_stream_url
        );
    }
}
