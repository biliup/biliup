//! 弹幕 XML 用手写的构造数据（格式照 `crates/danmaku` 的写法），没有真实录下的 XML。

use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;

/// 文件创建于 Unix 1_750_000_000 秒。
const CREATED: i64 = 1_750_000_000;

fn d(secs: f64, text: &str) -> String {
    format!(
        "<d p=\"{secs:.3},1,25,16777215,{},0,42,0\">{text}</d>\n",
        CREATED + secs as i64
    )
}

fn s(unix: i64, price: f64) -> String {
    format!(
        "<s timestamp=\"{unix}\" uid=\"1\" username=\"甲\" price=\"{price}\" type=\"super_chat\" num=\"1\" giftname=\"醒目留言\">加油</s>\n"
    )
}

fn xml(body: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<i>\n{body}</i>\n").into_bytes()
}

#[test]
fn parses_comments_and_paid_events() {
    let body = [
        d(1.5, "第一条"),
        d(2.0, "a &amp; b &lt;3"),
        "<d p=\"3,1,25,0,0,0,0,0\"></d>\n".into(),
        s(CREATED + 4, 30.0),
        "<s timestamp=\"1750000005\" price=\"5\" type=\"gift\"/>\n".into(),
        "<o timestamp=\"1750000006\">raw</o>\n".into(),
        d(7.25, "最后"),
    ]
    .concat();
    let parsed = parse_xml(&xml(&body));
    let texts: Vec<&str> = parsed.comments.iter().map(|c| c.2.as_str()).collect();
    assert_eq!(texts, vec!["第一条", "a & b <3", "最后"]);
    assert_eq!(parsed.comments[0].0, 1.5);
    assert_eq!(parsed.comments[0].1, Some(CREATED + 1));
    assert_eq!(parsed.paid, vec![(CREATED + 4, 30.0), (CREATED + 5, 5.0)]);
    // Unix 秒是截断的，推出来的创建时刻最多早 1 秒
    let created = parsed.created_at_ms().unwrap();
    assert!((CREATED * 1000 - 1000..=CREATED * 1000).contains(&created));
}

#[test]
fn a_half_written_file_keeps_what_was_read() {
    let mut bytes = xml(&[d(1.0, "一"), d(2.0, "二")].concat());
    // 进程被杀：没有 `</i>`，最后一条写到一半
    bytes.truncate(bytes.len() - "</i>\n".len());
    bytes.extend_from_slice(b"<d p=\"3.000,1,25,0,17500");
    let parsed = parse_xml(&bytes);
    assert_eq!(parsed.comments.len(), 2);

    let broken = b"<i><d p=\"1,1,25,0,1750000001,0,0,0\">ok</d><d p=\"2\">\xff\xfe</d><<<";
    let parsed = parse_xml(broken);
    assert_eq!(parsed.comments[0].2, "ok");
    assert!(parse_xml(b"").comments.is_empty());
    assert!(parse_xml(b"not xml at all").comments.is_empty());
}

#[test]
fn bad_attributes_are_skipped() {
    let body = [
        "<d p=\"-1,1,25,0,0,0,0,0\">负数</d>\n".to_string(),
        "<d p=\"abc,1\">不是数</d>\n".into(),
        "<d>没有 p</d>\n".into(),
        "<d p=\"5\">只有秒数</d>\n".into(),
    ]
    .concat();
    let parsed = parse_xml(&xml(&body));
    assert_eq!(parsed.comments, vec![(5.0, None, "只有秒数".to_string())]);
}

