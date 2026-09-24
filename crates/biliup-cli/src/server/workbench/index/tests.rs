use super::*;
use std::fs::OpenOptions;

// ---------- FLV ----------

const TAG_AUDIO: u8 = 8;
const TAG_VIDEO: u8 = 9;
const TAG_SCRIPT: u8 = 18;

pub(crate) struct Flv {
    pub(crate) bytes: Vec<u8>,
    /// 每个视频关键帧 tag 的 `(原始时间戳, 偏移)`
    pub(crate) keyframes: Vec<(u32, u64)>,
    /// 第一个非序列头媒体 tag 的偏移
    pub(crate) header_len: u64,
    pub(crate) last_ts: u32,
}

pub(crate) fn flv_tag(out: &mut Vec<u8>, tag_type: u8, ts: u32, body: &[u8]) -> u64 {
    let offset = out.len() as u64;
    let len = body.len() as u32;
    out.push(tag_type);
    out.extend_from_slice(&len.to_be_bytes()[1..]);
    out.extend_from_slice(&ts.to_be_bytes()[1..]);
    out.push((ts >> 24) as u8);
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(body);
    out.extend_from_slice(&(11 + len).to_be_bytes());
    offset
}

fn amf_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn amf_number_array(out: &mut Vec<u8>, values: &[f64]) {
    out.push(0x0A);
    out.extend_from_slice(&(values.len() as u32).to_be_bytes());
    for v in values {
        out.push(0x00);
        out.extend_from_slice(&v.to_be_bytes());
    }
}

/// onMetaData，带（可选的）`keyframes.times / filepositions`。数值定长，内容不影响字节布局。
fn on_metadata(keyframes: Option<(&[f64], &[f64])>) -> Vec<u8> {
    let mut out = vec![0x02];
    amf_string(&mut out, "onMetaData");
    out.push(0x08);
    out.extend_from_slice(&2u32.to_be_bytes());
    amf_string(&mut out, "duration");
    out.push(0x00);
    out.extend_from_slice(&0f64.to_be_bytes());
    if let Some((times, positions)) = keyframes {
        amf_string(&mut out, "keyframes");
        out.push(0x03);
        amf_string(&mut out, "times");
        amf_number_array(&mut out, times);
        amf_string(&mut out, "filepositions");
        amf_number_array(&mut out, positions);
        out.extend_from_slice(&[0, 0, 9]);
    }
    out.extend_from_slice(&[0, 0, 9]);
    out
}

/// 4 字节长度 + NALU 头 + 填充，共 `len` 字节 NALU。
pub(crate) fn avcc_nalu(out: &mut Vec<u8>, header: u8, len: usize) {
    out.extend_from_slice(&(len as u32).to_be_bytes());
    out.push(header);
    out.extend(std::iter::repeat_n(0xAB, len - 1));
}

