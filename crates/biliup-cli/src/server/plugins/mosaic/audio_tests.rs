use super::*;
use crate::server::plugins::audio::flv_audio_encoding;
use crate::server::plugins::mosaic::EffectType;
use std::path::PathBuf;

fn tag(kind: u8, timestamp: u32, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![kind];
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
    bytes.extend_from_slice(&timestamp.to_be_bytes()[1..]);
    bytes.push(timestamp.to_be_bytes()[0]);
    bytes.extend_from_slice(&[0; 3]);
    bytes.extend_from_slice(body);
    bytes.extend_from_slice(&(11 + body.len() as u32).to_be_bytes());
    bytes
}

fn flv(tags: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"FLV\x01\x05\0\0\0\x09\0\0\0\0".to_vec();
    bytes.extend(tags.iter().flatten());
    bytes
}

#[test]
fn audio_scan_ignores_headers_and_other_tracks_and_supports_timestamp_rollover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.flv");
    std::fs::write(
        &path,
        flv(&[
            tag(8, 0, &[0xaf, 0, 0x12, 0x10]),
            tag(8, u32::MAX - 10, &[0xaf, 1, 0xe0]),
            tag(9, 700, &[0x17]),
            tag(8, 0, &[0xaf, 0, 0x12, 0x10]),
            tag(8, 12, &[0xaf, 1, 0xe0]),
            tag(8, 35, &[0xaf, 1, 0xe0]),
        ]),
    )
    .unwrap();
    assert_eq!(flv_audio_encoding(&path).unwrap(), AudioEncoding::Copy);
    std::fs::write(
        &path,
        flv(&[
            tag(8, 23, &[0xaf, 1, 0xe0]),
            tag(8, 23, &[0xaf, 1, 0xe0]),
            tag(8, 21, &[0xaf, 1, 0xe0]),
            tag(8, 47, &[0xaf, 1, 0xe0]),
        ]),
    )
    .unwrap();
    assert_eq!(
        flv_audio_encoding(&path).unwrap(),
        AudioEncoding::RepairAac {
            non_increasing_packets: 2
        }
    );
}

#[test]
fn audio_scan_rejects_truncation_invalid_offsets_and_previous_tag_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.flv");
    let valid = flv(&[tag(8, 0, &[0xaf, 1, 0xe0])]);
    for length in 0..valid.len() {
        // The 13-byte FLV header alone is valid and has no audio tags.
        if length == 13 {
            continue;
        }
        std::fs::write(&path, &valid[..length]).unwrap();
        assert!(flv_audio_encoding(&path).is_err(), "length {length}");
    }
    let mut bad_offset = valid.clone();
    bad_offset[5..9].copy_from_slice(&u32::MAX.to_be_bytes());
    std::fs::write(&path, bad_offset).unwrap();
    assert!(flv_audio_encoding(&path).is_err());
    let mut bad_size = valid;
    *bad_size.last_mut().unwrap() ^= 1;
    std::fs::write(&path, bad_size).unwrap();
    assert!(flv_audio_encoding(&path).is_err());
}

fn tags(bytes: &[u8]) -> Vec<(u8, u32, Vec<u8>)> {
    let mut position = u32::from_be_bytes(bytes[5..9].try_into().unwrap()) as usize + 4;
    let mut result = Vec::new();
    while position < bytes.len() {
        let head = &bytes[position..position + 11];
        let size = u32::from_be_bytes([0, head[1], head[2], head[3]]) as usize;
        let timestamp = u32::from_be_bytes([head[7], head[4], head[5], head[6]]);
        result.push((
            head[0],
            timestamp,
            bytes[position + 11..position + 11 + size].to_vec(),
        ));
        position += 11 + size + 4;
    }
    result
}

fn audio_packets(bytes: &[u8]) -> Vec<(u32, Vec<u8>)> {
    tags(bytes)
        .into_iter()
        .filter_map(|(kind, timestamp, body)| {
            (kind == 8 && body.starts_with(&[0xaf, 1])).then_some((timestamp, body))
        })
        .collect()
}

fn av_start_offset(bytes: &[u8]) -> i64 {
    let tags = tags(bytes);
    let video = tags
        .iter()
        .find(|(kind, _, body)| *kind == 9 && body.starts_with(&[0x17, 1]))
        .unwrap();
    let composition_time = (i32::from_be_bytes([0, video.2[2], video.2[3], video.2[4]]) << 8) >> 8;
    let audio = audio_packets(bytes)[0].0;
    i64::from(video.1) + i64::from(composition_time) - i64::from(audio)
}

async fn pcm(path: &Path, expect_clean: bool) -> Vec<i16> {
    let output = tools::ffmpeg_command()
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-ac", "1", "-f", "s16le", "-"])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !expect_clean || output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
        .collect()
}

