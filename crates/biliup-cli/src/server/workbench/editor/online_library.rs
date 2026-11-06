//! Openverse/Freesound audio, ported from Uncut's library sources and request limits.
use super::{ConnectionPool, keys, public_http};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Mutex, MutexGuard, PoisonError},
};

const PAGE: usize = 20;
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    kind: String,
    query: String,
    source: Option<String>,
    license: Option<String>,
    length: Option<String>,
    songs: Option<bool>,
    page: Option<usize>,
}
#[derive(Clone)]
pub struct Hit {
    pub hit: Value,
    pub url: String,
}
#[derive(Default)]
struct Quota {
    sent: VecDeque<i64>,
    blocked: i64,
}
impl Quota {
    fn limit(&mut self, source: &str) -> Option<Value> {
        let now = super::now_ms();
        while self.sent.front().is_some_and(|t| now - t >= 86_400_000) {
            self.sent.pop_front();
        }
        let (minute, day) = if source == "openverse" {
            (20, 200)
        } else {
            (60, 2000)
        };
        let recent: Vec<_> = self.sent.iter().filter(|t| now - **t < 60_000).collect();
        let (scope, until) = if self.blocked > now {
            ("minute", self.blocked)
        } else if self.sent.len() >= day {
            ("day", self.sent[0] + 86_400_000)
        } else if recent.len() >= minute {
            ("minute", *recent[0] + 60_000)
        } else {
            return None;
        };
        Some(
            json!({"scope":scope,"until":chrono::DateTime::from_timestamp_millis(until).unwrap().to_rfc3339()}),
        )
    }
}
#[derive(Default)]
struct State {
    quotas: HashMap<String, Quota>,
    searches: HashMap<String, (i64, Value)>,
    hits: HashMap<String, (i64, Hit)>,
}
pub struct Library {
    pool: ConnectionPool,
    root: PathBuf,
    state: Mutex<State>,
}
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
fn clean(value: &Value, max: usize) -> String {
    value
        .as_str()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .into()
}
fn usable_url(value: &Value) -> Option<String> {
    public_http::https(value.as_str()?)
        .ok()
        .map(|url| url.to_string())
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}
fn decode(source: &str, r: &Value) -> Option<Hit> {
    let (id, license, version, file, landing, title, creator, seconds, tags, filetype) =
        if source == "openverse" {
            let license = r["license"].as_str()?;
            if !["cc0", "by"].contains(&license) {
                return None;
            }
            (
                clean(&r["id"], 64),
                license.to_string(),
                clean(&r["license_version"], 10),
                usable_url(&r["url"])?,
                usable_url(&r["foreign_landing_url"])?,
                clean(&r["title"], 200),
                clean(&r["creator"], 120),
                r["duration"].as_f64().map(|d| d / 1000.),
                r["tags"]
                    .as_array()
                    .map(|tags| {
                        tags.iter()
                            .take(12)
                            .map(|t| clean(&t["name"], 40))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
                clean(&r["filetype"], 8),
            )
        } else {
            let url = url::Url::parse(r["license"].as_str()?).ok()?;
            if !matches!(
                url.host_str(),
                Some("creativecommons.org" | "www.creativecommons.org")
            ) {
                return None;
            }
            let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
            let (license, version) = match parts.as_slice() {
                ["publicdomain", "zero", "1.0", ..] => ("cc0", "1.0"),
                ["licenses", "by", version, ..] if version.len() <= 10 => ("by", *version),
                _ => return None,
            };
            (
                r["id"].as_u64()?.to_string(),
                license.to_string(),
                version.to_string(),
                usable_url(&r["previews"]["preview-hq-mp3"])
                    .or_else(|| usable_url(&r["previews"]["preview-lq-mp3"]))?,
                usable_url(&r["url"])?,
                clean(&r["name"], 200),
                clean(&r["username"], 120),
                r["duration"].as_f64(),
                r["tags"]
                    .as_array()
                    .map(|tags| tags.iter().take(12).map(|t| clean(t, 40)).collect())
                    .unwrap_or_default(),
                "mp3".into(),
            )
        };
    if !valid_id(&id) {
        return None;
    }
    let version = if version.is_empty() {
        if license == "cc0" { "1.0" } else { "4.0" }.to_string()
    } else {
        version
    };
    if !version.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let license_url: String = if license == "cc0" {
        "https://creativecommons.org/publicdomain/zero/1.0/".into()
    } else {
        format!("https://creativecommons.org/licenses/by/{version}/")
    };
    let title = if title.is_empty() {
        "Untitled".into()
    } else {
        title
    };
    let creator = if creator.is_empty() {
        "Unknown".into()
    } else {
        creator
    };
    let attribution = format!(
        "\"{title}\" by {creator}, {} {version}. {license_url}",
        if license == "cc0" { "CC0" } else { "CC BY" }
    );
    let mut hit = json!({"source":source,"id":id,"title":title,"creator":creator,"landingUrl":landing,"license":license,"licenseVersion":version,"licenseUrl":license_url,"attribution":attribution,"tags":tags,"filetype":filetype});
    if let Some(seconds) = seconds.filter(|v| v.is_finite() && *v > 0.) {
        hit["seconds"] = json!(seconds)
    }
    if let Some(url) = usable_url(&r["creator_url"]) {
        hit["creatorUrl"] = json!(url)
    }
    let provider = clean(&r["provider"], 40);
    hit["provider"] = json!(if provider.is_empty() {
        source
    } else {
        &provider
    });
    Some(Hit { hit, url: file })
}
impl Library {
    pub fn new(pool: ConnectionPool, root: PathBuf) -> Self {
        Self {
            pool,
            root,
            state: Mutex::new(State::default()),
        }
    }
    fn limit(&self, source: &str) -> Option<Value> {
        lock(&self.state)
            .quotas
            .entry(source.into())
            .or_default()
            .limit(source)
    }
    pub async fn state(&self) -> Result<Value, String> {
        let mut sources = vec!["openverse"];
        if keys::get(&self.pool, &self.root, "freesound")
            .await?
            .is_some()
        {
            sources.push("freesound")
        }
        Ok(json!({"sources":sources}))
    }
    async fn api(&self, source: &str, url: &str) -> Result<Value, String> {
        let credential = if source == "freesound" {
            Some(format!(
                "Token {}",
                keys::get(&self.pool, &self.root, "freesound")
                    .await?
                    .ok_or("请先在设置中保存 Freesound API 密钥")?
            ))
        } else {
            None
        };
        {
            let mut state = lock(&self.state);
            let quota = state.quotas.entry(source.into()).or_default();
            if quota.limit(source).is_some() {
                return Err("音源请求次数已达上限，请稍后重试".into());
            }
            quota.sent.push_back(super::now_ms());
        }
        let response = public_http::get(url, credential.as_deref(), None).await?;
        if response.status().as_u16() == 429 {
            let wait = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60)
                .clamp(1, 86400);
            lock(&self.state)
                .quotas
                .entry(source.into())
                .or_default()
                .blocked = super::now_ms() + wait * 1000;
            return Err("音源服务限流，请稍后重试".into());
        }
        public_http::json(response).await
    }
    pub async fn search(&self, q: Query) -> Result<Value, String> {
        let source = q.source.as_deref().unwrap_or("openverse");
        let page = q.page.unwrap_or(1);
        if !["openverse", "freesound"].contains(&source)
            || !["music", "sfx"].contains(&q.kind.as_str())
            || q.query.chars().count() > 200
            || page == 0
            || page > 100
            || q.license
                .as_deref()
                .is_some_and(|v| !["cc0", "by"].contains(&v))
            || q.length
                .as_deref()
                .is_some_and(|v| !["short", "medium", "long"].contains(&v))
        {
            return Err("音源搜索条件无效".into());
        }
        let key = serde_json::to_string(&q).map_err(|_| "搜索条件无效")?;
        {
            let state = lock(&self.state);
            if let Some((at, result)) = state
                .searches
                .get(&key)
                .filter(|(at, _)| super::now_ms() - at < 1_800_000)
            {
                let mut result = result.clone();
                result["cached"] = json!(true);
                let _ = at;
                return Ok(result);
            }
        }
        let limited = |limit: Value| json!({"hits":[],"total":0,"page":page,"pages":page,"source":source,"cached":false,"limit":limit});
        if let Some(limit) = self.limit(source) {
            return Ok(limited(limit));
        }
        let mut url = url::Url::parse(if source == "openverse" {
            "https://api.openverse.org/v1/audio/"
        } else {
            "https://freesound.org/apiv2/search/text/"
        })
        .unwrap();
        {
            let mut params = url.query_pairs_mut();
            params
                .append_pair("page_size", &PAGE.to_string())
                .append_pair("page", &page.to_string());
            if source == "openverse" {
                params
                    .append_pair("q", q.query.trim())
                    .append_pair(
                        "license",
                        if q.license.as_deref() == Some("cc0") {
                            "cc0"
                        } else {
                            "cc0,by"
                        },
                    )
                    .append_pair("license_type", "commercial,modification")
                    .append_pair("excluded_source", "wikimedia_audio")
                    .append_pair("mature", "false");
                if q.kind == "music" && q.songs == Some(true) {
                    params.append_pair("category", "music");
                }
                if let Some(length) = q.length.as_deref() {
                    params.append_pair(
                        "length",
                        match length {
                            "short" => "shortest",
                            "medium" => "short",
                            _ => "medium,long",
                        },
                    );
                }
            } else {
                let mut filters = vec![if q.license.as_deref() == Some("cc0") {
                    r#"license:"Creative Commons 0""#
                } else {
                    r#"(license:"Creative Commons 0" OR license:"Attribution")"#
                }];
                if let Some(length) = q.length.as_deref() {
                    filters.push(match length {
                        "short" => "duration:[0 TO 30]",
                        "medium" => "duration:[30 TO 120]",
                        _ => "duration:[120 TO *]",
                    })
                }
                if q.kind == "music" && q.songs == Some(true) {
                    filters.push("tag:music")
                }
                params
                    .append_pair("query", q.query.trim())
                    .append_pair("filter", &filters.join(" "))
                    .append_pair(
                        "fields",
                        "id,name,username,license,duration,previews,tags,url,type",
                    );
            }
        }
        let response = match self.api(source, url.as_str()).await {
            Ok(value) => value,
            Err(error) => {
                if let Some(limit) = self.limit(source) {
                    return Ok(limited(limit));
                }
                return Err(error);
            }
        };
        let hits: Vec<_> = response["results"]
            .as_array()
            .ok_or("音源搜索结果无效")?
            .iter()
            .take(PAGE)
            .filter_map(|r| decode(source, r))
            .collect();
        let total = response[if source == "openverse" {
            "result_count"
        } else {
            "count"
        }]
        .as_u64()
        .unwrap_or(hits.len() as u64);
        let pages = response["page_count"]
            .as_u64()
            .unwrap_or(total.div_ceil(PAGE as u64))
            .max(1);
        let result = json!({"hits":hits.iter().map(|hit|&hit.hit).collect::<Vec<_>>(),"total":total,"page":page,"pages":pages,"source":source,"cached":false});
        let mut state = lock(&self.state);
        let now = super::now_ms();
        state.searches.retain(|_, (at, _)| now - *at < 1_800_000);
        if state.searches.len() >= 200 {
            state.searches.clear()
        }
        state.searches.insert(key, (now, result.clone()));
        state.hits.retain(|_, (at, _)| now - *at < 1_800_000);
        if state.hits.len() + hits.len() > 4000 {
            state.hits.clear()
        }
        for hit in hits {
            state.hits.insert(
                format!("{source}:{}", hit.hit["id"].as_str().unwrap()),
                (now, hit),
            );
        }
        Ok(result)
    }
    pub async fn hit(&self, source: &str, id: &str) -> Result<Hit, String> {
        if !["openverse", "freesound"].contains(&source) || !valid_id(id) {
            return Err("音源标识无效".into());
        }
        let key = format!("{source}:{id}");
        {
            if let Some((_, hit)) = lock(&self.state)
                .hits
                .get(&key)
                .filter(|(at, _)| super::now_ms() - at < 1_800_000)
            {
                return Ok(hit.clone());
            }
        }
        let url = if source == "openverse" {
            format!("https://api.openverse.org/v1/audio/{id}/")
        } else {
            format!(
                "https://freesound.org/apiv2/sounds/{id}/?fields=id,name,username,license,duration,previews,tags,url,type"
            )
        };
        decode(source, &self.api(source, &url).await?)
            .ok_or_else(|| "该音源没有可用的 CC0 / CC BY 音频".into())
    }
    pub async fn add(&self, project: i64, source: &str, id: &str) -> Result<Value, String> {
        self.add_limited(project,source,id,200*1024*1024,|_,_|{}).await
    }
    pub(crate) async fn add_limited(&self,project:i64,source:&str,id:&str,max:usize,mut progress:impl FnMut(u64,Option<u64>)+Send+'static)->Result<Value,String>{
        let hit = self.hit(source, id).await?;
        let response = public_http::get(&hit.url, None, None).await?;
        if !response.status().is_success() {
            return Err(format!("音源下载失败（{}）", response.status().as_u16()));
        }
        let max=max.min(200*1024*1024);let total=response.content_length();
        if total.is_some_and(|n|n>max as u64){return Err("音源超过本次下载大小限制".into())}
        use futures::TryStreamExt;
        let mut downloaded=0u64;
        let stream=public_http::stream(response,max).inspect_ok(move|chunk|{downloaded+=chunk.len() as u64;progress(downloaded,total);});
        let mut value = super::imports::upload(
            &self.pool,
            &self.root,
            project,
            "library-audio",
            None,
            axum::body::Body::from_stream(stream),
        )
        .await?;
        if value["media"]["kind"] != "audio" {
            return Err("下载的文件不是音频".into());
        }
        let mut credit = hit.hit;
        let title = credit["title"].clone();
        credit["module"] = json!(format!("library.{source}"));
        credit["libraryId"] = json!(format!("{source}:{id}"));
        credit["downloaded"] = json!(chrono::Utc::now().to_rfc3339());
        credit["sha256"] = value["source"]["assetId"].clone();
        value["media"]["name"] = title;
        value["media"]["source"] = credit;
        Ok(value)
    }
    pub async fn preview(
        &self,
        source: &str,
        id: &str,
        range: Option<&str>,
    ) -> Result<reqwest::Response, String> {
        if range.is_some_and(|r| {
            r.len() > 80
                || !r.starts_with("bytes=")
                || r[6..].chars().any(|c| !c.is_ascii_digit() && c != '-')
        }) {
            return Err("音源范围无效".into());
        }
        let hit = self.hit(source, id).await?;
        public_http::get(&hit.url, None, range).await
    }
}