/// `frames` 个视频帧（每 `gop` 帧一个关键帧、间隔 40 ms，从 `base_ts` 开始），中间夹音频。
/// 关键帧是 SEI + IDR slice，其余是非 IDR slice。
pub(crate) fn build_flv(
    base_ts: u32,
    frames: u32,
    gop: u32,
    metadata: Option<(&[f64], &[f64])>,
) -> Flv {
    let mut out = b"FLV\x01\x05\x00\x00\x00\x09".to_vec();
    out.extend_from_slice(&[0, 0, 0, 0]);
    flv_tag(&mut out, TAG_SCRIPT, 0, &on_metadata(metadata));
    flv_tag(&mut out, TAG_VIDEO, 0, &[0x17, 0x00, 0, 0, 0, 1, 2, 3]);
    flv_tag(&mut out, TAG_AUDIO, 0, &[0xAF, 0x00, 0x12, 0x10]);
    let mut header_len = 0;
    let mut keyframes = Vec::new();
    let mut last_ts = base_ts;
    for i in 0..frames {
        let ts = base_ts + i * 40;
        let key = i % gop == 0;
        let mut body = vec![if key { 0x17 } else { 0x27 }, 0x01, 0, 0, 0];
        if key {
            avcc_nalu(&mut body, 0x06, 8);
            avcc_nalu(&mut body, 0x65, 180);
        } else {
            avcc_nalu(&mut body, 0x41, 190);
        }
        let offset = flv_tag(&mut out, TAG_VIDEO, ts, &body);
        if i == 0 {
            header_len = offset;
        }
        if key {
            keyframes.push((ts, offset));
        }
        flv_tag(&mut out, TAG_AUDIO, ts + 5, &[0xAF, 0x01, 0xCD, 0xCD, 0xCD]);
        last_ts = ts + 5;
    }
    Flv {
        bytes: out,
        keyframes,
        header_len,
        last_ts,
    }
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn expected(base: i64, timescale: i64, frames: &[(i64, u64)]) -> Vec<Keyframe> {
    frames
        .iter()
        .map(|(raw, offset)| Keyframe {
            t_ms: ((raw - base) * 1000 / timescale) as u32,
            offset: *offset,
        })
        .collect()
}

fn flv_expected(flv: &Flv) -> Vec<Keyframe> {
    let frames: Vec<(i64, u64)> = flv.keyframes.iter().map(|(t, o)| (*t as i64, *o)).collect();
    expected(flv.keyframes[0].0 as i64, 1000, &frames)
}

#[test]
fn flv_scan_indexes_keyframes_relative_to_the_first_one() {
    let dir = tempfile::tempdir().unwrap();
    // stream-gears 的绝对时间戳：不从 0 开始
    let flv = build_flv(3_600_000, 100, 25, None);
    let path = write(dir.path(), "a.flv", &flv.bytes);

    let index = refresh(&path, true).unwrap();
    assert_eq!(index.container, Container::Flv);
    assert!(index.complete);
    assert_eq!(index.base_ts, Some(3_600_000));
    assert_eq!(index.header_len, flv.header_len);
    assert_eq!(index.keyframes, flv_expected(&flv));
    assert_eq!(
        index.keyframes.iter().map(|k| k.t_ms).collect::<Vec<_>>(),
        vec![0, 1000, 2000, 3000]
    );
    assert_eq!(index.duration_ms, flv.last_ts - 3_600_000);
    assert_eq!(index.scanned_upto, flv.bytes.len() as u64);

    // 缓存命中：内容一致，且写在 `<分段>.idx`
    assert!(path.with_extension("flv.idx").exists());
    assert_eq!(load(&path).unwrap(), index);
    assert_eq!(refresh(&path, true).unwrap(), index);
}

#[test]
fn flv_offsets_point_at_keyframe_tags() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 60, 20, None);
    let path = write(dir.path(), "a.flv", &flv.bytes);
    let index = refresh(&path, true).unwrap();
    for k in &index.keyframes {
        let at = k.offset as usize;
        assert_eq!(flv.bytes[at], TAG_VIDEO);
        assert_eq!(
            flv.bytes[at + 11],
            0x17,
            "offset {at} should start a key frame tag"
        );
    }
}

#[test]
fn flv_growing_file_is_scanned_incrementally() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 100, 25, None);
    let cut = flv.keyframes[2].1 as usize + 100; // 第三个关键帧 tag 写到一半
    let path = write(dir.path(), "a.flv.part", &flv.bytes[..cut]);

    let partial = refresh(&path, false).unwrap();
    assert!(!partial.complete);
    assert_eq!(partial.keyframes, flv_expected(&flv)[..2].to_vec());
    assert_eq!(partial.scanned_upto, flv.keyframes[2].1);

    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(&flv.bytes[cut..]).unwrap();
    drop(file);
    let full = refresh(&path, false).unwrap();
    assert_eq!(full.keyframes, flv_expected(&flv));
    assert_eq!(full.scanned_upto, flv.bytes.len() as u64);
}

