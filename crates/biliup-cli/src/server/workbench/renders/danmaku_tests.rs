use super::*;

#[test]
fn validates_complete_xml_and_accepts_empty_recordings() {
    assert!(
        parse_xml(b"<?xml version=\"1.0\"?><i></i>")
            .unwrap()
            .comments
            .is_empty()
    );
    assert!(parse_xml(b"<i/>").unwrap().comments.is_empty());
    for broken in [
        "",
        "<i>",
        "<i><d p=\"1,1,25,1\">x",
        "<i><d p=\"NaN,1,25,1\">x</d></i>",
        "<i><d p=\"1,1,25,1\">&bad;</d></i>",
        "<i></i><i></i>",
        "<i></i>x",
        "<!DOCTYPE i><i/>",
    ] {
        assert!(parse_xml(broken.as_bytes()).is_err(), "accepted {broken}");
    }
}

#[test]
fn reads_precise_origin_and_estimates_old_origin_by_median() {
    let parsed = parse_xml(b"<i><recording_start_time_ms>100000</recording_start_time_ms><d p=\"1.125,1,25,16777215,101,0,0,0\">A &amp; B</d></i>").unwrap();
    assert_eq!(timing_origin(&parsed), (Some(100000), false));
    assert_eq!(parsed.comments[0].text, "A & B");
    assert_eq!(parsed.comments[0].elapsed_ms, 1125);
    let old = parse_xml(b"<i><d p=\"1,1,25,1,101\">a</d><d p=\"2,1,25,1,102\">b</d><d p=\"100,1,25,1,999\">c</d></i>").unwrap();
    assert_eq!(timing_origin(&old), (Some(100000), true));
    assert!(parse_xml(b"<i recording_start_time_ms=\"1\"><recording_start_time_ms>2</recording_start_time_ms></i>").is_err());
}
