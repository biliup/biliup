use crate::ReqwestClientBuilderExt;
use crate::error::{Kind, Result};
use crate::uploader::credential::LoginInfo;
use serde::ser::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

use bon::Builder;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Serialize, Deserialize, Debug, Builder)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct Studio {
    /// 是否转载, 1-自制 2-转载
    #[cfg_attr(feature = "cli", clap(long, default_value = "1"))]
    #[builder(default = 1)]
    #[serde(default = "default_copyright")]
    pub copyright: u8,

    /// 转载来源
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    #[serde(default)]
    pub source: String,

    /// 投稿分区
    #[cfg_attr(feature = "cli", clap(long, default_value = "171"))]
    #[builder(default = 171)]
    pub tid: u16,

    /// 视频封面
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    #[serde(default)]
    pub cover: String,

    /// 视频标题
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    pub title: String,

    #[cfg_attr(feature = "cli", clap(skip))]
    #[serde(default)]
    #[builder(default)]
    pub desc_format_id: u32,

    /// 视频简介
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    #[serde(default)]
    pub desc: String,

    /// 视频简介v2
    #[serde(default)]
    #[cfg_attr(feature = "cli", clap(skip))]
    pub desc_v2: Option<Vec<Credit>>,

    /// 空间动态
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    #[serde(default)]
    pub dynamic: String,

    #[cfg_attr(feature = "cli", clap(skip))]
    #[serde(default)]
    #[builder(default)]
    pub subtitle: Subtitle,

    /// 视频标签，逗号分隔多个tag
    #[cfg_attr(feature = "cli", clap(long, default_value_t))]
    #[serde(default)]
    pub tag: String,

    #[serde(default)]
    #[cfg_attr(feature = "cli", clap(skip))]
    pub videos: Vec<Video>,

    /// 延时发布时间，距离提交大于4小时，格式为10位时间戳
    #[cfg_attr(feature = "cli", clap(long))]
    pub dtime: Option<u32>,

    #[cfg_attr(feature = "cli", clap(skip))]
    #[serde(default)]
    #[builder(default)]
    pub open_subtitle: bool,

    #[cfg_attr(feature = "cli", clap(long, default_value = "0"))]
    #[serde(default)]
    #[builder(default)]
    pub interactive: u8,

    #[cfg_attr(feature = "cli", clap(long))]
    #[serde(default)]
    pub mission_id: Option<u32>,

    // #[clap(long, default_value = "0")]
    // pub act_reserve_create: u8,
    /// 是否开启杜比音效, 0-关闭 1-开启
    #[cfg_attr(feature = "cli", clap(long, default_value = "0"))]
    #[serde(default)]
    pub dolby: u8,

    /// 是否开启 Hi-Res, 0-关闭 1-开启
    #[cfg_attr(feature = "cli", clap(long = "hires", default_value = "0"))]
    #[serde(default)]
    #[builder(default)]
    pub lossless_music: u8,

    /// 0-允许转载，1-禁止转载
    #[cfg_attr(feature = "cli", clap(long, default_value = "0"))]
    #[serde(default)]
    pub no_reprint: u8,

    /// 仅自己可见
    #[cfg_attr(feature = "cli", clap(long))]
    #[serde(default)]
    pub is_only_self: Option<u8>,

    /// 是否开启充电, 0-关闭 1-开启
    #[cfg_attr(feature = "cli", clap(long, default_value = "0"))]
    #[serde(default)]
    pub charging_pay: u8,

    /// aid 要追加视频的 avid
    #[cfg_attr(feature = "cli", clap(skip))]
    pub aid: Option<u64>,

    /// 是否开启精选评论，仅提交接口为app时可用
    #[cfg_attr(feature = "cli", clap(long))]
    #[serde(default)]
    pub up_selection_reply: bool,

    /// 是否关闭评论，仅提交接口为app时可用
    #[cfg_attr(feature = "cli", clap(long))]
    #[serde(default)]
    pub up_close_reply: bool,

    /// 是否关闭弹幕，仅提交接口为app时可用
    #[cfg_attr(feature = "cli", clap(long))]
    #[serde(default)]
    pub up_close_danmu: bool,

    /// 自定义提交参数
    #[cfg_attr(feature = "cli", clap(long, value_parser = parse_extra_fields))]
    #[serde(flatten)]
    pub extra_fields: Option<HashMap<String, Value>>,
}