#[test]
fn rescan_continues_in_memory_without_touching_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 100, 25, None);
    let cut = flv.keyframes[2].1 as usize + 100;
    let path = write(dir.path(), "a.flv.part", &flv.bytes[..cut]);

    let partial = rescan(&path, None).unwrap();
    assert_eq!(partial.keyframes, flv_expected(&flv)[..2].to_vec());
    assert_eq!(partial.source_len, cut as u64);

    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(&flv.bytes[cut..]).unwrap();
    drop(file);
    let full = rescan(&path, Some(partial)).unwrap();
    assert_eq!(full.keyframes, flv_expected(&flv));
    assert_eq!(full.duration_ms, 3965);
    assert!(!index_path(&path).exists(), "不写缓存");
    assert_eq!(
        full,
        refresh(&path, false).unwrap(),
        "与读写缓存的续扫结果一致"
    );
}

#[test]
fn flv_truncated_file_truncates_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 100, 25, None);
    let path = write(dir.path(), "a.flv", &flv.bytes);
    assert_eq!(refresh(&path, true).unwrap().keyframes.len(), 4);

    let cut = flv.keyframes[3].1 + 50;
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(cut)
        .unwrap();
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes, flv_expected(&flv)[..3].to_vec());
    assert_eq!(index.source_len, cut);
    assert!(index.scanned_upto <= cut);
}

#[test]
fn flv_cache_is_rebuilt_when_the_file_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let old = build_flv(0, 100, 25, None);
    let path = write(dir.path(), "a.flv", &old.bytes);
    refresh(&path, true).unwrap();

    // 同名覆盖成另一段更长的录像，旧缓存最后一个关键帧的偏移处已不是关键帧
    let new = build_flv(500, 130, 30, None);
    assert_ne!(
        new.keyframes.last().unwrap().1,
        old.keyframes.last().unwrap().1
    );
    fs::write(&path, &new.bytes).unwrap();
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes, flv_expected(&new));
}

#[test]
fn flv_sparse_metadata_keyframes_do_not_thin_out_the_index() {
    let dir = tempfile::tempdir().unwrap();
    // 先用占位数值排出字节布局，再填真值（AMF 数值定长，布局不变）
    let zeros = [0f64; 2];
    let layout = build_flv(0, 100, 25, Some((&zeros, &zeros)));
    // mesio 的元数据只记了前两个关键帧（间隔过密被跳过的情形）
    let times: Vec<f64> = layout.keyframes[..2]
        .iter()
        .map(|(t, _)| *t as f64 / 1000.0)
        .collect();
    let positions: Vec<f64> = layout.keyframes[..2]
        .iter()
        .map(|(_, o)| *o as f64)
        .collect();
    let flv = build_flv(0, 100, 25, Some((&times, &positions)));
    assert_eq!(flv.keyframes, layout.keyframes);
    let path = write(dir.path(), "a.flv", &flv.bytes);

    let index = refresh(&path, true).unwrap();
    assert_eq!(index.header_len, flv.header_len);
    assert_eq!(index.keyframes, flv_expected(&flv));
}

#[test]
fn flv_bogus_metadata_positions_fall_back_to_a_scan() {
    let dir = tempfile::tempdir().unwrap();
    let zeros = [0f64; 2];
    let layout = build_flv(0, 100, 25, Some((&zeros, &zeros)));
    // 进程被杀时 mesio 留下的占位：位置不指向关键帧
    let positions = [
        layout.header_len as f64 + 3.0,
        layout.header_len as f64 + 7.0,
    ];
    let flv = build_flv(0, 100, 25, Some((&[0.0, 1.0], &positions)));
    let path = write(dir.path(), "a.flv", &flv.bytes);

    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes, flv_expected(&flv));
}

/// 在 `(时间戳, 偏移)` 处的视频 tag 里，把帧类型改成关键帧、NALU 头改成 `nalu_header`。
fn relabel_video_tag(flv: &mut Flv, offset: u64, first_byte: u8, nalu_header: u8) {
    let at = offset as usize + 11;
    flv.bytes[at] = first_byte;
    flv.bytes[at + 5 + 4] = nalu_header;
}

