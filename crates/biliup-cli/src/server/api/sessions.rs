//! 切片工作台的场次接口：场次列表 / 详情、可落刀位置、DVR 回看流。
//!
//! 全部只按场次 id 与场次时间（毫秒）寻址，不接受文件路径。
//! 回看流是 chunked 响应，没有 `Content-Length`，`--auth` 下会话失效时由访问控制层截断。

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::dvr::{self, OpenError};
use crate::server::workbench::store::{self, SegmentRow, SegmentState, SessionListRow};
use crate::server::workbench::{live, segment_index, session_keyframes};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
use tracing::{info, warn};

/// 进程内同时允许的 DVR 回看连接数（所有场次合计），超出返回 429。
pub const MAX_DVR_CONNECTIONS: usize = 8;
pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_PAGE_SIZE: u32 = 100;
/// 一次最多返回这么多个可落刀位置（1080p 一小时大约 1000–2500 个）。
pub const MAX_KEYFRAMES: usize = 20_000;

fn dvr_limit() -> &'static Arc<Semaphore> {
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    LIMIT.get_or_init(|| Arc::new(Semaphore::new(MAX_DVR_CONNECTIONS)))
}

fn internal(error: impl std::fmt::Display) -> Response {
    warn!(%error, "场次接口出错");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "场次不存在").into_response()
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: i64,
    pub streamer_id: Option<i64>,
    /// 开播时记下的主播名（主播删掉之后仍在）。
    pub streamer_name: String,
    pub title: String,
    /// Unix 毫秒。
    pub started_at: i64,
    /// Unix 毫秒；进行中为 `null`。
    pub ended_at: Option<i64>,
    pub retain_until: Option<i64>,
    /// 本进程正在录这一场。
    pub recording: bool,
    /// 时间轴长度（毫秒）：可读分段的最远位置。
    pub duration_ms: i64,
    pub segment_count: i64,
    pub bytes: i64,
}

