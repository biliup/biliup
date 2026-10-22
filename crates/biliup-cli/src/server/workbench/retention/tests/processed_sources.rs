use super::*;

async fn processed(pool: &ConnectionPool, input: &Path, output: &Path) {
    std::fs::write(output, b"processed video").unwrap();
    std::fs::write(index::index_path(output), b"index").unwrap();
    let mut tx = pool.begin().await.unwrap();
    record_processed_sources(&mut tx, output, &[input.to_owned()])
        .await
        .unwrap();
    sqlx::query("UPDATE segments SET path = ?, index_path = ?, danmaku_path = COALESCE(danmaku_path, ?) WHERE path = ?")
        .bind(path_string(output))
        .bind(path_string(&index::index_path(output)))
        .bind(path_string(&input.with_extension("xml")))
        .bind(path_string(input))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn rm_removes_original_intermediate_output_and_indexes_but_not_similar_files() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (id, raw) = segment(&pool, dir.path(), s, "recording.flv", 0, Some(1000), 10).await;
    let intermediate = dir.path().join("recording.mp4");
    let output = dir.path().join("recording.processed.mp4");
    let unrelated = dir.path().join("recording.ts");
    std::fs::write(&unrelated, b"unrelated recording").unwrap();
    processed(&pool, &raw, &intermediate).await;
    processed(&pool, &intermediate, &output).await;
    let xml = raw.with_extension("xml");
    remove(&retention(&pool, 0), &[&output, &xml])
        .await
        .unwrap();
    for path in [&raw, &intermediate, &output] {
        assert!(
            gone(path),
            "{} and its index must be removed",
            path.display()
        );
    }
    assert!(!xml.exists());
    assert!(unrelated.exists());
    assert_eq!(state(&pool, id).await, "deleted");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processed_video_sources")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn pending_sources_survive_restart_until_retention_and_pin_are_released() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (_, raw) = segment(&pool, dir.path(), s, "recording.flv", 0, Some(1000), 10).await;
    let output = dir.path().join("recording.processed.mp4");
    let xml = raw.with_extension("xml");
    processed(&pool, &raw, &output).await;
    let mut conn = pool.acquire().await.unwrap();
    pin(&mut conn, "clip:source", s, 0, 1000).await.unwrap();
    drop(conn);
    assert_eq!(
        remove(&retention(&pool, HOUR), &[&output, &xml])
            .await
            .unwrap(),
        [Disposal::Deferred, Disposal::Deferred]
    );
    pool.close().await;
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let later = now_ms() + 2 * HOUR;
    assert_eq!(
        sweep_pending(&pool, later).await.unwrap(),
        0,
        "pin still protects the raw file"
    );
    let mut conn = pool.acquire().await.unwrap();
    unpin(&mut conn, "clip:source").await.unwrap();
    drop(conn);
    assert_eq!(
        sweep_pending(&pool, now_ms()).await.unwrap(),
        0,
        "retention has not expired"
    );
    for path in [&raw, &output, &xml] {
        assert!(path.exists());
    }
    assert_eq!(sweep_pending(&pool, later).await.unwrap(), 1);
    assert!(gone(&raw) && gone(&output) && !xml.exists());
}

#[tokio::test]
async fn moved_output_keeps_original_cleanup_association() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (_, raw) = segment(&pool, dir.path(), s, "recording.flv", 0, Some(1000), 10).await;
    let output = dir.path().join("recording.processed.mp4");
    let moved_output = dir.path().join("moved.mp4");
    processed(&pool, &raw, &output).await;
    std::fs::rename(&output, &moved_output).unwrap();
    moved(Some(&pool), &output, &moved_output).await;
    remove(
        &retention(&pool, 0),
        &[&moved_output, &raw.with_extension("xml")],
    )
    .await
    .unwrap();
    assert!(gone(&raw) && gone(&moved_output));
}

#[tokio::test]
async fn cleanup_retries_source_errors_and_tolerates_missing_inputs() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (_, raw) = segment(&pool, dir.path(), s, "recording.flv", 0, Some(1000), 10).await;
    let output = dir.path().join("recording.processed.mp4");
    processed(&pool, &raw, &output).await;
    // A directory at the exact source path gives a portable deletion error.
    std::fs::remove_file(&raw).unwrap();
    std::fs::create_dir(&raw).unwrap();
    assert!(
        remove(&retention(&pool, 0), &[&output, &raw.with_extension("xml")])
            .await
            .is_err()
    );
    let later = now_ms() + 2 * HOUR;
    assert_eq!(sweep_pending(&pool, later).await.unwrap(), 0);
    assert!(output.exists());
    std::fs::remove_dir(&raw).unwrap();
    assert_eq!(sweep_pending(&pool, later).await.unwrap(), 1);
    assert!(gone(&raw) && gone(&output));
}

#[tokio::test]
async fn disk_cleanup_includes_sources_and_untracked_results_use_exact_associations() {
    let (dir, pool) = setup().await;
    let s = session(&pool, 1_000_000).await;
    let (_, raw) = segment(&pool, dir.path(), s, "recording.flv", 0, Some(1000), 10).await;
    let output = dir.path().join("recording.processed.mp4");
    processed(&pool, &raw, &output).await;
    assert_eq!(
        enforce_free_space(&pool, 1000, now_ms(), |_| Ok(0))
            .await
            .unwrap(),
        1
    );
    assert!(gone(&raw) && gone(&output));

    let old = dir.path().join("untracked.ts");
    let result = dir.path().join("untracked.mp4");
    std::fs::write(&old, b"source").unwrap();
    processed(&pool, &old, &result).await;
    assert_eq!(
        remove(&retention(&pool, 0), &[&result]).await.unwrap(),
        [Disposal::Untracked]
    );
    assert!(gone(&old) && gone(&result));
}

#[tokio::test]
async fn shared_source_is_removed_only_with_its_last_output() {
    let (dir, pool) = setup().await;
    let raw = dir.path().join("source.flv");
    let first = dir.path().join("first.mp4");
    let second = dir.path().join("second.mp4");
    std::fs::write(&raw, b"source").unwrap();
    processed(&pool, &raw, &first).await;
    processed(&pool, &raw, &second).await;
    remove(&retention(&pool, 0), &[&first]).await.unwrap();
    assert!(raw.exists() && second.exists());
    remove(&retention(&pool, 0), &[&second]).await.unwrap();
    assert!(!raw.exists());
}