#[test]
fn flv_keyframe_flag_on_a_non_idr_slice_is_not_indexed() {
    let dir = tempfile::tempdir().unwrap();
    let mut flv = build_flv(0, 100, 25, None);
    let tags = video_tag_offsets(&flv);
    // 虎牙：非 IDR 的 I 帧（NALU 类型 1）也被标成关键帧
    relabel_video_tag(&mut flv, tags[10], 0x17, 0x41);
    let path = write(dir.path(), "a.flv", &flv.bytes);
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes, flv_expected(&flv));
}

#[test]
fn flv_hevc_keyframes_need_an_irap_nalu() {
    let dir = tempfile::tempdir().unwrap();
    let mut flv = build_flv(0, 100, 25, None);
    let tags = video_tag_offsets(&flv);
    // 国内扩展的 codec id 12（H.265）：IDR_W_RADL（19）算，TRAIL_R（1）不算
    for (i, &offset) in tags.iter().enumerate() {
        if i % 25 == 0 {
            // SEI（39）+ IDR_W_RADL（19）
            relabel_video_tag(&mut flv, offset, 0x1C, 39 << 1);
            flv.bytes[offset as usize + 11 + 5 + 4 + 8 + 4] = 19 << 1;
        } else {
            relabel_video_tag(&mut flv, offset, 0x2C, 1 << 1);
        }
    }
    relabel_video_tag(&mut flv, tags[10], 0x1C, 1 << 1);
    let path = write(dir.path(), "a.flv", &flv.bytes);
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes, flv_expected(&flv));
}

/// 每个非序列头视频 tag 的偏移，按帧序。
fn video_tag_offsets(flv: &Flv) -> Vec<u64> {
    let mut offsets = Vec::new();
    let mut offset =
        13 + 11 + u32::from_be_bytes([0, flv.bytes[14], flv.bytes[15], flv.bytes[16]]) as u64 + 4;
    while (offset as usize) < flv.bytes.len() {
        let at = offset as usize;
        let size =
            u32::from_be_bytes([0, flv.bytes[at + 1], flv.bytes[at + 2], flv.bytes[at + 3]]) as u64;
        if flv.bytes[at] == TAG_VIDEO && flv.bytes[at + 12] == 0x01 {
            offsets.push(offset);
        }
        offset += 11 + size + 4;
    }
    offsets
}

#[test]
fn flv_timestamps_never_go_backwards_in_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut flv = build_flv(10_000, 50, 25, None);
    // 第二个关键帧的时间戳回退到 0
    let at = flv.keyframes[1].1 as usize;
    flv.bytes[at + 4..at + 7].copy_from_slice(&[0, 0, 0]);
    flv.bytes[at + 7] = 0;
    let path = write(dir.path(), "a.flv", &flv.bytes);
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.keyframes.len(), 2);
    assert_eq!(index.keyframes[1].t_ms, 0);
    assert_eq!(index.keyframes[1].offset, flv.keyframes[1].1);
}

// ---------- 缓存格式与查询 ----------

#[test]
fn cache_round_trips_and_other_versions_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let flv = build_flv(0, 100, 25, None);
    let path = write(dir.path(), "a.flv", &flv.bytes);
    let index = refresh(&path, true).unwrap();
    let bytes = index.encode();
    assert_eq!(&bytes[..8], MAGIC);
    assert_eq!(
        bytes.len(),
        FIXED_HEADER_SIZE + index.keyframes.len() * ENTRY_SIZE
    );
    assert_eq!(KeyframeIndex::decode(&bytes).unwrap(), index);

    let mut other = bytes.clone();
    other[8..10].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
    fs::write(index_path(&path), &other).unwrap();
    assert!(load(&path).is_none());
    // 读不了的缓存直接重建
    assert_eq!(refresh(&path, true).unwrap(), index);
    assert!(KeyframeIndex::decode(&bytes[..bytes.len() - 1]).is_err());
}

