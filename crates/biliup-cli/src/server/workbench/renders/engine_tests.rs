use super::super::model::{DanmakuSettings, TimeInterval};
use super::*;
use sha2::{Digest, Sha256};

fn hash(path: &Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(path).unwrap()).to_vec()
}

async fn fixture(path: &Path, color: &str, size: &str, codec: &str, audio: bool, duration: f64) {
    let mut command = crate::tools::ffmpeg_command();
    command.args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        &format!("color=c={color}:s={size}:r=30:d={duration}"),
    ]);
    if audio {
        command.args([
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:sample_rate=44100:duration={duration}"),
        ]);
    }
    command
        .args(["-c:v", codec, "-pix_fmt", "yuv420p", "-threads", "1", "-y"])
        .arg(path);
    let output = command.output().await.unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn source(path: PathBuf, id: i64, start: i64, end: i64, from: i64, to: i64) -> SourceSpan {
    SourceSpan {
        segment_id: id,
        path,
        danmaku_path: None,
        start_ms: start,
        end_ms: end,
        source_origin_ms: Some(100000 + start),
        trim_start_ms: from,
        trim_end_ms: to,
    }
}

async fn pixel(path: &Path, at: f64, x: usize, y: usize, width: usize) -> [u8; 3] {
    let output = crate::tools::ffmpeg_command()
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args([
            "-ss",
            &at.to_string(),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pos = (y * width + x) * 3;
    output.stdout[pos..pos + 3].try_into().unwrap()
}

#[tokio::test]
async fn ffmpeg_combines_mixed_sources_timed_masks_and_images_without_touching_originals() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("first.mp4");
    let b = dir.path().join("second.avi");
    fixture(&a, "blue", "96x64", "libx264", true, 1.).await;
    fixture(&b, "green", "80x80", "mpeg4", false, 1.).await;
    let image_path = dir.path().join("cover.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 0, 255]))
        .save(&image_path)
        .unwrap();
    let originals = (hash(&a), hash(&b), hash(&image_path));
    let settings = RenderRecipe {
        danmaku: DanmakuSettings::default(),
        regions: vec![
            MaskRegion {
                id: "solid".into(),
                effect_type: EffectType::Solid,
                color: "#ff0000".into(),
                x: 0.,
                y: 0.,
                width: 0.3,
                height: 0.3,
                intervals: vec![TimeInterval {
                    from_ms: 200,
                    to_ms: 600,
                }],
                ..Default::default()
            },
            MaskRegion {
                id: "image".into(),
                effect_type: EffectType::Image,
                asset_id: Some(1),
                x: 0.6,
                y: 0.1,
                width: 0.3,
                height: 0.3,
                intervals: vec![TimeInterval {
                    from_ms: 2100,
                    to_ms: 2500,
                }],
                ..Default::default()
            },
            MaskRegion {
                id: "mosaic".into(),
                effect_type: EffectType::Mosaic,
                x: 0.,
                y: 0.6,
                width: 0.3,
                height: 0.3,
                ..Default::default()
            },
            MaskRegion {
                id: "blur".into(),
                effect_type: EffectType::Blur,
                x: 0.4,
                y: 0.6,
                width: 0.3,
                height: 0.3,
                ..Default::default()
            },
        ],
    };
    let target = dir.path().join("result.mp4");
    let mut phases = vec![];
    let done = render(
        EngineRequest {
            sources: vec![
                source(a.clone(), 1, 0, 1000, 100, 1000),
                source(b.clone(), 2, 2000, 3000, 0, 800),
            ],
            settings,
            assets: HashMap::from([("1".into(), image_path.clone())]),
            output_path: target.clone(),
            font_path: None,
        },
        CancellationToken::new(),
        |progress| phases.push(progress),
    )
    .await
    .unwrap();
    assert_eq!((hash(&a), hash(&b), hash(&image_path)), originals);
    assert!(done.output_bytes > 1000);
    assert!((done.duration_ms - 1700).abs() < 100, "{done:?}");
    assert!(phases.iter().any(|p| p.phase.contains("合成分段")));
    assert_eq!(phases.last().unwrap().ratio, Some(1.));
    let info = probe(&target, &CancellationToken::new()).await.unwrap();
    assert_eq!((info.width, info.height, info.has_audio), (96, 64, true));
    let before = pixel(&target, 0.02, 8, 8, 96).await;
    let masked = pixel(&target, 0.3, 8, 8, 96).await;
    let after = pixel(&target, 0.65, 8, 8, 96).await;
    assert!(before[2] > 180 && after[2] > 180, "{before:?} {after:?}");
    assert!(masked[0] > 180 && masked[2] < 80, "{masked:?}");
    // Second square input is pillarboxed into the first canvas. Image x=.6
    // maps to frame x=54 after the contain transform.
    let picture = pixel(&target, 1.15, 58, 14, 96).await;
    let absent = pixel(&target, 1.55, 58, 14, 96).await;
    assert!(
        picture[0] > 180 && picture[1] > 180 && picture[2] < 80,
        "{picture:?}"
    );
    assert!(absent[0] < 80 && absent[1] > 60, "{absent:?}");
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".render-")
    }));
}

#[tokio::test]
async fn cancellation_reaps_children_and_never_publishes_partial_output() {
    let cancel = CancellationToken::new();
    let mut command = crate::tools::ffmpeg_command();
    command.args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-re",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=s=64x64:r=30",
        "-f",
        "null",
        "-",
    ]);
    let cancelling = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        cancelling.cancel();
    });
    let started = std::time::Instant::now();
    let error = run_process(command, &cancel, Duration::from_secs(10), |_, _| {})
        .await
        .unwrap_err();
    assert!(error.contains("取消"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn non_advancing_process_is_stopped_by_watchdog() {
    let mut command = crate::tools::ffmpeg_command();
    command.args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-re",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=s=64x64:r=30",
        "-f",
        "null",
        "-",
    ]);
    let error = run_process(
        command,
        &CancellationToken::new(),
        Duration::from_millis(100),
        |_, _| {},
    )
    .await
    .unwrap_err();
    assert!(error.contains("没有推进"), "{error}");
}

