use super::*;
use chrono::{DateTime, Utc};

pub(crate) fn template() -> UploadStreamer {
    UploadStreamer {
        id: 1,
        template_name: "默认".into(),
        title: Some("{streamer} %Y-%m-%d 录播".into()),
        tid: Some(171),
        tid_v2: None,
        copyright: Some(1),
        copyright_source: None,
        cover_path: None,
        description: Some("{streamer} 的直播：{url}".into()),
        dynamic: None,
        dtime: None,
        dolby: None,
        hires: None,
        charging_pay: None,
        no_reprint: None,
        is_only_self: None,
        uploader: None,
        user_cookie: Some("cookies.json".into()),
        tags: vec!["直播".into(), "切片".into()],
        credits: None,
        up_selection_reply: None,
        up_close_reply: None,
        up_close_danmu: None,
        extra_fields: Some(r#"{"copyright":1,"source":"x","watermark":{"state":1}}"#.into()),
    }
}

fn info() -> StreamerInfo {
    StreamerInfo {
        id: 1,
        name: "主播%d".into(),
        url: "https://live.bilibili.com/123".into(),
        title: "100% 通关".into(),
        date: DateTime::<Utc>::from_timestamp(1_790_000_000, 0).unwrap(),
        live_cover_path: String::new(),
    }
}

fn clip(title: &str, at_ms: i64) -> ClipVars {
    ClipVars {
        id: 12,
        title: title.into(),
        at_ms,
    }
}

fn archive(template: UploadStreamer, over: StudioOverride, parts: Vec<ClipVars>) -> Archive {
    Archive {
        template,
        over,
        info: info(),
        parts,
        cover: None,
    }
}

fn local(ms: i64, format: &str) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .unwrap()
        .format(format)
        .to_string()
}

#[test]
fn variables_are_filled_and_percent_signs_in_values_are_not_formats() {
    let at = 1_790_000_600_000;
    let text = render(
        "{clip_title}|{clip_time}|{streamer}|{title}|%Y",
        &info(),
        &clip("50%off", at),
    );
    let year = info().date.with_timezone(&Local).format("%Y").to_string();
    assert_eq!(
        text,
        format!(
            "50%off|{}|主播%d|100% 通关|{year}",
            local(at, "%Y-%m-%d %H:%M")
        )
    );
    // 模板本身写错了 `%` 格式：原样保留，不 panic
    assert_eq!(
        render("{clip_title} 100%", &info(), &clip("a", at)),
        "a 100%"
    );
}

#[test]
fn whole_stream_titles_are_not_reused_for_clips() {
    let t = template();
    let over = StudioOverride::default();
    // 模板标题没有切片变量：同一场的切片会同名，改用切片标题
    assert_eq!(title_template(&t, &over, "名场面", false), "{clip_title}");
    assert_eq!(
        title_template(&t, &over, "  ", false),
        "{streamer} {clip_time}"
    );
    assert_eq!(
        title_template(&t, &over, "名场面", true),
        "{streamer} 切片合集 {clip_time}"
    );
    let mut with_vars = template();
    with_vars.title = Some("【{streamer}】{clip_title}".into());
    assert_eq!(
        title_template(&with_vars, &over, "名场面", false),
        "【{streamer}】{clip_title}"
    );
    let over = StudioOverride {
        title: Some("自己写的".into()),
        ..Default::default()
    };
    assert_eq!(
        title_template(&with_vars, &over, "名场面", true),
        "自己写的"
    );
}

#[test]
fn untitled_clips_are_called_by_their_number_in_titles() {
    let mut t = template();
    t.title = Some("【{streamer}】{clip_title}".into());
    let over = StudioOverride::default();
    let a = archive(t.clone(), over.clone(), vec![clip(" ", 0)]);
    assert_eq!(a.render().title, "【主播%d】切片 #12");
    t.title = Some("{clip_title}".into());
    let a = archive(t, over, vec![clip("", 0)]);
    assert_eq!(a.render().title, "切片 #12");
    assert_eq!(a.problem(), None, "标题不再是空的");
    // 起了名的照旧
    assert_eq!(
        render("{clip_title}", &info(), &clip("名场面", 0)),
        "名场面"
    );
}

