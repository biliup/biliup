use super::*;
use crate::server::auto_clip::danmaku::{PaidBucket, Peak, Sample};

#[test]
fn windows_overlap_by_two_minutes() {
    assert!(windows(0).is_empty());
    assert_eq!(
        windows(10 * 60_000),
        vec![Window {
            from_ms: 0,
            to_ms: 600_000
        }]
    );
    let four_hours = windows(4 * 3_600_000);
    assert_eq!(four_hours.len(), 9);
    assert_eq!(four_hours[1].from_ms, WINDOW_MS - OVERLAP_MS);
    assert!(
        four_hours
            .windows(2)
            .all(|w| w[0].to_ms - w[1].from_ms == OVERLAP_MS)
    );
    assert_eq!(four_hours.last().unwrap().to_ms, 4 * 3_600_000);
    // 正好 30 分钟不多切一窗
    assert_eq!(windows(WINDOW_MS).len(), 1);
}

#[test]
fn halves_keep_an_overlap_and_stop_at_five_minutes() {
    let (left, right) = Window {
        from_ms: 0,
        to_ms: WINDOW_MS,
    }
    .halves()
    .unwrap();
    assert_eq!(left.to_ms, 16 * 60_000);
    assert_eq!(right.from_ms, 14 * 60_000);
    assert!(
        Window {
            from_ms: 0,
            to_ms: 9 * 60_000
        }
        .halves()
        .is_none()
    );
}

#[test]
fn clock_round_trips() {
    assert_eq!(clock(0), "0:00:00");
    assert_eq!(clock(5_430_999), "1:30:30");
    assert_eq!(clock(-5), "0:00:00");
    assert_eq!(parse_clock("1:30:30"), Some(5_430_000));
    assert_eq!(parse_clock(" 0:02:05.5 "), Some(125_500));
    assert_eq!(parse_clock("12:34"), Some(754_000));
    assert_eq!(
        parse_clock("75:00"),
        Some(4_500_000),
        "只有分秒时分钟可以超过 60"
    );
    assert_eq!(parse_clock("95.25"), Some(95_250));
    for bad in [
        "", "1:60:00", "1:00:60", "a:00:00", "1::00", "-1:00", "1:2:3:4", "-3",
    ] {
        assert_eq!(parse_clock(bad), None, "{bad}");
    }
}

#[test]
fn estimates_count_han_characters_one_each() {
    assert_eq!(estimate_text_tokens("弹幕高峰"), 4);
    assert_eq!(estimate_text_tokens("abcdefgh"), 2);
    assert_eq!(estimate_text_tokens("abc"), 1);
    assert_eq!(estimate_text_tokens(""), 0);
}

fn line(from_s: i64, to_s: i64, text: &str) -> Line {
    Line {
        chunk: "1-0".into(),
        from_ms: from_s * 1000,
        to_ms: to_s * 1000,
        text: text.into(),
    }
}

fn sample_density() -> Density {
    let mut counts = vec![2u32; 12];
    counts[3] = 30;
    counts[4] = 25;
    Density {
        bucket_ms: 10_000,
        counts,
        baseline: vec![2.0; 12],
        peaks: vec![Peak {
            from_ms: 20_000,
            to_ms: 60_000,
            count: 59,
            baseline: 2.0,
            ratio: 7.4,
            samples: vec![
                Sample {
                    text: "哈哈哈".into(),
                    count: 20,
                },
                Sample {
                    text: "？？？".into(),
                    count: 9,
                },
            ],
        }],
        paid: vec![PaidBucket {
            from_ms: 40_000,
            count: 3,
            price: 90.0,
        }],
        total: 79,
    }
}

fn session() -> SessionInfo {
    SessionInfo {
        title: "深夜  杂谈\n回".into(),
        streamer: "某主播".into(),
        platform: "live.bilibili.com".into(),
    }
}

#[test]
fn prompt_snapshot() {
    let session = session();
    let density = sample_density();
    let lines = [
        line(5, 9, "今天先把这关打完"),
        line(41, 44, "这都能赢？"),
        line(200, 205, "窗外的一句"),
    ];
    let shots = [
        Shot {
            t_ms: 45_000,
            rank: 0,
        },
        Shot {
            t_ms: 90_000,
            rank: 1,
        },
    ];
    let prompt = build(
        &PromptInput::builder()
            .session(&session)
            .window(Window {
                from_ms: 0,
                to_ms: 120_000,
            })
            .lines(&lines)
            .density(&density)
            .shots(&shots)
            .min_clip_secs(15)
            .max_clip_secs(180)
            .build(),
    );
    assert_eq!(
        prompt.user,
        "场次：深夜 杂谈 回｜主播：某主播｜平台：live.bilibili.com｜本窗：0:00:00–0:02:00\n\
         要求：找出适合单独剪成短视频的片段（高能操作、搞笑、名场面、情绪爆发、与观众的精彩互动、才艺高潮）；每段 15 秒到 180 秒；最多 10 段；没有就返回空数组；只能用下面给出的时间范围。\n\
         [弹幕密度] 每 10 秒条数（基线约 2）：0:00:00 2 2 2 30 25 2 2 2 2 2 2 2\n\
         [弹幕高峰] 0:00:20–0:01:00 共 59 条（基线 7 倍）：「哈哈哈」×20 「？？？」×9\n\
         [礼物与醒目留言] 最集中的时段：0:00:40 3 次\n\
         [语音] 0:00:05–0:00:09 今天先把这关打完\n\
         [语音] 0:00:41–0:00:44 这都能赢？\n\
         [画面] 图1 = 0:00:45，图2 = 0:01:30（随后附图）"
    );
    assert_eq!(prompt.images, vec![45_000, 90_000]);
    assert!(prompt.system.starts_with("你是直播切片助手"));
    assert!(prompt.system.contains("不得编造材料里没有出现的时间"));
    assert!(prompt.system.contains("\"candidates\""));

    let plain = prompt.without_images();
    assert!(plain.images.is_empty());
    assert!(!plain.user.contains("[画面]"));
    assert!(prompt.estimate_tokens() - plain.estimate_tokens() > 2 * IMAGE_TOKENS);
}

#[test]
fn prompt_without_danmaku_or_speech() {
    let session = session();
    let prompt = build(
        &PromptInput::builder()
            .session(&session)
            .window(Window {
                from_ms: 1_800_000,
                to_ms: 1_900_000,
            })
            .lines(&[])
            .min_clip_secs(20)
            .max_clip_secs(60)
            .build(),
    );
    let lines: Vec<&str> = prompt.user.lines().collect();
    assert_eq!(
        lines[0],
        "场次：深夜 杂谈 回｜主播：某主播｜平台：live.bilibili.com｜本窗：0:30:00–0:31:40"
    );
    assert!(lines[1].contains("每段 20 秒到 60 秒"));
    assert_eq!(
        &lines[2..],
        ["[弹幕] 本场无弹幕记录", "[语音] 本窗没有转写到语音"]
    );
    assert!(prompt.images.is_empty());
}
