use super::*;

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

#[tokio::test]
async fn shared_render_repairs_conflicting_aac_clocks_without_an_audio_intermediate() {
    use super::super::{engine, model::RenderRecipe};
    let dir = tempfile::tempdir().unwrap();
    let name = format!(
        "input-{}",
        dir.path().file_name().unwrap().to_string_lossy()
    );
    let input = dir.path().join(format!("{name}.unmasked.flv"));
    let relative = std::path::PathBuf::from(super::super::OUTPUT_DIR).join(format!("{name}.flv"));
    let expected = tempfile::TempPath::try_from_path(&relative).unwrap();
    let made = crate::tools::ffmpeg_command()
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=s=64x64:r=30:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=2",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-c:a",
            "aac",
            "-f",
            "flv",
            "-y",
        ])
        .arg(&input)
        .output()
        .await
        .unwrap();
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let bytes = std::fs::read(&input).unwrap();
    let mut pos = u32::from_be_bytes(bytes[5..9].try_into().unwrap()) as usize + 4;
    let mut damaged = bytes[..pos].to_vec();
    let mut packets = 0;
    while pos < bytes.len() {
        let head = &bytes[pos..pos + 11];
        let size = u32::from_be_bytes([0, head[1], head[2], head[3]]) as usize;
        let end = pos + 15 + size;
        if head[0] == 8 && bytes[pos + 11..pos + 13] == [0xaf, 1] {
            if packets % 30 == 0 {
                damaged.extend_from_slice(&bytes[pos..end]);
            }
            packets += 1;
        }
        damaged.extend_from_slice(&bytes[pos..end]);
        pos = end;
    }
    std::fs::write(&input, &damaged).unwrap();
    assert!(matches!(
        flv_audio_encoding(&input).unwrap(),
        AudioEncoding::RepairAac { .. }
    ));
    let output = engine::process_video(
        &input,
        None,
        &RenderRecipe {
            regions: vec![super::super::model::MaskRegion {
                id: "mask".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(output, relative);
    assert!(expected.exists(), "输出必须落在 renders/ 下");
    assert_eq!(flv_audio_encoding(&output).unwrap(), AudioEncoding::Copy);
    let decoded = crate::tools::ffmpeg_command()
        .args(["-v", "error", "-i"])
        .arg(&output)
        .args(["-map", "0:a:0", "-f", "null", "-"])
        .output()
        .await
        .unwrap();
    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    assert_eq!(std::fs::read(&input).unwrap(), damaged);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
