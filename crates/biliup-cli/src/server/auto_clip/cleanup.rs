//! 随分段清理任务每分钟跑一次的收尾：候选过期、删掉用不上了的场次分析数据。
//!
//! 分析数据目录（`auto_clip/`）不存在时什么都不做：没用过自动切片的库不多一次查询。

use super::runner;
use super::suggestions;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use std::path::Path;
use tracing::{info, warn};

/// 一轮收尾做了什么。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swept {
    /// 过期的候选数
    pub expired: u64,
    /// 删掉了分析数据的场次
    pub removed: Vec<i64>,
}

pub async fn sweep(pool: &ConnectionPool, now: i64) -> Swept {
    sweep_in(pool, &runner::root(), now).await
}

pub async fn sweep_in(pool: &ConnectionPool, root: &Path, now: i64) -> Swept {
    let mut swept = Swept::default();
    if !tokio::fs::try_exists(root).await.unwrap_or(false) {
        return swept;
    }
    match suggestions::expire(pool, now).await {
        Ok(0) => {}
        Ok(n) => {
            info!(
                suggestions = n,
                "自动切片：72 小时没处理的候选已过期，撤销素材引用"
            );
            swept.expired = n;
        }
        Err(error) => warn!(%error, "自动切片：候选过期处理失败"),
    }
    match remove_stale(pool, root).await {
        Ok(removed) => swept.removed = removed,
        Err(error) => warn!(%error, "自动切片：清理用不上的分析数据失败"),
    }
    swept
}

/// 删掉用不上了的 `auto_clip/<场次 id>/`（转写、弹幕密度和缩图）。只动名字是纯数字的目录：
///
/// - 这一场的分段都已删除（全是 `deleted`），而且没有排队或运行中的任务、没有待处理的候选；
///   录像删光之后这些数据既没法再切，也没有任务会再读；
/// - 兜底：场次行已经不在（任务、候选随外键级联删除，磁盘上的在这里跟着删）。
pub async fn remove_stale(pool: &ConnectionPool, root: &Path) -> sqlx::Result<Vec<i64>> {
    let mut dirs = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(root).await else {
        return Ok(Vec::new());
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let is_dir = entry.file_type().await.is_ok_and(|t| t.is_dir());
        let id = entry
            .file_name()
            .to_str()
            .filter(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|name| name.parse::<i64>().ok());
        if let (true, Some(id)) = (is_dir, id) {
            dirs.push((id, entry.path()));
        }
    }
    let mut removed = Vec::new();
    for (id, path) in dirs {
        if !stale(pool, id).await? {
            continue;
        }
        match tokio::fs::remove_dir_all(&path).await {
            Ok(()) => {
                info!(
                    session = id,
                    "自动切片：场次的录像已删光或场次已删除，删掉它的分析数据"
                );
                removed.push(id);
            }
            Err(error) => warn!(%error, session = id, "自动切片：删除分析数据失败"),
        }
    }
    removed.sort_unstable();
    Ok(removed)
}

async fn stale(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT NOT EXISTS (SELECT 1 FROM stream_sessions WHERE id = ?1)
             OR (EXISTS (SELECT 1 FROM segments WHERE session_id = ?1)
                 AND NOT EXISTS (SELECT 1 FROM segments
                                 WHERE session_id = ?1 AND state <> 'deleted')
                 AND NOT EXISTS (SELECT 1 FROM auto_clip_jobs
                                 WHERE session_id = ?1 AND state IN ('queued', 'running'))
                 AND NOT EXISTS (SELECT 1 FROM clip_suggestions
                                 WHERE session_id = ?1 AND state = 'pending'))",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::auto_clip::files::SessionFiles;
    use crate::server::auto_clip::jobs::{self, NewJob, Trigger};
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    async fn execute(pool: &ConnectionPool, sql: &str) {
        sqlx::query(sql).execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn analysis_files_go_when_the_session_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let root = dir.path().join("auto_clip");
        assert_eq!(
            sweep_in(&pool, &root, 0).await,
            Swept::default(),
            "没有目录时什么都不做"
        );

        for id in [1, 2] {
            sqlx::query(
                "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path)
                 VALUES (?, 'a', 'https://a', 't', '2026-09-24 00:00:00', '')",
            )
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
            let files = SessionFiles::new(&root, id);
            std::fs::create_dir_all(files.thumbs_dir()).unwrap();
            std::fs::write(files.transcript(), "{}\n").unwrap();
            std::fs::write(files.thumb(1000), b"jpeg").unwrap();
        }
        // 不是场次目录的不动
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("7"), b"a file named like a session").unwrap();

        assert_eq!(sweep_in(&pool, &root, 0).await, Swept::default());
        sqlx::query("DELETE FROM stream_sessions WHERE id = 2")
            .execute(&pool)
            .await
            .unwrap();
        let swept = sweep_in(&pool, &root, 0).await;
        assert_eq!(swept.removed, vec![2]);
        assert!(root.join("1").join("transcript.jsonl").exists());
        assert!(!root.join("2").exists());
        assert!(root.join("notes").exists());
        assert!(root.join("7").exists());
        assert!(sweep_in(&pool, &root, 0).await.removed.is_empty());
    }

    #[tokio::test]
    async fn analysis_files_go_once_the_footage_is_gone_and_nothing_waits_on_them() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let root = dir.path().join("auto_clip");
        execute(
            &pool,
            "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path)
             VALUES (3, 'a', 'https://a', 't', '2026-09-24 00:00:00', '')",
        )
        .await;
        execute(
            &pool,
            "INSERT INTO segments (id, session_id, path, container, state, start_ms, end_ms, gap_before_ms)
             VALUES (31, 3, '/nonexistent/a.flv', 'flv', 'deleted', 0, 60000, 0),
                    (32, 3, '/nonexistent/b.flv', 'flv', 'pending_delete', 60000, 120000, 0)",
        )
        .await;
        let files = SessionFiles::new(&root, 3);
        std::fs::create_dir_all(files.thumbs_dir()).unwrap();
        std::fs::write(files.transcript(), "{}\n").unwrap();
        let job = NewJob::builder()
            .session_id(3)
            .trigger(Trigger::Manual)
            .not_before(0)
            .created_at(0)
            .build();
        jobs::insert(&pool, &job).await.unwrap().unwrap();
        execute(
            &pool,
            "INSERT INTO clip_suggestions (session_id, in_ms, out_ms, created_at, updated_at)
             VALUES (3, 0, 30000, 0, 0)",
        )
        .await;

        assert!(
            sweep_in(&pool, &root, 0).await.removed.is_empty(),
            "还有录像"
        );
        execute(&pool, "UPDATE segments SET state = 'deleted' WHERE id = 32").await;
        assert!(
            sweep_in(&pool, &root, 0).await.removed.is_empty(),
            "任务还在排队"
        );
        jobs::cancel(&pool, 3, 0).await.unwrap().unwrap();
        assert!(
            sweep_in(&pool, &root, 0).await.removed.is_empty(),
            "候选还没处理"
        );
        execute(&pool, "UPDATE clip_suggestions SET state = 'dismissed'").await;
        assert!(files.dir().exists());
        assert_eq!(sweep_in(&pool, &root, 0).await.removed, vec![3]);
        assert!(!files.dir().exists());
        let kept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM stream_sessions WHERE id = 3")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kept, 1, "场次记录不动");
    }
}