#[test]
fn range_and_nearest_keyframe_lookups() {
    let mut index = KeyframeIndex::new(Container::Flv);
    for (t, o) in [(0, 100), (2000, 200), (4000, 300)] {
        index.push_keyframe(t, o);
    }
    assert_eq!(index.range(0, 4000).len(), 3);
    assert_eq!(
        index.range(1, 3999),
        &[Keyframe {
            t_ms: 2000,
            offset: 200
        }]
    );
    assert!(index.range(2001, 3999).is_empty());
    assert_eq!(index.at_or_before(3999).unwrap().offset, 200);
    assert_eq!(index.at_or_before(4000).unwrap().offset, 300);
    assert_eq!(index.at_or_after(2001).unwrap().offset, 300);
    assert!(index.at_or_after(4001).is_none());
    let keys = keyframes(Path::new("/nonexistent/a.flv"), 0, 1, true);
    assert!(keys.is_err());
}

#[test]
fn unsupported_container_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "a.mkv", b"\x1a\x45\xdf\xa3");
    assert_eq!(
        refresh(&path, true).unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert_eq!(
        Container::from_path(Path::new("x/a.flv.part")),
        Some(Container::Flv)
    );
    assert_eq!(Container::from_path(Path::new("a.TS")), Some(Container::Ts));
    assert_eq!(
        Container::from_path(Path::new("a.mp4")),
        Some(Container::Fmp4)
    );
}

// ---------- TS ----------

const PMT_PID: u16 = 0x1000;
const VIDEO_PID: u16 = 0x100;
const AUDIO_PID: u16 = 0x101;

fn ts_packet(pid: u16, pusi: bool, payload: &[u8], random_access: bool) -> Vec<u8> {
    let mut p = vec![
        0x47,
        ((pid >> 8) as u8 & 0x1F) | if pusi { 0x40 } else { 0 },
    ];
    p.push(pid as u8);
    if random_access {
        // 自适应字段只放 flags，其余用填充
        let stuffing = 184 - payload.len() - 2;
        p.push(0x30);
        p.push((1 + stuffing) as u8);
        p.push(0x40);
        p.extend(std::iter::repeat_n(0xFF, stuffing));
    } else if payload.len() < 184 {
        let stuffing = 184 - payload.len() - 1;
        p.push(0x30);
        p.push(stuffing as u8);
        if stuffing > 0 {
            p.push(0x00);
            p.extend(std::iter::repeat_n(0xFF, stuffing - 1));
        }
    } else {
        p.push(0x10);
    }
    p.extend_from_slice(payload);
    assert_eq!(p.len(), 188);
    p
}

fn psi(table_id: u8, id: u16, body: &[u8]) -> Vec<u8> {
    let section_length = (5 + body.len() + 4) as u16;
    let mut s = vec![0x00, table_id, 0xB0 | (section_length >> 8) as u8];
    s.push(section_length as u8);
    s.extend_from_slice(&id.to_be_bytes());
    s.extend_from_slice(&[0xC1, 0x00, 0x00]);
    s.extend_from_slice(body);
    s.extend_from_slice(&[0, 0, 0, 0]);
    s
}

fn pat() -> Vec<u8> {
    let mut body = 1u16.to_be_bytes().to_vec();
    body.extend_from_slice(&(0xE000 | PMT_PID).to_be_bytes());
    ts_packet(0, true, &psi(0x00, 1, &body), false)
}

fn pmt(video_type: u8) -> Vec<u8> {
    let mut body = (0xE000 | VIDEO_PID).to_be_bytes().to_vec();
    body.extend_from_slice(&0xF000u16.to_be_bytes());
    for (stream_type, pid) in [(video_type, VIDEO_PID), (0x0F, AUDIO_PID)] {
        body.push(stream_type);
        body.extend_from_slice(&(0xE000 | pid).to_be_bytes());
        body.extend_from_slice(&0xF000u16.to_be_bytes());
    }
    ts_packet(PMT_PID, true, &psi(0x02, 1, &body), false)
}

