use super::super::plan::{self, Attempt};
use super::super::tests::{add_segment, flv_session, setup};
use super::*;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::dvr::ts::{payload_start, pid_of, pusi, read_ts};
use crate::server::workbench::index::tests::{build_fmp4, build_ts};
use std::collections::HashMap;

async fn plan_of(pool: &ConnectionPool, session: i64, in_ms: i64, out_ms: i64) -> Plan {
    match plan::compute(pool, session, in_ms, out_ms).await.unwrap() {
        Attempt::Ready(plan) => plan,
        Attempt::Wait => panic!("不该等待"),
    }
}

async fn cut(plan: &Plan, dir: &Path, name: &str) -> (Vec<u8>, Remuxed) {
    let path = dir.join(name);
    let mut seen = 0;
    let done = to_file(plan, &path, &mut |n| seen = n).await.unwrap();
    assert_eq!(seen, plan.bytes());
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len() as u64, done.bytes);
    (bytes, done)
}

struct Tag {
    tag_type: u8,
    ts: u32,
    offset: usize,
    first: u8,
}

fn tags(bytes: &[u8]) -> Vec<Tag> {
    assert_eq!(&bytes[..3], b"FLV");
    let mut at = 13;
    let mut out = Vec::new();
    while at + 11 <= bytes.len() {
        let size = u32::from_be_bytes([0, bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
        let ts = u32::from_be_bytes([bytes[at + 7], bytes[at + 4], bytes[at + 5], bytes[at + 6]]);
        let prev = u32::from_be_bytes(bytes[at + 11 + size..at + 15 + size].try_into().unwrap());
        assert_eq!(prev as usize, 11 + size);
        out.push(Tag {
            tag_type: bytes[at],
            ts,
            offset: at,
            first: bytes[at + 11],
        });
        at += 15 + size;
    }
    assert_eq!(at, bytes.len());
    out
}

/// `onMetaData` 里某个数值字段 / 数组（按键名找，数值紧跟在键之后）。
fn amf_number(bytes: &[u8], key: &str) -> f64 {
    let mut needle = (key.len() as u16).to_be_bytes().to_vec();
    needle.extend_from_slice(key.as_bytes());
    needle.push(0x00);
    let at = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
        + needle.len();
    f64::from_be_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn amf_array(bytes: &[u8], key: &str) -> Vec<f64> {
    let mut needle = (key.len() as u16).to_be_bytes().to_vec();
    needle.extend_from_slice(key.as_bytes());
    needle.push(0x0A);
    let at = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
        + needle.len();
    let n = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    (0..n)
        .map(|i| {
            let p = at + 4 + i * 9 + 1;
            f64::from_be_bytes(bytes[p..p + 8].try_into().unwrap())
        })
        .collect()
}

#[tokio::test]
async fn flv_cut_starts_on_a_keyframe_at_zero_and_splices_across_gaps() {
    let (dir, pool, session, _) = flv_session().await;
    // 第一段 3000 起、第二段整段（绝对时间戳）、断流、第三段到 1000
    let plan = plan_of(&pool, session, 3500, 31_000).await;
    let (bytes, done) = cut(&plan, dir.path(), "out.flv").await;
    let all = tags(&bytes);

    assert_eq!(all[0].tag_type, 18, "文件头之后是新写的 onMetaData");
    assert_eq!((all[1].tag_type, all[1].ts, all[1].first), (9, 0, 0x17));
    assert_eq!((all[2].tag_type, all[2].ts), (8, 0));
    let media: Vec<&Tag> = all[3..].iter().collect();
    assert!(
        media.iter().all(|t| t.tag_type != 18),
        "源文件的 script tag 不带过来"
    );
    assert_eq!(
        (media[0].tag_type, media[0].ts, media[0].first),
        (9, 0, 0x17)
    );

    let video: Vec<u32> = media
        .iter()
        .filter(|t| t.tag_type == 9)
        .map(|t| t.ts)
        .collect();
    assert!(video.windows(2).all(|w| w[0] < w[1]), "视频时间戳严格递增");
    let audio: Vec<u32> = media
        .iter()
        .filter(|t| t.tag_type == 8)
        .map(|t| t.ts)
        .collect();
    assert!(audio.windows(2).all(|w| w[0] <= w[1]));
    // 25 + 100 + 25 帧，接缝处紧接（上一段最后一帧 + 40 ms）
    assert_eq!(video.len(), 25 + 100 + 25);
    assert_eq!(&video[24..26], &[960, 1000]);
    assert_eq!(&video[124..126], &[4960, 5000]);
    assert_eq!(*video.last().unwrap(), 5960);
    assert_eq!(done.duration_ms, 6000);

    let times = amf_array(&bytes, "times");
    let positions = amf_array(&bytes, "filepositions");
    let keyframes: Vec<&&Tag> = media.iter().filter(|t| t.first == 0x17).collect();
    assert_eq!(times.len(), keyframes.len());
    assert_eq!(times.len(), 1 + 4 + 1);
    for ((time, position), tag) in times.iter().zip(&positions).zip(&keyframes) {
        assert_eq!(*position as usize, tag.offset);
        assert_eq!((*time * 1000.0).round() as u32, tag.ts);
    }
    assert_eq!(amf_number(&bytes, "filesize") as usize, bytes.len());
    assert_eq!(amf_number(&bytes, "duration"), 6.0);
    assert_eq!(amf_number(&bytes, "videocodecid"), 7.0);
    assert_eq!(amf_number(&bytes, "audiocodecid"), 10.0);
}

#[tokio::test]
async fn piped_flv_has_no_metadata_to_backfill() {
    let (_dir, pool, session, _) = flv_session().await;
    let plan = plan_of(&pool, session, 1000, 2000).await;
    let mut out = Vec::new();
    let done = to_pipe(&plan, &mut out, &mut |_| {}).await.unwrap();
    assert_eq!(done.bytes as usize, out.len());
    let all = tags(&out);
    assert_eq!((all[0].tag_type, all[0].ts), (9, 0));
    assert!(all.iter().all(|t| t.tag_type != 18));
}

struct Packet {
    pid: u16,
    cc: u8,
    pts: Option<i64>,
}

fn packets(bytes: &[u8]) -> Vec<Packet> {
    assert_eq!(bytes.len() % 188, 0);
    bytes
        .chunks(188)
        .map(|p| {
            assert_eq!(p[0], 0x47);
            let pts = (pusi(p) && pid_of(p) >= 0x100)
                .then(|| payload_start(p))
                .flatten()
                .map(|s| read_ts(&p[s + 9..s + 14]));
            Packet {
                pid: pid_of(p),
                cc: p[3] & 0x0F,
                pts,
            }
        })
        .collect()
}

#[tokio::test]
async fn ts_cut_rewrites_pts_from_zero_and_keeps_counters_continuous() {
    let (dir, pool, session) = setup().await;
    // 第二段的 PTS 跨过 33 位回绕点
    for (name, first_pts, start) in [("a.ts", 900_000u64, 0), ("b.ts", (1 << 33) - 90_000, 3300)] {
        let path = dir.path().join(name);
        std::fs::write(&path, build_ts(first_pts, 100, false, false).bytes).unwrap();
        add_segment(&pool, session, &path, "finished", start, Some(start + 3300)).await;
    }
    let plan = plan_of(&pool, session, 2500, 4200).await;
    assert_eq!(plan.pieces.len(), 2);
    let (bytes, done) = cut(&plan, dir.path(), "out.ts").await;
    let all = packets(&bytes);
    assert_eq!((all[0].pid, all[1].pid), (0, 0x1000), "PAT、PMT 在最前面");
    assert_eq!(
        (all[2].pid, all[2].pts),
        (0x100, Some(0)),
        "第一个包是关键帧 PES"
    );

    let mut next: HashMap<u16, u8> = HashMap::new();
    for p in &all {
        if let Some(n) = next.get(&p.pid) {
            assert_eq!(p.cc, *n, "PID {:#x} 的连续计数器断了", p.pid);
        }
        next.insert(p.pid, (p.cc + 1) & 0x0F);
    }
    let video: Vec<i64> = all
        .iter()
        .filter(|p| p.pid == 0x100)
        .filter_map(|p| p.pts)
        .collect();
    assert!(video.windows(2).all(|w| w[0] < w[1]), "{video:?}");
    // 第一段 2000 起到末尾（40 帧），第二段 0..1000（30 帧）
    assert_eq!(video.len(), 40 + 30);
    assert_eq!(video[39], 39 * 3000);
    // 接缝处紧接上一帧（按毫秒对齐，差不到 1 ms）
    assert!(
        (2910..=3090).contains(&(video[40] - video[39])),
        "{video:?}"
    );
    let audio: Vec<i64> = all
        .iter()
        .filter(|p| p.pid == 0x101)
        .filter_map(|p| p.pts)
        .collect();
    assert!(audio.iter().all(|pts| *pts >= 0));
    assert!((done.duration_ms - (video[69] + 3000) / 90).abs() <= 1);
}

fn boxes(data: &[u8]) -> Vec<([u8; 4], usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 8 <= data.len() {
        let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
        out.push((data[at + 4..at + 8].try_into().unwrap(), at, size));
        at += size;
    }
    assert_eq!(at, data.len());
    out
}

/// moof 里 `(mfhd 序号, [(track_id, tfdt)])`。
fn moof_times(moof: &[u8]) -> (u32, Vec<(u32, u64)>) {
    let mut seq = 0;
    let mut times = Vec::new();
    for (kind, at, size) in boxes(&moof[8..]) {
        let body = &moof[8 + at + 8..8 + at + size];
        match &kind {
            b"mfhd" => seq = u32::from_be_bytes(body[4..8].try_into().unwrap()),
            b"traf" => {
                let mut track = 0;
                let mut tfdt = 0;
                for (kind, at, size) in boxes(body) {
                    let inner = &body[at + 8..at + size];
                    match &kind {
                        b"tfhd" => track = u32::from_be_bytes(inner[4..8].try_into().unwrap()),
                        b"tfdt" => {
                            tfdt = if inner[0] == 1 {
                                u64::from_be_bytes(inner[4..12].try_into().unwrap())
                            } else {
                                u32::from_be_bytes(inner[4..8].try_into().unwrap()) as u64
                            }
                        }
                        _ => {}
                    }
                }
                times.push((track, tfdt));
            }
            _ => {}
        }
    }
    (seq, times)
}

#[tokio::test]
async fn fmp4_cut_copies_init_and_rebases_tfdt_per_track() {
    let (dir, pool, session) = setup().await;
    let (bytes, _, _) = build_fmp4(6);
    for (name, start) in [("a.mp4", 0), ("b.mp4", 6000)] {
        let path = dir.path().join(name);
        std::fs::write(&path, &bytes).unwrap();
        add_segment(&pool, session, &path, "finished", start, Some(start + 5920)).await;
    }
    let plan = plan_of(&pool, session, 4500, 8500).await;
    let (out, _) = cut(&plan, dir.path(), "out.mp4").await;
    let top = boxes(&out);
    let kinds: Vec<&[u8; 4]> = top.iter().map(|b| &b.0).collect();
    assert_eq!((kinds[0], kinds[1]), (b"ftyp", b"moov"));
    assert!(kinds.iter().all(|k| *k != b"styp"));
    let moofs: Vec<(u32, Vec<(u32, u64)>)> = top
        .iter()
        .filter(|b| &b.0 == b"moof")
        .map(|b| moof_times(&out[b.1..b.1 + b.2]))
        .collect();
    // 第一段 4000..5920 两个分片，第二段 0..4000 四个分片
    assert_eq!(moofs.len(), 2 + 4);
    assert_eq!(
        moofs.iter().map(|m| m.0).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    let video: Vec<u64> = moofs.iter().map(|m| m.1[1].1).collect();
    let audio: Vec<u64> = moofs.iter().map(|m| m.1[0].1).collect();
    assert_eq!(moofs[0].1[1].0, 2);
    // 视频 tfdt（1000/s）：起始关键帧的解码时间为 0，第二段紧接第一段最后一帧
    assert_eq!(video, vec![0, 1000, 2000, 3000, 4000, 5000]);
    // 音频（48000/s）跟着视频换算，同一个时钟
    assert_eq!(audio, vec![0, 48_000, 96_000, 144_000, 192_000, 240_000]);
}
