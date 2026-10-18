//! Intermediate layout assertions against frozen, externally generated C ASS.
//! This parser only reads evidence; it is independent of the new ASS writer.
use super::layout::{self, Emission, Placement, Style};
use super::{model, parse};
use crate::ConvertOptions;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    name: String,
    input: String,
    options: ConvertOptions,
}

fn time(text: &str) -> i64 {
    let parts: Vec<_> = text
        .split([':', '.'])
        .map(|part| part.parse::<i64>().unwrap())
        .collect();
    parts[0] * 3_600_000 + parts[1] * 60_000 + parts[2] * 1000 + parts[3] * 10
}

fn arguments(text: &str, tag: &str) -> Option<Vec<i64>> {
    let tail = text.split_once(tag)?.1;
    Some(
        tail.split_once(')')?
            .0
            .split(',')
            .map(|part| part.parse().unwrap())
            .collect(),
    )
}

fn neutral(bytes: Vec<u8>) -> Vec<u8> {
    if !cfg!(windows) {
        return bytes;
    }
    let body = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap();
    body.iter()
        .enumerate()
        .filter_map(|(at, &b)| {
            if b == b'\r' && body.get(at + 1) == Some(&b'\n') {
                None
            } else {
                Some(b)
            }
        })
        .collect()
}

