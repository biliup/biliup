use super::*;

#[tokio::test]
#[ignore = "requires Chromium and FFmpeg"]
async fn recorded_danmaku_is_a_transparent_layer_above_masked_video() {
    use crate::server::{infrastructure::connection_pool::ConnectionManager, workbench::editor};
    let temp = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let source = temp.path().join("source.mp4");
    let xml = temp.path().join("comments.xml");
    let mut fixture = crate::tools::ffmpeg_command();
    fixture
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=blue:size=320x180:rate=30:duration=2",
            "-c:v",
            "libx264",
            "-y",
        ])
        .arg(&source);
    engine::run_process(fixture, &cancel, Duration::from_secs(30), |_, _| {})
        .await
        .unwrap();
    std::fs::write(
        &xml,
        r#"<?xml version="1.0"?><i><d p="0,5,25,16777215,0,0,0,0">弹幕图层</d></i>"#,
    )
    .unwrap();
    let pool = ConnectionManager::new_pool(temp.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let session:i64=sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at) VALUES('test','https://test','test','2026-10-25','',1000,3000) RETURNING id").fetch_one(&pool).await.unwrap();
    let segment:i64=sqlx::query_scalar("INSERT INTO segments(session_id,path,danmaku_path,container,state,start_ms,end_ms) VALUES(?,?,?,'mp4','finished',0,2000) RETURNING id")
        .bind(session).bind(source.to_string_lossy().as_ref()).bind(xml.to_string_lossy().as_ref()).fetch_one(&pool).await.unwrap();
    let settings = json!({"enabled":true,"font":"Noto Sans CJK SC","font_size":25,"opacity":1,"outline":0,"scroll_seconds":12,"display_area":0.4,"density":-1,"offset_ms":0,"segment_offsets":{}});
    let masked = json!({"id":"video","type":"clip","media":"source","duration":60,"effects":[{"module":"effect.biliup-mask","params":{"regions":[{"id":"cover","effect_type":"solid","x":0,"y":0,"width":0.2,"height":0.2,"strength":12,"color":"#000000","opacity":1,"asset_id":null,"lock_aspect":true,"intervals":[]}]}}]});
    let comments = json!({"id":"danmaku","type":"danmaku","media":"source","in":0,"start":0,"duration":60,"linkedTo":"video","danmaku":{"width":320,"height":180,"settings":settings}});
    let tracks = json!([{"id":"comments","kind":"text","name":"弹幕","items":[comments]},{"id":"main","kind":"video","name":"主视频","role":"main","items":[masked]}]);
    let project = json!({"format":"biliup.timeline/1","name":"layer check","canvas":{"width":320,"height":180,"fps":30},"media":[{"id":"source","kind":"video","name":"source.mp4","path":"source.mp4","probe":{"width":320,"height":180,"fps":30,"frames":60,"hasAudio":false}}],"tracks":tracks});
    let mut doc:editor::Document=serde_json::from_value(json!({"format":"biliup.editor/1","origin":{"sessionId":session,"inMs":0,"outMs":2000},"sourceMarkers":[],"warnings":[],
      "sources":{"source":{"kind":"recording","sessionId":session,"segmentId":segment,"viewSessionId":session,"viewStartMs":0,"sourceStartMs":0,"durationMs":2000,"fileName":"source.mp4","state":"finished","hasDanmaku":true}},"project":project})).unwrap();
    let mut paths = BTreeMap::from([("source".into(), source.to_string_lossy().into_owned())]);
    let mut browser = browser::Browser::launch(&cancel).await.unwrap();
    recording_effects::prepare(
        &pool,
        &mut browser,
        recording_effects::Preparation {
            document: &mut doc,
            range: &Value::Null,
            paths: &mut paths,
            directory: temp.path(),
        },
        &cancel,
        |_, _| {},
    )
    .await
    .unwrap();
    browser.close().await;
    let layer = doc.project["tracks"][0]["items"][0]["media"]
        .as_str()
        .unwrap();
    assert_ne!(
        layer, "source",
        "text layers have no effects array, but still need an alpha intermediate"
    );
    let frame = crate::tools::ffmpeg_command()
        .args([
            "-v",
            "error",
            "-ss",
            "1",
            "-i",
            paths.get(layer).unwrap(),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "pipe:1",
        ])
        .output()
        .await
        .unwrap();
    assert!(frame.status.success());
    assert_eq!(frame.stdout.len(), 320 * 180 * 4);
    assert!(
        frame.stdout.chunks_exact(4).any(|p| p[3] > 128),
        "comments must be visible"
    );
    assert_eq!(
        frame.stdout[(160 * 320 + 200) * 4 + 3],
        0,
        "background must stay transparent"
    );
    let output = temp.path().join("final.mp4");
    render(
        doc.project,
        paths,
        serde_json::from_value(
            json!({"quality":"draft","video":{"codec":"h264","encoder":"software"}}),
        )
        .unwrap(),
        temp.path(),
        &output,
        &cancel,
        |_| {},
    )
    .await
    .unwrap();
    let frame = crate::tools::ffmpeg_command()
        .args(["-v", "error", "-ss", "1", "-i"])
        .arg(&output)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .output()
        .await
        .unwrap();
    assert!(frame.status.success());
    assert_eq!(frame.stdout.len(), 320 * 180 * 3);
    assert!(
        frame.stdout[(10 * 320 + 10) * 3..(10 * 320 + 10) * 3 + 3]
            .iter()
            .all(|v| *v < 25),
        "mask must remain visible below comments"
    );
    assert!(
        frame.stdout[(160 * 320 + 200) * 3 + 2] > 180,
        "video must show through the text layer"
    );
    assert!(
        frame
            .stdout
            .chunks_exact(3)
            .any(|p| p.iter().all(|v| *v > 180)),
        "text must survive final compositing"
    );
}

