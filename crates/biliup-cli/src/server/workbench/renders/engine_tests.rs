use super::super::encoding::{EncoderConfig, VideoEncoder};
use super::super::model::{DanmakuSettings, TimeInterval};
use super::*;

fn cpu_recipe() -> RenderRecipe {
    RenderRecipe {
        encoding: EncoderConfig {
            encoder: VideoEncoder::Libx264,
            device: None,
            ..Default::default()
        },
        ..Default::default()
    }
}
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

#[test]
fn qsv_graph_keeps_mosaic_blocks_and_separates_smooth_blur() {
    let source = source(PathBuf::from("source.flv"), 1, 0, 60_000, 0, 60_000);
    let media = MediaInfo {
        width: 1920,
        height: 1080,
        has_audio: true,
        duration_ms: Some(60_000),
        format_start_seconds: 0.,
        video_origin_seconds: 0.05,
        frame_rate: "60/1".into(),
    };
    let encoder = super::encoding::SelectedEncoder {
        encoder: VideoEncoder::H264Qsv,
        device: None,
        fallback_reason: None,
    };
    let region = MaskRegion {
        id: "face".into(),
        effect_type: EffectType::SmoothBlur,
        x: 0.5,
        y: 0.4,
        width: 0.2,
        height: 0.2,
        strength: 12.,
        ..Default::default()
    };
    let settings = RenderRecipe {
        encoding: EncoderConfig {
            encoder: VideoEncoder::H264Qsv,
            device: None,
            ..Default::default()
        },
        danmaku: DanmakuSettings {
            enabled: true,
            ..Default::default()
        },
        regions: vec![region.clone()],
    };
    let smooth = qsv_filter_graph(
        &source,
        &settings,
        &media,
        (1920, 1080),
        0,
        true,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    assert!(smooth.contains("scale_qsv=w=384:h=216:format=nv12"));
    assert!(smooth.contains("async_depth=8"));
    assert!(smooth.contains("cw=1920:ch=432:cx=0:cy=0:w=1920:h=432"));
    assert!(smooth.contains("ass=filename=danmaku-qsv.ass"));
    assert!(smooth.contains("fps=60/1,format=nv12,hwupload=extra_hw_frames=64[danmaku]"));
    assert_eq!(smooth.matches("hwdownload").count(), 1); // ASS only
    assert!(smooth.contains("alpha=255"));
    assert!(smooth.contains("setpts=PTS-(0.050000000)/TB"));

    // VPP smooth blur rounds odd low-resolution sizes up for NV12 surfaces;
    // OpenCL mosaic retains the exact software grid in its coordinate maps.
    let odd_smooth_region = MaskRegion {
        x: 0.,
        y: 0.,
        width: 0.135,
        height: 0.1,
        ..region.clone()
    };
    let odd_smooth_settings = RenderRecipe {
        regions: vec![odd_smooth_region.clone()],
        ..settings.clone()
    };
    let odd_smooth = qsv_filter_graph(
        &source,
        &odd_smooth_settings,
        &media,
        (1920, 1080),
        0,
        true,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    assert!(odd_smooth.contains(
        "vpp_qsv=cw=260:ch=108:cx=0:cy=0:w=22:h=10:async_depth=8,scale_qsv=w=260:h=108:format=nv12"
    ));
    let odd_mosaic = RenderRecipe {
        regions: vec![MaskRegion {
            effect_type: EffectType::Mosaic,
            ..odd_smooth_region
        }],
        ..settings.clone()
    };
    let odd_mosaic_graph = qsv_filter_graph(
        &source,
        &odd_mosaic,
        &media,
        (1920, 1080),
        0,
        true,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    assert!(odd_mosaic_graph.contains("scale=21:9:flags=neighbor,scale=260:108:flags=neighbor"));

    let band_ass =
        qsv_danmaku_ass(&super::super::danmaku::empty_ass(1920, 1080), 1080, 432).unwrap();
    assert!(band_ass.contains("PlayResY: 432"));
    let scrolling_ass = format!(
        "{}Dialogue: 0,0:00:00.00,0:00:12.00,R2L,,0000,0000,0000,,{{\\move(2030,38,-110,38)}}scrolling\n",
        super::super::danmaku::empty_ass(1920, 1080),
    );
    assert!(qsv_danmaku_ass(&scrolling_ass, 1080, 432).is_some());
    let special_ass = format!(
        "{}Dialogue: 0,0:00:00.00,0:00:05.00,SP,,0000,0000,0000,,{{\\pos(100,864)}}special\n",
        super::super::danmaku::empty_ass(1920, 1080),
    );
    assert!(qsv_danmaku_ass(&special_ass, 1080, 432).is_none());

    let mosaic = RenderRecipe {
        regions: vec![MaskRegion {
            effect_type: EffectType::Mosaic,
            ..region.clone()
        }],
        ..settings.clone()
    };
    let mosaic_graph = qsv_filter_graph(
        &source,
        &mosaic,
        &media,
        (1920, 1080),
        0,
        true,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    assert!(mosaic_graph.contains(
        "vpp_qsv=cw=384:ch=216:cx=960:cy=432:w=384:h=216:async_depth=8,hwmap=derive_device=opencl:mode=read+direct,format=opencl"
    ));
    assert!(mosaic_graph.contains("format=opencl[remap-source0]"));
    assert!(mosaic_graph.contains("scale=32:18:flags=neighbor,scale=384:216:flags=neighbor"));
    assert!(mosaic_graph.contains("remap_opencl=interp=near"));
    assert!(mosaic_graph.contains(
        "remap_opencl=interp=near,hwmap=derive_device=qsv:mode=write+direct:reverse=1,format=qsv[effect0]"
    ));
    assert_eq!(mosaic_graph.matches("hwdownload").count(), 1); // ASS band only

    let legacy_mosaic_graph = qsv_filter_graph(
        &source,
        &mosaic,
        &media,
        (1920, 1080),
        0,
        true,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::QsvVpp,
    )
    .unwrap();
    assert!(legacy_mosaic_graph.contains(
        "vpp_qsv=cw=384:ch=216:cx=960:cy=432:w=32:h=18:async_depth=8,hwdownload,format=nv12,scale=384:216:flags=neighbor"
    ));
    assert_eq!(legacy_mosaic_graph.matches("hwdownload").count(), 2); // ROI plus ASS

    let mut timed = mosaic;
    timed.regions[0].intervals.push(TimeInterval {
        from_ms: 0,
        to_ms: 1000,
    });
    assert!(
        qsv_filter_graph(
            &source,
            &timed,
            &media,
            (1920, 1080),
            0,
            true,
            1,
            false,
            "60/1",
            &encoder,
            QsvMosaicBackend::OpenClNearest,
        )
        .is_none()
    );
    assert_eq!(
        serde_json::to_string(&EffectType::SmoothBlur).unwrap(),
        "\"smooth_blur\""
    );
}

#[test]
fn qsv_parallel_parts_keep_integer_rate_frame_boundaries() {
    let mut span = source(PathBuf::from("source.flv"), 7, 0, 90_000, 100, 60_100);
    span.source_origin_ms = Some(900_000);
    let parts = qsv_parallel_parts(&span, "60/1").unwrap();
    assert_eq!(parts.len(), 6);
    assert_eq!(
        parts
            .iter()
            .map(|part| (part.trim_start_ms, part.trim_end_ms))
            .collect::<Vec<_>>(),
        [
            (100, 10_100),
            (10_100, 20_100),
            (20_100, 30_100),
            (30_100, 40_100),
            (40_100, 50_100),
            (50_100, 60_100),
        ]
    );
    assert!(parts.iter().all(|part| {
        part.segment_id == span.segment_id
            && part.path == span.path
            && part.source_origin_ms == span.source_origin_ms
            && part.start_ms == span.start_ms
            && part.end_ms == span.end_ms
    }));

    let encoder = super::encoding::SelectedEncoder {
        encoder: VideoEncoder::H264Qsv,
        device: None,
        fallback_reason: None,
    };
    let settings = RenderRecipe {
        encoding: EncoderConfig {
            encoder: VideoEncoder::H264Qsv,
            device: None,
            ..Default::default()
        },
        ..Default::default()
    };
    let media = MediaInfo {
        width: 320,
        height: 180,
        has_audio: false,
        duration_ms: Some(90_000),
        format_start_seconds: 0.,
        video_origin_seconds: 0.,
        frame_rate: "60/1".into(),
    };
    let second_part_graph = qsv_filter_graph(
        &parts[1],
        &settings,
        &media,
        (320, 180),
        0,
        false,
        1,
        false,
        "60/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    // Each independent part starts at PTS zero; concat advances it by the
    // preceding file durations.
    assert!(
        second_part_graph.contains("trim=start=10.100000:end=20.100000,setpts=PTS-(10.100000)/TB"),
        "{second_part_graph}"
    );

    let mut short = span.clone();
    short.trim_end_ms = short.trim_start_ms + 39_000;
    assert!(qsv_parallel_parts(&short, "60/1").is_none());
    let mut fractional_duration = span.clone();
    fractional_duration.trim_end_ms += 20;
    let fractional_parts = qsv_parallel_parts(&fractional_duration, "60/1").unwrap();
    assert_eq!(fractional_parts.len(), 6);
    assert_eq!(
        fractional_parts.first().unwrap().trim_start_ms,
        span.trim_start_ms
    );
    assert_eq!(
        fractional_parts.last().unwrap().trim_end_ms,
        fractional_duration.trim_end_ms
    );
    assert!(
        fractional_parts[..5]
            .iter()
            .all(|part| { (part.trim_end_ms - part.trim_start_ms) % 1_000 == 0 })
    );
    assert_eq!(
        fractional_parts
            .iter()
            .map(|part| part.trim_end_ms - part.trim_start_ms)
            .sum::<i64>(),
        60_020
    );
    assert!(qsv_parallel_parts(&span, "30000/1001").is_none());
}

#[test]
fn qsv_parallel_parts_cover_minimum_and_maximum_boundaries() {
    let base = source(PathBuf::from("source.flv"), 9, 0, 120_000, 0, 120_000);
    let mut below_minimum = base.clone();
    below_minimum.trim_end_ms = 39_999;
    assert!(qsv_parallel_parts(&below_minimum, "60/1").is_none());

    let mut minimum = base.clone();
    minimum.trim_end_ms = 40_000;
    let minimum_parts = qsv_parallel_parts(&minimum, "60/1").unwrap();
    assert_eq!(minimum_parts.len(), 4);
    assert!(
        minimum_parts
            .iter()
            .all(|part| part.trim_end_ms - part.trim_start_ms == 10_000)
    );

    let maximum_parts = qsv_parallel_parts(&base, "60/1").unwrap();
    assert_eq!(maximum_parts.len(), 6);
    assert_eq!(maximum_parts.first().unwrap().trim_start_ms, 0);
    assert_eq!(maximum_parts.last().unwrap().trim_end_ms, 120_000);
    assert!(
        maximum_parts
            .iter()
            .all(|part| part.trim_end_ms - part.trim_start_ms == 20_000)
    );
}

#[test]
fn qsv_parallel_retry_progress_does_not_count_the_same_work_twice() {
    let selected = super::encoding::SelectedEncoder {
        encoder: VideoEncoder::H264Qsv,
        device: None,
        fallback_reason: None,
    };
    let mut source_progress_ms = 0;
    let mut updates = Vec::new();
    for _ in 0..2 {
        report_qsv_parallel_progress(
            &selected,
            "render",
            &[5_000, 0],
            &[150, 0],
            Duration::from_secs(5),
            0,
            10_000,
            &mut source_progress_ms,
            &mut |update| updates.push(update),
        );
    }
    assert_eq!(source_progress_ms, 5_000);
    assert!(
        updates
            .iter()
            .all(|update| update.processed_ms == Some(5_000))
    );
}

#[test]
fn mosaic_overlay_positions_match_region_coordinates() {
    let encoder = super::encoding::SelectedEncoder {
        encoder: VideoEncoder::H264Qsv,
        device: Some("0".into()),
        fallback_reason: None,
    };
    let media = MediaInfo {
        width: 1920,
        height: 1080,
        has_audio: false,
        duration_ms: Some(10_000),
        format_start_seconds: 0.,
        video_origin_seconds: 0.,
        frame_rate: "30/1".into(),
    };
    let source = source(PathBuf::from("video.mp4"), 1, 0, 10_000, 0, 10_000);
    let top_left = MaskRegion {
        id: "tl".into(),
        effect_type: EffectType::Mosaic,
        x: 0.1,
        y: 0.1,
        width: 0.2,
        height: 0.2,
        strength: 16.,
        ..Default::default()
    };
    let bottom_right = MaskRegion {
        id: "br".into(),
        effect_type: EffectType::Mosaic,
        x: 0.7,
        y: 0.7,
        width: 0.2,
        height: 0.2,
        strength: 16.,
        ..Default::default()
    };
    let settings = RenderRecipe {
        regions: vec![top_left.clone(), bottom_right.clone()],
        ..Default::default()
    };
    let graph = qsv_filter_graph(
        &source,
        &settings,
        &media,
        (1920, 1080),
        0,
        false,
        0,
        false,
        "30/1",
        &encoder,
        QsvMosaicBackend::OpenClNearest,
    )
    .unwrap();
    assert!(
        graph.contains("vpp_qsv=cw=384:ch=216:cx=192:cy=108"),
        "Top-left mosaic crop missing in graph"
    );
    assert!(
        graph.contains("[keep0][effect0]overlay_qsv=x=192:y=108"),
        "Top-left overlay position missing in graph"
    );
    assert!(
        graph.contains("vpp_qsv=cw=384:ch=216:cx=1344:cy=756"),
        "Bottom-right mosaic crop missing in graph"
    );
    assert!(
        graph.contains("[keep1][effect1]overlay_qsv=x=1344:y=756"),
        "Bottom-right overlay position missing in graph"
    );
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
        encoding: cpu_recipe().encoding,
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
    assert_eq!(done.encoder.as_deref(), Some("libx264"));
    assert!((done.duration_ms - 1700).abs() < 100, "{done:?}");
    assert!(phases.iter().any(|p| p.phase.contains("合成分段")));
    assert_eq!(phases.last().unwrap().ratio, Some(1.));
    assert!(
        phases
            .iter()
            .any(|p| p.phase.contains("合成分段") && p.fps.is_some() && p.speed.is_some()),
        "{phases:?}"
    );
    assert!(phases.iter().all(|p| p.total_ms == Some(1700)));
    assert!(
        phases
            .iter()
            .filter(|p| p.phase != "检测编码器")
            .all(|p| p.encoder.as_deref() == Some("libx264"))
    );
    assert_eq!(phases.last().unwrap().processed_ms, Some(1700));
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

/// Run on an opted-in GPU runner and exercise its explicit encoder through a
/// two-source render. Set BILIUP_TEST_ENCODER_DEVICE to select a device.
#[tokio::test]
async fn hardware_render() {
    let Ok(name) = std::env::var("BILIUP_TEST_ENCODER") else {
        return;
    };
    let encoder: VideoEncoder = serde_json::from_value(serde_json::Value::String(name.clone()))
        .unwrap_or_else(|_| panic!("unsupported BILIUP_TEST_ENCODER: {name}"));
    assert!(
        !matches!(encoder, VideoEncoder::Auto | VideoEncoder::Libx264),
        "choose an explicit GPU encoder"
    );
    let device = std::env::var("BILIUP_TEST_ENCODER_DEVICE")
        .ok()
        .filter(|s| !s.is_empty());
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("first.mp4");
    let b = dir.path().join("second.mp4");
    fixture(&a, "blue", "320x192", "libx264", true, 0.8).await;
    fixture(&b, "green", "256x192", "libx264", false, 0.8).await;
    let originals = (hash(&a), hash(&b));
    let target = dir.path().join("hardware.mp4");
    let is_qsv = encoder == VideoEncoder::H264Qsv;
    let recipe = RenderRecipe {
        encoding: EncoderConfig {
            encoder,
            device: device.clone(),
            ..Default::default()
        },
        regions: vec![MaskRegion {
            id: "hardware-mask".into(),
            effect_type: if is_qsv {
                EffectType::Mosaic
            } else {
                EffectType::Solid
            },
            color: "#ff0000".into(),
            x: 0.1,
            y: 0.1,
            width: 0.3,
            height: 0.3,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut progress = vec![];
    let done = render(
        EngineRequest {
            sources: vec![
                source(a.clone(), 1, 0, 800, 0, 800),
                source(b.clone(), 2, 800, 1600, 0, 800),
            ],
            settings: recipe,
            assets: HashMap::new(),
            output_path: target.clone(),
            font_path: None,
        },
        CancellationToken::new(),
        |update| progress.push(update),
    )
    .await
    .unwrap();
    assert_eq!(done.encoder.as_deref(), Some(name.as_str()));
    if device.is_some() {
        assert_eq!(done.encoder_device, device);
    }
    assert!((done.duration_ms - 1600).abs() < 150, "{done:?}");
    assert_eq!((hash(&a), hash(&b)), originals);
    assert!(
        progress
            .iter()
            .filter(|p| p.phase != "检测编码器")
            .all(|p| p.encoder.as_deref() == Some(name.as_str()))
    );
    assert!(
        progress
            .iter()
            .any(|p| p.phase == "合成分段 2/2" && p.processed_ms.unwrap_or(0) > 800),
        "{progress:?}"
    );
    assert!(
        progress
            .iter()
            .any(|p| p.phase.contains("合成分段") && p.fps.is_some() && p.speed.is_some()),
        "{progress:?}"
    );
    let output_info = probe(&target, &CancellationToken::new()).await.unwrap();
    assert_eq!(
        (output_info.width, output_info.height, output_info.has_audio),
        (320, 192, true)
    );
    assert_eq!(output_info.frame_rate, "30/1");
    for at in [0.3, 1.1] {
        let masked = pixel(&target, at, 70, 40, 320).await;
        if is_qsv {
            let expected_channel = if at < 0.8 { 2 } else { 1 };
            assert!(
                masked[expected_channel] > 120,
                "mosaic changed source color at {at}: {masked:?}"
            );
        } else {
            assert!(
                masked[0] > 160 && masked[2] < 100,
                "solid mask missing at {at}: {masked:?}"
            );
        }
    }
    let decoded = crate::tools::ffmpeg_command()
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-xerror",
            "-i",
        ])
        .arg(&target)
        .args(["-f", "null", "-"])
        .output()
        .await
        .unwrap();
    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
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
    // XML -> ASS is built in; the embedded CJK font works without render-tools.
    if crate::tools::render_font().is_none() {
        assert!(
            std::env::var_os("BILIUP_REQUIRE_RENDER_TESTS").is_none(),
            "BILIUP_REQUIRE_RENDER_TESTS is set but no render font was found; set BILIUP_RENDER_TOOLS_DIR"
        );
        crate::tools::note_skipped_test(
            "set BILIUP_RENDER_TOOLS_DIR to a directory with fonts/ to test the real libass export",
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("source.mp4");
    fixture(&original, "black", "320x180", "libx264", false, 12.).await;
    let xml = dir.path().join("source.xml");
    std::fs::write(&xml, "<i><recording_start_time_ms>100000</recording_start_time_ms><d p=\"8.000,1,25,16777215,108,0,0,0\">你好</d></i>").unwrap();
    let original_hash = (hash(&original), hash(&xml));
    let mut span = source(original.clone(), 1, 0, 12000, 10000, 11000);
    span.danmaku_path = Some(xml.clone());
    let mut recipe = cpu_recipe();
    recipe.danmaku.enabled = true;
    recipe.danmaku.font_size = 26.;
    recipe.danmaku.scroll_seconds = 4.;
    recipe.danmaku.opacity = 1.;
    let preview = preview(&span, &recipe, &CancellationToken::new())
        .await
        .unwrap();
    assert!(preview.ass.contains("\\move("), "{}", preview.ass);
    assert!(preview.ass.contains("你好"), "{}", preview.ass);
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
            settings: cpu_recipe(),
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

#[tokio::test]
async fn tail_seek_preserves_distinct_container_and_video_origins() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("offset.mkv");
    let output = crate::tools::ffmpeg_command()
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-itsoffset",
            "0.5",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=96x64:r=24:d=4,drawbox=color=blue:t=fill:enable='gte(t,2)'",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=4.5",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-g",
            "12",
            "-fps_mode",
            "passthrough",
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
    let media = probe(&original, &CancellationToken::new()).await.unwrap();
    assert!((media.format_start_seconds - 5.).abs() < 0.01);
    assert!((media.video_origin_seconds - 5.5).abs() < 0.01, "{media:?}");

    let target = dir.path().join("tail.mp4");
    let done = render(
        EngineRequest {
            sources: vec![source(original.clone(), 1, 0, 4000, 3000, 4000)],
            settings: cpu_recipe(),
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
    assert!((done.duration_ms - 1000).abs() < 100, "{done:?}");
    let info = probe(&target, &CancellationToken::new()).await.unwrap();
    assert_eq!(info.frame_rate, "24/1");
    assert!(info.video_origin_seconds.abs() < 0.05, "{info:?}");
    assert!(info.has_audio);
    let color = pixel(&target, 0., 48, 32, 96).await;
    assert!(color[2] > 180 && color[0] < 60, "{color:?}");
}

#[test]
fn ffmpeg_progress_is_parsed_as_a_complete_block() {
    let progress = FfmpegProgress::parse(
        "frame=32\nfps=29.97\nout_time_us=1234567\nout_time_ms=1234567\nspeed=2.5x\nprogress=continue\n",
    );
    assert_eq!(
        progress,
        FfmpegProgress {
            processed_ms: Some(1234),
            frames: Some(32),
            fps: Some(29.97),
            speed: Some(2.5)
        }
    );
    assert_eq!(
        FfmpegProgress::parse("out_time_ms=9000\nfps=NaN\nspeed=N/A\n"),
        FfmpegProgress {
            processed_ms: Some(9),
            frames: None,
            fps: None,
            speed: None
        }
    );
    assert_eq!(FfmpegProgress::parse("fps=-1\nspeed=infx\n").fps, None);
    let zero_clock = FfmpegProgress::parse("frame=24\nout_time_us=0\nfps=0.00\nspeed=0x\n")
        .with_elapsed("30/1", Duration::from_millis(200));
    assert_eq!(zero_clock.encoded_time("30/1"), 800);
    assert_eq!(zero_clock.fps, Some(120.));
    assert_eq!(zero_clock.speed, Some(4.));
}
