use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::context::Worker;
use crate::server::workbench::{index, store};

#[tokio::test]
async fn danmaku_pipeline_cleans_sources_only_when_successful_result_reaches_rm() {
    for (container, upload_ok, cleanup) in [
        ("flv", true, true),
        ("flv", true, false),
        ("flv", false, true),
        ("ts", true, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let name = format!(
            "cleanup-{}",
            dir.path().file_name().unwrap().to_string_lossy()
        );
        let raw = dir.path().join(format!("{name}.{container}"));
        let xml = raw.with_extension("xml");
        let output = PathBuf::from(crate::server::common::processing::OUTPUT_DIR)
            .join("1")
            .join(format!("{name}.processed.mp4"));
        // All tests use unique generated paths; clean the fixture even on failure.
        let _output_guard = tempfile::TempPath::try_from_path(&output).unwrap();
        let fixture = crate::tools::ffmpeg_command()
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=320x180:r=25:d=0.4",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-y",
            ])
            .arg(&raw)
            .output()
            .await
            .unwrap();
        assert!(
            fixture.status.success(),
            "{}",
            String::from_utf8_lossy(&fixture.stderr)
        );
        std::fs::write(&xml, "<i><d p=\"0.1,1,25,16777215\">弹幕</d></i>").unwrap();
        std::fs::write(index::index_path(&raw), b"old index").unwrap();

        let pool = ConnectionManager::new_pool(dir.path().join("db.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("INSERT INTO stream_sessions (id,name,url,title,date,live_cover_path,started_at,ended_at) VALUES (1,'test','https://www.douyu.com/1','test','2026-10-23 00:00:00','',1,2)")
            .execute(&pool).await.unwrap();
        let segment = store::insert_segment(&pool, 1, raw.to_str().unwrap(), container, 0, 0)
            .await
            .unwrap();
        sqlx::query("UPDATE segments SET state='finished',end_ms=400,danmaku_path=? WHERE id=?")
            .bind(xml.to_str().unwrap())
            .bind(segment)
            .execute(&pool)
            .await
            .unwrap();
        let base = test_context(Config::default());
        let mut room = base.live_streamer().clone();
        room.postprocessor = Some(if cleanup {
            vec![HookStep::Remove("rm".into())]
        } else {
            vec![]
        });
        let worker = Worker::new(
            room,
            None,
            Arc::new(std::sync::RwLock::new(Config::default())),
            Default::default(),
        );
        let ctx = Context::new(1, Arc::new(worker), pool, base.live_stream().clone());
        let mut steps: Vec<HookStep> = serde_json::from_value(serde_json::json!([
            {"render": {"encoding": {"encoder": "libx264"}, "danmaku": {"enabled": true}}}
        ]))
        .unwrap();
        if container == "ts" {
            steps.insert(
                0,
                HookStep::Remux {
                    remux: "mp4".into(),
                },
            );
        }
        let result = pipeline_upload_videos(
            futures::stream::iter([SegmentInfo::new(raw.clone(), Some(xml.clone()), None, 0)]),
            &steps,
            &ctx,
            |path| {
                assert_eq!(path, output, "only the rendered video is uploaded");
                assert!(
                    raw.exists() && xml.exists(),
                    "uploading must not remove inputs"
                );
                async move {
                    if upload_ok {
                        Ok(Video::new(&path.to_string_lossy()))
                    } else {
                        Err(AppError::Custom("test upload failure".into()).into())
                    }
                }
            },
        )
        .await
        .unwrap();
        assert!(output.exists(), "the real FLV + XML render must succeed");
        assert!(
            raw.exists() && xml.exists(),
            "wait for successful submission before cleanup"
        );
        if upload_ok {
            assert_eq!(
                result.paths,
                [output.clone(), xml.clone()],
                "hook arguments stay unchanged"
            );
        } else {
            assert!(result.paths.is_empty());
        }
        execute_postprocessor(result.paths, &ctx).await.unwrap();
        let kept = !upload_ok || !cleanup;
        for path in [&raw, &output, &xml] {
            assert_eq!(path.exists(), kept, "{}", path.display());
        }
        if !kept {
            assert!(!index::index_path(&raw).exists());
            assert!(!index::index_path(&output).exists());
            assert!(
                !raw.with_extension("mp4").exists(),
                "remux intermediate must be removed"
            );
        }
        index::remove(&raw);
        index::remove(&output);
    }
}

#[tokio::test]
async fn source_registration_rolls_back_if_segment_update_fails() {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("db.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_processed_path BEFORE UPDATE OF path ON segments BEGIN SELECT RAISE(FAIL, 'test update failure'); END")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stream_sessions (id,name,url,title,date,live_cover_path) VALUES (1,'test','url','test','2026-10-23 00:00:00','')")
        .execute(&pool).await.unwrap();
    let raw = dir.path().join("source.flv");
    let output = dir.path().join("output.mp4");
    std::fs::write(&raw, b"raw").unwrap();
    std::fs::write(&output, b"processed").unwrap();
    store::insert_segment(&pool, 1, raw.to_str().unwrap(), "flv", 0, 0)
        .await
        .unwrap();
    let base = test_context(Config::default());
    let ctx = Context::new(1, base.worker().clone(), pool, base.live_stream().clone());
    assert!(
        update_processed_path(&ctx, &raw, &output, &[raw.clone()], None)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processed_video_sources")
        .fetch_one(ctx.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    let current: String = sqlx::query_scalar("SELECT path FROM segments")
        .fetch_one(ctx.pool())
        .await
        .unwrap();
    assert_eq!(current, raw.to_string_lossy());
    assert!(raw.exists() && output.exists());
}