/// Real browser + FFmpeg integration, opt-in for hosts with both installed.
/// No user's recording, browser profile, or project is modified.
#[tokio::test]
#[ignore = "requires Chromium and FFmpeg"]
async fn multi_track_export_renders_chinese_text_transition_and_audio() {
    let temp = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let source = temp.path().join("source.mp4");
    let mut fixture = crate::tools::ffmpeg_command();
    fixture
        .args([
            "-hide_banner",
            "-nostdin",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=30:duration=3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&source);
    engine::run_process(fixture, &cancel, Duration::from_secs(30), |_, _| {})
        .await
        .unwrap();
    let project = json!({"format":"biliup.timeline/1","name":"多轨导出验收","canvas":{"width":320,"height":180,"fps":30},
    "media":[{"id":"video","name":"原片","kind":"video","path":"media.mp4","probe":{"width":320,"height":180,"fps":30,"frames":90,"hasAudio":true}}],
    "tracks":[
        {"id":"text","name":"标题","kind":"text","items":[{"id":"title","type":"text","start":0,"duration":45,"text":"中文标题","style":{"size":24}}]},
        {"id":"main","name":"主视频","kind":"video","role":"main","items":[
            {"id":"first","type":"clip","media":"video","in":0,"duration":45},
            {"id":"second","type":"clip","media":"video","in":15,"duration":45,"transitionIn":{"module":"transition.crossfade","duration":12}}
        ]},
        {"id":"music","name":"音乐","kind":"audio","items":[{"id":"sound","type":"audio","media":"video","start":0,"in":0,"duration":90,"gainDb":-12}]}
    ]});
    let output = temp.path().join("output.mp4");
    let mut updates = vec![];
    let options: Options = serde_json::from_value(
        json!({"quality":"draft","video":{"encoder":"software"},"targetLufs":-14}),
    )
    .unwrap();
    let result = render(
        project,
        BTreeMap::from([("video".into(), source.to_string_lossy().into_owned())]),
        options,
        temp.path(),
        &output,
        &cancel,
        |p| updates.push(p),
    )
    .await
    .unwrap();
    assert_eq!((result.width, result.height), (320, 180));
    assert!((result.seconds - 3.).abs() < 0.05, "{}", result.seconds);
    assert!(result.bytes > 1000);
    assert!(
        updates
            .iter()
            .any(|p| matches!(p, Progress::Preparing { done: 1, .. }))
    );
    assert!(
        updates
            .iter()
            .any(|p| matches!(p, Progress::Encoding { .. }))
    );
    let probe = engine::probe(&output, &cancel).await.unwrap();
    assert!(probe.has_audio);
    assert_eq!((probe.width, probe.height), (320, 180));
    let mut audio = crate::tools::ffmpeg_command();
    audio
        .arg("-i")
        .arg(&output)
        .args(["-af", "volumedetect", "-vn", "-f", "null", "-"]);
    let levels = engine::run_process(audio, &cancel, Duration::from_secs(30), |_, _| {})
        .await
        .unwrap();
    assert!(!levels.contains("max_volume: -inf"));
    // Keep only an explicit test artifact outside the user's projects for visual review.
    if let Some(directory) = std::env::var_os("BILIUP_EDITOR_TEST_OUTPUT") {
        let directory = std::path::PathBuf::from(directory);
        tokio::fs::create_dir_all(&directory).await.unwrap();
        tokio::fs::copy(&output, directory.join("multi-track.mp4"))
            .await
            .unwrap();
        let mut image = crate::tools::ffmpeg_command();
        image
            .args(["-y", "-ss", "0.5", "-i"])
            .arg(&output)
            .args(["-frames:v", "1"])
            .arg(directory.join("multi-track.png"));
        engine::run_process(image, &cancel, Duration::from_secs(30), |_, _| {})
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn renderer_blocks_network_and_honors_cancellation() {
    let cancel = CancellationToken::new();
    let mut browser = browser::Browser::launch(&cancel).await.unwrap();
    assert_eq!(
        browser
            .eval("isSecureContext && location.href === 'https://biliup-editor.invalid/'")
            .await
            .unwrap(),
        true
    );
    let value = browser
        .eval("fetch('http://127.0.0.1:9/forbidden').then(()=>false,()=>true)")
        .await
        .unwrap();
    assert_eq!(value, true);
    cancel.cancel();
    assert!(
        browser
            .eval("new Promise(()=>{})")
            .await
            .unwrap_err()
            .contains("取消")
    );
    browser.close().await;
}

#[tokio::test]
#[ignore = "requires Chromium with WebGPU and FFmpeg"]
async fn shader_export_preserves_time_parameters_and_alpha() {
    let temp = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let source = temp.path().join("source.mp4");
    let mut fixture = crate::tools::ffmpeg_command();
    fixture
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=64x32:r=30:d=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&source);
    engine::run_process(fixture, &cancel, Duration::from_secs(30), |_, _| {})
        .await
        .unwrap();
    let project = json!({"format":"biliup.timeline/1","name":"着色器验收","canvas":{"width":64,"height":32,"fps":30},
      "media":[{"id":"video","name":"蓝色原片","kind":"video","path":"source.mp4","probe":{"width":64,"height":32,"fps":30,"frames":30,"hasAudio":false}}],
      "tracks":[{"id":"main","name":"主视频","kind":"video","role":"main","items":[{"id":"clip","type":"clip","media":"video","in":0,"duration":30,
        "effects":[{"module":"effect.shader","params":{"shader":"project.clock","name":"时间与颜色","fields":[{"key":"amount","label":"亮度","type":"number","min":0,"max":1,"default":1}],"values":{"amount":0.25},
          "wgsl":"struct Params { amount: f32 } fn effect(uv: vec2f, t: f32, size: vec2f, p: Params) -> vec4f { return vec4f(t, p.amount, frame(uv).b, 1.0); }"}}]}]}]});
    let paths = BTreeMap::from([("video".into(), source.to_string_lossy().into_owned())]);
    let mut browser = browser::Browser::launch(&cancel).await.unwrap();
    let plan = browser
        .eval(&format!(
            "BiliupExport.prepare({project},{},{{quality:'draft'}})",
            serde_json::to_string(&paths).unwrap()
        ))
        .await
        .unwrap();
    let index = plan["shaders"][0]["index"].as_u64().unwrap();
    let rgba = STANDARD.encode([10, 20, 255, 128].repeat(64 * 32));
    let drawn = browser
        .eval(&format!("BiliupExport.shade({index},15,'{rgba}')"))
        .await
        .unwrap();
    let pixels = STANDARD.decode(drawn.as_str().unwrap()).unwrap();
    assert_eq!(pixels.len(), 64 * 32 * 4);
    assert_eq!(
        &pixels[..4],
        &[128, 64, 255, 128],
        "clip time, uniform and straight alpha must match preview"
    );
    browser.close().await;
    let output = temp.path().join("shader.mp4");
    let options =
        serde_json::from_value(json!({"quality":"draft","video":{"encoder":"software"}})).unwrap();
    let result = render(
        project,
        paths,
        options,
        temp.path(),
        &output,
        &cancel,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!((result.width, result.height), (64, 32));
    assert!((result.seconds - 1.).abs() < 0.01);
    let mut decode = crate::tools::ffmpeg_command();
    let picture = decode
        .args(["-v", "error", "-ss", "0.5", "-i"])
        .arg(&output)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .output()
        .await
        .unwrap();
    assert!(picture.status.success());
    assert_eq!(picture.stdout.len(), 64 * 32 * 3);
    for (actual, expected) in picture.stdout[..3].iter().zip([128u8, 64, 255]) {
        assert!(
            actual.abs_diff(expected) < 9,
            "encoded pixel {:?}",
            &picture.stdout[..3]
        );
    }
    if let Some(directory) = std::env::var_os("BILIUP_EDITOR_TEST_OUTPUT") {
        let directory = std::path::PathBuf::from(directory);
        tokio::fs::create_dir_all(&directory).await.unwrap();
        tokio::fs::copy(output, directory.join("shader.mp4"))
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "requires Chromium and FFmpeg"]
async fn project_export_queue_freezes_revision_cancels_and_persists_results() {
    use crate::server::{infrastructure::connection_pool::ConnectionManager, workbench::editor};
    use std::sync::Arc;
    let temp = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(temp.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let doc:editor::Document=serde_json::from_value(json!({"format":"biliup.editor/1","origin":{"sessionId":null,"inMs":0,"outMs":0},"sources":{},"sourceMarkers":[],"warnings":[],
        "project":{"format":"biliup.timeline/1","name":"Title export","canvas":{"width":320,"height":180,"fps":30},"media":[],"tracks":[{"id":"main","name":"主视频","kind":"video","role":"main","items":[]},{"id":"text","name":"字幕","kind":"text","items":[{"id":"title","type":"text","start":0,"duration":30,"text":"独立工程"}]}]}})).unwrap();
    let saved = editor::save(&pool, None, doc).await.unwrap();
    let service = Arc::new(jobs::Jobs::new(pool.clone(), temp.path().join("outputs")));
    service.initialize().await.unwrap();
    let options = json!({"quality":"draft","loudness":"off","output":"../../source.mp4"});
    assert!(
        service
            .enqueue(saved.summary.id, 99, options.clone())
            .await
            .is_err()
    );
    let permit = crate::server::common::processing::transcode::acquire(&CancellationToken::new())
        .await
        .unwrap();
    let id = service
        .enqueue(saved.summary.id, 1, options.clone())
        .await
        .unwrap();
    let id = id
        .strip_prefix("editor-export-")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    assert!(editor::delete(&pool, saved.summary.id, 1).await.is_err());
    assert!(service.cancel(saved.summary.id + 1, id).await.is_err());
    service.cancel(saved.summary.id, id).await.unwrap();
    drop(permit);
    async fn terminal(service: &jobs::Jobs, project: i64) -> Value {
        tokio::time::timeout(Duration::from_secs(45), async {
            loop {
                let row = service.list(project).await.unwrap().remove(0);
                if ["done", "failed", "cancelled"].contains(&row["phase"].as_str().unwrap()) {
                    break row;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap()
    }
    assert_eq!(
        terminal(&service, saved.summary.id).await["phase"],
        "cancelled"
    );
    assert!(service.download(saved.summary.id, id).await.is_err());
    let id = service.enqueue(saved.summary.id, 1, options).await.unwrap();
    let id = id
        .strip_prefix("editor-export-")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let job = terminal(&service, saved.summary.id).await;
    assert_eq!(job["phase"], "done", "{job}");
    let (file, name) = service.download(saved.summary.id, id).await.unwrap();
    assert_eq!(name, "source.mp4");
    assert!(file.starts_with(temp.path().join("outputs")));
    assert!(file.exists());
    assert!(!temp.path().join("source.mp4").exists());
    // Finished outputs survive deletion of their editable source project.
    editor::delete(&pool, saved.summary.id, 1).await.unwrap();
    let (retained, _) = service.download(saved.summary.id, id).await.unwrap();
    assert_eq!(retained, file);
    assert!(retained.exists());
    let reopened = jobs::Jobs::new(pool, temp.path().join("outputs"));
    reopened.initialize().await.unwrap();
    assert_eq!(
        reopened.list(saved.summary.id).await.unwrap()[0]["phase"],
        "done"
    );
}