#[test]
fn legacy_layout_decisions_match_frozen_c_events() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(root.join("cases.json")).unwrap()).unwrap();
    let mut checked = 0;
    for case in cases {
        // These four inputs have separate assertions for intentional changes.
        // All other cases retain their immutable byte expectations.
        if matches!(
            case.input.as_str(),
            "long_text.xml" | "special_chars.xml" | "trailing_ref.xml" | "unsorted.xml"
        ) {
            continue;
        }
        let input = std::fs::File::open(root.join("inputs").join(&case.input)).unwrap();
        let mut pool = parse::read(input, parse::Budget::RAW).unwrap();
        model::normalize_and_sort(&mut pool, &case.options);
        let actual = layout::layout(pool, &case.options)
            .unwrap_or_else(|error| panic!("{}: {error}", case.name));
        let mut serialized = Vec::new();
        super::writer::write(&mut serialized, &actual, &case.options).unwrap();
        serialized = neutral(serialized);
        let legacy =
            std::fs::read(root.join("expected").join(format!("{}.ass", case.name))).unwrap();
        let first = serialized
            .iter()
            .zip(&legacy)
            .position(|(a, b)| a != b)
            .unwrap_or(serialized.len().min(legacy.len()));
        if case.input != "emoji.xml" {
            assert!(
                serialized == legacy,
                "{}: writer byte difference at {} (line {})",
                case.name,
                first,
                legacy[..first.min(legacy.len())]
                    .iter()
                    .filter(|&&b| b == b'\n')
                    .count()
                    + 1
            );
        }
        let expected =
            std::fs::read_to_string(root.join("expected").join(format!("{}.ass", case.name)))
                .unwrap();
        let expected: Vec<_> = expected
            .lines()
            .filter(|line| line.starts_with("Dialogue:") || line.starts_with("Comment:"))
            .collect();
        let emitted: Vec<_> = actual
            .iter()
            .enumerate()
            .filter(|(_, item)| item.resolved().emission != Emission::Omit)
            .collect();
        assert_eq!(
            emitted.len(),
            expected.len(),
            "{}: emission count",
            case.name
        );
        for ((index, record), line) in emitted.iter().zip(expected) {
            let item = record.resolved();
            let ordinal = index + 1;
            let context = format!(
                "{}: input {} / sorted {}",
                case.name, record.input_sequence, ordinal
            );
            if matches!(item.style, Style::Code | Style::Unknown) {
                let expected = if item.style == Style::Code {
                    format!(
                        "Comment: NO.{}(Code danmaku):Unable to read this type.",
                        ordinal
                    )
                } else {
                    format!("Comment: NO.{}:unknow type", ordinal)
                };
                assert_eq!(line, expected, "{context}");
                continue;
            }
            let fields: Vec<_> = line.splitn(10, ',').collect();
            assert_eq!(
                fields[0],
                format!(
                    "{}: {}",
                    if item.emission == Emission::Dialogue {
                        "Dialogue"
                    } else {
                        "Comment"
                    },
                    item.layer
                ),
                "{context}"
            );
            assert_eq!(
                (item.start_ms.max(0), item.end_ms.max(0)),
                (time(fields[1]), time(fields[2])),
                "{context}: timing"
            );
            let style = match item.style {
                Style::RightToLeft => "R2L",
                Style::LeftToRight => "L2R",
                Style::Top => "TOP",
                Style::Bottom => "BTM",
                Style::Special => "SP",
                _ => unreachable!(),
            };
            assert_eq!(fields[3], style, "{context}: style");
            match &item.placement {
                Placement::Position { x, y } => assert_eq!(
                    arguments(fields[9], "\\pos(").unwrap(),
                    vec![*x, i64::from(*y)],
                    "{context}: position"
                ),
                Placement::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    timing,
                } => {
                    let mut args = vec![*from_x, i64::from(*from_y), *to_x, i64::from(*to_y)];
                    if let Some(timing) = timing {
                        args.extend([timing.start_ms, timing.end_ms]);
                    }
                    assert_eq!(
                        arguments(fields[9], "\\move(").unwrap(),
                        args,
                        "{context}: motion"
                    );
                }
                Placement::None => panic!("{context}: emitted event has no placement"),
            }
            if let Some(size) = item.font_size_override {
                assert!(
                    fields[9].contains(&format!("\\fs{size}")),
                    "{context}: font size"
                );
            }
            if let Some(rows) = &item.pixel_rows {
                assert!(
                    rows.start < rows.end && rows.end <= case.options.height as usize,
                    "{context}: pixel rows"
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 26);
}

#[test]
fn intentional_change_events_have_independent_text_and_layout_assertions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(root.join("cases.json")).unwrap()).unwrap();
    for case in cases.into_iter().filter(|case| {
        matches!(
            case.input.as_str(),
            "emoji.xml"
                | "special_chars.xml"
                | "trailing_ref.xml"
                | "long_text.xml"
                | "unsorted.xml"
        )
    }) {
        let mut pool = parse::read(
            std::fs::File::open(root.join("inputs").join(&case.input)).unwrap(),
            parse::Budget::RAW,
        )
        .unwrap();
        model::normalize_and_sort(&mut pool, &case.options);
        let events = layout::layout(pool, &case.options).unwrap();
        let mut bytes = Vec::new();
        super::writer::write(&mut bytes, &events, &case.options).unwrap();
        let text = String::from_utf8(neutral(bytes)).unwrap();
        let legacy_bytes =
            std::fs::read(root.join("expected").join(format!("{}.ass", case.name))).unwrap();
        let expected = if case.input == "long_text.xml" {
            assert!(
                std::str::from_utf8(&legacy_bytes).is_err(),
                "frozen truncation must be visible"
            );
            String::from_utf8_lossy(&legacy_bytes).into_owned()
        } else {
            String::from_utf8(legacy_bytes).unwrap()
        };
        assert_eq!(
            text.split_once("\nDialogue:").unwrap().0,
            expected.split_once("\nDialogue:").unwrap().0,
            "{}: header unchanged",
            case.name
        );
        let lines: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("Dialogue:") || line.starts_with("Comment:"))
            .collect();
        let old: Vec<_> = expected
            .lines()
            .filter(|line| line.starts_with("Dialogue:") || line.starts_with("Comment:"))
            .collect();
        assert_eq!(lines.len(), old.len());
        match case.input.as_str() {
            "emoji.xml" => {
                for (at, (new, old)) in lines.iter().zip(&old).enumerate() {
                    assert_eq!(
                        new.splitn(10, ',').take(9).collect::<Vec<_>>(),
                        old.splitn(10, ',').take(9).collect::<Vec<_>>()
                    );
                    if !(2..=6).contains(&at) {
                        assert_eq!(new, old);
                    } else {
                        let literal = [
                            "&#128514;\\hnumeric\\hreference",
                            "a&#b\\hmid-text\\hmarker",
                            "&#;\\hempty\\hreference",
                            "&#9999999;\\hout\\hof\\hrange",
                            "&#12\\hno\\hsemicolon\\hhere",
                        ][at - 2];
                        assert!(new.ends_with(literal), "{at}: {new}");
                        assert_eq!(
                            new.split_once('}').unwrap().0,
                            old.split_once('}').unwrap().0,
                            "IC05 affects text serialization only here"
                        );
                    }
                }
            }
            "special_chars.xml" => {
                for at in 0..10 {
                    assert_eq!(lines[at], old[at], "special_chars event {at}");
                }
                assert_eq!(
                    lines[10].split_once('}').unwrap().0,
                    old[10].split_once('}').unwrap().0
                );
                assert!(lines[10].ends_with("&amp;\\halready\\hescaped"));
            }
            "trailing_ref.xml" => {
                for at in 0..2 {
                    assert_eq!(
                        lines[at].split_once('}').unwrap().0,
                        old[at].split_once('}').unwrap().0
                    );
                }
                assert!(lines[0].ends_with("ends\\hwith\\h&#"));
                assert!(lines[1].ends_with("half\\hemoji\\h&#55357;"));
            }
            "long_text.xml" => {
                let metadata = [
                    (500, 12500, 0, Style::RightToLeft),
                    (900, 12900, 0, Style::RightToLeft),
                    (1300, 13300, 0, Style::RightToLeft),
                    (1700, 13700, 1, Style::Top),
                    (2100, 14100, 1, Style::Bottom),
                    (2500, 14500, 0, Style::RightToLeft),
                ];
                for (record, (start, end, layer, style)) in events.iter().zip(metadata) {
                    let event = record.resolved();
                    assert_eq!(
                        (
                            event.start_ms,
                            event.end_ms,
                            event.layer,
                            event.style,
                            event.emission,
                            record.blocked()
                        ),
                        (start, end, layer, style, Emission::Dialogue, None)
                    );
                }
                for at in [0, 2, 3] {
                    assert_eq!(
                        lines[at].as_bytes(),
                        old[at].as_bytes(),
                        "unchanged long event {at}"
                    );
                }
                assert!(lines[1].ends_with(&"x".repeat(1500)));
                assert!(lines[4].ends_with(&format!("a{}", "长".repeat(400))));
                assert_eq!(
                    events[1].resolved().placement,
                    Placement::Move {
                        from_x: 25670,
                        from_y: 38,
                        to_x: -23750,
                        to_y: 38,
                        timing: None
                    }
                );
                assert_eq!(events[4].text.chars().count(), 401);
                assert_eq!(
                    events[4].resolved().placement,
                    Placement::Position { x: 960, y: 394 }
                );
                assert_eq!(events[5].text.as_ref(), "空 格 很 多 的 弹 幕 ".repeat(40));
                assert_eq!(
                    events[5].resolved().placement,
                    Placement::Move {
                        from_x: 10786,
                        from_y: 114,
                        to_x: -8866,
                        to_y: 114,
                        timing: None
                    }
                );
                assert!(lines[5].ends_with(&"空\\h格\\h很\\h多\\h的\\h弹\\h幕\\h".repeat(40)));
            }
            "unsorted.xml" => {
                let oracle = include_bytes!("../../tests/fixtures/unsorted-stable.ass");
                assert_eq!(
                    text.as_bytes(),
                    oracle,
                    "IC01/02: entire downstream layout matches C on independently stable-permuted input"
                );
                assert!(
                    events
                        .windows(2)
                        .all(|pair| pair[0].source_time_ms <= pair[1].source_time_ms)
                );
                assert!(
                    events
                        .windows(2)
                        .all(|pair| pair[0].source_time_ms != pair[1].source_time_ms
                            || pair[0].input_sequence < pair[1].input_sequence)
                );
                for event in events.iter() {
                    assert_eq!(event.text.as_ref(), format!("#{}", event.input_sequence));
                }
            }
            _ => unreachable!(),
        }
    }
}
