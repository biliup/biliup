use super::*;
use crate::server::auto_clip::danmaku::Peak;

fn parse_ok(content: &str) -> (Vec<Proposed>, Vec<Rejection>) {
    let mut rejected = Vec::new();
    let proposed = parse(content, &mut rejected).unwrap();
    (proposed, rejected)
}

#[test]
fn parses_the_documented_schema() {
    let (proposed, rejected) = parse_ok(
        r#"{"candidates": [
          {"start": "1:42:05", "end": "1:43:20", "title": "残血1v3翻盘",
           "reason": "弹幕在 1:42:10 起 30 秒内是基线 3 倍，主播喊「这都能赢」",
           "confidence": 0.82, "tags": ["高能", ""], "evidence": ["danmaku", "asr", "image:2", "图 3", "image:x"]}
        ]}"#,
    );
    assert!(rejected.is_empty());
    assert_eq!(
        proposed,
        vec![Proposed {
            start_ms: 6_125_000,
            end_ms: 6_200_000,
            title: "残血1v3翻盘".into(),
            reason: "弹幕在 1:42:10 起 30 秒内是基线 3 倍，主播喊「这都能赢」".into(),
            confidence: Some(0.82),
            tags: vec!["高能".into()],
            images: vec![2, 3],
        }]
    );
}

#[test]
fn accepts_wrapped_replies_and_bare_arrays() {
    let fenced = "好的，结果如下：\n```json\n{\"candidates\": [{\"start\": \"0:00:10\", \"end\": \"0:00:40\"}]}\n```";
    assert_eq!(parse_ok(fenced).0.len(), 1);
    let bare = "[{\"start\": 10, \"end\": 40.5}]";
    let (proposed, _) = parse_ok(bare);
    assert_eq!((proposed[0].start_ms, proposed[0].end_ms), (10_000, 40_500));
    assert_eq!(proposed[0].title, "");
    assert!(parse_ok("{\"candidates\": []}").0.is_empty());
}

#[test]
fn rejects_replies_that_are_not_json() {
    let mut rejected = Vec::new();
    assert_eq!(
        parse("抱歉，我无法完成这个任务。", &mut rejected),
        Err(ReplyError::NotJson)
    );
    assert_eq!(
        parse("{\"candidates\": [{\"start\": \"0:00:10\",", &mut rejected),
        Err(ReplyError::NotJson),
        "截断的 JSON"
    );
    assert_eq!(
        parse("{\"clips\": []}", &mut rejected),
        Err(ReplyError::NoCandidates)
    );
    assert_eq!(
        parse("{\"candidates\": \"none\"}", &mut rejected),
        Err(ReplyError::NoCandidates)
    );
    assert!(rejected.is_empty());
}

#[test]
fn bad_times_are_counted_and_confidence_is_optional() {
    let (proposed, rejected) = parse_ok(
        r#"{"candidates": [
          {"start": "0:01:00", "end": "0:01:30"},
          {"start": "0:01:00", "end": "0:01:30", "confidence": "85%"},
          {"start": "0:01:00", "end": "0:01:30", "confidence": 85},
          {"start": "0:01:00", "end": "0:01:30", "confidence": -0.3},
          {"start": "0:01:00", "end": "0:01:30", "confidence": 250},
          {"start": "0:01:00", "end": "0:01:30", "confidence": "高"},
          {"start": "1:75:00", "end": "0:01:30"},
          {"end": "0:01:30"},
          {"start": null, "end": "0:01:30"},
          "不是对象"
        ]}"#,
    );
    let confidences: Vec<Option<f64>> = proposed.iter().map(|p| p.confidence).collect();
    assert_eq!(
        confidences,
        vec![None, Some(0.85), Some(0.85), None, None, None]
    );
    assert_eq!(rejected, vec![Rejection::BadTime; 4]);
}

#[test]
fn titles_are_single_line_and_capped() {
    let long = "长".repeat(200);
    let content = format!(
        "{{\"candidates\": [{{\"start\": 1, \"end\": 30, \"title\": \"第一行\\n第二行\", \"reason\": \"{long}\"}}]}}"
    );
    let (proposed, _) = parse_ok(&content);
    assert_eq!(proposed[0].title, "第一行 第二行");
    assert_eq!(
        proposed[0].reason.chars().count(),
        MAX_REASON_CHARS.min(200)
    );
}

fn line(from_s: i64, to_s: i64) -> Line {
    Line {
        chunk: "1-0".into(),
        from_ms: from_s * 1000,
        to_ms: to_s * 1000,
        text: "说话".into(),
    }
}