impl From<SessionListRow> for SessionSummary {
    fn from(row: SessionListRow) -> Self {
        Self {
            recording: live::is_recording(row.id),
            id: row.id,
            streamer_id: row.streamer_id,
            streamer_name: row.streamer_name,
            title: row.title,
            started_at: row.started_at,
            ended_at: row.ended_at,
            retain_until: row.retain_until,
            duration_ms: row.end_ms,
            segment_count: row.segment_count,
            bytes: row.bytes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SessionPage {
    pub items: Vec<SessionSummary>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct SessionListQuery {
    pub streamer_id: Option<i64>,
    /// 从 1 开始。
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// `GET /v1/sessions?streamer_id=&page=&page_size=`
pub async fn list_sessions(
    State(pool): State<ConnectionPool>,
    Query(query): Query<SessionListQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page as i64 - 1) * page_size as i64;
    match store::list_sessions(&pool, query.streamer_id, page_size as i64, offset).await {
        Ok((rows, total)) => Json(SessionPage {
            items: rows.into_iter().map(SessionSummary::from).collect(),
            total,
            page,
            page_size,
        })
        .into_response(),
        Err(e) => internal(e),
    }
}

#[derive(Debug, Serialize)]
pub struct SegmentView {
    pub id: i64,
    /// `flv` / `ts` / `mp4` / `mkv`
    pub container: String,
    /// `recording` / `finished` / `missing` / `deleted` / `pending_delete`；
    /// 只有 `recording`、`finished` 能回看和剪。
    pub state: &'static str,
    pub start_ms: i64,
    /// 正在写的分段按已写入的内容时长算。
    pub end_ms: Option<i64>,
    pub bytes: Option<i64>,
    /// 与上一段之间断流的时长（毫秒）。
    pub gap_before_ms: i64,
    /// 文件名（不含目录）。
    pub file_name: String,
    pub has_danmaku: bool,
}

#[derive(Debug, Serialize)]
pub struct Gap {
    pub from_ms: i64,
    pub to_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    #[serde(flatten)]
    pub summary: SessionSummary,
    pub segments: Vec<SegmentView>,
    /// 断流缺口：没有录到内容的时间段。
    pub gaps: Vec<Gap>,
}

async fn segment_view(row: SegmentRow) -> SegmentView {
    let path = std::path::PathBuf::from(&row.path);
    let (mut end_ms, mut bytes) = (row.end_ms, row.bytes);
    if row.state == SegmentState::Recording {
        if let Ok(index) = segment_index(&row).await {
            end_ms = Some(row.start_ms + index.duration_ms as i64);
        }
        bytes = tokio::fs::metadata(&path)
            .await
            .ok()
            .map(|m| m.len() as i64);
    }
    SegmentView {
        id: row.id,
        container: row.container,
        state: row.state.as_str(),
        start_ms: row.start_ms,
        end_ms,
        bytes,
        gap_before_ms: row.gap_before_ms,
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        has_danmaku: row.danmaku_path.is_some(),
    }
}

/// `GET /v1/sessions/{id}`
pub async fn get_session(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    let summary = match store::session_summary(&pool, id).await {
        Ok(Some(row)) => SessionSummary::from(row),
        Ok(None) => return not_found(),
        Err(e) => return internal(e),
    };
    let rows = match store::session_segments(&pool, id).await {
        Ok(rows) => rows,
        Err(e) => return internal(e),
    };
    let mut segments = Vec::with_capacity(rows.len());
    for row in rows {
        segments.push(segment_view(row).await);
    }
    let gaps = segments
        .iter()
        .filter(|s| s.gap_before_ms > 0)
        .map(|s| Gap {
            from_ms: s.start_ms - s.gap_before_ms,
            to_ms: s.start_ms,
        })
        .collect();
    let mut summary = summary;
    if let Some(end) = segments
        .iter()
        .filter(|s| matches!(s.state, "recording" | "finished"))
        .filter_map(|s| s.end_ms)
        .max()
    {
        summary.duration_ms = summary.duration_ms.max(end);
    }
    Json(SessionDetail {
        summary,
        segments,
        gaps,
    })
    .into_response()
}

#[derive(Debug, Default, Deserialize)]
pub struct RangeQuery {
    /// 场次时间（毫秒），缺省 0。
    pub from: Option<i64>,
    /// 场次时间（毫秒），缺省到最后。
    pub to: Option<i64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct KeyframeView {
    pub t_ms: i64,
    pub segment_id: i64,
}

#[derive(Debug, Serialize)]
pub struct KeyframeList {
    pub keyframes: Vec<KeyframeView>,
    /// 超过 [`MAX_KEYFRAMES`] 被截断了，请缩小范围。
    pub truncated: bool,
}

/// `GET /v1/sessions/{id}/keyframes?from=&to=`：场次时间 `[from, to]` 内可以落刀的位置。
pub async fn get_session_keyframes(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Query(query): Query<RangeQuery>,
) -> Response {
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(),
        Err(e) => return internal(e),
    }
    let from = query.from.unwrap_or(0).max(0);
    let to = query.to.unwrap_or(i64::MAX);
    if to < from {
        return (StatusCode::BAD_REQUEST, "to 不能早于 from").into_response();
    }
    match session_keyframes(&pool, id, from, to).await {
        Ok(mut keyframes) => {
            let truncated = keyframes.len() > MAX_KEYFRAMES;
            keyframes.truncate(MAX_KEYFRAMES);
            Json(KeyframeList {
                keyframes: keyframes
                    .into_iter()
                    .map(|k| KeyframeView {
                        t_ms: k.t_ms,
                        segment_id: k.segment_id,
                    })
                    .collect(),
                truncated,
            })
            .into_response()
        }
        Err(e) => internal(e),
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct MediaQuery {
    /// 场次时间（毫秒）；从不晚于它的最近关键帧起播。缺省 0。
    pub from: Option<i64>,
}

/// `GET /v1/sessions/{id}/media?from=<t_ms>`：DVR 回看流。
///
/// FLV 分段返回 `video/x-flv`，TS 分段返回 `video/mp2t`（都给 mpegts.js）；分片 MP4 暂不支持（415）。
/// 响应头 `X-Dvr-Start-Ms` 是起播关键帧的场次时间；媒体时间戳 = 场次时间 + 1000 ms。
/// 遇到断流缺口、编码参数变化、场次结束时响应结束，播放器按 `GET /v1/sessions/{id}` 从下一段重开。
pub async fn get_session_media(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Query(query): Query<MediaQuery>,
) -> Response {
    let Ok(permit) = dvr_limit().clone().try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "回看连接数已达上限（进程内最多 {MAX_DVR_CONNECTIONS} 路），请关掉其它回看窗口"
            ),
        )
            .into_response();
    };
    let from = query.from.unwrap_or(0).max(0);
    let dvr = match dvr::open(&pool, id, from).await {
        Ok(dvr) => dvr,
        Err(OpenError::NotFound) => return not_found(),
        Err(e @ OpenError::NoMedia) => {
            return (StatusCode::NOT_FOUND, e.to_string()).into_response();
        }
        Err(OpenError::Unsupported(reason)) => {
            return (StatusCode::UNSUPPORTED_MEDIA_TYPE, reason).into_response();
        }
        Err(e) => return internal(e),
    };
    info!(
        session = id,
        from,
        start_ms = dvr.start_ms,
        segment = dvr.segment_id,
        "开始 DVR 回看"
    );
    let content_type = dvr.content_type();
    let start_ms = dvr.start_ms;
    let segment_id = dvr.segment_id;
    // 许可随响应体活：客户端断开、响应结束即释放
    let stream = dvr.into_stream().map(move |chunk| {
        let _ = &permit;
        chunk
    });
    let mut response = Body::from_stream(stream).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));
    headers.insert("X-Dvr-Start-Ms", HeaderValue::from(start_ms));
    headers.insert("X-Dvr-Segment-Id", HeaderValue::from(segment_id));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::access::require_permission;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::models::StreamerInfo;
    use crate::server::infrastructure::permissions::Role;
    use crate::server::infrastructure::users::{Backend, Credentials};
    use crate::server::workbench::index::tests::build_flv;
    use axum::Router;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum_login::AuthManagerLayerBuilder;
    use biliup::downloader::util::ByteCounter;
    use chrono::{DateTime, Utc};
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    struct Fixture {
        _dir: tempfile::TempDir,
        backend: Backend,
        app: Router,
        session: i64,
        legacy: i64,
        _live: live::LiveGuard,
    }

    /// 一场录制中的场次：一个已写完两秒多的 FLV 分段，写入端还在（计数器不再增长）。
    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (1, 'https://a', '主播a')")
            .execute(&pool)
            .await
            .unwrap();
        let at = 1_700_000_000_000;
        let info = StreamerInfo::new(
            "主播a",
            "https://a",
            "标题",
            DateTime::<Utc>::from_timestamp_millis(at).unwrap(),
            "",
        );
        // 老版本留下的一场：没有分段，也没有时间轴
        let legacy = store::open_session(&pool, 1, &info, at, 0)
            .await
            .unwrap()
            .id;
        let session = store::open_session(&pool, 1, &info, at, 0)
            .await
            .unwrap()
            .id;
        let session = live::unique_session_id(&pool, session).await;
        store::set_started_at(&pool, session, at).await.unwrap();
        let flv = build_flv(0, 100, 25, None);
        let cut = flv.keyframes[3].1 as usize;
        let path = dir.path().join("live.flv");
        std::fs::write(&path, &flv.bytes[..cut]).unwrap();
        store::insert_segment(&pool, session, &path.to_string_lossy(), "flv", 0, 0)
            .await
            .unwrap();
        let live = live::register(session, 1, Some(ByteCounter::new()));

        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();
        let backend = Backend::new(pool.clone());
        backend
            .bootstrap_admin(Credentials {
                username: "biliup".into(),
                password: "admin-password".into(),
                next: None,
            })
            .await
            .unwrap();
        backend
            .create_user("ro", "viewer-password".into(), Role::Viewer)
            .await
            .unwrap();
        let auth_layer = AuthManagerLayerBuilder::new(
            backend.clone(),
            SessionManagerLayer::new(session_store).with_secure(false),
        )
        .build();
        let app = Router::new()
            .route("/v1/sessions", get(list_sessions))
            .route("/v1/sessions/{id}", get(get_session))
            .route("/v1/sessions/{id}/keyframes", get(get_session_keyframes))
            .route("/v1/sessions/{id}/media", get(get_session_media))
            .with_state(pool)
            .route_layer(from_fn(require_permission))
            .merge(crate::server::api::auth::router())
            .layer(auth_layer);
        Fixture {
            _dir: dir,
            backend,
            app,
            session,
            legacy,
            _live: live,
        }
    }