fn parse_extra_fields(s: &str) -> std::result::Result<HashMap<String, Value>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn default_copyright() -> u8 {
    1
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub aid: u64,
    pub bvid: String,
    pub title: String,
    pub cover: String,
    pub reject_reason: String,
    pub reject_reason_url: String,
    pub duration: u64,
    pub desc: String,
    pub state: i16,
    pub state_desc: String,
    pub dtime: u64,
    pub ptime: u64,
    pub ctime: u64,
}

impl Archive {
    pub fn to_string_pretty(&self) -> String {
        let status_string = match self.state {
            0 => format!("\x1b[1;92m{}\x1b[0m", self.state_desc),
            -2 => format!("\x1b[1;91m{}\x1b[0m", self.state_desc),
            -30 => format!("\x1b[1;93m{}\x1b[0m", self.state_desc),
            _ => self.desc.to_string(),
        };
        format!("{}\t{}\t{}", self.bvid, self.title, status_string)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Subtitle {
    open: i8,
    lan: String,
}

#[derive(PartialEq, Deserialize, Serialize, Debug, Clone)]
pub struct Credit {
    #[serde(rename(deserialize = "type_id", serialize = "type"))]
    pub type_id: i8,
    pub raw_text: String,
    pub biz_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Video {
    pub title: Option<String>,
    pub filename: String,
    pub desc: String,
}

impl Video {
    pub fn new(filename: &str) -> Video {
        Video {
            title: None,
            filename: filename.into(),
            desc: "".into(),
        }
    }

    /// 截断标题到指定的最大字符数（默认80个字符，B站限制）
    pub fn truncate_title(title: &str, max_chars: usize) -> String {
        // 统计字符数（不是字节数）
        let char_count = title.chars().count();
        if char_count <= max_chars {
            return title.to_string();
        }

        // 截断到max_chars-3个字符，然后添加"..."
        let truncated: String = title.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Vid {
    Aid(u64),
    Bvid(String),
}

impl FromStr for Vid {
    type Err = ParseIntError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() < 3 {
            return s.parse::<u64>().map(Vid::Aid);
        }
        match &s[..2] {
            "BV" => Ok(Vid::Bvid(s.to_string())),
            "av" => Ok(Vid::Aid(s[2..].parse()?)),
            _ => Ok(Vid::Aid(s.parse()?)),
        }
    }
}

impl Display for Vid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Vid::Aid(aid) => write!(f, "aid={}", aid),
            Vid::Bvid(bvid) => write!(f, "bvid={}", bvid),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BiliBili {
    pub client: reqwest::Client,
    pub login_info: LoginInfo,
}

impl BiliBili {
    #[deprecated(note = "no longer working, fallback to `submit_by_app`")]
    pub async fn submit(&self, studio: &Studio, proxy: Option<&str>) -> Result<ResponseData> {
        warn!("客户端接口已失效, 将使用APP接口");
        self.submit_by_app(studio, proxy).await
    }

    /// 使用必剪接口投稿
    pub async fn submit_by_bcut_android(
        &self,
        studio: &Studio,
        proxy: Option<&str>,
    ) -> Result<ResponseData> {
        let payload = {
            let mut payload = json!({
                "access_key": self.login_info.token_info.access_token,
                "appkey": crate::credential::AppKeyStore::BCutAndroid.app_key(),
                "aurora_version": "2.39.0",
                "build": 2800030,
                "c_locale": "zh-Hans_CN",
                "channel": "master",
                "mobi_app": "android_bbs",
                "montage_version": "1.42.1.0",
                "platform": "android",
                "s_locale": "zh-Hans_CN",
                "sdk_type": "mon",
                "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });

            let urlencoded = serde_urlencoded::to_string(&payload)?;
            let sign = crate::credential::Credential::sign(
                &urlencoded,
                crate::credential::AppKeyStore::BCutAndroid.appsec(),
            );
            payload["sign"] = Value::from(sign);
            payload
        };

        let ret: ResponseData = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 os/android model/Mi 10 Pro mobi_app/android_bbs build/2800030 channel/master osVer/13 kernel_version/V14.0.4.0.TJACNXM BiliDroid/5.6.0 (bbcallen@gmail.com)")
            .timeout(Duration::new(60, 0))
            .build()?
            .post("https://member.bilibili.com/x/vu/mvp/add")
            .query(&payload)
            .json(studio)
            .send()
            .await?
            .json()
            .await?;
        info!("{:?}", ret);
        if ret.code == 0 {
            info!("BCUT接口投稿成功");
            Ok(ret)
        } else {
            Err(Kind::Custom(format!("{:?}", ret)))
        }
    }

    pub async fn submit_by_app(
        &self,
        studio: &Studio,
        proxy: Option<&str>,
    ) -> Result<ResponseData> {
        let payload = {
            let mut payload = json!({
                "access_key": self.login_info.token_info.access_token,
                "appkey": crate::credential::AppKeyStore::BiliTV.app_key(),
                "build": 7800300,
                "c_locale": "zh-Hans_CN",
                "channel": "bili",
                "disable_rcmd": 0,
                "mobi_app": "android",
                "platform": "android",
                "s_locale": "zh-Hans_CN",
                "statistics": "\"appId\":1,\"platform\":3,\"version\":\"7.80.0\",\"abtest\":\"\"",
                "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });

            let urlencoded = serde_urlencoded::to_string(&payload)?;
            let sign = crate::credential::Credential::sign(
                &urlencoded,
                crate::credential::AppKeyStore::BiliTV.appsec(),
            );
            payload["sign"] = Value::from(sign);
            payload
        };

        let ret: ResponseData = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 BiliDroid/7.80.0 (bbcallen@gmail.com) os/android model/MI 6 mobi_app/android build/7800300 channel/bili innerVer/7800310 osVer/13 network/2")
            .timeout(Duration::new(60, 0))
            .build()?
            .post("https://member.bilibili.com/x/vu/app/add")
            .query(&payload)
            .json(studio)
            .send()
            .await?
            .json()
            .await?;
        info!("{:?}", ret);
        if ret.code == 0 {
            info!("APP接口投稿成功");
            Ok(ret)
        } else {
            Err(Kind::Custom(format!("{:?}", ret)))
        }
    }

    /// 通过 Web 接口投稿
    pub async fn submit_by_web(
        &self,
        studio: &Studio,
        proxy: Option<&str>,
    ) -> Result<ResponseData> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let csrf = self.get_csrf()?;

        let url_str = "https://member.bilibili.com/x/vu/web/add/v3";
        let params = [("t", ts.to_string()), ("csrf", csrf.to_string())];
        let url = reqwest::Url::parse_with_params(url_str, &params).unwrap();

        let cookie = self.get_cookie()?;
        let jar = reqwest::cookie::Jar::default();
        jar.add_cookie_str(&cookie, &url);

        let ret: ResponseData = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
            .cookie_provider(std::sync::Arc::new(jar))
            .timeout(Duration::new(60, 0))
            .build()?
            .post(url)
            .json(studio)
            .send()
            .await?
            .json()
            .await?;
        info!("{:?}", ret);

        if ret.code == 0 {
            info!("Web 接口投稿成功");
            Ok(ret)
        } else {
            Err(Kind::Custom(format!("{:?}", ret)))
        }
    }

    #[deprecated(note = "no longer working, fallback to `edit_by_app`")]
    pub async fn edit(&self, studio: &Studio, proxy: Option<&str>) -> Result<serde_json::Value> {
        warn!("客户端接口已失效, 将使用app接口");
        self.edit_by_app(studio, proxy).await
    }

    pub async fn edit_by_web(&self, studio: &Studio) -> Result<serde_json::Value> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let ret: serde_json::Value = self
            .client
            .post(format!(
                "https://member.bilibili.com/x/vu/web/edit?t={ts}&csrf={}",
                self.get_csrf()?
            ))
            .json(studio)
            .send()
            .await?
            .json()
            .await?;
        info!("{}", ret);
        if ret["code"] == 0 {
            info!("稿件修改成功");
            Ok(ret)
        } else {
            Err(Kind::Custom(ret.to_string()))
        }
    }

    pub async fn edit_by_app(
        &self,
        studio: &Studio,
        proxy: Option<&str>,
    ) -> Result<serde_json::Value> {
        let payload = {
            let mut payload = json!({
                "access_key": self.login_info.token_info.access_token,
                "appkey": crate::credential::AppKeyStore::BiliTV.app_key(),
                "build": 7800300,
                "c_locale": "zh-Hans_CN",
                "channel": "bili",
                "disable_rcmd": 0,
                "mobi_app": "android",
                "platform": "android",
                "s_locale": "zh-Hans_CN",
                "statistics": "\"appId\":1,\"platform\":3,\"version\":\"7.80.0\",\"abtest\":\"\"",
                "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });

            let urlencoded = serde_urlencoded::to_string(&payload)?;
            let sign = crate::credential::Credential::sign(
                &urlencoded,
                crate::credential::AppKeyStore::BiliTV.appsec(),
            );
            payload["sign"] = Value::from(sign);
            payload
        };

        let ret: Value = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 BiliDroid/7.80.0 (bbcallen@gmail.com) os/android model/MI 6 mobi_app/android build/7800300 channel/bili innerVer/7800310 osVer/13 network/2")
            .timeout(Duration::new(60, 0))
            .build()?
            .post("https://member.bilibili.com/x/vu/app/edit/full")
            .query(&payload)
            .json(studio)
            .send()
            .await?
            .json()
            .await?;
        info!("{:?}", ret);
        if ret["code"] == 0 {
            info!("稿件修改成功");
            Ok(ret)
        } else {
            Err(Kind::Custom(ret.to_string()))
        }
    }

    /// 查询视频的 json 信息
    pub async fn video_data(&self, vid: &Vid, proxy: Option<&str>) -> Result<Value> {
        let res: ResponseData = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/63.0.3239.108")
            .timeout(Duration::new(60, 0))
            .build()?
            .get(format!(
                "https://member.bilibili.com/x/client/archive/view?access_key={}&{vid}",
                self.login_info.token_info.access_token
            ))
            .send()
            .await?
            .json()
            .await?;
        match res {
            res @ ResponseData {
                code: _,
                data: None,
                ..
            } => Err(Kind::Custom(format!("{res:?}"))),
            ResponseData {
                code: _,
                data: Some(v),
                ..
            } => Ok(v),
        }
    }

    pub async fn studio_data(&self, vid: &Vid, proxy: Option<&str>) -> Result<Studio> {
        let mut video_info = self.video_data(vid, proxy).await?;
        const EXTRA_FIELDS_BLACKLIST: &[&str] = &["limited_free"];

        let mut archive_value = video_info["archive"].take();

        if let Some(obj) = archive_value.as_object_mut() {
            for key in EXTRA_FIELDS_BLACKLIST {
                obj.remove(*key);
            }
        }

        let mut studio: Studio = serde_json::from_value(archive_value)?;
        let videos: Vec<Video> = serde_json::from_value(video_info["videos"].take())?;

        studio.videos = videos;
        Ok(studio)
    }

    pub async fn my_info(&self) -> Result<Value> {
        Ok(self
            .client
            .get("https://api.bilibili.com/x/space/myinfo")
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn archive_pre(&self) -> Result<Value> {
        Ok(self
            .client
            .get("https://member.bilibili.com/x/vupre/web/archive/pre")
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn recommend_tag(&self, subtype_id: u16, title: &str, key: &str) -> Result<Value> {
        let result: ResponseData = self
            .client
            .get(format!("https://member.bilibili.com/x/vupre/web/tag/recommend?upload_id=&subtype_id={subtype_id}&title={title}&filename={key}&description=&cover_url=&t="))
            .send()
            .await?
            .json()
            .await?;
        if result.code == 0 {
            return Ok(result.data.unwrap_or_default());
        }
        Err(Kind::Custom(result.message))
    }

    fn get_csrf(&self) -> Result<&str> {
        let csrf = self
            .login_info
            .cookie_info
            .get("cookies")
            .and_then(|c| c.as_array())
            .ok_or("cookie error")?
            .iter()
            .filter_map(|c| c.as_object())
            .find(|c| c["name"] == "bili_jct")
            .ok_or("jct error")?
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("csrf error")?;
        Ok(csrf)
    }

    pub async fn cover_up(&self, input: &[u8]) -> Result<String> {
        let response = self
            .client
            .post("https://member.bilibili.com/x/vu/web/cover/up")
            .form(&json!({
                "cover": format!("data:image/jpeg;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, input)),
                "csrf": self.get_csrf()?
            }))
            .send()
            .await?;
        let res: ResponseData = if !response.status().is_success() {
            return Err(Kind::Custom(response.text().await?));
        } else {
            response.json().await?
        };

        if let ResponseData {
            code: _,
            data: Some(value),
            ..
        } = res
        {
            Ok(value["url"].as_str().ok_or("cover_up error")?.into())
        } else {
            Err(Kind::Custom(format!("{res:?}")))
        }
    }

    /// 稿件管理
    async fn archives(&self, status: &str, page_num: u32) -> Result<Value> {
        let url_str = "https://member.bilibili.com/x/web/archives";
        let params = [("status", status), ("pn", &page_num.to_string())];
        let url = reqwest::Url::parse_with_params(url_str, &params).unwrap();

        let cookie = self.get_cookie()?;
        let jar = reqwest::cookie::Jar::default();
        jar.add_cookie_str(&cookie, &url);

        let res: ResponseData = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/63.0.3239.108")
            .cookie_provider(std::sync::Arc::new(jar))
            .timeout(Duration::new(60, 0))
            .build()?
            .get(url)
            .send()
            .await?
            .json()
            .await?;

        match res {
            ResponseData {
                code: _,
                data: None,
                ..
            } => Err(Kind::Custom(format!("{:?}", res))),
            ResponseData {
                code: _,
                data: Some(v),
                ..
            } => Ok(v),
        }
    }

    async fn recent_archives_data(
        &self,
        status: &str,
        from_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<Value>> {
        let mut first_page = self.archives(status, from_page).await?;

        let (page_size, count) = {
            let page = first_page["page"].take();
            let page_size = page["ps"].as_u64().ok_or("all_studios ps error")?;
            let count = page["count"].as_u64().ok_or("all_studios count error")?;
            (page_size as u32, count as u32)
        };

        let total_pages = {
            let mut pages = count / page_size;
            if pages * page_size < count {
                pages += 1;
            }
            pages
        };

        let fetch_pages = match max_pages {
            Some(mp) => std::cmp::min(total_pages - from_page + 1, mp),
            None => total_pages - from_page + 1,
        };
        let to_page = from_page - 1 + fetch_pages;

        let mut all_pages = vec![first_page];
        for page_num in from_page + 1..=to_page {
            let page = self.archives(status, page_num).await?;
            all_pages.push(page);
        }

        Ok(all_pages)
    }

    /// 获取所有稿件
    #[deprecated(note = "use `recent_archives` instead")]
    pub async fn all_archives(&self, status: &str) -> Result<Vec<Archive>> {
        self.recent_archives(status, 1, None).await
    }

    /// 获取页数范围内的稿件
    pub async fn recent_archives(
        &self,
        status: &str,
        from_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<Archive>> {
        let studios = self
            .recent_archives_data(status, from_page, max_pages)
            .await?
            .iter_mut()
            .map(|page| page["arc_audits"].take())
            .filter_map(|audits| serde_json::from_value::<Vec<Value>>(audits).ok())
            .flat_map(|archives| archives.into_iter())
            .map(|mut arc| arc["Archive"].take())
            .filter_map(|studio| serde_json::from_value::<_>(studio).ok())
            .collect::<Vec<_>>();

        Ok(studios)
    }

    fn get_cookie(&self) -> Result<String> {
        let cookie = self
            .login_info
            .cookie_info
            .get("cookies")
            .and_then(|c: &Value| c.as_array())
            .ok_or("get cookie error")?
            .iter()
            .filter_map(|c| match (c["name"].as_str(), c["value"].as_str()) {
                (Some(name), Some(value)) => Some((name, value)),
                _ => None,
            })
            .map(|c| format!("{}={}", c.0, c.1))
            .collect::<Vec<_>>()
            .join("; ");
        Ok(cookie)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ResponseData<T = Value> {
    pub code: i32,
    pub data: Option<T>,
    message: String,
    ttl: Option<u8>,
}

impl<T: Serialize> Display for ResponseData<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).map_err(std::fmt::Error::custom)?
        )
    }
}
