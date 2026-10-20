//! 标记（`markers`）：看直播时按一下「标记」记下的场次时刻，之后在工作台里从这里剪。
//!
//! 预览里打的标记没有现成的场次时间，要按「按下的那一刻 − 播放器延迟」换算（[`watched_at_ms`]）：
//!
//! - 客户端带上按下标记时（`pressed_at`）和发请求时（`client_now`）自己的时钟，服务端只用两者之差，
//!   把按下的时刻搬到服务端时钟上，不要求客户端时钟准；请求在路上的时间忽略不计；
//! - 播放器报告的延迟（`latency_ms`，缓冲里还没播出的时长）是画面落后于服务端收到这段内容的时间；
//! - 两者相减得到屏幕上那一帧到达服务端的墙钟，再经 [`live`] 换算成场次时间：优先用最近写盘的
//!   观测，没有时用开段 / 关段锚点。
//!
//! 每个标记以 `marker:<id>` 的名义引用它覆盖的场次时间 `[at_ms − lookback_ms, at_ms + lookahead_ms]`
//! （[`retention::pin`]），与标记的增删改在同一个事务里：被覆盖的分段在删除点推迟删除，标记删掉后
//! 由清理任务删。

use super::{live, retention, session_groups};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::Serialize;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection};
use std::collections::{BTreeSet, HashMap};
use tracing::debug;

/// 标记默认覆盖它之前多长（毫秒）。
pub const DEFAULT_LOOKBACK_MS: i64 = 60_000;
/// 回看 / 前看范围的上限（毫秒）。
pub const MAX_RANGE_MS: i64 = 600_000;
pub const MAX_LABEL_CHARS: usize = 100;
/// 一场最多这么多个标记，防止按住不放或脚本把库写满。
pub const MAX_MARKERS_PER_SESSION: i64 = 5_000;
/// 播放器延迟、按下到发出请求的间隔超过这么多就按这么多算（客户端报错了值也不至于标到很远）。
pub const MAX_DELAY_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Marker {
    pub id: i64,
    pub session_id: i64,
    /// 查看合并场次时的坐标所属场次；`session_id` 始终保留标记的真实归属。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_session_id: Option<i64>,
    /// 从来源场次继承的标记，供调用方显示来源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<i64>,
    /// 场次时间轴上的位置（毫秒）。
    pub at_ms: i64,
    pub label: String,
    pub color: Option<String>,
    /// 打标记的 Web 用户；未开 `--auth` 时为 `null`。
    pub created_by: Option<i64>,
    /// Unix 毫秒。
    pub created_at: i64,
    pub lookback_ms: i64,
    pub lookahead_ms: i64,
}

impl Marker {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            view_session_id: None,
            source_session_id: None,
            at_ms: row.try_get("at_ms")?,
            label: row.try_get("label")?,
            color: row.try_get("color")?,
            created_by: row.try_get("created_by")?,
            created_at: row.try_get("created_at")?,
            lookback_ms: row.try_get("lookback_ms")?,
            lookahead_ms: row.try_get("lookahead_ms")?,
        })
    }

    fn in_view(mut self, view_session_id: i64, offset_ms: i64) -> Self {
        if view_session_id != self.session_id {
            self.view_session_id = Some(view_session_id);
            self.source_session_id = Some(self.session_id);
            self.at_ms = self.at_ms.saturating_add(offset_ms);
        }
        self
    }
}

const COLUMNS: &str =
    "id, session_id, at_ms, label, color, created_by, created_at, lookback_ms, lookahead_ms";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMarker {
    pub at_ms: i64,
    pub label: String,
    pub color: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub lookback_ms: i64,
    pub lookahead_ms: i64,
}

/// 只改给出的字段；`color: Some(None)` 清掉颜色。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkerChanges {
    pub at_ms: Option<i64>,
    pub label: Option<String>,
    pub color: Option<Option<String>>,
    pub lookback_ms: Option<i64>,
    pub lookahead_ms: Option<i64>,
}

/// 客户端报告的时间信息，见模块文档。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Timing {
    pub client_now: Option<i64>,
    pub pressed_at: Option<i64>,
    pub latency_ms: Option<i64>,
}