fn pes_header(stream_id: u8, pts: u64) -> Vec<u8> {
    let mut h = vec![0, 0, 1, stream_id, 0, 0, 0x80, 0x80, 5];
    h.push(0x21 | (((pts >> 30) as u8 & 0x07) << 1));
    h.push((pts >> 22) as u8);
    h.push((((pts >> 15) as u8) << 1) | 1);
    h.push((pts >> 7) as u8);
    h.push(((pts as u8) << 1) | 1);
    h
}

pub(crate) struct Ts {
    pub(crate) bytes: Vec<u8>,
    pub(crate) keyframes: Vec<(i64, u64)>,
    pub(crate) header_len: u64,
}

/// 30 帧一个 IDR，帧间隔 3000（90 kHz 下 33.3 ms）；`use_rai` 时关键帧另带随机访问标志。
pub(crate) fn build_ts(first_pts: u64, frames: u64, hevc: bool, use_rai: bool) -> Ts {
    let mut out = pat();
    out.extend(pmt(if hevc { 0x24 } else { 0x1B }));
    let mut keyframes = Vec::new();
    let mut header_len = 0;
    for i in 0..frames {
        let pts = (first_pts + i * 3000) % (1 << 33);
        let key = i % 30 == 0;
        let mut first = pes_header(0xE0, pts);
        // AUD，然后是图像 NAL
        if hevc {
            first.extend_from_slice(&[0, 0, 0, 1, 35 << 1, 1, 0x50]);
            first.extend_from_slice(&[0, 0, 1, if key { 19 << 1 } else { 1 << 1 }, 1]);
        } else {
            first.extend_from_slice(&[0, 0, 0, 1, 0x09, 0xF0]);
            first.extend_from_slice(&[0, 0, 1, if key { 0x65 } else { 0x41 }]);
        }
        first.resize(184 - if use_rai { 2 } else { 0 }, 0xAB);
        let offset = out.len() as u64;
        if i == 0 {
            header_len = offset;
        }
        out.extend(ts_packet(VIDEO_PID, true, &first, use_rai && key));
        out.extend(ts_packet(VIDEO_PID, false, &[0xAB; 184], false));
        if key {
            keyframes.push((pts as i64, offset));
        }
        let mut audio = pes_header(0xC0, pts);
        audio.resize(184, 0xCD);
        out.extend(ts_packet(AUDIO_PID, true, &audio, false));
    }
    Ts {
        bytes: out,
        keyframes,
        header_len,
    }
}

fn ts_expected(ts: &Ts) -> Vec<Keyframe> {
    let base = ts.keyframes[0].0;
    let frames: Vec<(i64, u64)> = ts
        .keyframes
        .iter()
        .map(|(pts, o)| (if *pts < base { pts + (1 << 33) } else { *pts }, *o))
        .collect();
    expected(base, 90_000, &frames)
}

#[test]
fn ts_scan_finds_idr_pes_across_a_pts_wrap() {
    let dir = tempfile::tempdir().unwrap();
    // 起点离 33 位回绕点不到 1 秒
    let ts = build_ts((1 << 33) - 60_000, 100, false, false);
    let path = write(dir.path(), "a.ts", &ts.bytes);
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.container, Container::Ts);
    assert_eq!(index.timescale, 90_000);
    assert_eq!(index.track.id, VIDEO_PID as u32);
    assert_eq!(index.header_len, ts.header_len);
    assert_eq!(index.keyframes, ts_expected(&ts));
    assert_eq!(
        index.keyframes.iter().map(|k| k.t_ms).collect::<Vec<_>>(),
        vec![0, 1000, 2000, 3000]
    );
    assert_eq!(index.duration_ms, 99 * 3000 / 90);
    for k in &index.keyframes {
        assert_eq!(k.offset % 188, 0);
        assert_eq!(ts.bytes[k.offset as usize], 0x47);
    }
}

#[test]
fn ts_hevc_and_random_access_indicator() {
    let dir = tempfile::tempdir().unwrap();
    let hevc = build_ts(900_000, 70, true, false);
    let path = write(dir.path(), "h.ts", &hevc.bytes);
    assert_eq!(refresh(&path, true).unwrap().keyframes, ts_expected(&hevc));

    let rai = build_ts(900_000, 70, false, true);
    let path = write(dir.path(), "r.ts", &rai.bytes);
    assert_eq!(refresh(&path, true).unwrap().keyframes, ts_expected(&rai));
}

