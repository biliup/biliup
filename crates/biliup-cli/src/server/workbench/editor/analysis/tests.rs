use super::*;
use crate::server::workbench::editor;

async fn settled(jobs: &Jobs, project: i64) -> Value {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let all = jobs.list(project).await.unwrap();
            if let Some(job) = all.first()
                && !active(job)
            {
                return job.clone();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("analysis finishes")
}

#[tokio::test]
async fn analysis_queue_deduplicates_scopes_and_cancels_waiters() {
    let (dir, pool, doc, path) = editor::tests::fixture().await;
    let saved = editor::save(&pool, None, doc.clone()).await.unwrap();
    let other = editor::save(&pool, None, doc).await.unwrap();
    let media = saved.document.sources.keys().next().unwrap();
    let jobs = Arc::new(Jobs::new(pool.clone(), dir.path().to_owned()));
    let permit = crate::server::common::processing::transcode::acquire(&CancellationToken::new())
        .await
        .unwrap();
    let first = jobs
        .start(saved.summary.id, media.clone(), None)
        .await
        .unwrap();
    let again = jobs
        .start(saved.summary.id, media.clone(), None)
        .await
        .unwrap();
    assert_eq!(first[0]["jobId"], again[0]["jobId"]);
    assert_eq!(jobs.list(saved.summary.id).await.unwrap().len(), 1);
    assert!(
        jobs.start(saved.summary.id, "foreign-media".into(), None)
            .await
            .is_err()
    );
    assert!(
        jobs.cached(saved.summary.id, "foreign-media", "loudness", &[])
            .await
            .is_err()
    );
    assert!(
        jobs.cancel(other.summary.id, first[0]["jobId"].as_str().unwrap())
            .await
            .is_err()
    );
    jobs.cancel(saved.summary.id, first[0]["jobId"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(settled(&jobs, saved.summary.id).await["phase"], "cancelled");
    let multiple = jobs
        .start(
            saved.summary.id,
            media.clone(),
            Some(vec!["loudness".into(), "beats".into(), "beats".into()]),
        )
        .await
        .unwrap();
    assert_eq!(multiple.len(), 2);
    assert_ne!(multiple[0]["jobId"], multiple[1]["jobId"]);
    for job in &multiple {
        jobs.cancel(saved.summary.id, job["jobId"].as_str().unwrap())
            .await
            .unwrap();
    }
    let seconds = saved.document.sources[media].duration_ms().unwrap() as f64 / 1000.;
    let beats_key = fingerprint_for(&path, seconds, "beats").await.unwrap();
    assert_ne!(beats_key, fingerprint(&path, seconds).await.unwrap());
    let result = json!({"hasAudio":true,"seconds":seconds,"bpm":120,"confidence":0.9,"beats":[0.5,1.,1.5],"onsets":[],"beatLevels":[1.,1.,1.]});
    write(dir.path(), &beats_key, &result).await.unwrap();
    let cached = jobs
        .cached(saved.summary.id, media, "beats", &["onsets".into()])
        .await
        .unwrap();
    assert_eq!(cached["result"]["bpm"], 120);
    assert!(cached["result"].get("onsets").is_none());
    assert!(
        jobs.cached(saved.summary.id, media, "loudness", &[])
            .await
            .unwrap()["result"]
            .is_null()
    );
    assert_eq!(
        jobs.start(saved.summary.id, media.clone(), Some(vec!["beats".into()]))
            .await
            .unwrap(),
        vec![json!({"analyzer":"beats","state":"done"})]
    );
    let reopened = Jobs::new(pool.clone(), dir.path().into());
    assert_eq!(
        reopened
            .cached(saved.summary.id, media, "beats", &[])
            .await
            .unwrap()["result"],
        result
    );
    std::fs::write(&path, b"different audio").unwrap();
    assert!(
        reopened
            .cached(saved.summary.id, media, "beats", &[])
            .await
            .unwrap()["result"]
            .is_null()
    );
    drop(permit);
    use crate::server::infrastructure::permissions::{Permission, required_permission};
    assert_eq!(
        required_permission(
            &axum::http::Method::POST,
            "/v1/editor/projects/{id}/analysis",
            "/v1/editor/projects/1/analysis"
        ),
        Some(Permission::ClipEdit)
    );
    assert_eq!(
        required_permission(
            &axum::http::Method::GET,
            "/v1/editor/projects/{id}/analysis/media/{media}",
            "/v1/editor/projects/1/analysis/media/example"
        ),
        Some(Permission::FileView)
    );
}

#[tokio::test]
#[ignore = "requires FFmpeg"]
async fn analysis_ffmpeg_silence_envelope_cache_and_source_change() {
    let (dir, pool, doc, path) = editor::tests::fixture().await;
    let status = crate::tools::ffmpeg_command()
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            r"aevalsrc=if(between(t\,1\,3)\,0.25*sin(2*PI*440*t)\,0):s=16000:d=4",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
        ])
        .arg(&path)
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let saved = editor::save(&pool, None, doc).await.unwrap();
    let media = saved.document.sources.keys().next().unwrap();
    let jobs = Arc::new(Jobs::new(pool.clone(), dir.path().to_owned()));
    assert!(
        jobs.cached(saved.summary.id, media, "loudness", &[])
            .await
            .unwrap()["result"]
            .is_null()
    );
    jobs.start(saved.summary.id, media.clone(), None)
        .await
        .unwrap();
    let job = settled(&jobs, saved.summary.id).await;
    assert_eq!(job["phase"], "done", "{job}");
    let result = jobs
        .cached(saved.summary.id, media, "loudness", &[])
        .await
        .unwrap()["result"]
        .clone();
    assert_eq!(result["seconds"], 4.);
    assert!(result["integratedLufs"].as_f64().unwrap() < -10.);
    let ranges = result["silence"]["ranges"].as_array().unwrap();
    assert_eq!(ranges.len(), 2, "{ranges:?}");
    assert!((ranges[0]["end"].as_f64().unwrap() - 1.).abs() < 0.02);
    assert!((ranges[1]["start"].as_f64().unwrap() - 3.).abs() < 0.02);
    use base64::Engine;
    let bins = base64::engine::general_purpose::STANDARD
        .decode(result["envelope"]["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(bins.len(), 400);
    assert_eq!(bins[40], 0);
    assert!((165..=175).contains(&bins[150]));
    assert_eq!(
        jobs.start(saved.summary.id, media.clone(), None)
            .await
            .unwrap()[0]["state"],
        "done"
    );
    let restarted = Jobs::new(pool, dir.path().to_owned());
    assert_eq!(
        restarted
            .cached(saved.summary.id, media, "loudness", &[])
            .await
            .unwrap()["result"],
        result
    );
    let without = restarted
        .cached(saved.summary.id, media, "loudness", &["envelope".into()])
        .await
        .unwrap();
    assert!(without["result"].get("envelope").is_none());
    tokio::fs::write(&path, b"changed media").await.unwrap();
    assert!(
        restarted
            .cached(saved.summary.id, media, "loudness", &[])
            .await
            .unwrap()["result"]
            .is_null()
    );
    assert!(
        loudness::analyze(&path, 4., &CancellationToken::new(), |_| {})
            .await
            .is_err()
    );
    let silent_video = dir.path().join("video-no-audio.mp4");
    assert!(
        crate::tools::ffmpeg_command()
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=32x32:d=1",
                "-an"
            ])
            .arg(&silent_video)
            .status()
            .await
            .unwrap()
            .success()
    );
    assert_eq!(
        loudness::analyze(&silent_video, 1., &CancellationToken::new(), |_| {})
            .await
            .unwrap(),
        json!({"hasAudio":false})
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        beats::analyze(&silent_video, 1., &CancellationToken::new(), |_| {})
            .await
            .unwrap(),
        json!({"hasAudio":false})
    );
    assert!(beats::analyze(&path, 4., &cancel, |_| {}).await.is_err());
    assert!(
        beats::analyze(&path, 4., &CancellationToken::new(), |_| {})
            .await
            .is_err()
    );
    assert!(
        loudness::analyze(&silent_video, 1., &cancel, |_| {})
            .await
            .unwrap_err()
            .contains("取消")
    );
}
