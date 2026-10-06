//! `biliup season …`：管理自己的合集（列合集、查小节、加入 / 移出稿件、排序）。

use crate::server::errors::{AppError, AppResult};
use crate::uploader::login_by_cookies;
use biliup::uploader::bilibili::Vid;
use biliup::uploader::season::{
    Episode, EpisodeAdd, EpisodeSort, SeasonApi, SeasonEntry, SeasonPage, SectionDetail,
};
use clap::{Args, Subcommand};
use error_stack::{Report, ResultExt, bail};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

const PAGE_SIZE: u32 = 30;
const MAX_PAGES: u32 = 100;
const DEFAULT_SECTION_TITLE: &str = "正片";

#[derive(Args, Debug)]
pub struct SeasonArgs {
    /// 以 JSON 输出
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub action: SeasonAction,
}

#[derive(Subcommand, Debug)]
pub enum SeasonAction {
    /// 列出自己的合集及其小节
    List,
    /// 查看合集的各个小节和其中的稿件（episode id 用于 remove / sort）
    Sections {
        /// 合集 ID，见 `biliup season list`
        season_id: u64,
    },
    /// 把稿件加入小节，cid 和标题自动获取
    Add {
        /// 小节 ID，见 `biliup season list`
        section_id: u64,

        /// 稿件 av 或 bv 号，可重复指定以一次加入多个
        #[arg(short, long = "vid", value_name = "VID", required = true)]
        vids: Vec<Vid>,
    },
    /// 从合集移出一个稿件
    Remove {
        /// 稿件在合集里的 episode id（不是 aid），见 `biliup season sections`
        episode_id: u64,
    },
    /// 重排小节里的稿件：列出的 episode id 按给定顺序排在最前，其余保持原有顺序
    Sort {
        /// 小节 ID
        section_id: u64,

        /// 新顺序里排在最前的 episode id
        #[arg(required_unless_present = "reverse")]
        episode_ids: Vec<u64>,

        /// 把现有顺序整体倒过来
        #[arg(long, conflicts_with = "episode_ids")]
        reverse: bool,

        /// 合集 ID；B 站返回的小节信息里没有时才需要指定
        #[arg(long)]
        season_id: Option<u64>,
    },
}

