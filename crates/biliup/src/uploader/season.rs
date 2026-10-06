//! 合集（season）管理：列合集、查小节、公共稿件信息、加入 / 移出小节、排序。
//!
//! 接口与参数对齐 `biliup/plugins/bili_webup.py` 里 `list_seasons` 到
//! `sort_season_episodes` 的实现。响应结构体只声明用得到的字段，其余字段收进
//! `extra`，B 站新增字段不会导致解析失败。

use crate::error::{Kind, Result};
use crate::uploader::bilibili::{BiliBili, Vid};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};
use std::time::Duration;

const MEMBER_HOST: &str = "https://member.bilibili.com";
const API_HOST: &str = "https://api.bilibili.com";
const TIMEOUT: Duration = Duration::from_secs(10);
const SORT_TIMEOUT: Duration = Duration::from_secs(15);

fn null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// `list_seasons` 的返回：一页合集。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonPage {
    #[serde(default, deserialize_with = "null_as_default")]
    pub seasons: Vec<SeasonEntry>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonEntry {
    pub season: SeasonInfo,
    #[serde(default, deserialize_with = "null_as_default")]
    pub sections: SectionList,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonInfo {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectionList {
    #[serde(default, deserialize_with = "null_as_default")]
    pub sections: Vec<SectionInfo>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// 合集下的小节；每个合集至少有一个默认小节（通常叫「正片」）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionInfo {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "seasonId")]
    pub season_id: Option<u64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `get_season_section` 的返回：小节信息和其中的稿件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionDetail {
    pub section: SectionInfo,
    #[serde(default, deserialize_with = "null_as_default")]
    pub episodes: Vec<Episode>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// 小节里的一条稿件。`id` 是合集内部的 episode id，不是 aid。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: u64,
    pub aid: u64,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `api.bilibili.com/x/web-interface/view` 的返回，任何公开稿件都能查。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveView {
    pub aid: u64,
    #[serde(default)]
    pub bvid: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub cid: u64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub pages: Vec<ViewPage>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewPage {
    pub cid: u64,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub part: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ArchiveView {
    /// 第一个分 P 的 cid，加入合集时用它。
    pub fn first_cid(&self) -> u64 {
        self.pages.first().map_or(self.cid, |page| page.cid)
    }
}

/// 加入小节时每条稿件的参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeAdd {
    pub aid: u64,
    pub cid: u64,
    pub title: String,
    pub charging_pay: u8,
}

impl From<&ArchiveView> for EpisodeAdd {
    fn from(view: &ArchiveView) -> Self {
        Self {
            aid: view.aid,
            cid: view.first_cid(),
            title: view.title.clone(),
            charging_pay: 0,
        }
    }
}

/// 排序时每条稿件的新位置；`sort` 从 1 开始。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeSort {
    pub id: u64,
    pub sort: u32,
}

/// 合集接口的调用入口，`BiliBili` 上的同名方法都转到这里。
pub struct SeasonApi<'a> {
    bili: &'a BiliBili,
    member: &'a str,
    api: &'a str,
}

impl<'a> SeasonApi<'a> {
    /// 指向其它主机，供测试接本地假服务；`member` 对应 `member.bilibili.com`，
    /// `api` 对应 `api.bilibili.com`。
    #[doc(hidden)]
    pub fn with_hosts(bili: &'a BiliBili, member: &'a str, api: &'a str) -> Self {
        Self { bili, member, api }
    }
}

impl BiliBili {
    pub fn season_api(&self) -> SeasonApi<'_> {
        SeasonApi::with_hosts(self, MEMBER_HOST, API_HOST)
    }

    /// 列出自己的合集，`pn` 从 1 开始。
    pub async fn seasons(&self, pn: u32, ps: u32) -> Result<SeasonPage> {
        self.season_api().seasons(pn, ps).await
    }

    /// 查询小节及其中的稿件；`sort` 目前 B 站只认 `"desc"`。
    pub async fn season_section(
        &self,
        section_id: u64,
        sort: Option<&str>,
    ) -> Result<SectionDetail> {
        self.season_api().season_section(section_id, sort).await
    }

    /// 查询公开稿件信息（标题、cid 等），不要求是自己的稿件。
    pub async fn archive_view(&self, aid: u64) -> Result<ArchiveView> {
        self.season_api().archive_view(aid).await
    }

    /// 同 [`BiliBili::archive_view`]，但也接受 BV 号。
    pub async fn archive_view_by_vid(&self, vid: &Vid) -> Result<ArchiveView> {
        self.season_api().archive_view_by_vid(vid).await
    }

    /// 把稿件加入小节，可一次加入多条。
    pub async fn add_to_season(&self, section_id: u64, episodes: &[EpisodeAdd]) -> Result<()> {
        self.season_api().add_to_season(section_id, episodes).await
    }

    /// 从合集移出一条稿件；参数是 [`Episode::id`]，不是 aid。
    pub async fn remove_from_season(&self, episode_id: u64) -> Result<()> {
        self.season_api().remove_from_season(episode_id).await
    }

    /// 重排小节里的稿件。`sorts` 必须覆盖小节里的全部稿件，
    /// `section_id`、`season_id`、`section_title` 缺一不可，否则 B 站返回 -400。
    pub async fn sort_season_episodes(
        &self,
        section_id: u64,
        season_id: u64,
        sorts: &[EpisodeSort],
        section_title: &str,
    ) -> Result<()> {
        self.season_api()
            .sort_season_episodes(section_id, season_id, sorts, section_title)
            .await
    }
}

impl SeasonApi<'_> {
    pub async fn seasons(&self, pn: u32, ps: u32) -> Result<SeasonPage> {
        let request = self
            .bili
            .client
            .get(format!("{}/x2/creative/web/seasons", self.member))
            .query(&[
                ("pn", pn.to_string()),
                ("ps", ps.to_string()),
                ("order", "mtime".into()),
                ("sort", "desc".into()),
                ("draft", "1".into()),
            ]);
        let data = call(request, TIMEOUT, "list seasons").await?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn season_section(
        &self,
        section_id: u64,
        sort: Option<&str>,
    ) -> Result<SectionDetail> {
        let mut query = vec![("id", section_id.to_string())];
        if let Some(sort) = sort.filter(|sort| !sort.is_empty()) {
            query.push(("sort", sort.to_string()));
        }
        let request = self
            .bili
            .client
            .get(format!("{}/x2/creative/web/season/section", self.member))
            .query(&query);
        let data = call(request, TIMEOUT, "get season section").await?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn archive_view(&self, aid: u64) -> Result<ArchiveView> {
        self.archive_view_by_vid(&Vid::Aid(aid)).await
    }

    pub async fn archive_view_by_vid(&self, vid: &Vid) -> Result<ArchiveView> {
        let query = match vid {
            Vid::Aid(aid) => ("aid", aid.to_string()),
            Vid::Bvid(bvid) => ("bvid", bvid.clone()),
        };
        let request = self
            .bili
            .client
            .get(format!("{}/x/web-interface/view", self.api))
            .query(&[query]);
        let data = call(request, TIMEOUT, "get archive view").await?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn add_to_season(&self, section_id: u64, episodes: &[EpisodeAdd]) -> Result<()> {
        let csrf = self.bili.get_csrf()?;
        let request = self
            .bili
            .client
            .post(format!(
                "{}/x2/creative/web/season/section/episodes/add",
                self.member
            ))
            .query(&[("csrf", csrf)])
            .json(&json!({
                "sectionId": section_id,
                "episodes": episodes,
                "csrf": csrf,
            }));
        call(request, TIMEOUT, "add to season").await?;
        Ok(())
    }

    pub async fn remove_from_season(&self, episode_id: u64) -> Result<()> {
        let csrf = self.bili.get_csrf()?;
        let request = self
            .bili
            .client
            .post(format!(
                "{}/x2/creative/web/season/section/episode/del",
                self.member
            ))
            .form(&[("id", episode_id.to_string()), ("csrf", csrf.to_string())]);
        call(request, TIMEOUT, "remove from season").await?;
        Ok(())
    }

    pub async fn sort_season_episodes(
        &self,
        section_id: u64,
        season_id: u64,
        sorts: &[EpisodeSort],
        section_title: &str,
    ) -> Result<()> {
        let csrf = self.bili.get_csrf()?;
        let request = self
            .bili
            .client
            .post(format!(
                "{}/x2/creative/web/season/section/edit",
                self.member
            ))
            .query(&[("csrf", csrf)])
            .json(&json!({
                "section": {
                    "id": section_id,
                    "type": 1,
                    "seasonId": season_id,
                    "title": section_title,
                },
                "sorts": sorts,
                "captcha_token": "",
            }));
        call(request, SORT_TIMEOUT, "sort season episodes").await?;
        Ok(())
    }
}

/// 发出请求并检查 B 站的 `code`，成功时返回 `data`（可能是 `null`）。
async fn call(request: reqwest::RequestBuilder, timeout: Duration, action: &str) -> Result<Value> {
    let mut body: Value = request
        .timeout(timeout)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match body["code"].as_i64() {
        Some(0) => Ok(body["data"].take()),
        Some(code) => Err(Kind::Custom(format!(
            "{action} failed: code {code}, message {}",
            body["message"].as_str().unwrap_or_default()
        ))),
        None => Err(Kind::Custom(format!(
            "{action} failed: response has no code"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, Method, Uri, header};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    const CSRF: &str = "test-csrf";

    #[derive(Debug, Clone)]
    struct Captured {
        method: Method,
        path: String,
        query: HashMap<String, String>,
        content_type: String,
        body: Bytes,
    }

    impl Captured {
        fn json(&self) -> Value {
            serde_json::from_slice(&self.body).unwrap()
        }

        fn form(&self) -> HashMap<String, String> {
            serde_urlencoded::from_bytes(&self.body).unwrap()
        }
    }

    #[derive(Clone)]
    struct FakeState {
        response: Value,
        captured: Arc<Mutex<Vec<Captured>>>,
    }

    struct FakeServer {
        base: String,
        captured: Arc<Mutex<Vec<Captured>>>,
    }

    impl FakeServer {
        async fn start(response: Value) -> Self {
            async fn handler(
                State(state): State<FakeState>,
                method: Method,
                uri: Uri,
                headers: HeaderMap,
                body: Bytes,
            ) -> axum::Json<Value> {
                let query = uri
                    .query()
                    .map(|q| serde_urlencoded::from_str(q).unwrap())
                    .unwrap_or_default();
                let content_type = headers
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                state.captured.lock().unwrap().push(Captured {
                    method,
                    path: uri.path().to_string(),
                    query,
                    content_type,
                    body,
                });
                axum::Json(state.response.clone())
            }

            let captured = Arc::new(Mutex::new(Vec::new()));
            let app = Router::new().fallback(handler).with_state(FakeState {
                response,
                captured: captured.clone(),
            });
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            Self { base, captured }
        }

        fn only_request(&self) -> Captured {
            let captured = self.captured.lock().unwrap();
            assert_eq!(captured.len(), 1, "expected exactly one request");
            captured[0].clone()
        }
    }

    fn bili() -> BiliBili {
        BiliBili {
            client: reqwest::Client::new(),
            login_info: serde_json::from_value(json!({
                "cookie_info": {"cookies": [
                    {"name": "SESSDATA", "value": "sess"},
                    {"name": "bili_jct", "value": CSRF}
                ]},
                "sso": [],
                "token_info": {
                    "access_token": "",
                    "expires_in": 0,
                    "mid": 0,
                    "refresh_token": ""
                },
                "platform": null
            }))
            .unwrap(),
        }
    }

    fn api<'a>(bili: &'a BiliBili, server: &'a FakeServer) -> SeasonApi<'a> {
        SeasonApi {
            bili,
            member: &server.base,
            api: &server.base,
        }
    }

    fn ok(data: Value) -> Value {
        json!({"code": 0, "message": "0", "ttl": 1, "data": data})
    }

    fn failure() -> Value {
        json!({"code": -101, "message": "账号未登录", "ttl": 1, "data": null})
    }

    fn assert_failure(result: Result<impl std::fmt::Debug>, action: &str) {
        let message = result.unwrap_err().to_string();
        assert!(message.contains(action), "{message}");
        assert!(message.contains("code -101"), "{message}");
    }

    #[tokio::test]
    async fn seasons_request_shape_and_parsing() {
        let server = FakeServer::start(ok(json!({
            "seasons": [{
                "season": {"id": 7320255, "title": "我的合集", "isEnd": 0},
                "sections": {"sections": [
                    {"id": 8081933, "title": "正片", "seasonId": 7320255, "epCount": 2}
                ]},
                "part_episodes": null
            }],
            "total": 1,
            "play_type": 1
        })))
        .await;
        let bili = bili();

        let page = api(&bili, &server).seasons(2, 10).await.unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/x2/creative/web/seasons");
        let expected: HashMap<String, String> = [
            ("pn", "2"),
            ("ps", "10"),
            ("order", "mtime"),
            ("sort", "desc"),
            ("draft", "1"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
        assert_eq!(req.query, expected);

        assert_eq!(page.total, Some(1));
        assert_eq!(page.extra["play_type"], 1);
        let entry = &page.seasons[0];
        assert_eq!(entry.season.id, 7320255);
        assert_eq!(entry.season.title, "我的合集");
        assert_eq!(entry.season.extra["isEnd"], 0);
        let section = &entry.sections.sections[0];
        assert_eq!(section.id, 8081933);
        assert_eq!(section.season_id, Some(7320255));
        assert_eq!(section.extra["epCount"], 2);
    }

    #[tokio::test]
    async fn seasons_accepts_null_list() {
        let server = FakeServer::start(ok(json!({"seasons": null, "total": 0}))).await;
        let bili = bili();
        let page = api(&bili, &server).seasons(1, 30).await.unwrap();
        assert!(page.seasons.is_empty());
    }

    #[tokio::test]
    async fn seasons_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(api(&bili, &server).seasons(1, 30).await, "list seasons");
    }

    #[tokio::test]
    async fn season_section_request_shape_and_parsing() {
        let server = FakeServer::start(ok(json!({
            "section": {"id": 8081933, "title": "正片", "seasonId": 7320255, "type": 1},
            "episodes": [
                {"id": 176218279, "aid": 12345, "cid": 67890, "title": "第一集", "order": 1}
            ]
        })))
        .await;
        let bili = bili();

        let detail = api(&bili, &server)
            .season_section(8081933, Some("desc"))
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/x2/creative/web/season/section");
        assert_eq!(req.query.len(), 2);
        assert_eq!(req.query["id"], "8081933");
        assert_eq!(req.query["sort"], "desc");

        assert_eq!(detail.section.title, "正片");
        assert_eq!(detail.section.season_id, Some(7320255));
        let episode = &detail.episodes[0];
        assert_eq!(episode.id, 176218279);
        assert_eq!(episode.aid, 12345);
        assert_eq!(episode.cid, 67890);
        assert_eq!(episode.extra["order"], 1);
    }

    #[tokio::test]
    async fn season_section_omits_empty_sort_and_accepts_null_episodes() {
        let server = FakeServer::start(ok(json!({
            "section": {"id": 1, "title": "正片"},
            "episodes": null
        })))
        .await;
        let bili = bili();

        let detail = api(&bili, &server)
            .season_section(1, Some(""))
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.query.len(), 1);
        assert_eq!(req.query["id"], "1");
        assert!(detail.episodes.is_empty());
    }

    #[tokio::test]
    async fn season_section_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(
            api(&bili, &server).season_section(1, None).await,
            "get season section",
        );
    }

    #[tokio::test]
    async fn archive_view_request_shape_and_parsing() {
        let server = FakeServer::start(ok(json!({
            "aid": 12345,
            "bvid": "BV1xx411c7mD",
            "title": "测试稿件",
            "cid": 67890,
            "pages": [{"cid": 67890, "page": 1, "part": "P1", "duration": 60}],
            "owner": {"mid": 1}
        })))
        .await;
        let bili = bili();

        let view = api(&bili, &server).archive_view(12345).await.unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/x/web-interface/view");
        assert_eq!(req.query.len(), 1);
        assert_eq!(req.query["aid"], "12345");

        assert_eq!(view.bvid, "BV1xx411c7mD");
        assert_eq!(view.first_cid(), 67890);
        assert_eq!(view.pages[0].extra["duration"], 60);
        assert_eq!(view.extra["owner"]["mid"], 1);
        assert_eq!(
            EpisodeAdd::from(&view),
            EpisodeAdd {
                aid: 12345,
                cid: 67890,
                title: "测试稿件".into(),
                charging_pay: 0,
            }
        );
    }

    #[tokio::test]
    async fn archive_view_by_bvid_queries_bvid() {
        let server = FakeServer::start(ok(json!({
            "aid": 12345,
            "bvid": "BV1xx411c7mD",
            "title": "测试稿件",
            "pages": [{"cid": 67890}]
        })))
        .await;
        let bili = bili();

        let view = api(&bili, &server)
            .archive_view_by_vid(&Vid::Bvid("BV1xx411c7mD".into()))
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.path, "/x/web-interface/view");
        assert_eq!(req.query.len(), 1);
        assert_eq!(req.query["bvid"], "BV1xx411c7mD");
        assert_eq!(view.aid, 12345);
        assert_eq!(view.first_cid(), 67890);
    }

    #[tokio::test]
    async fn archive_view_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(
            api(&bili, &server).archive_view(1).await,
            "get archive view",
        );
    }

    #[tokio::test]
    async fn add_to_season_request_shape() {
        let server = FakeServer::start(ok(Value::Null)).await;
        let bili = bili();
        let episodes = [EpisodeAdd {
            aid: 12345,
            cid: 67890,
            title: "测试稿件".into(),
            charging_pay: 0,
        }];

        api(&bili, &server)
            .add_to_season(8081933, &episodes)
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.path, "/x2/creative/web/season/section/episodes/add");
        assert_eq!(req.query.len(), 1);
        assert_eq!(req.query["csrf"], CSRF);
        assert!(req.content_type.starts_with("application/json"));
        assert_eq!(
            req.json(),
            json!({
                "sectionId": 8081933,
                "episodes": [
                    {"aid": 12345, "cid": 67890, "title": "测试稿件", "charging_pay": 0}
                ],
                "csrf": CSRF,
            })
        );
    }

    #[tokio::test]
    async fn add_to_season_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(
            api(&bili, &server).add_to_season(1, &[]).await,
            "add to season",
        );
    }

    #[tokio::test]
    async fn remove_from_season_request_shape() {
        let server = FakeServer::start(ok(Value::Null)).await;
        let bili = bili();

        api(&bili, &server)
            .remove_from_season(176218279)
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.path, "/x2/creative/web/season/section/episode/del");
        assert!(req.query.is_empty());
        assert_eq!(req.content_type, "application/x-www-form-urlencoded");
        let form = req.form();
        assert_eq!(form.len(), 2);
        assert_eq!(form["id"], "176218279");
        assert_eq!(form["csrf"], CSRF);
    }

    #[tokio::test]
    async fn remove_from_season_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(
            api(&bili, &server).remove_from_season(1).await,
            "remove from season",
        );
    }

    #[tokio::test]
    async fn sort_season_episodes_request_shape() {
        let server = FakeServer::start(ok(Value::Null)).await;
        let bili = bili();
        let sorts = [
            EpisodeSort { id: 2, sort: 1 },
            EpisodeSort { id: 1, sort: 2 },
        ];

        api(&bili, &server)
            .sort_season_episodes(8081933, 7320255, &sorts, "正片")
            .await
            .unwrap();

        let req = server.only_request();
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.path, "/x2/creative/web/season/section/edit");
        assert_eq!(req.query.len(), 1);
        assert_eq!(req.query["csrf"], CSRF);
        assert!(req.content_type.starts_with("application/json"));
        assert_eq!(
            req.json(),
            json!({
                "section": {"id": 8081933, "type": 1, "seasonId": 7320255, "title": "正片"},
                "sorts": [{"id": 2, "sort": 1}, {"id": 1, "sort": 2}],
                "captcha_token": "",
            })
        );
    }

    #[tokio::test]
    async fn sort_season_episodes_reports_api_error() {
        let server = FakeServer::start(failure()).await;
        let bili = bili();
        assert_failure(
            api(&bili, &server)
                .sort_season_episodes(1, 2, &[], "正片")
                .await,
            "sort season episodes",
        );
    }

    #[tokio::test]
    async fn write_calls_require_csrf_cookie() {
        let server = FakeServer::start(ok(Value::Null)).await;
        let mut bili = bili();
        bili.login_info.cookie_info = json!({"cookies": []});

        let api = api(&bili, &server);
        assert!(api.add_to_season(1, &[]).await.is_err());
        assert!(api.remove_from_season(1).await.is_err());
        assert!(api.sort_season_episodes(1, 2, &[], "正片").await.is_err());
        assert!(server.captured.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn http_error_status_is_reported() {
        let app =
            Router::new().fallback(|| async { (axum::http::StatusCode::PRECONDITION_FAILED, "") });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let bili = bili();
        let api = SeasonApi {
            bili: &bili,
            member: &base,
            api: &base,
        };
        let message = api.seasons(1, 30).await.unwrap_err().to_string();
        assert!(message.contains("412"), "{message}");
    }

    /// 真实账号只读冒烟：`BILIUP_SEASON_COOKIES=/path/to/cookies.json cargo test -p biliup -- --ignored season_live`
    #[tokio::test]
    #[ignore = "needs a real Bilibili cookie file in BILIUP_SEASON_COOKIES"]
    async fn season_live_read_only() {
        let path = std::env::var("BILIUP_SEASON_COOKIES")
            .expect("set BILIUP_SEASON_COOKIES to a cookies.json path");
        let bili = crate::credential::login_by_cookies(&path, None)
            .await
            .unwrap();

        let page = bili.seasons(1, 30).await.unwrap();
        println!("seasons: {}", page.seasons.len());
        if let Some(section) = page
            .seasons
            .first()
            .and_then(|entry| entry.sections.sections.first())
        {
            let detail = bili.season_section(section.id, None).await.unwrap();
            println!("section {} episodes: {}", section.id, detail.episodes.len());
            if let Some(episode) = detail.episodes.first() {
                let view = bili.archive_view(episode.aid).await.unwrap();
                assert_eq!(view.aid, episode.aid);
            }
        }
    }
}