/// 屏幕上那一帧被录下来的墙钟（服务端时钟，Unix 毫秒）。
pub fn watched_wall_ms(server_now: i64, timing: Timing) -> i64 {
    let press_delay = match (timing.client_now, timing.pressed_at) {
        (Some(now), Some(pressed)) => now.saturating_sub(pressed).clamp(0, MAX_DELAY_MS),
        _ => 0,
    };
    let latency = timing.latency_ms.unwrap_or(0).clamp(0, MAX_DELAY_MS);
    server_now - press_delay - latency
}

/// 正在录的场次里，客户端按下标记时屏幕上那一帧的场次时间；场次没在录或还没有分段时为 `None`。
pub fn watched_at_ms(session_id: i64, server_now: i64, timing: Timing) -> Option<i64> {
    let fallback = live::anchor(session_id)?;
    let wall = watched_wall_ms(server_now, timing);
    let anchor = match live::written_anchor(session_id, server_now) {
        Some(written) => {
            debug!(
                session = session_id,
                ahead_ms = written.session_ms_at(wall) - fallback.session_ms_at(wall),
                "按写盘观测换算"
            );
            written
        }
        None => fallback,
    };
    Some(anchor.session_ms_at(wall))
}

/// 合并场次使用仍在录的真实来源锚点，再平移到查看坐标。
pub async fn watched_at_in_view(
    pool: &ConnectionPool,
    session_id: i64,
    server_now: i64,
    timing: Timing,
) -> sqlx::Result<Option<i64>> {
    let Some(source) = session_groups::active_source(pool, session_id).await? else {
        return Ok(None);
    };
    Ok(watched_at_ms(source.source_session_id, server_now, timing)
        .map(|at_ms| at_ms.saturating_add(source.offset_ms)))
}

/// 返回允许通过此场次操作的标记归属和坐标偏移。
pub async fn owner_in_view(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
) -> sqlx::Result<Option<(i64, i64)>> {
    let Some(owner): Option<i64> =
        sqlx::query_scalar("SELECT session_id FROM markers WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(None);
    };
    Ok(session_groups::view_offset(pool, session_id, owner)
        .await?
        .map(|offset_ms| (owner, offset_ms)))
}

fn pin_owner(id: i64) -> String {
    format!("marker:{id}")
}

/// 按标记当前的时间与范围登记（或改登记）它对分段的引用。
async fn pin(conn: &mut SqliteConnection, marker: &Marker) -> sqlx::Result<()> {
    let from_ms = marker.at_ms.saturating_sub(marker.lookback_ms).max(0);
    let to_ms = marker.at_ms.saturating_add(marker.lookahead_ms);
    retention::pin(
        conn,
        &pin_owner(marker.id),
        marker.session_id,
        from_ms,
        to_ms,
    )
    .await
}

pub async fn insert(
    pool: &ConnectionPool,
    session_id: i64,
    marker: &NewMarker,
) -> sqlx::Result<Marker> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let sql = format!(
        "INSERT INTO markers (session_id, at_ms, label, color, created_by, created_at, lookback_ms, lookahead_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    );
    let mut tx = pool.begin().await?;
    let row = sqlx::query(&sql)
        .bind(session_id)
        .bind(marker.at_ms)
        .bind(&marker.label)
        .bind(&marker.color)
        .bind(marker.created_by)
        .bind(marker.created_at)
        .bind(marker.lookback_ms)
        .bind(marker.lookahead_ms)
        .fetch_one(&mut *tx)
        .await?;
    let marker = Marker::from_row(&row)?;
    pin(&mut tx, &marker).await?;
    tx.commit().await?;
    Ok(marker)
}

/// 场次的全部标记，按时间轴排序。
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<Marker>> {
    let sql = format!(
        "SELECT {COLUMNS}, COALESCE((
             SELECT offset_ms FROM session_group_members gm
             WHERE gm.group_session_id = ?1 AND gm.source_session_id = markers.session_id), 0) AS view_offset
         FROM markers
         WHERE session_id = ?1 OR session_id IN (
             SELECT source_session_id FROM session_group_members WHERE group_session_id = ?1)
         ORDER BY at_ms + view_offset, id"
    );
    sqlx::query(&sql)
        .bind(session_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| Ok(Marker::from_row(row)?.in_view(session_id, row.try_get("view_offset")?)))
        .collect()
}