pub async fn run(args: SeasonArgs, user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<()> {
    let bili = login_by_cookies(user_cookie, proxy).await?;
    execute(&bili.season_api(), args, &mut std::io::stdout()).await
}

async fn execute(api: &SeasonApi<'_>, args: SeasonArgs, out: &mut impl Write) -> AppResult<()> {
    let json = args.json;
    match args.action {
        SeasonAction::List => {
            let seasons = all_seasons(api).await?;
            if json {
                return print_json(out, &seasons);
            }
            print_seasons(out, &seasons)
        }
        SeasonAction::Sections { season_id } => {
            let seasons = all_seasons(api).await?;
            let Some(entry) = seasons.into_iter().find(|e| e.season.id == season_id) else {
                bail!(AppError::Custom(format!(
                    "没有找到合集 {season_id}；用 `biliup season list` 查看自己的合集"
                )));
            };
            let mut sections = Vec::new();
            for section in &entry.sections.sections {
                sections.push(section_detail(api, section.id).await?);
            }
            if json {
                return print_json(out, &json!({"season": entry.season, "sections": sections}));
            }
            writeln!(out, "合集 {}  {}", entry.season.id, entry.season.title).map_err(io)?;
            for detail in &sections {
                print_section(out, detail)?;
            }
            Ok(())
        }
        SeasonAction::Add { section_id, vids } => {
            let mut episodes = Vec::with_capacity(vids.len());
            for vid in &vids {
                let view = api
                    .archive_view_by_vid(vid)
                    .await
                    .change_context_lazy(|| AppError::Custom(format!("查询稿件 {vid} 失败")))?;
                episodes.push(EpisodeAdd::from(&view));
            }
            api.add_to_season(section_id, &episodes)
                .await
                .change_context_lazy(|| AppError::Custom(format!("加入小节 {section_id} 失败")))?;
            if json {
                return print_json(
                    out,
                    &json!({"section_id": section_id, "episodes": episodes}),
                );
            }
            for ep in &episodes {
                writeln!(out, "av{}  cid {}  {}", ep.aid, ep.cid, ep.title).map_err(io)?;
            }
            writeln!(out, "已将 {} 个稿件加入小节 {section_id}", episodes.len()).map_err(io)
        }
        SeasonAction::Remove { episode_id } => {
            api.remove_from_season(episode_id)
                .await
                .change_context_lazy(|| {
                    AppError::Custom(format!("移出 episode {episode_id} 失败"))
                })?;
            if json {
                return print_json(out, &json!({"episode_id": episode_id}));
            }
            writeln!(out, "已从合集移出 episode {episode_id}").map_err(io)
        }
        SeasonAction::Sort {
            section_id,
            episode_ids,
            reverse,
            season_id,
        } => {
            let detail = section_detail(api, section_id).await?;
            let Some(season_id) = season_id.or(detail.section.season_id) else {
                bail!(AppError::Custom(format!(
                    "B 站没有返回小节 {section_id} 所属的合集 ID，请用 --season-id 指定"
                )));
            };
            let current: Vec<u64> = detail.episodes.iter().map(|e| e.id).collect();
            let order = new_order(&current, &episode_ids, reverse).map_err(AppError::Custom)?;
            let sorts: Vec<EpisodeSort> = order
                .iter()
                .zip(1..)
                .map(|(&id, sort)| EpisodeSort { id, sort })
                .collect();
            let title = match detail.section.title.as_str() {
                "" => DEFAULT_SECTION_TITLE,
                title => title,
            };
            api.sort_season_episodes(section_id, season_id, &sorts, title)
                .await
                .change_context_lazy(|| AppError::Custom(format!("重排小节 {section_id} 失败")))?;
            if json {
                return print_json(
                    out,
                    &json!({"section_id": section_id, "season_id": season_id, "sorts": sorts}),
                );
            }
            writeln!(out, "已重排小节 {section_id}（合集 {season_id}）：").map_err(io)?;
            for (index, id) in order.iter().enumerate() {
                let episode = detail.episodes.iter().find(|e| e.id == *id);
                print_episode(out, index + 1, episode.expect("order only holds known ids"))?;
            }
            Ok(())
        }
    }
}

async fn all_seasons(api: &SeasonApi<'_>) -> AppResult<Vec<SeasonEntry>> {
    let mut seasons = Vec::new();
    for pn in 1..=MAX_PAGES {
        let page = api
            .seasons(pn, PAGE_SIZE)
            .await
            .change_context_lazy(|| AppError::Custom("获取合集列表失败".into()))?;
        let total = page_total(&page);
        let fetched = page.seasons.len();
        seasons.extend(page.seasons);
        if fetched < PAGE_SIZE as usize || total.is_some_and(|t| seasons.len() as u64 >= t) {
            break;
        }
    }
    Ok(seasons)
}

/// 总数可能在 `total`，也可能在 `page.total`。
fn page_total(page: &SeasonPage) -> Option<u64> {
    page.total
        .or_else(|| page.extra.get("page")?.get("total")?.as_u64())
}

async fn section_detail(api: &SeasonApi<'_>, section_id: u64) -> AppResult<SectionDetail> {
    api.season_section(section_id, None)
        .await
        .change_context_lazy(|| AppError::Custom(format!("获取小节 {section_id} 失败")))
}

/// `front` 里的 episode 按给定顺序排在最前，其余保持 `current` 里的相对顺序；
/// `reverse` 时把 `current` 整体倒过来。
fn new_order(current: &[u64], front: &[u64], reverse: bool) -> Result<Vec<u64>, String> {
    if reverse {
        return Ok(current.iter().rev().copied().collect());
    }
    let mut seen = HashSet::new();
    for id in front {
        if !current.contains(id) {
            return Err(format!("episode {id} 不在这个小节里"));
        }
        if !seen.insert(*id) {
            return Err(format!("episode {id} 重复出现"));
        }
    }
    Ok(front
        .iter()
        .copied()
        .chain(current.iter().copied().filter(|id| !seen.contains(id)))
        .collect())
}

fn print_seasons(out: &mut impl Write, seasons: &[SeasonEntry]) -> AppResult<()> {
    if seasons.is_empty() {
        return writeln!(out, "还没有合集；可以在 B 站创作中心创建。").map_err(io);
    }
    for entry in seasons {
        writeln!(out, "合集 {}  {}", entry.season.id, entry.season.title).map_err(io)?;
        for section in &entry.sections.sections {
            writeln!(out, "  小节 {}  {}", section.id, section.title).map_err(io)?;
        }
    }
    Ok(())
}

fn print_section(out: &mut impl Write, detail: &SectionDetail) -> AppResult<()> {
    let section = &detail.section;
    writeln!(
        out,
        "  小节 {}  {}（{} 个稿件）",
        section.id,
        section.title,
        detail.episodes.len()
    )
    .map_err(io)?;
    for (index, episode) in detail.episodes.iter().enumerate() {
        print_episode(out, index + 1, episode)?;
    }
    Ok(())
}

fn print_episode(out: &mut impl Write, index: usize, episode: &Episode) -> AppResult<()> {
    writeln!(
        out,
        "    {index:>3}. episode {}  av{}  {}",
        episode.id, episode.aid, episode.title
    )
    .map_err(io)
}

fn print_json(out: &mut impl Write, value: &impl Serialize) -> AppResult<()> {
    let text = serde_json::to_string_pretty(value).change_context(AppError::Unknown)?;
    writeln!(out, "{text}").map_err(io)
}

fn io(error: std::io::Error) -> Report<AppError> {
    Report::new(error).change_context(AppError::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{Method, Uri};
    use biliup::uploader::bilibili::BiliBili;
    use clap::Parser;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    const CSRF: &str = "test-csrf";

    fn parse(args: &[&str]) -> SeasonArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Season(args) => args,
            _ => panic!("expected the season command"),
        }
    }

    #[test]
    fn parses_all_subcommands() {
        let args = parse(&["biliup", "season", "list"]);
        assert!(!args.json);
        assert!(matches!(args.action, SeasonAction::List));

        let args = parse(&["biliup", "season", "sections", "7320255", "--json"]);
        assert!(args.json);
        assert!(matches!(
            args.action,
            SeasonAction::Sections { season_id: 7320255 }
        ));

        let args = parse(&[
            "biliup",
            "season",
            "add",
            "8081933",
            "--vid",
            "BV1xx411c7mD",
            "-v",
            "av12345",
        ]);
        let SeasonAction::Add { section_id, vids } = args.action else {
            panic!("expected add");
        };
        assert_eq!(section_id, 8081933);
        assert_eq!(
            vids,
            [Vid::Bvid("BV1xx411c7mD".into()), Vid::Aid(12345)].to_vec()
        );

        let args = parse(&["biliup", "season", "remove", "176218279"]);
        assert!(matches!(
            args.action,
            SeasonAction::Remove {
                episode_id: 176218279
            }
        ));

        let args = parse(&["biliup", "season", "sort", "8081933", "3", "1"]);
        let SeasonAction::Sort {
            section_id,
            episode_ids,
            reverse,
            season_id,
        } = args.action
        else {
            panic!("expected sort");
        };
        assert_eq!((section_id, reverse, season_id), (8081933, false, None));
        assert_eq!(episode_ids, [3, 1]);

        let args = parse(&[
            "biliup",
            "season",
            "--json",
            "sort",
            "8081933",
            "--reverse",
            "--season-id",
            "7320255",
        ]);
        assert!(args.json);
        assert!(matches!(
            args.action,
            SeasonAction::Sort {
                reverse: true,
                season_id: Some(7320255),
                ..
            }
        ));
    }

    #[test]
    fn global_cookie_flag_is_accepted_before_season() {
        let cli = Cli::try_parse_from(["biliup", "-u", "/tmp/a.json", "season", "list"]).unwrap();
        assert_eq!(cli.user_cookie, std::path::Path::new("/tmp/a.json"));
        assert!(matches!(cli.command, Commands::Season(_)));
    }

    #[test]
    fn rejects_invalid_arguments() {
        for args in [
            &["biliup", "season", "add", "8081933"][..],
            &["biliup", "season", "add", "8081933", "--vid", "notavid"],
            &["biliup", "season", "sort", "8081933"],
            &["biliup", "season", "sort", "8081933", "1", "--reverse"],
            &["biliup", "season", "remove"],
            &["biliup", "season"],
        ] {
            assert!(Cli::try_parse_from(args).is_err(), "{args:?} should fail");
        }
    }

    #[test]
    fn new_order_moves_listed_episodes_to_front() {
        assert_eq!(
            new_order(&[1, 2, 3, 4], &[3, 1], false).unwrap(),
            [3, 1, 2, 4]
        );
        assert_eq!(new_order(&[1, 2, 3], &[], true).unwrap(), [3, 2, 1]);
        assert!(new_order(&[1, 2], &[5], false).unwrap_err().contains("5"));
        assert!(
            new_order(&[1, 2], &[1, 1], false)
                .unwrap_err()
                .contains("重复")
        );
    }

    #[derive(Debug, Clone)]
    struct Captured {
        method: Method,
        path: String,
        query: HashMap<String, String>,
        body: Bytes,
    }

    #[derive(Clone)]
    struct FakeState {
        routes: Arc<HashMap<&'static str, Value>>,
        captured: Arc<Mutex<Vec<Captured>>>,
    }

    /// 按路径返回固定响应的 B 站假服务，记录收到的每个请求。
    struct FakeBili {
        base: String,
        captured: Arc<Mutex<Vec<Captured>>>,
    }

    impl FakeBili {
        async fn start(routes: &[(&'static str, Value)]) -> Self {
            async fn handler(
                State(state): State<FakeState>,
                method: Method,
                uri: Uri,
                body: Bytes,
            ) -> axum::Json<Value> {
                let query = uri
                    .query()
                    .map(|q| serde_urlencoded::from_str(q).unwrap())
                    .unwrap_or_default();
                let response = state
                    .routes
                    .get(uri.path())
                    .cloned()
                    .unwrap_or_else(|| json!({"code": -404, "message": "啥都木有"}));
                state.captured.lock().unwrap().push(Captured {
                    method,
                    path: uri.path().to_string(),
                    query,
                    body,
                });
                axum::Json(response)
            }

            let captured = Arc::new(Mutex::new(Vec::new()));
            let app = Router::new().fallback(handler).with_state(FakeState {
                routes: Arc::new(routes.iter().cloned().collect()),
                captured: captured.clone(),
            });
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            Self { base, captured }
        }

        fn requests(&self) -> Vec<Captured> {
            self.captured.lock().unwrap().clone()
        }

        async fn run(&self, args: &[&str]) -> AppResult<String> {
            let bili = bili();
            let api = SeasonApi::with_hosts(&bili, &self.base, &self.base);
            let mut out = Vec::new();
            execute(&api, parse(args), &mut out).await?;
            Ok(String::from_utf8(out).unwrap())
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

    fn ok(data: Value) -> Value {
        json!({"code": 0, "message": "0", "ttl": 1, "data": data})
    }

    const SEASONS: &str = "/x2/creative/web/seasons";
    const SECTION: &str = "/x2/creative/web/season/section";
    const VIEW: &str = "/x/web-interface/view";
    const ADD: &str = "/x2/creative/web/season/section/episodes/add";
    const DEL: &str = "/x2/creative/web/season/section/episode/del";
    const EDIT: &str = "/x2/creative/web/season/section/edit";

    fn seasons_page() -> Value {
        ok(json!({
            "seasons": [
                {
                    "season": {"id": 7320255, "title": "我的合集"},
                    "sections": {"sections": [
                        {"id": 8081933, "title": "正片", "seasonId": 7320255}
                    ]}
                },
                {"season": {"id": 42, "title": "空合集"}, "sections": null}
            ],
            "page": {"pn": 1, "ps": 30, "total": 2}
        }))
    }

    fn section() -> Value {
        ok(json!({
            "section": {"id": 8081933, "title": "正片", "seasonId": 7320255},
            "episodes": [
                {"id": 11, "aid": 101, "cid": 1001, "title": "第一集"},
                {"id": 22, "aid": 102, "cid": 1002, "title": "第二集"},
                {"id": 33, "aid": 103, "cid": 1003, "title": "第三集"}
            ]
        }))
    }

    #[tokio::test]
    async fn list_prints_seasons_and_sections() {
        let fake = FakeBili::start(&[(SEASONS, seasons_page())]).await;

        let out = fake.run(&["biliup", "season", "list"]).await.unwrap();

        assert_eq!(
            out,
            "合集 7320255  我的合集\n  小节 8081933  正片\n合集 42  空合集\n"
        );
        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::GET);
        assert_eq!(requests[0].query["pn"], "1");
        assert_eq!(requests[0].query["ps"], PAGE_SIZE.to_string());
    }

    #[tokio::test]
    async fn list_json_is_machine_readable() {
        let fake = FakeBili::start(&[(SEASONS, seasons_page())]).await;

        let out = fake
            .run(&["biliup", "season", "list", "--json"])
            .await
            .unwrap();

        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value[0]["season"]["id"], 7320255);
        assert_eq!(value[0]["sections"]["sections"][0]["id"], 8081933);
        assert_eq!(value[1]["season"]["title"], "空合集");
    }

    #[tokio::test]
    async fn list_paging_stops_on_short_page_or_page_cap() {
        let full: Vec<Value> = (1..=PAGE_SIZE)
            .map(|id| json!({"season": {"id": id, "title": "x"}}))
            .collect();
        let fake = FakeBili::start(&[(SEASONS, ok(json!({"seasons": full})))]).await;

        // 假服务每页都返回满页且不给总数，靠 MAX_PAGES 兜底停下来。
        fake.run(&["biliup", "season", "list"]).await.unwrap();
        assert_eq!(fake.requests().len(), MAX_PAGES as usize);

        let fake = FakeBili::start(&[(SEASONS, ok(json!({"seasons": [], "total": 0})))]).await;
        let out = fake.run(&["biliup", "season", "list"]).await.unwrap();
        assert!(out.contains("还没有合集"));
        assert_eq!(fake.requests().len(), 1);
    }

    #[tokio::test]
    async fn sections_prints_episodes_of_every_section() {
        let fake = FakeBili::start(&[(SEASONS, seasons_page()), (SECTION, section())]).await;

        let out = fake
            .run(&["biliup", "season", "sections", "7320255"])
            .await
            .unwrap();

        assert_eq!(
            out,
            "合集 7320255  我的合集\n  小节 8081933  正片（3 个稿件）\n      1. episode 11  av101  第一集\n      2. episode 22  av102  第二集\n      3. episode 33  av103  第三集\n"
        );
        let requests = fake.requests();
        assert_eq!(requests[1].path, SECTION);
        assert_eq!(requests[1].query["id"], "8081933");
    }

    #[tokio::test]
    async fn sections_reports_unknown_season() {
        let fake = FakeBili::start(&[(SEASONS, seasons_page())]).await;

        let err = fake
            .run(&["biliup", "season", "sections", "999"])
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("没有找到合集 999"), "{err:?}");
    }

    #[tokio::test]
    async fn add_fetches_cid_and_title_from_view() {
        let fake = FakeBili::start(&[
            (
                VIEW,
                ok(json!({
                    "aid": 12345,
                    "bvid": "BV1xx411c7mD",
                    "title": "测试稿件",
                    "cid": 1,
                    "pages": [{"cid": 67890, "page": 1}, {"cid": 67891, "page": 2}]
                })),
            ),
            (ADD, ok(Value::Null)),
        ])
        .await;

        let out = fake
            .run(&[
                "biliup",
                "season",
                "add",
                "8081933",
                "--vid",
                "BV1xx411c7mD",
            ])
            .await
            .unwrap();

        assert_eq!(
            out,
            "av12345  cid 67890  测试稿件\n已将 1 个稿件加入小节 8081933\n"
        );
        let requests = fake.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path, VIEW);
        assert_eq!(requests[0].query["bvid"], "BV1xx411c7mD");
        assert_eq!(requests[1].method, Method::POST);
        assert_eq!(requests[1].path, ADD);
        assert_eq!(requests[1].query["csrf"], CSRF);
        let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(
            body,
            json!({
                "sectionId": 8081933,
                "episodes": [{"aid": 12345, "cid": 67890, "title": "测试稿件", "charging_pay": 0}],
                "csrf": CSRF,
            })
        );
    }

    #[tokio::test]
    async fn add_by_aid_and_json_output() {
        let fake = FakeBili::start(&[
            (
                VIEW,
                ok(json!({"aid": 12345, "title": "测试稿件", "pages": [{"cid": 67890}]})),
            ),
            (ADD, ok(Value::Null)),
        ])
        .await;

        let out = fake
            .run(&[
                "biliup", "season", "add", "8081933", "-v", "av12345", "--json",
            ])
            .await
            .unwrap();

        assert_eq!(fake.requests()[0].query["aid"], "12345");
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["section_id"], 8081933);
        assert_eq!(value["episodes"][0]["cid"], 67890);
    }

    #[tokio::test]
    async fn add_does_not_submit_when_view_fails() {
        let fake = FakeBili::start(&[(ADD, ok(Value::Null))]).await;

        let err = fake
            .run(&["biliup", "season", "add", "8081933", "--vid", "av1"])
            .await
            .unwrap_err();

        let message = format!("{err:?}");
        assert!(message.contains("查询稿件 aid=1 失败"), "{message}");
        assert!(message.contains("code -404"), "{message}");
        assert!(fake.requests().iter().all(|r| r.path != ADD));
    }

    #[tokio::test]
    async fn remove_posts_episode_id() {
        let fake = FakeBili::start(&[(DEL, ok(Value::Null))]).await;

        let out = fake
            .run(&["biliup", "season", "remove", "22"])
            .await
            .unwrap();

        assert_eq!(out, "已从合集移出 episode 22\n");
        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        let form: HashMap<String, String> =
            serde_urlencoded::from_bytes(&requests[0].body).unwrap();
        assert_eq!(form["id"], "22");
        assert_eq!(form["csrf"], CSRF);
    }

    #[tokio::test]
    async fn sort_sends_every_episode_in_new_order() {
        let fake = FakeBili::start(&[(SECTION, section()), (EDIT, ok(Value::Null))]).await;

        let out = fake
            .run(&["biliup", "season", "sort", "8081933", "33"])
            .await
            .unwrap();

        assert_eq!(
            out,
            "已重排小节 8081933（合集 7320255）：\n      1. episode 33  av103  第三集\n      2. episode 11  av101  第一集\n      3. episode 22  av102  第二集\n"
        );
        let requests = fake.requests();
        assert_eq!(requests[1].path, EDIT);
        let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(
            body,
            json!({
                "section": {"id": 8081933, "type": 1, "seasonId": 7320255, "title": "正片"},
                "sorts": [{"id": 33, "sort": 1}, {"id": 11, "sort": 2}, {"id": 22, "sort": 3}],
                "captcha_token": "",
            })
        );
    }

    #[tokio::test]
    async fn sort_needs_season_id_when_section_lacks_it() {
        let detail = ok(json!({
            "section": {"id": 8081933, "title": ""},
            "episodes": [{"id": 11, "aid": 101}, {"id": 22, "aid": 102}]
        }));
        let fake = FakeBili::start(&[(SECTION, detail), (EDIT, ok(Value::Null))]).await;

        let err = fake
            .run(&["biliup", "season", "sort", "8081933", "--reverse"])
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("--season-id"), "{err:?}");

        fake.run(&[
            "biliup",
            "season",
            "sort",
            "8081933",
            "--reverse",
            "--season-id",
            "7320255",
        ])
        .await
        .unwrap();
        let requests = fake.requests();
        let body: Value = serde_json::from_slice(&requests.last().unwrap().body).unwrap();
        assert_eq!(body["section"]["seasonId"], 7320255);
        assert_eq!(body["section"]["title"], DEFAULT_SECTION_TITLE);
        assert_eq!(
            body["sorts"],
            json!([{"id": 22, "sort": 1}, {"id": 11, "sort": 2}])
        );
    }

    #[tokio::test]
    async fn sort_rejects_unknown_episode_without_submitting() {
        let fake = FakeBili::start(&[(SECTION, section()), (EDIT, ok(Value::Null))]).await;

        let err = fake
            .run(&["biliup", "season", "sort", "8081933", "99"])
            .await
            .unwrap_err();

        assert!(format!("{err:?}").contains("episode 99"), "{err:?}");
        assert!(fake.requests().iter().all(|r| r.path != EDIT));
    }
}