#[test]
fn aligns_to_segment_start_or_wall_clock() {
    let parsed = parse_xml(&xml(
        &[d(1.5, "a"), d(10.0, "b"), s(CREATED + 20, 1.0)].concat()
    ));
    let (comments, paid) = align(&parsed, Alignment::Segment { start_ms: 60_000 });
    let at: Vec<i64> = comments.iter().map(|c| c.at_ms).collect();
    assert_eq!(at, vec![61_500, 70_000]);
    assert_eq!(paid[0].at_ms, 80_000, "礼物按同一文件的创建时刻换算");

    // 场次 0 点比文件创建早 30 秒
    let started_at = (CREATED - 30) * 1000;
    let (comments, paid) = align(&parsed, Alignment::Wallclock { started_at });
    let at: Vec<i64> = comments.iter().map(|c| c.at_ms).collect();
    assert_eq!(at, vec![31_000, 40_000], "按 Unix 秒只精确到秒");
    assert_eq!(paid[0].at_ms, 50_000);

    // 没有 Unix 秒的弹幕按墙钟对不上，礼物也换算不了
    let bare = parse_xml(&xml(
        "<d p=\"1\">x</d><s timestamp=\"1750000000\" price=\"1\">y</s>",
    ));
    assert_eq!(align(&bare, Alignment::Wallclock { started_at }).0.len(), 0);
    assert!(
        align(&bare, Alignment::Segment { start_ms: 0 })
            .1
            .is_empty()
    );
}