fn correlation(source: &[i16], output: &[i16]) -> f64 {
    let mut product = 0.0;
    let mut source_energy = 0.0;
    let mut output_energy = 0.0;
    for (&a, &b) in source.iter().zip(output) {
        let (a, b) = (f64::from(a), f64::from(b));
        product += a * b;
        source_energy += a * a;
        output_energy += b * b;
    }
    product / (source_energy * output_energy).sqrt()
}

#[tokio::test]
async fn ffmpeg_preserves_normal_audio_and_repairs_duplicate_pts_without_dropping_samples() {
    if tools::ffmpeg_command()
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_err()
    {
        tools::note_skipped_test("FFmpeg unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("normal.flv");
    let output = tools::ffmpeg_command()
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x64:rate=30:duration=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=2",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-f",
            "flv",
            "-y",
        ])
        .arg(&input)
        .output()
        .await
        .unwrap();
    if !output.status.success() {
        tools::note_skipped_test("FFmpeg libx264/AAC encoders unavailable");
        return;
    }
    let original = std::fs::read(&input).unwrap();
    assert!(!audio::repair_file_if_needed(&input).await.unwrap());
    assert_eq!(std::fs::read(&input).unwrap(), original);
    let region = MosaicRegion {
        id: "test".into(),
        x: 0.0,
        y: 0.0,
        width: 0.25,
        height: 0.25,
        effect_type: EffectType::Solid,
        strength: 4,
        color: None,
    };
    assert_eq!(flv_audio_encoding(&input).unwrap(), AudioEncoding::Copy);
    MosaicPlugin::new()
        .process_file(&input, std::slice::from_ref(&region))
        .await
        .unwrap();
    let normal_output = std::fs::read(&input).unwrap();
    let payloads = |bytes: &[u8]| {
        audio_packets(bytes)
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        payloads(&normal_output),
        payloads(&original),
        "normal AAC must remain bit-for-bit copied"
    );

    for duplicate_voice in [false, true] {
        let path = dir.path().join(if duplicate_voice {
            "voice.flv"
        } else {
            "ancillary.flv"
        });
        let mut synthetic = Vec::new();
        let mut audio_count = 0;
        for (kind, timestamp, body) in tags(&original) {
            if kind == 8 && body.starts_with(&[0xaf, 1]) {
                if audio_count % 40 == 0 {
                    // AAC FIL(count=1, fill byte=0), followed by END. This valid
                    // ancillary-only frame produces no samples; no live fixture.
                    let extra = if duplicate_voice {
                        body.clone()
                    } else {
                        vec![0xaf, 1, 0xc2, 0x01, 0xc0]
                    };
                    synthetic.push(tag(8, timestamp, &extra));
                }
                audio_count += 1;
            }
            synthetic.push(tag(kind, timestamp, &body));
        }
        let synthetic = flv(&synthetic);
        std::fs::write(&path, &synthetic).unwrap();
        assert!(matches!(
            flv_audio_encoding(&path).unwrap(),
            AudioEncoding::RepairAac { .. }
        ));
        let decoded_before = pcm(&path, !duplicate_voice).await;
        let audio_only = dir.path().join(if duplicate_voice {
            "voice-audio-only.flv"
        } else {
            "ancillary-audio-only.flv"
        });
        std::fs::write(&audio_only, &synthetic).unwrap();
        assert!(audio::repair_file_if_needed(&audio_only).await.unwrap());
        assert_eq!(
            flv_audio_encoding(&audio_only).unwrap(),
            AudioEncoding::Copy
        );
        assert!(!audio::repair_file_if_needed(&audio_only).await.unwrap());
        let repaired_tags = tags(&std::fs::read(&audio_only).unwrap());
        assert_eq!(
            repaired_tags
                .into_iter()
                .filter(|(kind, _, _)| *kind == 9)
                .map(|(_, _, body)| body)
                .collect::<Vec<_>>(),
            tags(&synthetic)
                .into_iter()
                .filter(|(kind, _, _)| *kind == 9)
                .map(|(_, _, body)| body)
                .collect::<Vec<_>>(),
            "audio-only repair must copy video bytes"
        );
        MosaicPlugin::new()
            .process_file(&path, std::slice::from_ref(&region))
            .await
            .unwrap();
        let processed = std::fs::read(&path).unwrap();
        let packets = audio_packets(&processed);
        assert!(
            packets.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "repaired packets must have strictly increasing PTS"
        );
        assert_eq!(flv_audio_encoding(&path).unwrap(), AudioEncoding::Copy);
        assert!(
            (av_start_offset(&processed) - av_start_offset(&synthetic)).abs() <= 25,
            "A/V start offset changed beyond one AAC encoder frame"
        );
        let decoded_after = pcm(&path, true).await;
        assert!(
            decoded_after.len().abs_diff(decoded_before.len()) <= 1024,
            "audio samples were dropped or invented"
        );
        // AAC adds one encoder-delay frame. Compare the audible waveform after
        // that delay; this also catches periodic silence/dropouts from repair.
        assert!(correlation(&decoded_before, &decoded_after[1024..]) > 0.98);
        for (before, after) in decoded_before
            .chunks(4410)
            .zip(decoded_after[1024..].chunks(4410))
        {
            let rms = |samples: &[i16]| {
                (samples
                    .iter()
                    .map(|sample| f64::from(*sample).powi(2))
                    .sum::<f64>()
                    / samples.len() as f64)
                    .sqrt()
            };
            let before_rms = rms(before);
            let after_rms = rms(after);
            if before_rms > 500.0 {
                assert!(
                    (0.5..1.5).contains(&(after_rms / before_rms)),
                    "repair introduced a 100 ms audio dropout: {before_rms} => {after_rms}"
                );
            }
        }
        assert_eq!(
            std::fs::read(path.with_extension("flv.unmasked")).unwrap(),
            synthetic,
            "repair must preserve the original"
        );
        if !duplicate_voice {
            // FLV input and MP4 output: the decision follows source magic, not
            // the destination extension; repaired MP4 packets are checked too.
            let mixed_format = dir.path().join("flv-input.mp4");
            std::fs::write(&mixed_format, &synthetic).unwrap();
            MosaicPlugin::new()
                .process_file(&mixed_format, std::slice::from_ref(&region))
                .await
                .unwrap();
            audio::validate_output(&mixed_format, "mp4", true)
                .await
                .unwrap();
        }
    }

    let broken = dir.path().join("broken.flv");
    let truncated = &original[..original.len() - 1];
    std::fs::write(&broken, truncated).unwrap();
    assert!(audio::repair_file_if_needed(&broken).await.is_err());
    assert_eq!(std::fs::read(&broken).unwrap(), truncated);
    assert!(
        audio::validate_output(&dir.path().join("missing.flv"), "flv", true)
            .await
            .is_err()
    );
}

