//! Reviewed IC expectations derived from the frozen C bytes, never from Rust
//! output. Every unmentioned byte stays unchanged; independent native assertions
//! verify the affected text and layout values.
pub fn expected(input: &str, legacy: Vec<u8>) -> Vec<u8> {
    if input == "unsorted.xml" {
        return include_bytes!("../fixtures/unsorted-stable.ass").to_vec();
    }
    let mut lines: Vec<Vec<u8>> = legacy
        .split(|&byte| byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect();
    let events: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            if line.starts_with(b"Dialogue:") || line.starts_with(b"Comment:") {
                Some(at)
            } else {
                None
            }
        })
        .collect();
    let replace_body = |line: &mut Vec<u8>, body: &str| {
        let at = line
            .iter()
            .position(|&b| b == b'}')
            .expect("override close")
            + 1;
        line.truncate(at);
        line.extend_from_slice(body.as_bytes());
    };
    match input {
        "emoji.xml" => {
            for (index, body) in [
                (2, "&#128514;\\hnumeric\\hreference"),
                (3, "a&#b\\hmid-text\\hmarker"),
                (4, "&#;\\hempty\\hreference"),
                (5, "&#9999999;\\hout\\hof\\hrange"),
                (6, "&#12\\hno\\hsemicolon\\hhere"),
            ] {
                replace_body(&mut lines[events[index]], body);
            }
        }
        "special_chars.xml" => replace_body(&mut lines[events[10]], "&amp;\\halready\\hescaped"),
        "trailing_ref.xml" => {
            replace_body(&mut lines[events[0]], "ends\\hwith\\h&#");
            replace_body(&mut lines[events[1]], "half\\hemoji\\h&#55357;");
        }
        "long_text.xml" => {
            assert_eq!(events.len(), 6);
            lines[events[1]]=format!("Dialogue: 0,0:00:00.90,0:00:12.90,R2L,,0000,0000,0000,,{{\\move(25670,38,-23750,38)}}{}","x".repeat(1500)).into_bytes();
            replace_body(&mut lines[events[4]], &format!("a{}", "长".repeat(400)));
            lines[events[5]]=format!("Dialogue: 0,0:00:02.50,0:00:14.50,R2L,,0000,0000,0000,,{{\\move(10786,114,-8866,114)}}{}","空\\h格\\h很\\h多\\h的\\h弹\\h幕\\h".repeat(40)).into_bytes();
        }
        _ => {}
    }
    lines.join(&b'\n')
}