/// 改一个标记；标记不存在或不属于这一场时返回 `None`。改了时间或范围时引用跟着改。
pub async fn update(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
    changes: &MarkerChanges,
) -> sqlx::Result<Option<Marker>> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let Some((owner_session_id, offset_ms)) = owner_in_view(pool, session_id, id).await? else {
        return Ok(None);
    };
    let local_at_ms = changes.at_ms.map(|at_ms| at_ms.saturating_sub(offset_ms));
    if local_at_ms.is_some_and(|at_ms| at_ms < 0) {
        return Err(sqlx::Error::Protocol("标记不能移到来源场次开始之前".into()));
    }
    let (color_set, color) = match &changes.color {
        Some(color) => (true, color.clone()),
        None => (false, None),
    };
    let sql = format!(
        "UPDATE markers SET
             at_ms = COALESCE(?, at_ms),
             label = COALESCE(?, label),
             color = CASE WHEN ? THEN ? ELSE color END,
             lookback_ms = COALESCE(?, lookback_ms),
             lookahead_ms = COALESCE(?, lookahead_ms)
         WHERE id = ? AND session_id = ?
         RETURNING {COLUMNS}"
    );
    let mut tx = pool.begin().await?;
    let marker = sqlx::query(&sql)
        .bind(local_at_ms)
        .bind(&changes.label)
        .bind(color_set)
        .bind(color)
        .bind(changes.lookback_ms)
        .bind(changes.lookahead_ms)
        .bind(id)
        .bind(owner_session_id)
        .fetch_optional(&mut *tx)
        .await?
        .as_ref()
        .map(Marker::from_row)
        .transpose()?;
    let range_changed =
        changes.at_ms.is_some() || changes.lookback_ms.is_some() || changes.lookahead_ms.is_some();
    if let Some(marker) = &marker
        && range_changed
    {
        pin(&mut tx, marker).await?;
    }
    tx.commit().await?;
    Ok(marker.map(|marker| marker.in_view(session_id, offset_ms)))
}

/// 删一个标记，返回是否删到了。它的引用一起撤销。
pub async fn delete(pool: &ConnectionPool, session_id: i64, id: i64) -> sqlx::Result<bool> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let Some((owner_session_id, _)) = owner_in_view(pool, session_id, id).await? else {
        return Ok(false);
    };
    let mut tx = pool.begin().await?;
    let done = sqlx::query("DELETE FROM markers WHERE id = ? AND session_id = ?")
        .bind(id)
        .bind(owner_session_id)
        .execute(&mut *tx)
        .await?;
    let deleted = done.rows_affected() > 0;
    if deleted {
        retention::unpin(&mut tx, &pin_owner(id)).await?;
    }
    tx.commit().await?;
    Ok(deleted)
}

/// 给还没有登记引用的标记补登，返回补了几个。启动时调用，补上引用接入之前打的标记。
pub async fn pin_unpinned(pool: &ConnectionPool) -> sqlx::Result<usize> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let mut tx = pool.begin().await?;
    let sessions: Vec<i64> = sqlx::query_scalar(
        "INSERT INTO segment_pins (owner, session_id, from_ms, to_ms)
         SELECT 'marker:' || m.id, m.session_id, MAX(m.at_ms - m.lookback_ms, 0), m.at_ms + m.lookahead_ms
         FROM markers m
         WHERE NOT EXISTS (SELECT 1 FROM segment_pins p WHERE p.owner = 'marker:' || m.id)
         RETURNING session_id",
    )
    .fetch_all(&mut *tx)
    .await?;
    for session_id in sessions.iter().collect::<BTreeSet<_>>() {
        retention::refresh_pin_counts(&mut *tx, *session_id).await?;
    }
    tx.commit().await?;
    Ok(sessions.len())
}

