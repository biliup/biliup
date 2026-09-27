//! 任务从入队起占住整场素材：排队时的删除只标 `pending_delete`，任务结束后引用释放、清理照常；
//! 录像删光且没有要处理的任务和候选之后，分析数据跟着删。

use super::*;
use crate::server::auto_clip::cleanup;
use crate::server::workbench::retention::{self, Disposal, Retention};

async fn segment(env: &Env, id: i64) -> (PathBuf, String) {
    let (path, state): (String, String) =
        sqlx::query_as("SELECT path, state FROM segments WHERE id = ?")
            .bind(id)
            .fetch_one(&env.pool)
            .await
            .unwrap();
    (PathBuf::from(path), state)
}

#[tokio::test]
async fn a_queued_job_holds_the_footage_until_it_finishes() {
    let Some(env) = env(Scenario::Ok).await else {
        return;
    };
    let retention = Retention::without_delay(env.pool.clone());
    let job = env.enqueue(true).await;
    assert_eq!(env.pins().await, 1, "入队就引用整场");

    // 排队时整场投稿后删除录像：只标 pending_delete，文件还在，清理任务也不删
    let (first, _) = segment(&env, env.segments[0]).await;
    assert_eq!(
        retention::remove(&retention, &[first.as_path()])
            .await
            .unwrap(),
        vec![Disposal::Deferred]
    );
    assert_eq!(segment(&env, env.segments[0]).await.1, "pending_delete");
    assert_eq!(
        retention::sweep_pending(&env.pool, now_ms()).await.unwrap(),
        0
    );
    assert!(first.exists());

    // 任务照常跑完（标了删除的分段还在盘上，照样转写），结束时撤销引用
    let (id, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!((id, outcome), (job.id, Outcome::Done));
    assert!(
        env.files()
            .lines()
            .await
            .iter()
            .any(|line| line.from_ms < PIECES[1].start_ms),
        "第一段也转写了"
    );
    assert_eq!(env.pins().await, 0);

    // 下一轮清理照常删；还有录像，分析数据留着
    assert_eq!(
        retention::sweep_pending(&env.pool, now_ms()).await.unwrap(),
        1
    );
    assert_eq!(segment(&env, env.segments[0]).await.1, "deleted");
    assert!(!first.exists());
    let root = env.root();
    assert!(
        cleanup::sweep_in(&env.pool, &root, now_ms())
            .await
            .removed
            .is_empty()
    );
    assert!(env.files().dir().exists());

    // 其余分段也删掉之后，分析数据跟着删
    let mut rest = Vec::new();
    for id in &env.segments[1..] {
        rest.push(segment(&env, *id).await.0);
    }
    let rest: Vec<&Path> = rest.iter().map(PathBuf::as_path).collect();
    let disposals = retention::remove(&retention, &rest).await.unwrap();
    assert!(
        disposals.iter().all(|d| *d == Disposal::Deleted),
        "{disposals:?}"
    );
    assert_eq!(
        cleanup::sweep_in(&env.pool, &root, now_ms()).await.removed,
        vec![env.session]
    );
    assert!(!env.files().dir().exists());
}