#[test]
fn clips_are_always_reprints_with_the_live_room_as_source() {
    // 模板选了自制、extra_fields 里也写了 copyright：切片仍按转载投
    let a = archive(
        template(),
        StudioOverride::default(),
        vec![clip("名场面", 0)],
    );
    let rendered = a.render();
    assert_eq!(rendered.copyright, 2);
    assert_eq!(rendered.source, "https://live.bilibili.com/123");
    assert!(rendered.template_self_made);

    let studio = a.studio(vec![]);
    assert_eq!(studio.copyright, 2);
    assert_eq!(studio.source, "https://live.bilibili.com/123");
    let json = serde_json::to_value(&studio).unwrap();
    assert_eq!(json["copyright"], 2);
    assert_eq!(json["source"], "https://live.bilibili.com/123");
    assert_eq!(json["watermark"]["state"], 1, "其他自定义参数保留");
    assert_eq!(
        serde_json::to_string(&studio)
            .unwrap()
            .matches("\"copyright\"")
            .count(),
        1
    );

    // 模板填了来源就用模板的
    let mut t = template();
    t.copyright_source = Some("  https://b23.tv/x ".into());
    let studio = archive(t, StudioOverride::default(), vec![clip("a", 0)]).studio(vec![]);
    assert_eq!(studio.source, "https://b23.tv/x");
    assert_eq!(studio.copyright, 2);
}

#[test]
fn overrides_replace_template_fields() {
    let over = StudioOverride {
        title: Some("{clip_title}（{streamer}）".into()),
        desc: Some("来自 {url}".into()),
        tags: Some(vec![" 游戏 ".into(), "".into()]),
        tid: Some(17),
        tid_v2: Some(2001),
        dtime: Some(1_900_000_000),
        cover: None,
    };
    let a = archive(template(), over, vec![clip("名场面", 0)]);
    let r = a.render();
    assert_eq!(r.title, "名场面（主播%d）");
    assert_eq!(r.desc, "来自 https://live.bilibili.com/123");
    assert_eq!(r.tags, vec!["游戏"]);
    assert_eq!((r.tid, r.tid_v2), (Some(17), Some(2001)));
    assert_eq!(r.dtime, Some(1_900_000_000));
    let studio = a.studio(vec![]);
    assert_eq!(studio.title, "名场面（主播%d）");
    assert_eq!(studio.desc, "来自 https://live.bilibili.com/123");
    assert_eq!(studio.tag, "游戏");
    assert_eq!(studio.tid, 17);
    assert_eq!(studio.dtime, Some(1_900_000_000));

    // 模板里有定时，工作台选「立即发布」
    let mut t = template();
    t.dtime = Some(7200);
    let over = StudioOverride {
        dtime: Some(0),
        ..Default::default()
    };
    assert_eq!(
        archive(t.clone(), over, vec![clip("a", 0)]).render().dtime,
        None
    );
    assert!(
        archive(t, StudioOverride::default(), vec![clip("a", 0)])
            .render()
            .dtime
            .is_some()
    );
}

#[test]
fn credits_in_the_clip_description_are_expanded_like_whole_stream_uploads() {
    let mut t = template();
    t.credits = Some(serde_json::json!([{ "username": "@剪辑君", "uid": "42" }]));
    let over = StudioOverride {
        desc: Some("剪辑：@credit 来自 {url}".into()),
        ..Default::default()
    };
    let a = archive(t.clone(), over, vec![clip("名场面", 0)]);
    let expected = "剪辑：@剪辑君   来自 https://live.bilibili.com/123";
    assert_eq!(a.render().desc, expected);
    let studio = a.studio(vec![]);
    assert_eq!(studio.desc, expected);
    let nodes = studio.desc_v2.expect("有 @credit 时带 desc_v2");
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[1].type_id, 2);
    assert_eq!(nodes[1].raw_text, "剪辑君");
    assert_eq!(nodes[1].biz_id.as_deref(), Some("42"));

    // 简介里没有占位符：原样提交，不带 desc_v2
    let studio = archive(t, StudioOverride::default(), vec![clip("a", 0)]).studio(vec![]);
    assert_eq!(studio.desc, "主播%d 的直播：https://live.bilibili.com/123");
    assert!(studio.desc_v2.is_none());
}