/// 几场各有多少个标记；没有标记的场次不在结果里。
pub async fn counts(pool: &ConnectionPool, session_ids: &[i64]) -> sqlx::Result<HashMap<i64, i64>> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = vec!["(?)"; session_ids.len()].join(", ");
    let sql = format!(
        "WITH requested(session_id) AS (SELECT DISTINCT column1 FROM (VALUES {placeholders}))
         SELECT r.session_id, COUNT(m.id) FROM requested r JOIN markers m
           ON m.session_id = r.session_id OR EXISTS (
               SELECT 1 FROM session_group_members gm
               WHERE gm.group_session_id = r.session_id AND gm.source_session_id = m.session_id)
         GROUP BY r.session_id"
    );
    let mut query = sqlx::query_as::<_, (i64, i64)>(&sql);
    for id in session_ids {
        query = query.bind(id);
    }
    Ok(query.fetch_all(pool).await?.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::workbench::index::tests::build_flv;
    use crate::server::workbench::store;
    use std::time::{Duration, UNIX_EPOCH};

    async fn setup() -> (tempfile::TempDir, ConnectionPool, i64) {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let session = sqlx::query_scalar(
            "INSERT INTO stream_sessions (name, url, title, date, live_cover_path)
             VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        (dir, pool, session)
    }

    fn new_marker(at_ms: i64) -> NewMarker {
        NewMarker {
            at_ms,
            label: String::new(),
            color: None,
            created_by: None,
            created_at: 1,
            lookback_ms: DEFAULT_LOOKBACK_MS,
            lookahead_ms: 0,
        }
    }

    async fn group(pool: &ConnectionPool, first: i64) -> (i64, i64) {
        let second: i64 = sqlx::query_scalar(
            "INSERT INTO stream_sessions (name, url, title, date, live_cover_path)
             VALUES ('a', 'https://a', 't2', '2026-09-24 00:01:00', '') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let second = live::unique_session_id(pool, second).await;
        let view: i64 = sqlx::query_scalar(
            "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, kind)
             VALUES ('a', 'https://a', 'merged', '2026-09-24 00:00:00', '', 'group') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        for (ordinal, source, offset) in [(0, first, 0), (1, second, 60_000)] {
            sqlx::query(
                "INSERT INTO session_group_members(group_session_id, source_session_id, ordinal, offset_ms)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(view)
            .bind(source)
            .bind(ordinal)
            .bind(offset)
            .execute(pool)
            .await
            .unwrap();
        }
        (view, second)
    }

    #[tokio::test]
    async fn inherited_markers_keep_ownership_and_translate_edits() {
        let (_dir, pool, first) = setup().await;
        let (view, second) = group(&pool, first).await;
        let a = insert(&pool, first, &new_marker(1_000)).await.unwrap();
        let b = insert(&pool, second, &new_marker(2_000)).await.unwrap();
        let c = insert(&pool, view, &new_marker(40_000)).await.unwrap();
        let all = list(&pool, view).await.unwrap();
        assert_eq!(
            all.iter().map(|m| (m.id, m.at_ms)).collect::<Vec<_>>(),
            [(a.id, 1_000), (c.id, 40_000), (b.id, 62_000)]
        );
        assert_eq!(all[2].session_id, second);
        assert_eq!(all[2].source_session_id, Some(second));
        assert_eq!(all[2].view_session_id, Some(view));
        assert_eq!(
            counts(&pool, &[view, first, second]).await.unwrap(),
            HashMap::from([(view, 3), (first, 1), (second, 1)])
        );

        let changes = MarkerChanges {
            at_ms: Some(63_000),
            ..Default::default()
        };
        let changed = update(&pool, view, b.id, &changes).await.unwrap().unwrap();
        assert_eq!(changed.at_ms, 63_000);
        let source = list(&pool, second).await.unwrap();
        assert_eq!(source[0].at_ms, 3_000);
        assert_eq!(source[0].view_session_id, None);
        assert!(
            update(&pool, first, b.id, &changes)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            update(
                &pool,
                view,
                b.id,
                &MarkerChanges {
                    at_ms: Some(59_999),
                    ..Default::default()
                }
            )
            .await
            .is_err()
        );
        assert_eq!(list(&pool, second).await.unwrap()[0].at_ms, 3_000);
        assert!(delete(&pool, view, b.id).await.unwrap());
        assert!(list(&pool, second).await.unwrap().is_empty());
        assert!(
            !pinned(&pool)
                .await
                .iter()
                .any(|(owner, _, _)| owner == &pin_owner(b.id))
        );
    }

    #[tokio::test]
    async fn live_group_marker_time_uses_the_tail_source_anchor() {
        let (_dir, pool, first) = setup().await;
        let (view, second) = group(&pool, first).await;
        let guard = live::register(second, 1, None);
        guard.anchor(2_000, 100_000);
        assert_eq!(
            watched_at_in_view(&pool, view, 100_500, Timing::default())
                .await
                .unwrap(),
            Some(62_500)
        );
        drop(guard);
        assert_eq!(
            watched_at_in_view(&pool, view, 100_500, Timing::default())
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn insert_list_update_delete() {
        let (_dir, pool, s) = setup().await;
        let b = insert(&pool, s, &new_marker(5_000)).await.unwrap();
        let a = insert(&pool, s, &new_marker(1_000)).await.unwrap();
        assert_eq!(b.lookback_ms, 60_000);
        let all = list(&pool, s).await.unwrap();
        assert_eq!(
            all.iter().map(|m| m.id).collect::<Vec<_>>(),
            [a.id, b.id],
            "按时间轴排序"
        );

        let changes = MarkerChanges {
            label: Some("高光".into()),
            color: Some(Some("#ff0000".into())),
            ..Default::default()
        };
        let updated = update(&pool, s, a.id, &changes).await.unwrap().unwrap();
        assert_eq!(updated.label, "高光");
        assert_eq!(updated.color.as_deref(), Some("#ff0000"));
        assert_eq!(updated.at_ms, 1_000, "没给的字段不动");
        let cleared = MarkerChanges {
            color: Some(None),
            ..Default::default()
        };
        let updated = update(&pool, s, a.id, &cleared).await.unwrap().unwrap();
        assert_eq!(updated.color, None);
        assert_eq!(updated.label, "高光");
        assert!(
            update(&pool, s + 1, a.id, &changes)
                .await
                .unwrap()
                .is_none(),
            "不属于这一场的标记改不到"
        );

        assert_eq!(
            counts(&pool, &[s, s + 1]).await.unwrap(),
            HashMap::from([(s, 2)])
        );
        assert!(!delete(&pool, s + 1, a.id).await.unwrap());
        assert!(delete(&pool, s, a.id).await.unwrap());
        assert!(!delete(&pool, s, a.id).await.unwrap());
        assert_eq!(counts(&pool, &[s]).await.unwrap(), HashMap::from([(s, 1)]));
        assert!(counts(&pool, &[]).await.unwrap().is_empty());

        let next = insert(&pool, s, &new_marker(9_000)).await.unwrap();
        assert!(next.id > a.id, "撤销掉的最新标记，id 不给下一个标记复用");
    }

    #[tokio::test]
    async fn markers_go_with_their_session() {
        let (_dir, pool, s) = setup().await;
        insert(&pool, s, &new_marker(0)).await.unwrap();
        sqlx::query("DELETE FROM stream_sessions WHERE id = ?")
            .bind(s)
            .execute(&pool)
            .await
            .unwrap();
        assert!(list(&pool, s).await.unwrap().is_empty());
        let negative = insert(&pool, s, &new_marker(-1)).await;
        assert!(negative.is_err(), "外键 / CHECK 拦住不存在的场次与负时间");
    }

    async fn pinned(pool: &ConnectionPool) -> Vec<(String, i64, i64)> {
        sqlx::query_as("SELECT owner, from_ms, to_ms FROM segment_pins ORDER BY owner")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    async fn pin_count(pool: &ConnectionPool, segment: i64) -> i64 {
        sqlx::query_scalar("SELECT pin_count FROM segments WHERE id = ?")
            .bind(segment)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn markers_pin_their_range_until_deleted() {
        let (_dir, pool, s) = setup().await;
        let segment = store::insert_segment(&pool, s, "a.flv", "flv", 0, 0)
            .await
            .unwrap();
        let m = insert(&pool, s, &new_marker(20_000)).await.unwrap();
        let owner = format!("marker:{}", m.id);
        assert_eq!(
            pinned(&pool).await,
            [(owner.clone(), 0, 20_000)],
            "前 60 s 截到 0"
        );
        assert_eq!(pin_count(&pool, segment).await, 1);

        let changes = MarkerChanges {
            at_ms: Some(100_000),
            lookahead_ms: Some(5_000),
            ..Default::default()
        };
        update(&pool, s, m.id, &changes).await.unwrap().unwrap();
        assert_eq!(pinned(&pool).await, [(owner.clone(), 40_000, 105_000)]);
        assert!(
            update(&pool, s + 1, m.id, &changes)
                .await
                .unwrap()
                .is_none(),
            "改不到别的场次的标记，引用也不动"
        );
        assert_eq!(pinned(&pool).await, [(owner, 40_000, 105_000)]);

        assert!(!delete(&pool, s + 1, m.id).await.unwrap());
        assert_eq!(pin_count(&pool, segment).await, 1);
        assert!(delete(&pool, s, m.id).await.unwrap());
        assert!(pinned(&pool).await.is_empty());
        assert_eq!(pin_count(&pool, segment).await, 0);
    }

    #[tokio::test]
    async fn markers_made_before_pinning_get_pinned_on_startup() {
        let (_dir, pool, s) = setup().await;
        let segment = store::insert_segment(&pool, s, "a.flv", "flv", 0, 0)
            .await
            .unwrap();
        let m = insert(&pool, s, &new_marker(5_000)).await.unwrap();
        let old: i64 = sqlx::query_scalar(
            "INSERT INTO markers (session_id, at_ms, created_at, lookback_ms, lookahead_ms)
             VALUES (?, 90000, 1, 60000, 1000) RETURNING id",
        )
        .bind(s)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(pin_unpinned(&pool).await.unwrap(), 1);
        assert_eq!(
            pinned(&pool).await,
            [
                (format!("marker:{}", m.id), 0, 5_000),
                (format!("marker:{old}"), 30_000, 91_000)
            ]
        );
        assert_eq!(pin_count(&pool, segment).await, 2);
        assert_eq!(pin_unpinned(&pool).await.unwrap(), 0, "只补一次");
    }

    #[test]
    fn watched_time_subtracts_press_delay_and_player_latency() {
        let timing = Timing {
            client_now: Some(1_000_500),
            pressed_at: Some(1_000_000),
            latency_ms: Some(2_000),
        };
        // 客户端时钟比服务端慢一个小时也不影响：只用它自己两次读数之差
        assert_eq!(watched_wall_ms(4_600_000, timing), 4_600_000 - 500 - 2_000);
        assert_eq!(watched_wall_ms(10_000, Timing::default()), 10_000);
        let silly = Timing {
            client_now: Some(0),
            pressed_at: Some(10_000_000),
            latency_ms: Some(-5),
        };
        assert_eq!(watched_wall_ms(10_000, silly), 10_000, "负值按 0");
        let huge = Timing {
            client_now: Some(10_000_000),
            pressed_at: Some(0),
            latency_ms: Some(i64::MAX),
        };
        assert_eq!(
            watched_wall_ms(1_000_000, huge),
            1_000_000 - 2 * MAX_DELAY_MS
        );
    }

    #[tokio::test]
    async fn what_the_open_segment_has_written_takes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        // 100 帧、每 25 帧一个关键帧：已写内容 3965 ms
        let path = dir.path().join("a.flv");
        std::fs::write(&path, build_flv(0, 100, 25, None).bytes).unwrap();
        let written_at: i64 = 1_790_000_000_000;
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_millis(written_at as u64))
            .unwrap();
        let id = 9_200_002;
        let mut guard = live::register(id, 1, None);
        assert_eq!(watched_at_ms(id, 0, Timing::default()), None);
        // 开段锚点比盘上晚 4 s（CDN 缓存 / 修复管线攒 GOP）
        let opened_at = written_at - 3_965 + 4_000;
        guard.anchor(120_000, opened_at);
        let timing = Timing {
            latency_ms: Some(2_000),
            ..Default::default()
        };
        let now = written_at + 2_500;
        let from_open = Some(120_000 + (now - 2_000 - opened_at));
        assert_eq!(
            watched_at_ms(id, now, timing),
            from_open,
            "还没有盘上观测时用开段锚点"
        );

        guard.sample_segment(path, 120_000);
        for _ in 0..200 {
            if watched_at_ms(id, now, timing) != from_open {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(watched_at_ms(id, now, timing), Some(123_965 + 500));

        let later = written_at + live::WRITTEN_WINDOW_MS + 1;
        assert_eq!(
            watched_at_ms(id, later, timing),
            Some(120_000 + (later - 2_000 - opened_at)),
            "盘上观测过了窗口，只剩开段锚点"
        );
        drop(guard);
        assert_eq!(watched_at_ms(id, now, timing), None);
    }
}