/// Set BILIUP_TEST_AUDIO_SAMPLE to a local source FLV to run the real-file path.
/// The test copies the source to a temporary directory before invoking the
/// production repair, so even ignored tests cannot change user recordings.
#[tokio::test]
#[ignore = "requires BILIUP_TEST_AUDIO_SAMPLE pointing to a local FLV fixture"]
async fn production_audio_repair_on_local_flv_fixture() {
    let Some(source) = std::env::var_os("BILIUP_TEST_AUDIO_SAMPLE") else {
        panic!("set BILIUP_TEST_AUDIO_SAMPLE to a local FLV fixture");
    };
    let source = PathBuf::from(source);
    let original = std::fs::read(&source).unwrap();
    let dir = tempfile::tempdir().unwrap().keep();
    let path = dir.join("production-sample.flv");
    std::fs::write(&path, &original).unwrap();
    let before_video_packets = tags(&std::fs::read(&path).unwrap())
        .into_iter()
        .filter(|(kind, _, _)| *kind == 9)
        .count();
    let repaired = audio::repair_file_if_needed(&path).await.unwrap();
    if repaired {
        let after_video_packets = tags(&std::fs::read(&path).unwrap())
            .into_iter()
            .filter(|(kind, _, _)| *kind == 9)
            .count();
        assert!(before_video_packets > 0, "fixture has no video packets");
        assert!(after_video_packets > 0, "repair removed the video stream");
        let decoded = tools::ffmpeg_command()
            .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
            .arg(&path)
            .args(["-map", "0:v:0", "-f", "null", "-"])
            .output()
            .await
            .unwrap();
        assert!(
            decoded.status.success(),
            "repaired video no longer decodes: {}",
            String::from_utf8_lossy(&decoded.stderr)
        );
        assert_eq!(flv_audio_encoding(&path).unwrap(), AudioEncoding::Copy);
        assert!(
            !audio::repair_file_if_needed(&path).await.unwrap(),
            "repair should be idempotent"
        );
        assert!(audio::validate_output(&path, "flv", true).await.is_ok());
        eprintln!("BILIUP_TEST_AUDIO_REPAIRED={}", path.display());
    } else {
        assert!(matches!(
            flv_audio_encoding(&path).unwrap(),
            AudioEncoding::Copy
        ));
    }
    if std::env::var_os("BILIUP_TEST_AUDIO_MASK").as_deref() == Some(std::ffi::OsStr::new("1")) {
        let masked = dir.join("production-masked.flv");
        std::fs::write(&masked, &original).unwrap();
        let region = MosaicRegion {
            id: "production".into(),
            x: 0.0,
            y: 0.0,
            width: 0.05,
            height: 0.05,
            effect_type: EffectType::Solid,
            strength: 4,
            color: None,
        };
        MosaicPlugin::new()
            .process_file(&masked, &[region])
            .await
            .unwrap();
        audio::validate_output(&masked, "flv", true).await.unwrap();
        eprintln!("BILIUP_TEST_AUDIO_MASKED={}", masked.display());
    }
    eprintln!("BILIUP_TEST_AUDIO_OUTPUT_DIR={}", dir.display());
}