#[test]
fn multi_part_archives_name_each_part() {
    let at = 1_790_000_600_000;
    let a = archive(
        template(),
        StudioOverride::default(),
        vec![clip("开场", at), clip("", at + 60_000)],
    );
    let r = a.render();
    assert_eq!(
        r.title,
        format!("主播%d 切片合集 {}", local(at, "%Y-%m-%d %H:%M"))
    );
    assert_eq!(
        r.part_titles,
        vec![
            "开场".to_string(),
            format!("P2 {}", local(at + 60_000, "%Y-%m-%d %H:%M"))
        ]
    );
}

#[test]
fn problems_are_reported_before_uploading() {
    let mut t = template();
    t.tags = vec![];
    let a = archive(t, StudioOverride::default(), vec![clip("a", 0)]);
    assert!(a.problem().unwrap().contains("至少一个标签"));
    let over = StudioOverride {
        tags: Some(vec!["x".into()]),
        ..Default::default()
    };
    let mut t = template();
    t.uploader = Some("Noop".into());
    assert!(
        archive(t, over, vec![clip("a", 0)])
            .problem()
            .unwrap()
            .contains("Noop")
    );
    let long = "长".repeat(100);
    let a = archive(template(), StudioOverride::default(), vec![clip(&long, 0)]);
    assert_eq!(a.render().title.chars().count(), MAX_TITLE_CHARS);
    assert_eq!(a.problem(), None);
}

#[test]
fn scheduled_publishing_must_be_4_hours_to_15_days_ahead() {
    let now = 1_800_000_000_u64;
    let hour = 3600_u32;
    let at = |secs: u32| now as u32 + secs;
    assert!(
        schedule_problem(at(4 * hour - 60), now)
            .unwrap()
            .contains("4 小时之后")
    );
    assert_eq!(schedule_problem(at(4 * hour), now), None);
    assert_eq!(schedule_problem(at(15 * 24 * hour), now), None);
    assert!(
        schedule_problem(at(15 * 24 * hour + 60), now)
            .unwrap()
            .contains("15 天之内")
    );
    assert!(
        schedule_problem(now as u32 - 60, now).is_some(),
        "已经过去的时间"
    );

    let past = StudioOverride {
        dtime: Some(1_000_000_000),
        ..Default::default()
    };
    let a = archive(template(), past, vec![clip("a", 0)]);
    assert!(a.problem().unwrap().contains("4 小时之后"));
    let now_publish = StudioOverride {
        dtime: Some(0),
        ..Default::default()
    };
    assert_eq!(
        archive(template(), now_publish, vec![clip("a", 0)]).problem(),
        None
    );
}

#[test]
fn override_json_round_trips_and_is_validated() {
    assert_eq!(StudioOverride::default().to_json(), None);
    let over = StudioOverride {
        cover: Some(Cover::Frame { t_ms: 1500 }),
        tags: Some(vec!["a".into()]),
        ..Default::default()
    };
    let json = over.to_json().unwrap();
    assert_eq!(
        json,
        r#"{"tags":["a"],"cover":{"source":"frame","t_ms":1500}}"#
    );
    assert_eq!(StudioOverride::parse(Some(&json)), over);
    assert_eq!(
        StudioOverride::parse(Some("坏的")),
        StudioOverride::default()
    );
    assert!(serde_json::from_str::<StudioOverride>(r#"{"copyright":1}"#).is_err());

    let too_many = StudioOverride {
        tags: Some((0..13).map(|i| i.to_string()).collect()),
        ..Default::default()
    };
    assert!(too_many.validate().is_err());
    let comma = StudioOverride {
        tags: Some(vec!["a,b".into()]),
        ..Default::default()
    };
    assert!(comma.validate().is_err());
    let newline = StudioOverride {
        title: Some("a\nb".into()),
        ..Default::default()
    };
    assert!(newline.validate().is_err());
}

#[test]
fn cover_files_survive_re_exports() {
    let dir = Path::new("/x/clips/3");
    assert_eq!(cover_file(dir, 12), dir.join("12-cover.jpg"));
    assert!(!cover_file(dir, 12).to_string_lossy().contains("/12."));
}
