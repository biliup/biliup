use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use axum::body::to_bytes;
use sqlx::sqlite::SqlitePoolOptions;

/// 一个按当前迁移建好的库，装着 Python 版时代会有的那几场老录像。
///
/// 活动历史是读取时推导的，所以夹具里只需要真实的业务数据：老录像能不能出现在时间线上，
/// 取决于它们的业务时间，而不是升级时有没有被登记过。
async fn fixture() -> ConnectionPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path,started_at,ended_at) VALUES
            (1,'旧主播','https://example.test/1','旧主播','2020-01-01','',1577836800000,1577836810000),
            (2,'第二场','https://example.test/2','第二场','2020-01-01','',1577836800000,1577836810000),
            (3,'无时间','https://example.test/3','无时间','bad-date','',NULL,NULL),
            (4,'只有日期','https://example.test/4','只有日期','2020-01-02','',NULL,NULL);
        INSERT INTO segments(id,session_id,path,container,state,start_ms,end_ms) VALUES
            (1,1,'原始录像.flv','flv','finished',0,5000),
            (2,2,'第二场.flv','flv','finished',0,5000);",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn old_recordings_appear_with_their_business_times() {
    let pool = fixture().await;
    let events = recent(&pool, 12).await.unwrap();
    assert_eq!(events.len(), 7);
    // 时间无法推导的场次不出现
    assert!(events.iter().all(|event| event.session_id != Some(3)));
    // 两个分段在同一时刻结束，按文件名区分
    let mut files: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == "file")
        .filter_map(|event| event.sub.as_deref())
        .collect();
    files.sort_unstable();
    assert_eq!(files, ["原始录像.flv", "第二场.flv"]);
    let file = events
        .iter()
        .find(|event| event.sub.as_deref() == Some("原始录像.flv"))
        .unwrap();
    assert_eq!(file.ts, 1577836805000);
    // 只有 date 的老录像按日期算开始时间，与升级时的回填口径一致
    let dated = events
        .iter()
        .find(|event| event.session_id == Some(4))
        .unwrap();
    assert_eq!(dated.kind, "start");
    assert_eq!(dated.ts, 1577923200000);
    assert_eq!(dated.text, "只有日期 开始录制");
    // 老活动照旧可见，不受 24 小时窗口影响
    let response = list(State(pool), Query(ListQuery::default())).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(json["events"].as_array().unwrap().len(), 7);
    assert_eq!(json["events"][0]["ts"], 1577923200000i64);
}

