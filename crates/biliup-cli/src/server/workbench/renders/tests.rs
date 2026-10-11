use super::*;
use crate::server::infrastructure::connection_pool::{ConnectionManager, ConnectionPool};
use crate::server::workbench::recorder::now_ms;
use std::path::PathBuf;
use std::sync::Arc;

async fn setup() -> (tempfile::TempDir, ConnectionPool, i64, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let session:i64=sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at) VALUES('a','https://a','t','2026-10-09','',1000,2000) RETURNING id").fetch_one(&pool).await.unwrap();
    let path = dir.path().join("source.mp4");
    std::fs::write(&path, b"source").unwrap();
    let segment:i64=sqlx::query_scalar("INSERT INTO segments(session_id,path,container,state,start_ms,end_ms) VALUES(?,?,'mp4','finished',0,10000) RETURNING id").bind(session).bind(path.to_string_lossy().as_ref()).fetch_one(&pool).await.unwrap();
    (dir, pool, session, segment)
}
async fn spec(pool: &ConnectionPool, root: PathBuf, session: i64) -> RenderSpec {
    RenderJobs::new(pool.clone(), root)
        .make_spec(session, None, None, RenderRecipe::default())
        .await
        .unwrap()
}
#[test]
fn recipe_rejects_nonfinite_outside_duplicate_and_bad_intervals() {
    let mut recipe = RenderRecipe::default();
    assert!(recipe.validate().is_ok());
    recipe.regions.push(MaskRegion {
        id: "region".into(),
        ..Default::default()
    });
    recipe.regions[0].x = f64::NAN;
    assert!(recipe.validate().is_err());
    recipe.regions[0].x = 0.9;
    assert!(recipe.validate().is_err());
    recipe.regions[0].x = 0.;
    recipe.regions[0].intervals.push(TimeInterval {
        from_ms: 100,
        to_ms: 100,
    });
    assert!(recipe.validate().is_err());
    recipe.regions[0].intervals.clear();
    recipe.regions.push(recipe.regions[0].clone());
    assert!(recipe.validate().is_err());
}
#[tokio::test]
async fn immutable_job_snapshot_and_pin_lifecycle() {
    let (dir, pool, session, segment) = setup().await;
    let original = spec(&pool, dir.path().join("renders"), session).await;
    let job = store::insert(&pool, &original, now_ms()).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT pin_count FROM segments WHERE id=?")
        .bind(segment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let edited = RenderRecipe {
        danmaku: DanmakuSettings {
            font_size: 75.,
            ..Default::default()
        },
        ..Default::default()
    };
    store::save_recipe(&pool, session, &edited, now_ms())
        .await
        .unwrap();
    let saved = store::get(&pool, job.view.id).await.unwrap().unwrap();
    assert_eq!(saved.spec.recipe.danmaku.font_size, 38.);
    assert!(store::running(&pool, job.view.id, now_ms()).await.unwrap());
    assert!(
        store::finish(&pool, job.view.id, "render.mp4", 12, 10000, now_ms())
            .await
            .unwrap()
    );
    assert!(
        !store::terminal(&pool, job.view.id, "cancelled", None, now_ms())
            .await
            .unwrap()
    );
    let count: i64 = sqlx::query_scalar("SELECT pin_count FROM segments WHERE id=?")
        .bind(segment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let view = serde_json::to_string(&store::get(&pool, job.view.id).await.unwrap().unwrap().view)
        .unwrap();
    assert!(!view.contains("source.mp4"));
}
#[tokio::test]
async fn atomic_enqueue_rejects_changed_source_and_releases_failed_job() {
    let (dir, pool, session, segment) = setup().await;
    let original = spec(&pool, dir.path().join("renders"), session).await;
    sqlx::query("UPDATE segments SET state='deleted' WHERE id=?")
        .bind(segment)
        .execute(&pool)
        .await
        .unwrap();
    assert!(store::insert(&pool, &original, now_ms()).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM render_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("UPDATE segments SET state='finished' WHERE id=?")
        .bind(segment)
        .execute(&pool)
        .await
        .unwrap();
    let job = store::insert(&pool, &original, now_ms()).await.unwrap();
    store::recover(&pool, now_ms()).await.unwrap();
    let recovered = store::get(&pool, job.view.id).await.unwrap().unwrap();
    assert_eq!(recovered.view.state, "failed");
    let count: i64 = sqlx::query_scalar("SELECT pin_count FROM segments WHERE id=?")
        .bind(segment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
fn png() -> Vec<u8> {
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        3,
        2,
        image::Rgba([255, 0, 0, 128]),
    ));
    let mut result = std::io::Cursor::new(vec![]);
    image
        .write_to(&mut result, image::ImageFormat::Png)
        .unwrap();
    result.into_inner()
}
#[tokio::test]
async fn images_are_validated_and_references_block_deletion() {
    let (dir, pool, session, _) = setup().await;
    let root = dir.path().join("renders");
    assert!(
        assets::upload(&pool, &root, session, "image/jpeg", &png(), now_ms())
            .await
            .is_err()
    );
    assert!(
        assets::upload(&pool, &root, session, "image/png", b"bad", now_ms())
            .await
            .is_err()
    );
    let asset = assets::upload(&pool, &root, session, "image/png", &png(), now_ms())
        .await
        .unwrap();
    assert_eq!((asset.width, asset.height), (3, 2));
    let recipe = RenderRecipe {
        regions: vec![MaskRegion {
            id: "image".into(),
            effect_type: EffectType::Image,
            asset_id: Some(asset.id),
            ..Default::default()
        }],
        ..Default::default()
    };
    store::save_recipe(&pool, session, &recipe, now_ms())
        .await
        .unwrap();
    assert!(assets::delete(&pool, asset.id).await.is_err());
    store::save_recipe(&pool, session, &RenderRecipe::default(), now_ms())
        .await
        .unwrap();
    assert!(assets::delete(&pool, asset.id).await.unwrap());
}
#[tokio::test]
async fn full_session_allows_more_than_six_hours_and_live_session_is_blocked() {
    let (dir, pool, session, segment) = setup().await;
    sqlx::query("UPDATE segments SET end_ms=? WHERE id=?")
        .bind(7 * 3_600_000i64)
        .bind(segment)
        .execute(&pool)
        .await
        .unwrap();
    let jobs = Arc::new(RenderJobs::new(pool.clone(), dir.path().join("renders")));
    let full = jobs
        .make_spec(session, None, None, RenderRecipe::default())
        .await
        .unwrap();
    assert_eq!(full.out_ms, 7 * 3_600_000);
    sqlx::query("UPDATE stream_sessions SET ended_at=NULL WHERE id=?")
        .bind(session)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        jobs.make_spec(session, None, None, RenderRecipe::default())
            .await
            .unwrap_err()
            .contains("已结束")
    );
}

#[tokio::test]
async fn cancelling_a_queued_composition_keeps_the_previous_clip_product() {
    use crate::server::workbench::{clips, transcode};
    let (dir, pool, session, _) = setup().await;
    let clip = clips::insert(
        &pool,
        session,
        &clips::NewClip {
            marker_id: None,
            in_ms: 0,
            out_ms: 10000,
            title: "clip".into(),
            created_by: None,
            created_at: now_ms(),
        },
    )
    .await
    .unwrap();
    clips::begin_export(&pool, clip.id, clips::Mode::Precise, now_ms())
        .await
        .unwrap()
        .unwrap();
    let previous = dir.path().join("previous.mp4");
    std::fs::write(&previous, b"previous successful product").unwrap();
    clips::finish_export(
        &pool,
        clip.id,
        &clips::Exported {
            cut_in_ms: 0,
            cut_out_ms: 10000,
            output_path: previous.to_string_lossy().into_owned(),
            output_bytes: 27,
            duration_ms: 10000,
        },
        now_ms(),
    )
    .await
    .unwrap();
    // Holding the shared permit makes cancellation deterministic before source decoding.
    let permit = transcode::acquire(&tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    let jobs = Arc::new(RenderJobs::new(pool.clone(), dir.path().join("renders")));
    let view = jobs
        .enqueue_clip(
            &clips::get(&pool, clip.id).await.unwrap().unwrap(),
            RenderRecipe::default(),
        )
        .await
        .unwrap();
    assert_eq!(view.state, "queued");
    assert!(jobs.cancel(view.id).await.unwrap());
    jobs.shutdown().await;
    drop(permit);
    let cancelled = store::get(&pool, view.id).await.unwrap().unwrap();
    assert_eq!(cancelled.view.state, "cancelled");
    let saved = clips::get(&pool, clip.id).await.unwrap().unwrap();
    assert_eq!(
        saved.output_path,
        Some(previous.to_string_lossy().into_owned())
    );
    assert_eq!(
        std::fs::read(previous).unwrap(),
        b"previous successful product"
    );
    assert_eq!(saved.state, clips::State::Failed);
}

#[tokio::test]
async fn retry_uses_frozen_image_content_after_the_original_asset_changes() {
    use crate::server::workbench::transcode;
    let (dir, pool, session, _) = setup().await;
    let root = dir.path().join("renders");
    let original = png();
    let asset = assets::upload(&pool, &root, session, "image/png", &original, now_ms())
        .await
        .unwrap();
    let recipe = RenderRecipe {
        regions: vec![MaskRegion {
            id: "image".into(),
            effect_type: EffectType::Image,
            asset_id: Some(asset.id),
            ..Default::default()
        }],
        ..Default::default()
    };
    let jobs = Arc::new(RenderJobs::new(pool.clone(), root));
    let permit = transcode::acquire(&tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    let first = jobs
        .enqueue(jobs.make_spec(session, None, None, recipe).await.unwrap())
        .await
        .unwrap();
    let original_path = assets::get(&pool, asset.id)
        .await
        .unwrap()
        .unwrap()
        .render
        .path;
    std::fs::write(original_path, b"external modification").unwrap();
    let frozen = store::get(&pool, first.id).await.unwrap().unwrap();
    assert_eq!(
        std::fs::read(&frozen.spec.assets[0].path).unwrap(),
        original
    );
    jobs.cancel(first.id).await.unwrap();
    jobs.shutdown().await;
    let retry = jobs.retry(first.id).await.unwrap();
    let second = store::get(&pool, retry.id).await.unwrap().unwrap();
    assert_ne!(frozen.spec.assets[0].path, second.spec.assets[0].path);
    assert_eq!(
        std::fs::read(&second.spec.assets[0].path).unwrap(),
        original
    );
    assert!(assets::delete(&pool, asset.id).await.is_err());
    jobs.cancel(retry.id).await.unwrap();
    jobs.shutdown().await;
    drop(permit);
}

#[tokio::test]
async fn job_and_clip_product_publish_in_one_transaction() {
    use crate::server::workbench::clips;
    let (dir, pool, session, _) = setup().await;
    let clip = clips::insert(
        &pool,
        session,
        &clips::NewClip {
            marker_id: None,
            in_ms: 0,
            out_ms: 1000,
            title: "atomic".into(),
            created_by: None,
            created_at: now_ms(),
        },
    )
    .await
    .unwrap();
    let mut snapshot = spec(&pool, dir.path().join("renders"), session).await;
    snapshot.clip_id = Some(clip.id);
    snapshot.in_ms = 0;
    snapshot.out_ms = 1000;
    let job = store::insert(&pool, &snapshot, now_ms()).await.unwrap();
    clips::begin_render_export(&pool, clip.id, job.view.id, now_ms())
        .await
        .unwrap()
        .unwrap();
    store::running(&pool, job.view.id, now_ms()).await.unwrap();
    assert!(
        store::finish(&pool, job.view.id, "completed.mp4", 100, 1000, now_ms())
            .await
            .unwrap()
    );
    let saved = clips::get(&pool, clip.id).await.unwrap().unwrap();
    assert_eq!(saved.state, clips::State::Ready);
    assert_eq!(saved.output_path.as_deref(), Some("completed.mp4"));
    assert_eq!(saved.cut_out_ms, Some(1000));
    let active: Option<i64> = sqlx::query_scalar("SELECT active_render_id FROM clips WHERE id=?")
        .bind(clip.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(active, None);
}

#[tokio::test]
async fn global_jobs_filter_page_and_persist_telemetry_through_terminal_states() {
    let (dir, pool, session, _) = setup().await;
    let original = spec(&pool, dir.path().join("renders"), session).await;
    let first = store::insert(&pool, &original, 1000).await.unwrap();
    let second = store::insert(&pool, &original, 1100).await.unwrap();
    let other_session: i64 = sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at) VALUES('b','https://b','other','2026-10-10','',1,2) RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let other_segment_path = dir.path().join("other-source.mp4");
    std::fs::write(&other_segment_path, b"other source").unwrap();
    sqlx::query("INSERT INTO segments(session_id,path,container,state,start_ms,end_ms) VALUES(?,?,'mp4','finished',0,10000)")
        .bind(other_session)
        .bind(other_segment_path.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    let other_spec = spec(&pool, dir.path().join("renders"), other_session).await;
    let other_job = store::insert(&pool, &other_spec, 1200).await.unwrap();
    assert!(store::running(&pool, first.view.id, 2000).await.unwrap());
    let progress = engine::EngineProgress {
        phase: "合成分段 1/1".into(),
        ratio: Some(0.25),
        encoder: Some("h264_vaapi".into()),
        encoder_device: Some("/dev/dri/renderD128".into()),
        fallback_reason: Some("QSV 不可用，改用 VAAPI".into()),
        fps: Some(60.),
        speed: Some(2.),
        processed_ms: Some(2500),
        total_ms: Some(10000),
    };
    store::progress_snapshot(&pool, first.view.id, &progress, 3000)
        .await
        .unwrap();
    let page = store::list_all(&pool, None, None, 1, 0).await.unwrap();
    assert_eq!(page.total, 3);
    assert_eq!(page.jobs[0].id, other_job.view.id);
    assert_ne!(page.jobs[0].session_id, first.view.session_id);
    let page = store::list_all(&pool, Some("running"), Some(session), 25, 0)
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.jobs[0].id, first.view.id);
    let session_page = store::list_all(&pool, None, Some(session), 25, 0)
        .await
        .unwrap();
    assert_eq!(session_page.total, 2);
    assert_eq!(session_page.jobs[0].id, second.view.id);
    assert_eq!(session_page.jobs[1].id, first.view.id);
    let job = &page.jobs[0];
    assert_eq!(job.session_name, "a");
    assert_eq!(job.requested_encoder, "auto");
    assert_eq!(job.encoder.as_deref(), Some("h264_vaapi"));
    assert_eq!(job.eta_ms, Some(3750));
    assert_eq!(job.started_at, Some(2000));
    let no_fallback_update = engine::EngineProgress {
        fallback_reason: None,
        ..progress.clone()
    };
    store::progress_snapshot(&pool, first.view.id, &no_fallback_update, 4000)
        .await
        .unwrap();
    assert_eq!(
        store::get(&pool, first.view.id)
            .await
            .unwrap()
            .unwrap()
            .view
            .fallback_reason
            .as_deref(),
        Some("QSV 不可用，改用 VAAPI")
    );
    assert!(
        store::terminal(&pool, first.view.id, "cancelled", Some("已取消"), 7000)
            .await
            .unwrap()
    );
    store::progress_snapshot(&pool, first.view.id, &progress, 9000)
        .await
        .unwrap();
    let job = store::get(&pool, first.view.id)
        .await
        .unwrap()
        .unwrap()
        .view;
    assert_eq!(job.finished_at, Some(7000));
    assert_eq!(job.elapsed_ms, 5000);
    assert_eq!(job.eta_ms, None);
    assert_eq!(job.state, "cancelled");
    assert_eq!(job.updated_at, 7000);
    assert_eq!(job.encoder.as_deref(), Some("h264_vaapi"));
    assert_eq!(
        job.fallback_reason.as_deref(),
        Some("QSV 不可用，改用 VAAPI")
    );
    assert_eq!(
        store::list_all(&pool, None, Some(session + 999), 25, 0)
            .await
            .unwrap()
            .total,
        0
    );
    assert!(
        store::list_all(&pool, None, None, 25, 50)
            .await
            .unwrap()
            .jobs
            .is_empty()
    );
}

#[test]
fn legacy_recipes_default_to_auto_and_unknown_encoders_are_rejected() {
    let recipe: RenderRecipe = serde_json::from_str(r#"{"regions":[]}"#).unwrap();
    assert_eq!(
        serde_json::to_value(&recipe.encoding.encoder).unwrap(),
        "auto"
    );
    assert!(serde_json::from_str::<RenderRecipe>(r#"{"encoding":{"encoder":"shell"}}"#).is_err());
}