#[tokio::test]
async fn factory_ass_remains_in_flight_at_clip_start_and_preserves_xml() {
    if crate::tools::danmaku_factory().is_err() || crate::tools::render_font().is_none() {
        crate::tools::note_skipped_test(
            "set BILIUP_RENDER_TOOLS_DIR to test real DanmakuFactory/libass export",
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("source.mp4");
    fixture(&original, "black", "320x180", "libx264", false, 12.).await;
    let xml = dir.path().join("source.xml");
    std::fs::write(&xml, "<i><recording_start_time_ms>100000</recording_start_time_ms><d p=\"8.000,1,25,16777215,108,0,0,0\">HELLO</d></i>").unwrap();
    let original_hash = (hash(&original), hash(&xml));
    let mut span = source(original.clone(), 1, 0, 12000, 10000, 11000);
    span.danmaku_path = Some(xml.clone());
    let mut recipe = RenderRecipe::default();
    recipe.danmaku.enabled = true;
    recipe.danmaku.font_size = 26.;
    recipe.danmaku.scroll_seconds = 4.;
    recipe.danmaku.opacity = 1.;
    let preview = preview(&span, &recipe, &CancellationToken::new())
        .await
        .unwrap();
    assert!(preview.ass.contains("\\move("), "{}", preview.ass);
    assert!(
        preview.ass.contains("0:00:08.00,0:00:12.00"),
        "{}",
        preview.ass
    );
    assert!(!preview.estimated_timing);
    let target = dir.path().join("clip.mp4");
    let done = render(
        EngineRequest {
            sources: vec![span],
            settings: recipe,
            assets: HashMap::new(),
            output_path: target.clone(),
            font_path: crate::tools::render_font(),
        },
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!((hash(&original), hash(&xml)), original_hash);
    assert!((done.duration_ms - 1000).abs() < 100, "{done:?}");
    let frame = crate::tools::ffmpeg_command()
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(&target)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .await
        .unwrap();
    assert!(frame.status.success());
    let bright = frame
        .stdout
        .chunks_exact(3)
        .enumerate()
        .filter_map(|(i, p)| (p[0] > 160 && p[1] > 160 && p[2] > 160).then_some(i % 320))
        .collect::<Vec<_>>();
    assert!(
        bright.len() > 50,
        "first clip frame lost a comment that started before the in-point"
    );
    assert!(
        *bright.iter().max().unwrap() < 260 && *bright.iter().min().unwrap() > 60,
        "the pre-existing comment restarted at the right edge: {bright:?}"
    );

    // Compare with a same-clock libass reference: the first frame of a tail
    // export must show the same in-flight motion as rendering at segment t=10.
    let reference_dir = tempfile::tempdir().unwrap();
    std::fs::write(reference_dir.path().join("preview.ass"), &preview.ass).unwrap();
    let reference = crate::tools::ffmpeg_command()
        .current_dir(reference_dir.path())
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(&original)
        .args([
            "-vf",
            "ass=preview.ass",
            "-ss",
            "10",
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .await
        .unwrap();
    assert!(reference.status.success());
    let reference_bright = reference
        .stdout
        .chunks_exact(3)
        .enumerate()
        .filter_map(|(i, p)| (p[0] > 160 && p[1] > 160 && p[2] > 160).then_some(i % 320))
        .collect::<Vec<_>>();
    let center = |xs: &[usize]| xs.iter().sum::<usize>() as f64 / xs.len() as f64;
    assert!(
        (center(&bright) - center(&reference_bright)).abs() < 3.,
        "ASS motion changed across input seek"
    );
}

#[tokio::test]
async fn audio_delay_and_first_source_frame_rate_survive_export() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("delayed.mkv");
    let output = crate::tools::ffmpeg_command()
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=96x64:r=24:d=1",
        ])
        .args([
            "-itsoffset",
            "0.4",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=0.6",
        ])
        .args([
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-c:a",
            "pcm_s16le",
            "-output_ts_offset",
            "5",
            "-y",
        ])
        .arg(&original)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original_hash = hash(&original);
    let source_info = probe(&original, &CancellationToken::new()).await.unwrap();
    assert!((source_info.video_origin_seconds - 5.).abs() < 0.01);
    let target = dir.path().join("result.mp4");
    render(
        EngineRequest {
            sources: vec![source(original.clone(), 1, 0, 1000, 0, 1000)],
            settings: RenderRecipe::default(),
            assets: HashMap::new(),
            output_path: target.clone(),
            font_path: None,
        },
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(hash(&original), original_hash);
    let info = probe(&target, &CancellationToken::new()).await.unwrap();
    assert_eq!(info.frame_rate, "24/1");
    let audio = crate::tools::ffmpeg_command()
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(&target)
        .args([
            "-map", "0:a:0", "-f", "f32le", "-ac", "1", "-ar", "48000", "-",
        ])
        .output()
        .await
        .unwrap();
    assert!(audio.status.success());
    let samples: Vec<f32> = audio
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    let rms = |from: usize, to: usize| {
        (samples[from..to]
            .iter()
            .map(|sample| (*sample as f64).powi(2))
            .sum::<f64>()
            / (to - from) as f64)
            .sqrt()
    };
    assert!(
        rms(2400, 9600) < 0.002,
        "originally delayed audio moved to the clip start"
    );
    assert!(rms(26000, 33000) > 0.04, "delayed tone is missing");
}