    async fn login(app: &Router, username: &str, password: &str) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "username": username, "password": password })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers().get(header::SET_COOKIE).unwrap();
        cookie
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    async fn get_as(app: &Router, cookie: Option<&str>, uri: &str) -> Response {
        let mut request = Request::builder().uri(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn json(response: Response) -> serde_json::Value {
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn viewer_lists_sessions_segments_and_keyframes() {
        let f = fixture().await;
        let s = f.session;
        for uri in [
            "/v1/sessions".to_string(),
            format!("/v1/sessions/{s}"),
            format!("/v1/sessions/{s}/keyframes"),
            format!("/v1/sessions/{s}/media"),
        ] {
            let response = get_as(&f.app, None, &uri).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
        }
        let viewer = login(&f.app, "ro", "viewer-password").await;

        let page = json(get_as(&f.app, Some(&viewer), "/v1/sessions").await).await;
        assert_eq!(page["total"], 1);
        assert_eq!(page["page"], 1);
        assert_eq!(page["items"][0]["id"], s);
        assert_eq!(page["items"][0]["streamer_name"], "主播a");
        assert_eq!(page["items"][0]["recording"], true);
        assert_eq!(page["items"][0]["segment_count"], 1);
        assert_eq!(page["items"][0]["started_at"], 1_700_000_000_000i64);
        let other = json(get_as(&f.app, Some(&viewer), "/v1/sessions?streamer_id=2").await).await;
        assert_eq!(other["total"], 0);

        let detail = json(get_as(&f.app, Some(&viewer), &format!("/v1/sessions/{s}")).await).await;
        assert_eq!(detail["segments"][0]["state"], "recording");
        assert_eq!(detail["segments"][0]["file_name"], "live.flv");
        assert_eq!(
            detail["segments"][0]["end_ms"], 2965,
            "录制中按已写入的内容算"
        );
        assert_eq!(detail["duration_ms"], 2965);
        assert_eq!(detail["gaps"], serde_json::json!([]));
        assert!(detail.get("markers").is_none() && detail.get("clips").is_none());

        let uri = format!("/v1/sessions/{s}/keyframes?from=500&to=2500");
        let keyframes = json(get_as(&f.app, Some(&viewer), &uri).await).await;
        let times: Vec<i64> = keyframes["keyframes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k["t_ms"].as_i64().unwrap())
            .collect();
        assert_eq!(times, [1000, 2000]);
        assert_eq!(keyframes["truncated"], false);

        let uri = format!("/v1/sessions/{s}/keyframes?from=2000&to=1000");
        let response = get_as(&f.app, Some(&viewer), &uri).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        for uri in [
            "/v1/sessions/999".to_string(),
            "/v1/sessions/999/keyframes".to_string(),
            "/v1/sessions/999/media".to_string(),
            format!("/v1/sessions/{}", f.legacy),
        ] {
            let response = get_as(&f.app, Some(&viewer), &uri).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    /// 四条场次路由的要求都由策略层按路由表给出：只读接口要 `file.view`、回看流要 `preview.view`，
    /// 三个角色都满足；表里没登记的方法按默认拒绝只给超管。
    #[test]
    fn session_routes_are_declared_in_the_policy() {
        use crate::server::infrastructure::permissions::Permission;
        use crate::server::infrastructure::policy::{RouteRequirement, Subject};
        use axum::http::Method;
        for (route, raw, permission) in [
            ("/v1/sessions", "/v1/sessions", Permission::FileView),
            ("/v1/sessions/{id}", "/v1/sessions/1", Permission::FileView),
            (
                "/v1/sessions/{id}/keyframes",
                "/v1/sessions/1/keyframes",
                Permission::FileView,
            ),
            (
                "/v1/sessions/{id}/media",
                "/v1/sessions/1/media",
                Permission::PreviewView,
            ),
        ] {
            let requirement = RouteRequirement::of(&Method::GET, route, raw);
            assert_eq!(
                requirement,
                RouteRequirement::Permission(permission),
                "{route}"
            );
            for role in Role::ALL {
                assert!(
                    Subject::user(1, role).satisfies(requirement),
                    "{route} {role:?}"
                );
            }
            assert!(Subject::unrestricted().satisfies(requirement), "{route}");
            let write = RouteRequirement::of(&Method::DELETE, route, raw);
            assert_eq!(write, RouteRequirement::AdminOnly, "{route}");
            assert!(
                !Subject::user(1, Role::Operator).satisfies(write),
                "{route}"
            );
        }
    }

    /// 连接上限与会话失效截断共用进程级的许可，放在同一个用例里顺序跑。
    #[tokio::test]
    async fn media_streams_are_limited_and_end_when_the_session_is_revoked() {
        let f = fixture().await;
        let viewer = login(&f.app, "ro", "viewer-password").await;
        let admin = login(&f.app, "biliup", "admin-password").await;
        let uri = format!("/v1/sessions/{}/media?from=1500", f.session);

        let response = get_as(&f.app, Some(&viewer), &uri).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "video/x-flv");
        assert_eq!(response.headers()["X-Dvr-Start-Ms"], "1000");
        assert!(response.headers().get(header::CONTENT_LENGTH).is_none());
        let mut viewer_stream = response.into_body().into_data_stream();
        let header = viewer_stream.next().await.unwrap().unwrap();
        assert_eq!(&header[..3], b"FLV");

        let mut others = Vec::new();
        for _ in 1..MAX_DVR_CONNECTIONS {
            let response = get_as(&f.app, Some(&admin), &uri).await;
            assert_eq!(response.status(), StatusCode::OK);
            others.push(response);
        }
        let response = get_as(&f.app, Some(&admin), &uri).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        others.pop();
        let response = get_as(&f.app, Some(&admin), &uri).await;
        assert_eq!(response.status(), StatusCode::OK, "断开一路就能再开");
        others.push(response);

        // 录制中的流读完已落盘内容后停在等待写入；会话失效后由访问控制层截断
        let user = f.backend.find_by_username("ro").await.unwrap().unwrap();
        f.backend.logout_everywhere(user.id).await.unwrap();
        let drained = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(chunk) = viewer_stream.next().await {
                chunk.unwrap();
            }
        })
        .await;
        assert!(drained.is_ok(), "会话失效后回看流应在一两个复查周期内结束");
        drop(viewer_stream);
        drop(others);
        assert_eq!(dvr_limit().available_permits(), MAX_DVR_CONNECTIONS);
    }
}
