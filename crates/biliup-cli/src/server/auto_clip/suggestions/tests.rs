use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::workbench::clips::State as ClipState;

const T0: i64 = 1_750_000_000_000;

async fn setup() -> (tempfile::TempDir, ConnectionPool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    for id in [1, 2] {
        sqlx::query(
            "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path, started_at, ended_at)
             VALUES (?, 'a', 'https://a', 't', '2026-09-24 00:00:00', '', 0, 600000)",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    }
    // 两段：0–300 秒、300–600 秒
    for (start, end) in [(0, 300_000), (300_000, 600_000)] {
        sqlx::query(
            "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
             VALUES (1, ?, 'flv', 'finished', ?, ?, 0)",
        )
        .bind(format!("/tmp/none/{start}.flv"))
        .bind(start)
        .bind(end)
        .execute(&pool)
        .await
        .unwrap();
    }
    (dir, pool)
}

fn candidate(in_s: i64, out_s: i64, confidence: Option<f64>) -> Candidate {
    Candidate {
        in_ms: in_s * 1000,
        out_ms: out_s * 1000,
        title: format!("{in_s}"),
        reason: "弹幕刷屏".into(),
        confidence,
        tags: vec!["高能".into()],
        evidence: Evidence {
            asr_lines: 2,
            danmaku: 40,
            images: vec![in_s * 1000],
        },
    }
}

async fn pins(pool: &ConnectionPool) -> Vec<(String, i64, i64)> {
    sqlx::query_as("SELECT owner, from_ms, to_ms FROM segment_pins ORDER BY owner")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn pin_counts(pool: &ConnectionPool) -> Vec<i64> {
    sqlx::query_scalar("SELECT pin_count FROM segments ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn pending_suggestions_pin_their_ranges() {
    let (_dir, pool) = setup().await;
    let saved = replace_pending(
        &pool,
        1,
        None,
        &[candidate(250, 320, Some(0.8)), candidate(10, 40, None)],
        T0,
    )
    .await
    .unwrap();
    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].in_ms, 10_000, "按入点排序");
    assert_eq!(saved[1].state, SuggestionState::Pending);
    assert_eq!(saved[1].tags, vec!["高能".to_string()]);
    assert_eq!(saved[1].evidence.danmaku, 40);
    assert_eq!(saved[1].confidence, Some(0.8));
    assert_eq!(saved[0].confidence, None);
    assert_eq!(saved[0].expires_at, Some(T0 + EXPIRE_AFTER_MS));
    let mut expected = vec![
        (pin_owner(saved[0].id), 10_000, 40_000),
        (pin_owner(saved[1].id), 250_000, 320_000),
    ];
    expected.sort();
    assert_eq!(pins(&pool).await, expected);
    assert_eq!(pin_counts(&pool).await, vec![2, 1], "跨两段的候选两段都算");
    assert_eq!(list(&pool, 1).await.unwrap(), saved);
    assert!(list(&pool, 2).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_rerun_replaces_pending_and_respects_handled_ones() {
    let (_dir, pool) = setup().await;
    let first = replace_pending(
        &pool,
        1,
        None,
        &[
            candidate(10, 40, Some(0.5)),
            candidate(100, 130, Some(0.5)),
            candidate(200, 230, Some(0.5)),
        ],
        T0,
    )
    .await
    .unwrap();
    dismiss(&pool, 1, first[0].id, T0 + 1).await.unwrap();
    let accepted = accept(&pool, 1, first[1].id, &Acceptance::default(), T0 + 2)
        .await
        .unwrap();
    assert!(matches!(accepted, AcceptOutcome::Accepted { .. }));

    let second = replace_pending(
        &pool,
        1,
        None,
        &[
            candidate(12, 41, Some(0.9)),   // 与丢弃的重复
            candidate(101, 131, Some(0.9)), // 与接受的重复
            candidate(205, 235, Some(0.9)),
            candidate(400, 430, Some(0.9)),
        ],
        T0 + 10,
    )
    .await
    .unwrap();
    let starts: Vec<i64> = second.iter().map(|s| s.in_ms / 1000).collect();
    assert_eq!(starts, vec![205, 400]);
    let all = list(&pool, 1).await.unwrap();
    let states: Vec<(i64, SuggestionState)> =
        all.iter().map(|s| (s.in_ms / 1000, s.state)).collect();
    assert_eq!(
        states,
        vec![
            (10, SuggestionState::Dismissed),
            (100, SuggestionState::Accepted),
            (205, SuggestionState::Pending),
            (400, SuggestionState::Pending),
        ],
        "上一轮没处理的 200 秒那条被换掉"
    );
    let owners: Vec<String> = pins(&pool).await.into_iter().map(|p| p.0).collect();
    let clip_id = all[1].clip_id.unwrap();
    let mut expected = vec![
        clips::pin_owner(clip_id),
        pin_owner(second[0].id),
        pin_owner(second[1].id),
    ];
    expected.sort();
    assert_eq!(owners, expected);
}

#[tokio::test]
async fn accepting_creates_a_draft_clip_in_one_transaction() {
    let (_dir, pool) = setup().await;
    let saved = replace_pending(&pool, 1, None, &[candidate(250, 320, Some(0.7))], T0)
        .await
        .unwrap();
    let id = saved[0].id;
    assert_eq!(
        accept(&pool, 2, id, &Acceptance::default(), T0)
            .await
            .unwrap(),
        AcceptOutcome::NotFound,
        "别的场次的候选"
    );
    let acceptance = Acceptance::builder()
        .in_ms(245_000)
        .out_ms(330_000)
        .title("改过的标题")
        .build();
    let AcceptOutcome::Accepted { suggestion, clip } =
        accept(&pool, 1, id, &acceptance, T0 + 5).await.unwrap()
    else {
        panic!("应当接受成功");
    };
    assert_eq!(suggestion.state, SuggestionState::Accepted);
    assert_eq!(suggestion.clip_id, Some(clip.id));
    assert_eq!(suggestion.updated_at, T0 + 5);
    assert_eq!(suggestion.expires_at, None);
    assert_eq!((suggestion.in_ms, suggestion.out_ms), (250_000, 320_000));
    assert_eq!(clip.state, ClipState::Draft);
    assert_eq!((clip.in_ms, clip.out_ms), (245_000, 330_000));
    assert_eq!(clip.title, "改过的标题");
    assert_eq!(clip.created_at, T0 + 5);
    assert_eq!(
        pins(&pool).await,
        vec![(clips::pin_owner(clip.id), 245_000, 330_000)],
        "候选的引用换成切片自己的"
    );
    assert!(matches!(
        accept(&pool, 1, id, &Acceptance::default(), T0).await.unwrap(),
        AcceptOutcome::NotPending(s) if s.state == SuggestionState::Accepted
    ));
    assert_eq!(
        clips::count(&pool, 1).await.unwrap(),
        1,
        "重复接受不多建切片"
    );

    let saved = replace_pending(&pool, 1, None, &[candidate(10, 40, None)], T0)
        .await
        .unwrap();
    let AcceptOutcome::Accepted { clip, .. } =
        accept(&pool, 1, saved[0].id, &Acceptance::default(), T0)
            .await
            .unwrap()
    else {
        panic!("应当接受成功");
    };
    assert_eq!((clip.in_ms, clip.out_ms), (10_000, 40_000));
    assert_eq!(clip.title, "10", "不改就用候选的标题");

    sqlx::query("DELETE FROM clips WHERE id = ?")
        .bind(clip.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&pool, saved[0].id).await.unwrap().unwrap().clip_id,
        None
    );
}

#[tokio::test]
async fn accepting_respects_the_clip_limit() {
    let (_dir, pool) = setup().await;
    sqlx::query(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?)
         INSERT INTO clips (session_id, in_ms, out_ms, created_at, updated_at)
         SELECT 1, i, i + 1, 0, 0 FROM n",
    )
    .bind(MAX_CLIPS_PER_SESSION)
    .execute(&pool)
    .await
    .unwrap();
    let saved = replace_pending(&pool, 1, None, &[candidate(10, 40, None)], T0)
        .await
        .unwrap();
    assert_eq!(
        accept(&pool, 1, saved[0].id, &Acceptance::default(), T0)
            .await
            .unwrap(),
        AcceptOutcome::TooManyClips
    );
    assert_eq!(
        get(&pool, saved[0].id).await.unwrap().unwrap().state,
        SuggestionState::Pending,
        "没建成切片，候选保持原样"
    );
}

#[tokio::test]
async fn dismissing_releases_the_pin() {
    let (_dir, pool) = setup().await;
    let saved = replace_pending(&pool, 1, None, &[candidate(10, 40, None)], T0)
        .await
        .unwrap();
    let id = saved[0].id;
    assert_eq!(
        dismiss(&pool, 2, id, T0).await.unwrap(),
        DismissOutcome::NotFound
    );
    let DismissOutcome::Dismissed(dismissed) = dismiss(&pool, 1, id, T0 + 3).await.unwrap() else {
        panic!("应当丢弃成功");
    };
    assert_eq!(dismissed.state, SuggestionState::Dismissed);
    assert_eq!(dismissed.updated_at, T0 + 3);
    assert!(pins(&pool).await.is_empty());
    assert_eq!(pin_counts(&pool).await, vec![0, 0]);
    assert!(matches!(
        dismiss(&pool, 1, id, T0).await.unwrap(),
        DismissOutcome::NotPending(_)
    ));
    assert_eq!(
        dismiss(&pool, 1, 999, T0).await.unwrap(),
        DismissOutcome::NotFound
    );
}

#[tokio::test]
async fn unhandled_suggestions_expire_after_72_hours() {
    let (_dir, pool) = setup().await;
    let early = replace_pending(&pool, 1, None, &[candidate(10, 40, None)], T0)
        .await
        .unwrap();
    // 另一场晚一小时生成
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (2, '/tmp/none/2.flv', 'flv', 'finished', 0, 600000, 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let late = replace_pending(&pool, 2, None, &[candidate(10, 40, None)], T0 + 3_600_000)
        .await
        .unwrap();
    let dismissed = replace_pending(
        &pool,
        1,
        None,
        &[candidate(10, 40, None), candidate(100, 130, None)],
        T0,
    )
    .await
    .unwrap();
    assert_ne!(dismissed[0].id, early[0].id, "重跑换掉了上一轮的");
    dismiss(&pool, 1, dismissed[1].id, T0).await.unwrap();

    assert_eq!(expire(&pool, T0 + EXPIRE_AFTER_MS - 1).await.unwrap(), 0);
    assert_eq!(pins(&pool).await.len(), 2);
    assert_eq!(expire(&pool, T0 + EXPIRE_AFTER_MS).await.unwrap(), 1);
    let first = get(&pool, dismissed[0].id).await.unwrap().unwrap();
    assert_eq!(first.state, SuggestionState::Expired);
    assert_eq!(first.updated_at, T0 + EXPIRE_AFTER_MS);
    assert_eq!(first.expires_at, None);
    assert_eq!(
        get(&pool, dismissed[1].id).await.unwrap().unwrap().state,
        SuggestionState::Dismissed,
        "已处理的不动"
    );
    assert_eq!(
        pins(&pool).await,
        vec![(pin_owner(late[0].id), 10_000, 40_000)],
        "过期的撤销引用，另一场的还没到时间"
    );
    assert_eq!(pin_counts(&pool).await, vec![0, 0, 1]);
    assert_eq!(
        expire(&pool, T0 + EXPIRE_AFTER_MS).await.unwrap(),
        0,
        "只处理一次"
    );
    assert_eq!(
        expire(&pool, T0 + EXPIRE_AFTER_MS + 3_600_000)
            .await
            .unwrap(),
        1
    );
    assert!(pins(&pool).await.is_empty());
    assert!(matches!(
        accept(&pool, 1, first.id, &Acceptance::default(), T0).await.unwrap(),
        AcceptOutcome::NotPending(s) if s.state == SuggestionState::Expired
    ));
}

#[tokio::test]
async fn suggestions_go_with_their_session() {
    let (_dir, pool) = setup().await;
    replace_pending(&pool, 1, None, &[candidate(10, 40, None)], T0)
        .await
        .unwrap();
    sqlx::query("DELETE FROM stream_sessions WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    assert!(list(&pool, 1).await.unwrap().is_empty());
    assert!(pins(&pool).await.is_empty());
}