#[test]
fn ts_growing_file_is_scanned_incrementally() {
    let dir = tempfile::tempdir().unwrap();
    let ts = build_ts(0, 100, false, false);
    let cut = ts.keyframes[2].1 as usize + 188 + 50;
    let path = write(dir.path(), "a.ts.part", &ts.bytes[..cut]);
    let partial = refresh(&path, false).unwrap();
    assert_eq!(partial.keyframes, ts_expected(&ts)[..3].to_vec());

    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(&ts.bytes[cut..]).unwrap();
    drop(file);
    assert_eq!(refresh(&path, false).unwrap().keyframes, ts_expected(&ts));
}

// ---------- fMP4 ----------

fn mp4_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    b.extend_from_slice(kind);
    b.extend_from_slice(body);
    b
}

fn full_box(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut b = vec![version];
    b.extend_from_slice(&flags.to_be_bytes()[1..]);
    b.extend_from_slice(body);
    mp4_box(kind, &b)
}

fn trak(id: u32, timescale: u32, handler: &[u8; 4]) -> Vec<u8> {
    let mut tkhd = vec![0; 8];
    tkhd.extend_from_slice(&id.to_be_bytes());
    tkhd.resize(80, 0);
    let mut mdhd = vec![0; 8];
    mdhd.extend_from_slice(&timescale.to_be_bytes());
    mdhd.resize(20, 0);
    let mut hdlr = vec![0; 4];
    hdlr.extend_from_slice(handler);
    hdlr.resize(21, 0);
    let mdia = [
        full_box(b"mdhd", 0, 0, &mdhd),
        full_box(b"hdlr", 0, 0, &hdlr),
    ]
    .concat();
    mp4_box(
        b"trak",
        &[full_box(b"tkhd", 0, 3, &tkhd), mp4_box(b"mdia", &mdia)].concat(),
    )
}

fn init_segment(fragmented: bool) -> Vec<u8> {
    let mut moov = full_box(b"mvhd", 0, 0, &[0; 96]);
    moov.extend(trak(1, 48_000, b"soun"));
    moov.extend(trak(2, 1000, b"vide"));
    if fragmented {
        let trex = |id: u32, duration: u32| {
            let mut b = id.to_be_bytes().to_vec();
            b.extend_from_slice(&1u32.to_be_bytes());
            b.extend_from_slice(&duration.to_be_bytes());
            b.extend_from_slice(&0u32.to_be_bytes());
            b.extend_from_slice(&0x0101_0000u32.to_be_bytes());
            full_box(b"trex", 0, 0, &b)
        };
        moov.extend(mp4_box(b"mvex", &[trex(1, 1024), trex(2, 40)].concat()));
    }
    [
        mp4_box(b"ftyp", b"isom\0\0\0\x01isomiso6"),
        mp4_box(b"moov", &moov),
    ]
    .concat()
}

