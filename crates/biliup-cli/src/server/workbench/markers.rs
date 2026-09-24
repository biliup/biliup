//! 标记（`markers`）：看直播时按一下「标记」记下的场次时刻，之后在工作台里从这里剪。
//!
//! 预览里打的标记没有现成的场次时间，要按「按下的那一刻 − 播放器延迟」换算（[`watched_at_ms`]）：
//!
//! - 客户端带上按下标记时（`pressed_at`）和发请求时（`client_now`）自己的时钟，服务端只用两者之差，
//!   把按下的时刻搬到服务端时钟上，不要求客户端时钟准；请求在路上的时间忽略不计；
//! - 播放器报告的延迟（`latency_ms`，缓冲里还没播出的时长）是画面落后于服务端收到这段内容的时间；
//! - 两者相减得到屏幕上那一帧被录下来的墙钟，再经 [`live::anchor`] 换算成场次时间。

use super::live;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use std::collections::HashMap;

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
            at_ms: row.try_get("at_ms")?,
            label: row.try_get("label")?,
            color: row.try_get("color")?,
            created_by: row.try_get("created_by")?,
            created_at: row.try_get("created_at")?,
            lookback_ms: row.try_get("lookback_ms")?,
            lookahead_ms: row.try_get("lookahead_ms")?,
        })
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
    live::anchor(session_id).map(|a| a.session_ms_at(watched_wall_ms(server_now, timing)))
}

pub async fn insert(
    pool: &ConnectionPool,
    session_id: i64,
    marker: &NewMarker,
) -> sqlx::Result<Marker> {
    let sql = format!(
        "INSERT INTO markers (session_id, at_ms, label, color, created_by, created_at, lookback_ms, lookahead_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(session_id)
        .bind(marker.at_ms)
        .bind(&marker.label)
        .bind(&marker.color)
        .bind(marker.created_by)
        .bind(marker.created_at)
        .bind(marker.lookback_ms)
        .bind(marker.lookahead_ms)
        .fetch_one(pool)
        .await?;
    Marker::from_row(&row)
}

/// 场次的全部标记，按时间轴排序。
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<Marker>> {
    let sql = format!("SELECT {COLUMNS} FROM markers WHERE session_id = ? ORDER BY at_ms, id");
    sqlx::query(&sql)
        .bind(session_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(Marker::from_row)
        .collect()
}

/// 改一个标记；标记不存在或不属于这一场时返回 `None`。
pub async fn update(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
    changes: &MarkerChanges,
) -> sqlx::Result<Option<Marker>> {
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
    sqlx::query(&sql)
        .bind(changes.at_ms)
        .bind(&changes.label)
        .bind(color_set)
        .bind(color)
        .bind(changes.lookback_ms)
        .bind(changes.lookahead_ms)
        .bind(id)
        .bind(session_id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(Marker::from_row)
        .transpose()
}

/// 删一个标记，返回是否删到了。
pub async fn delete(pool: &ConnectionPool, session_id: i64, id: i64) -> sqlx::Result<bool> {
    let done = sqlx::query("DELETE FROM markers WHERE id = ? AND session_id = ?")
        .bind(id)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// 几场各有多少个标记；没有标记的场次不在结果里。
pub async fn counts(pool: &ConnectionPool, session_ids: &[i64]) -> sqlx::Result<HashMap<i64, i64>> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = vec!["?"; session_ids.len()].join(", ");
    let sql = format!(
        "SELECT session_id, COUNT(*) FROM markers WHERE session_id IN ({placeholders}) GROUP BY session_id"
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

        let guard = live::register(9_200_001, 1, None);
        assert_eq!(watched_at_ms(9_200_001, 0, Timing::default()), None);
        guard.anchor(30_000, 1_000_000);
        let timing = Timing {
            latency_ms: Some(2_000),
            ..Default::default()
        };
        assert_eq!(watched_at_ms(9_200_001, 1_010_000, timing), Some(38_000));
        drop(guard);
        assert_eq!(watched_at_ms(9_200_001, 1_010_000, timing), None);
    }
}