fn density_with_peak(from_s: i64, to_s: i64, count: u32) -> Density {
    Density {
        bucket_ms: 10_000,
        counts: vec![0; 60],
        baseline: vec![0.0; 60],
        peaks: vec![Peak {
            from_ms: from_s * 1000,
            to_ms: to_s * 1000,
            count,
            baseline: 1.0,
            ratio: 5.0,
            samples: Vec::new(),
        }],
        paid: Vec::new(),
        total: count as u64,
    }
}

fn proposed(start_s: i64, end_s: i64, images: Vec<usize>) -> Proposed {
    Proposed {
        start_ms: start_s * 1000,
        end_ms: end_s * 1000,
        title: String::new(),
        reason: String::new(),
        confidence: None,
        tags: Vec::new(),
        images,
    }
}

#[test]
fn check_enforces_range_length_and_evidence() {
    let lines = [line(100, 105), line(300, 302)];
    let density = density_with_peak(200, 240, 80);
    let images = [450_000, 480_000];
    let cx = Context {
        window: Window {
            from_ms: 60_000,
            to_ms: 540_000,
        },
        end_ms: 500_000,
        min_ms: 15_000,
        max_ms: 180_000,
        lines: &lines,
        density: Some(&density),
        images: &images,
    };
    let check_one = |start, end, images| check(&proposed(start, end, images), &cx);
    assert_eq!(
        check_one(90, 120, vec![]),
        Ok(Evidence {
            asr_lines: 1,
            danmaku: 0,
            images: vec![]
        })
    );
    assert_eq!(
        check_one(210, 260, vec![]),
        Ok(Evidence {
            asr_lines: 0,
            danmaku: 80,
            images: vec![]
        })
    );
    assert_eq!(
        check_one(440, 470, vec![1, 1, 3]),
        Ok(Evidence {
            asr_lines: 0,
            danmaku: 0,
            images: vec![450_000]
        }),
        "引用了不存在的图 3 不算"
    );
    assert_eq!(
        check_one(30, 70, vec![]),
        Err(Rejection::OutOfRange),
        "窗外"
    );
    assert_eq!(
        check_one(480, 520, vec![]),
        Err(Rejection::OutOfRange),
        "录像之外"
    );
    assert_eq!(check_one(120, 110, vec![]), Err(Rejection::BadLength));
    assert_eq!(
        check_one(100, 110, vec![]),
        Err(Rejection::BadLength),
        "太短"
    );
    assert_eq!(
        check_one(90, 300, vec![]),
        Err(Rejection::BadLength),
        "太长"
    );
    assert_eq!(
        check_one(350, 400, vec![]),
        Err(Rejection::NoEvidence),
        "编造的时间：区间里什么材料都没有"
    );
    let bare = Context {
        density: None,
        images: &[],
        ..cx
    };
    assert_eq!(
        check(&proposed(210, 260, vec![1]), &bare),
        Err(Rejection::NoEvidence)
    );
}

fn candidate(in_s: i64, out_s: i64, confidence: Option<f64>, title: &str) -> Candidate {
    Candidate {
        in_ms: in_s * 1000,
        out_ms: out_s * 1000,
        title: title.into(),
        reason: String::new(),
        confidence,
        tags: Vec::new(),
        evidence: Evidence::default(),
    }
}

#[test]
fn merge_keeps_the_more_confident_duplicate() {
    let merged = merge(
        vec![
            candidate(100, 160, Some(0.6), "窗一"),
            // 与上一条重叠 50/70 > 0.5：重复，置信度高的留下
            candidate(110, 170, Some(0.9), "窗二"),
            // 与「窗二」重叠 30/110：不算重复
            candidate(140, 220, Some(0.5), "相邻"),
            candidate(400, 430, None, "没给置信度"),
            candidate(400, 431, None, "同样没给"),
        ],
        10,
    );
    let titles: Vec<&str> = merged.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, vec!["窗二", "相邻", "没给置信度"]);
}

#[test]
fn merge_truncates_by_confidence_then_sorts_by_time() {
    let all: Vec<Candidate> = (0..30)
        .map(|i| candidate(i * 100, i * 100 + 30, Some(i as f64 / 100.0), "x"))
        .collect();
    let merged = merge(all, 5);
    let starts: Vec<i64> = merged.iter().map(|c| c.in_ms / 1000).collect();
    assert_eq!(starts, vec![2500, 2600, 2700, 2800, 2900]);
    assert!(merge(Vec::new(), 5).is_empty());
}

#[test]
fn iou_boundaries() {
    assert!(!overlaps_too_much((0, 100), (50, 150)), "1/3");
    assert!(overlaps_too_much((0, 100), (10, 100)));
    assert!(!overlaps_too_much((0, 100), (100, 200)), "首尾相接不重叠");
}