/// 一个 `moof` + `mdat`：音频 traf 在前，视频 traf 25 个 sample、每个 40 ms。
fn fragment(seq: u32, decode_time: u64, sync: bool) -> Vec<u8> {
    let audio = {
        let mut tfhd = 1u32.to_be_bytes().to_vec();
        tfhd.extend_from_slice(&1024u32.to_be_bytes());
        let mut tfdt = (decode_time * 48).to_be_bytes().to_vec();
        tfdt.truncate(8);
        let trun = [3u32.to_be_bytes(), 0u32.to_be_bytes()].concat();
        mp4_box(
            b"traf",
            &[
                full_box(b"tfhd", 0, 0x02_0008, &tfhd),
                full_box(b"tfdt", 1, 0, &tfdt),
                full_box(b"trun", 0, 0x01, &trun),
            ]
            .concat(),
        )
    };
    let video = {
        let tfhd = 2u32.to_be_bytes().to_vec();
        let tfdt = decode_time.to_be_bytes().to_vec();
        let mut trun = 25u32.to_be_bytes().to_vec();
        trun.extend_from_slice(&0u32.to_be_bytes());
        let first_flags: u32 = if sync { 0x0200_0000 } else { 0x0101_0000 };
        trun.extend_from_slice(&first_flags.to_be_bytes());
        for i in 0..25u32 {
            trun.extend_from_slice(&40u32.to_be_bytes());
            trun.extend_from_slice(&100u32.to_be_bytes());
            trun.extend_from_slice(&(if i == 0 { 80u32 } else { 0 }).to_be_bytes());
        }
        mp4_box(
            b"traf",
            &[
                full_box(b"tfhd", 0, 0x02_0000, &tfhd),
                full_box(b"tfdt", 1, 0, &tfdt),
                full_box(b"trun", 1, 0x01 | 0x04 | 0x100 | 0x200 | 0x800, &trun),
            ]
            .concat(),
        )
    };
    let moof = mp4_box(
        b"moof",
        &[full_box(b"mfhd", 0, 0, &seq.to_be_bytes()), audio, video].concat(),
    );
    [moof, mp4_box(b"mdat", &[0xEE; 2500])].concat()
}

/// B 站 `hls_fmp4` 保留源站的绝对 `tfdt`（这里取 65913 s）。
pub(crate) fn build_fmp4(fragments: u32) -> (Vec<u8>, Vec<(i64, u64)>, u64) {
    let base: u64 = 65_913_000;
    let mut out = init_segment(true);
    out.extend(mp4_box(b"styp", b"msdh\0\0\0\0msdhmsix"));
    let header_len = out.len() as u64;
    let mut keyframes = Vec::new();
    for i in 0..fragments {
        let sync = i % 2 == 0;
        if sync {
            keyframes.push(((base + i as u64 * 1000) as i64 + 80, out.len() as u64));
        }
        out.extend(fragment(i + 1, base + i as u64 * 1000, sync));
    }
    (out, keyframes, header_len)
}

#[test]
fn fmp4_scan_uses_the_video_track_and_absolute_tfdt() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, frames, header_len) = build_fmp4(5);
    let path = write(dir.path(), "a.mp4", &bytes);
    let index = refresh(&path, true).unwrap();
    assert_eq!(index.container, Container::Fmp4);
    assert_eq!(index.timescale, 1000);
    assert_eq!(index.track.id, 2);
    assert_eq!(index.base_ts, Some(65_913_080));
    // styp 属于第一个 moof 之前的头区；读取方先发它再从 moof 起读都合法
    assert_eq!(index.header_len, header_len);
    assert_eq!(index.keyframes, expected(65_913_080, 1000, &frames));
    assert_eq!(
        index.keyframes.iter().map(|k| k.t_ms).collect::<Vec<_>>(),
        vec![0, 2000, 4000]
    );
    assert_eq!(index.duration_ms, 5000 - 80);
    for k in &index.keyframes {
        assert_eq!(
            &bytes[k.offset as usize + 4..k.offset as usize + 8],
            b"moof"
        );
    }
}

#[test]
fn fmp4_growing_file_is_scanned_incrementally() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, frames, _) = build_fmp4(5);
    let cut = frames[1].1 as usize + 30; // 第二个关键帧的 moof 写到一半
    let path = write(dir.path(), "a.mp4", &bytes[..cut]);
    let partial = refresh(&path, false).unwrap();
    assert_eq!(partial.keyframes.len(), 1);
    assert_eq!(partial.scanned_upto, frames[1].1);

    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(&bytes[cut..]).unwrap();
    drop(file);
    let full = refresh(&path, true).unwrap();
    assert_eq!(full.keyframes, expected(65_913_080, 1000, &frames));
}

#[test]
fn non_fragmented_mp4_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = init_segment(false);
    bytes.extend(mp4_box(b"mdat", &[0; 64]));
    let path = write(dir.path(), "a.mp4", &bytes);
    assert_eq!(
        refresh(&path, true).unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(!index_path(&path).exists());
}