#[tokio::test]
async fn every_event_reflects_the_milestones_of_its_business_record() {
    let pool = fixture().await;
    sqlx::raw_sql(
        "INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path) \
            VALUES(5,'新主播','https://example.test/5','新主播','2026-10-13','');
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=5;
        UPDATE stream_sessions SET started_at=1900000000000 WHERE id=5;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=5;
        UPDATE stream_sessions SET ended_at=1900000009000 WHERE id=5;
        INSERT INTO segments(id,session_id,path,container,state,start_ms) \
            VALUES(5,5,'新分段.mp4','mp4','recording',0);
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=5;
        UPDATE segments SET state='finished',end_ms=9000 WHERE id=5;
        UPDATE segments SET state='missing' WHERE id=5;
        UPDATE segments SET state='finished' WHERE id=5;
        INSERT INTO render_jobs(id,session_id,state,phase,spec_json,created_at,updated_at) \
            VALUES(5,5,'queued','queued','{}',1900000010000,1900000010000);
        UPDATE render_jobs SET state='running',phase='preparing',started_at=1900000011000,updated_at=1900000011000 WHERE id=5;
        UPDATE render_jobs SET phase='编码',ratio=0.5,updated_at=1900000012000 WHERE id=5;
        UPDATE render_jobs SET state='failed',error='boom',updated_at=1900000013000,finished_at=1900000013000 WHERE id=5;
        INSERT INTO render_jobs(id,session_id,state,phase,spec_json,created_at,updated_at) \
            VALUES(6,5,'queued','queued','{}',1900000014000,1900000014000);
        UPDATE render_jobs SET state='cancelled',phase='cancelled',updated_at=1900000015000,finished_at=1900000015000 WHERE id=6;
        INSERT INTO upload_observation_tasks(id,source,scheduler_group,task_order,state,created_at,updated_at,snapshot) \
            VALUES('t1','sync','sync',1,'running',1900000020000,1900000020000,'{\"title\":\"新主播 投稿\",\"session_id\":5}');
        UPDATE upload_observation_tasks SET state='submitting',updated_at=1900000021000 WHERE id='t1';
        UPDATE upload_observation_tasks SET state='succeeded',updated_at=1900000022000 WHERE id='t1';",
    )
    .execute(&pool)
    .await
    .unwrap();
    let events = recent(&pool, 100).await.unwrap();
    let own: Vec<_> = events
        .iter()
        .filter(|event| event.session_id == Some(5))
        .collect();
    assert_eq!(own.len(), 10);
    // 分段的状态改来改去，只留下一次「已完成」
    assert_eq!(own.iter().filter(|event| event.kind == "file").count(), 1);
    // 压制任务：排队、开始、结束三个节点都在；中间的进度更新不产生事件
    let renders: Vec<_> = own.iter().filter(|event| event.kind == "render").collect();
    assert_eq!(renders.len(), 5);
    let mut job5: Vec<_> = renders
        .iter()
        .filter(|event| event.entity_id == Some(5))
        .copied()
        .collect();
    job5.sort_by_key(|event| event.ts);
    assert_eq!(
        job5.iter()
            .map(|event| event.sub.as_deref())
            .collect::<Vec<_>>(),
        [Some("等待压制"), Some("正在压制"), Some("压制失败")]
    );
    assert_eq!(
        job5.iter().map(|event| event.ts).collect::<Vec<_>>(),
        [1900000010000, 1900000011000, 1900000013000]
    );
    // 没跑起来就被取消的任务只有排队与结束两个节点
    let mut job6: Vec<_> = renders
        .iter()
        .filter(|event| event.entity_id == Some(6))
        .copied()
        .collect();
    job6.sort_by_key(|event| event.ts);
    assert_eq!(
        job6.iter()
            .map(|event| event.sub.as_deref())
            .collect::<Vec<_>>(),
        [Some("等待压制"), Some("压制已取消")]
    );
    // 上传任务：登记一次，加上当前状态（中间的过渡不落库，还原不了）
    let queued = own
        .iter()
        .find(|event| event.kind == "upload_queued")
        .unwrap();
    assert_eq!(queued.ts, 1900000020000);
    assert_eq!(queued.sub.as_deref(), Some("上传任务已登记"));
    assert_eq!(queued.text, "新主播 投稿");
    let succeeded = own
        .iter()
        .find(|event| event.kind == "upload_succeeded")
        .unwrap();
    assert_eq!(succeeded.ts, 1900000022000);
    assert_eq!(succeeded.sub.as_deref(), Some("投稿成功"));
    // 重复读取不会多出事件
    assert_eq!(recent(&pool, 100).await.unwrap().len(), events.len());
    // 业务记录删掉，它的场次、分段与压制事件跟着消失；上传观察不挂在场次上，仍然留下
    sqlx::query("DELETE FROM stream_sessions WHERE id=5")
        .execute(&pool)
        .await
        .unwrap();
    let after = recent(&pool, 100).await.unwrap();
    assert_eq!(after.len(), 9);
    assert!(after.iter().all(|event| !matches!(
        event.kind.as_str(),
        "start" | "end" | "file" | "render"
    ) || event.session_id != Some(5)));
    assert_eq!(
        after
            .iter()
            .filter(|event| event.text == "新主播 投稿")
            .count(),
        2
    );
}

/// 活动历史没有自己的表和触发器：时间线完全由业务表推导出来，写入路径不登记任何东西。
#[tokio::test]
async fn the_database_keeps_no_activity_table_or_trigger() {
    let pool = fixture().await;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'activity%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(tables.is_empty(), "{tables:?}");
    let triggers: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='trigger' AND sql LIKE '%activity_events%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(triggers.is_empty(), "{triggers:?}");
}

#[tokio::test]
async fn the_feed_comes_from_the_database_the_service_opened() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("data.sqlite3");
    let pool = ConnectionManager::new_pool(db.to_str().unwrap())
        .await
        .unwrap();
    for n in 0..15 {
        sqlx::query(
            "INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path,started_at) \
             VALUES(?,?,'https://example.test/x','标题','2026-10-17','',?)",
        )
        .bind(n + 1)
        .bind(format!("场次{n}"))
        .bind(1000 + n)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;
    let reopened = ConnectionManager::new_pool(db.to_str().unwrap())
        .await
        .unwrap();
    let events = recent(&reopened, 12).await.unwrap();
    assert_eq!(events.len(), 12);
    assert_eq!(events[0].ts, 1014);
    assert_eq!(events[0].text, "场次14 开始录制");
    assert_eq!(events[11].ts, 1003);
    let other = ConnectionManager::new_pool(dir.path().join("another.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    assert!(recent(&other, 12).await.unwrap().is_empty());
    for limit in [0, -1, 101] {
        assert_eq!(
            list(
                State(reopened.clone()),
                Query(ListQuery { limit: Some(limit) })
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
}