fn comments(at_ms: impl IntoIterator<Item = (i64, &'static str)>) -> Vec<Comment> {
    at_ms
        .into_iter()
        .map(|(at_ms, text)| Comment {
            at_ms,
            text: text.into(),
        })
        .collect()
}

#[test]
fn finds_peaks_over_a_rolling_median() {
    // 20 分钟，每 10 秒 2 条；第 600–620 秒突然 40 条（哈哈哈 25、牛 15）
    let mut list = Vec::new();
    for bucket in 0..120 {
        list.push((bucket * 10_000 + 1_000, "日常"));
        list.push((bucket * 10_000 + 5_000, "日常"));
    }
    for i in 0..40 {
        let text = if i % 8 < 5 { "哈哈哈" } else { "牛" };
        list.push((600_000 + i * 500, text));
    }
    list.sort_by_key(|c| c.0);
    let density = density(&comments(list), &[], 1_200_000);
    assert_eq!(density.counts.len(), 120);
    assert_eq!(density.total, 280);
    assert_eq!(density.counts[60], 22);
    assert_eq!(density.baseline[60], 2.0);
    assert_eq!(density.peaks.len(), 1, "{:?}", density.peaks);
    let peak = &density.peaks[0];
    // 两个高峰桶（600、610 秒）前后各扩一桶
    assert_eq!((peak.from_ms, peak.to_ms), (590_000, 630_000));
    assert_eq!(peak.count, 2 + 22 + 22 + 2);
    assert_eq!(peak.samples[0].text, "哈哈哈");
    assert_eq!(peak.samples[0].count, 25);
    assert_eq!(peak.samples[1].text, "牛");
    assert!(peak.ratio > 5.0);
    assert_eq!(density.peaks_in(0, 595_000).count(), 1);
    assert_eq!(density.peaks_in(630_000, 700_000).count(), 0);
}

#[test]
fn quiet_rooms_need_ten_more_than_baseline() {
    // 基线 0 时 3 倍也是 0，所以要求至少多 10 条
    let nine: Vec<(i64, &str)> = (0..9).map(|i| (300_000 + i, "x")).collect();
    assert!(density(&comments(nine), &[], 900_000).peaks.is_empty());
    let ten: Vec<(i64, &str)> = (0..10).map(|i| (300_000 + i, "x")).collect();
    assert_eq!(density(&comments(ten), &[], 900_000).peaks.len(), 1);
}

#[test]
fn nearby_peaks_merge_and_paid_events_are_bucketed() {
    let mut list = Vec::new();
    for base in [100_000, 120_000] {
        for i in 0..15 {
            list.push((base + i, "冲"));
        }
    }
    let paid = [
        Paid {
            at_ms: 125_000,
            price: 30.0,
        },
        Paid {
            at_ms: 129_000,
            price: 50.0,
        },
        Paid {
            at_ms: 5_000_000,
            price: 1.0,
        },
    ];
    let density = density(&comments(list), &paid, 600_000);
    assert_eq!(density.peaks.len(), 1, "隔一桶的两个高峰扩一桶后连上");
    assert_eq!(
        (density.peaks[0].from_ms, density.peaks[0].to_ms),
        (90_000, 140_000)
    );
    assert_eq!(
        density.paid,
        vec![PaidBucket {
            from_ms: 120_000,
            count: 2,
            price: 80.0
        }],
        "时间轴之外的丢掉"
    );
    let empty = density_of_nothing();
    assert_eq!(empty.counts, vec![0]);
    assert!(empty.peaks.is_empty());
}

fn density_of_nothing() -> Density {
    density(&[], &[], 0)
}

#[test]
fn samples_are_trimmed_and_capped() {
    let long = "这是一条非常非常非常非常非常非常非常长的弹幕内容";
    let mut list: Vec<(i64, &str)> = (0..20).map(|i| (50_000 + i, long)).collect();
    list.extend((0..3).map(|i| (51_000 + i, "  ")));
    let density = density(&comments(list), &[], 120_000);
    let sample = &density.peaks[0].samples[0];
    assert_eq!(sample.text.chars().count(), SAMPLE_CHARS);
    assert_eq!(sample.count, 20);
    assert_eq!(density.peaks[0].samples.len(), 1, "空白弹幕不当样例");
}

async fn database(dir: &Path) -> ConnectionPool {
    ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn session_density_reads_every_segment() {
    let dir = tempfile::tempdir().unwrap();
    let pool = database(dir.path()).await;
    let started_at = (CREATED - 100) * 1000;
    sqlx::query(
        "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES (1, 'a', 'https://a', 't', '2026-09-24 00:00:00', '', ?, ?)",
    )
    .bind(started_at)
    .bind(started_at + 400_000)
    .execute(&pool)
    .await
    .unwrap();
    // 第一段记了 danmaku_path；第二段没记，但旁边有同名 .xml；第三段什么都没有
    let first_xml = dir.path().join("first.xml");
    std::fs::write(&first_xml, xml(&[d(5.0, "一"), d(15.0, "二")].concat())).unwrap();
    let second = dir.path().join("second.flv");
    let second_xml = dir.path().join("second.xml");
    // 第二个文件在场次 200 秒时创建，第 3 秒一条
    let body = format!("<d p=\"3.000,1,25,0,{},0,0,0\">三</d>", CREATED - 100 + 203);
    std::fs::write(&second_xml, xml(&body)).unwrap();
    for (path, danmaku, start, end) in [
        (
            dir.path().join("first.flv"),
            Some(first_xml.to_string_lossy().into_owned()),
            0,
            200_000,
        ),
        (second, None, 200_000, 300_000),
        (dir.path().join("third.flv"), None, 300_000, 400_000),
    ] {
        sqlx::query(
            "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms, danmaku_path)
             VALUES (1, ?, 'flv', 'finished', ?, ?, 0, ?)",
        )
        .bind(path.to_string_lossy().into_owned())
        .bind(start)
        .bind(end)
        .bind(danmaku)
        .execute(&pool)
        .await
        .unwrap();
    }
    let density = session_density(&pool, 1).await.unwrap().unwrap();
    assert_eq!(density.total, 3);
    assert_eq!(density.counts.len(), 40);
    assert_eq!(density.counts[0], 1);
    assert_eq!(density.counts[1], 1);
    assert_eq!(density.counts[20], 1, "同名 .xml 按 Unix 秒对齐到 203 秒");

    assert!(session_density(&pool, 99).await.unwrap().is_none());
    std::fs::remove_file(&first_xml).unwrap();
    std::fs::remove_file(&second_xml).unwrap();
    assert!(
        session_density(&pool, 1).await.unwrap().is_none(),
        "一个文件都读不到时为空"
    );
}
