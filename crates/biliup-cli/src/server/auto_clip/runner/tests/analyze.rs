//! 候选生成端到端：ffmpeg 合成的 H.264 FLV（每 5 秒一个关键帧，纯音当作「说话」）、手写的弹幕 XML、
//! 假 OpenAI 兼容服务按请求构造回复（合法、越界、重复、没有证据、缺置信度、非法 JSON、拒收图片、
//! 超上下文）。没有 ffmpeg 或 libx264 的环境跳过。不调用任何真实服务。

use super::*;
use crate::server::auto_clip::analyze::ChatBasis;
use crate::server::auto_clip::candidates;
use crate::server::auto_clip::cleanup;
use crate::server::auto_clip::fake::{image_count, user_text};
use crate::server::auto_clip::probe::{self, Targets};
use crate::server::auto_clip::settings::Thumbnails;
use crate::server::auto_clip::suggestions::{self, AcceptOutcome, Acceptance, SuggestionState};
use crate::server::workbench::session_keyframes;
use serde_json::Value;

const KEYFRAME_MS: i64 = 5_000;

/// 合成 `secs` 秒的 H.264 + AAC FLV：`speech` 里的区间有纯音，其余静音。
fn synthesize(path: &Path, secs: i64, speech: &[(i64, i64)]) -> bool {
    let gate: Vec<String> = speech
        .iter()
        .map(|(from, to)| format!("between(t\\,{from}\\,{to})"))
        .collect();
    let tone = format!(
        "aevalsrc=0.4*sin(2*PI*440*t)*({}):s=44100:d={secs}",
        gate.join("+")
    );
    let output = std::process::Command::new(crate::tools::ffmpeg())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg(format!("color=c=gray:s=64x36:r=5:d={secs}"))
        .args(["-f", "lavfi", "-i"])
        .arg(&tone)
        .args([
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .args(["-g", "25", "-keyint_min", "25", "-sc_threshold", "0"])
        .args(["-c:a", "aac", "-shortest"])
        .arg(path)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!(
            "合成录像失败，跳过：{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output.status.success()
}

/// 每 10 秒一条日常弹幕，`burst_s` 起 5 秒内刷 40 条。
fn danmaku_xml(secs: i64, burst_s: i64) -> String {
    let mut body = String::new();
    let created = 1_750_000_000;
    let mut push = |at: f64, text: &str| {
        body.push_str(&format!(
            "<d p=\"{at:.3},1,25,16777215,{},0,1,0\">{text}</d>\n",
            created + at as i64
        ));
    };
    for i in 0..secs / 10 {
        push((i * 10) as f64 + 3.0, "日常");
    }
    for i in 0..40 {
        push(
            burst_s as f64 + i as f64 * 0.125,
            if i % 4 == 3 { "牛" } else { "哈哈哈" },
        );
    }
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<i>\n{body}</i>\n")
}

async fn media_env(
    scenario: Scenario,
    secs: i64,
    speech: &[(i64, i64)],
    burst_s: Option<i64>,
) -> Option<Env> {
    if !ffmpeg_available() {
        eprintln!("没有 ffmpeg，跳过");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let media = dir.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    let video = media.join("live.flv");
    if !synthesize(&video, secs, speech) {
        return None;
    }
    let pool = database(dir.path()).await;
    let server = FakeServer::start(scenario).await;
    let config = Arc::new(RwLock::new(Config {
        auto_clip: Some(auto_clip(server.base_url())),
        ..Config::default()
    }));
    let session = add_session(&pool, Some(secs * 1000 + 1000)).await;
    let segment = add_segment(&pool, session, &video, 0, secs * 1000).await;
    if let Some(burst_s) = burst_s {
        let xml = media.join("live.xml");
        std::fs::write(&xml, danmaku_xml(secs, burst_s)).unwrap();
        sqlx::query("UPDATE segments SET danmaku_path = ? WHERE id = ?")
            .bind(xml.to_string_lossy().into_owned())
            .bind(segment)
            .execute(&pool)
            .await
            .unwrap();
    }
    Some(Env {
        dir,
        pool,
        config,
        server,
        session,
        segments: vec![segment],
    })
}

impl Env {
    async fn run_job(&self) -> (Job, Outcome) {
        let job = self.enqueue(true).await;
        let (id, outcome) = self.runner().run_next().await.unwrap().unwrap();
        assert_eq!(id, job.id);
        (self.job(job.id).await, outcome)
    }

    async fn owners(&self) -> Vec<String> {
        sqlx::query_scalar("SELECT owner FROM segment_pins ORDER BY owner")
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }

    async fn probe(&self) {
        let config = self.config.read().unwrap().auto_clip.clone().unwrap();
        let report = probe::run(
            &probe::client_for(&config),
            &Targets::from_config(&config, None),
        )
        .await;
        probe::save(&self.pool, &report).await.unwrap();
    }
}

fn reply(candidates: Value) -> Result<String, (u16, String)> {
    Ok(json!({ "candidates": candidates }).to_string())
}

fn window_of(body: &Value) -> String {
    let text = user_text(body);
    text.split("本窗：")
        .nth(1)
        .and_then(|rest| rest.lines().next())
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn suggestions_come_from_checked_and_snapped_replies() {
    let Some(env) = media_env(Scenario::Ok, 240, &[(0, 60), (180, 240)], Some(120)).await else {
        return;
    };
    env.server.set_analyst(|_, _| {
        reply(json!([
            {"start": "0:01:52", "end": "0:02:25", "title": "弹幕刷屏", "reason": "「哈哈哈」×30",
             "confidence": 0.9, "tags": ["搞笑"], "evidence": ["danmaku"]},
            // 与上一条重复，置信度低
            {"start": "0:01:55", "end": "0:02:25", "title": "重复", "confidence": 0.5},
            // 没给置信度
            {"start": "0:00:12", "end": "0:00:38", "title": "开场", "reason": "主播说话", "evidence": ["asr"]},
            // 编造的时间：区间里没有语音、没有弹幕高峰、没有图
            {"start": "0:02:30", "end": "0:02:55", "title": "编的"},
            {"start": "0:03:50", "end": "0:04:30", "title": "超出录像"},
            {"start": "0:01:00", "end": "0:01:05", "title": "太短"},
            {"start": "abc", "end": "0:00:30"},
        ]))
    });
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    assert_eq!(job.stage, Some(Stage::Analyze));
    assert_eq!((job.progress_done, job.progress_total), (1, 1));
    assert!(job.tokens_in > 0 && job.tokens_out > 0);
    assert_eq!(job.images, 0, "没做连通性测试，缩图自动模式不发");
    assert_eq!(job.models.as_ref().unwrap()["chat_model"], "chat-model");
    assert_eq!(job.models.as_ref().unwrap()["thumbnails"], false);
    let rejected = job
        .warnings
        .iter()
        .find(|w| w.contains("没通过校验"))
        .unwrap_or_else(|| panic!("{:?}", job.warnings));
    assert!(rejected.contains("有 4 条"), "{rejected}");
    assert!(job.warnings.iter().any(|w| w.contains("没有发截图")));

    let requests = env.server.analysis_requests();
    assert_eq!(requests.len(), 1);
    let body = &requests[0].body;
    assert_eq!(body["response_format"]["type"], "json_object");
    assert_eq!(body["max_tokens"], 2000);
    assert_eq!(image_count(body), 0);
    let text = user_text(body);
    assert!(text.contains("本窗：0:00:00–0:04:00"), "{text}");
    assert!(text.contains("[弹幕高峰] 0:01:50–0:02:20"), "{text}");
    assert!(text.contains("「哈哈哈」×30"), "{text}");
    assert!(text.contains("[语音] 0:00:0"), "{text}");
    assert!(text.contains("每段 15 秒到 180 秒"), "{text}");

    let list = suggestions::list(&env.pool, env.session).await.unwrap();
    let ranges: Vec<(i64, i64)> = list.iter().map(|s| (s.in_ms, s.out_ms)).collect();
    assert_eq!(ranges, vec![(10_000, 40_000), (110_000, 145_000)]);
    // 吸附结果与 /keyframes 给的关键帧一致：入点是之前（含）最近的，出点是之后（含）最近的
    for (suggestion, (asked_in, asked_out)) in
        list.iter().zip([(12_000, 38_000), (112_000, 145_000)])
    {
        let before = session_keyframes(&env.pool, env.session, 0, asked_in)
            .await
            .unwrap();
        assert_eq!(before.last().unwrap().t_ms, suggestion.in_ms);
        let after = session_keyframes(&env.pool, env.session, asked_out, 240_000)
            .await
            .unwrap();
        assert_eq!(after.first().unwrap().t_ms, suggestion.out_ms);
        assert_eq!(suggestion.in_ms % KEYFRAME_MS, 0);
    }
    let opening = &list[0];
    assert_eq!(opening.title, "开场");
    assert_eq!(opening.confidence, None);
    assert!(opening.evidence.asr_lines > 0);
    assert_eq!(opening.evidence.danmaku, 0);
    assert_eq!(opening.job_id, Some(job.id));
    let burst = &list[1];
    assert_eq!(burst.confidence, Some(0.9));
    assert_eq!(burst.tags, vec!["搞笑".to_string()]);
    assert_eq!(burst.evidence.asr_lines, 0);
    assert!(burst.evidence.danmaku >= 40);
    assert!(list.iter().all(|s| s.state == SuggestionState::Pending));

    // 任务结束后不再引用整场，改由每个候选引用自己的区间
    let mut expected = vec![
        suggestions::pin_owner(opening.id),
        suggestions::pin_owner(burst.id),
    ];
    expected.sort();
    assert_eq!(env.owners().await, expected);
    let files = env.files();
    assert!(files.load_danmaku().await.unwrap().total >= 40);
    assert_eq!(files.load_analysis().await.len(), 1);
    assert!(!files.audio_dir().exists());

    // 接受 → 建切片草稿
    let AcceptOutcome::Accepted { clip, .. } = suggestions::accept(
        &env.pool,
        env.session,
        opening.id,
        &Acceptance::default(),
        now_ms(),
    )
    .await
    .unwrap() else {
        panic!("应当接受成功");
    };
    assert_eq!((clip.in_ms, clip.out_ms), (10_000, 40_000));
    assert_eq!(clip.title, "开场");

    // 72 小时没处理的过期、撤销引用（注入时钟）
    let created = burst.created_at;
    let root = env.root();
    assert_eq!(
        cleanup::sweep_in(&env.pool, &root, created + suggestions::EXPIRE_AFTER_MS - 1)
            .await
            .expired,
        0
    );
    assert_eq!(
        cleanup::sweep_in(&env.pool, &root, created + suggestions::EXPIRE_AFTER_MS)
            .await
            .expired,
        1
    );
    assert_eq!(
        env.owners().await,
        vec![crate::server::workbench::clips::pin_owner(clip.id)]
    );
    assert_eq!(
        suggestions::get(&env.pool, burst.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        SuggestionState::Expired
    );

    // 删场次：分析数据跟着删
    assert!(files.dir().exists());
    sqlx::query("DELETE FROM stream_sessions WHERE id = ?")
        .bind(env.session)
        .execute(&env.pool)
        .await
        .unwrap();
    assert_eq!(
        cleanup::sweep_in(&env.pool, &root, now_ms()).await.removed,
        vec![env.session]
    );
    assert!(!files.dir().exists());
}

fn opening(_: &Value, _: usize) -> Result<String, (u16, String)> {
    reply(json!([{"start": "0:00:12", "end": "0:00:38", "title": "开场", "confidence": 0.7}]))
}

#[tokio::test]
async fn thumbnails_are_sent_only_when_switched_on_or_probed() {
    let Some(env) = media_env(Scenario::Ok, 240, &[(0, 60), (180, 240)], Some(120)).await else {
        return;
    };
    env.server.set_analyst(opening);

    // on：总是发。高峰中心 2:05 一张（2:30 的定时采样离它不到 60 秒，去掉）
    env.set_config(|c| c.thumbnails = Some(Thumbnails::On));
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    let body = &env.server.analysis_requests()[0].body;
    assert_eq!(image_count(body), 1);
    assert!(
        user_text(body).contains("[画面] 图1 = 0:02:05"),
        "{}",
        user_text(body)
    );
    let url = body["messages"][1]["content"][1]["image_url"]["url"]
        .as_str()
        .unwrap();
    assert!(url.starts_with("data:image/jpeg;base64,"));
    assert_eq!(
        body["messages"][1]["content"][1]["image_url"]["detail"],
        "low"
    );
    assert_eq!(job.images, 1);
    assert_eq!(job.models.as_ref().unwrap()["thumbnails"], true);
    let thumb = env.files().thumb(125_000);
    let jpeg = std::fs::read(&thumb).unwrap();
    assert_eq!(&jpeg[..2], &[0xff, 0xd8], "JPEG");

    // off：不发，也不提示
    env.set_config(|c| c.thumbnails = Some(Thumbnails::Off));
    let (job, _) = env.run_job().await;
    assert_eq!(image_count(&env.server.analysis_requests()[1].body), 0);
    assert_eq!(job.images, 0);
    assert!(
        !job.warnings.iter().any(|w| w.contains("截图")),
        "{:?}",
        job.warnings
    );

    // auto：连通性测试确认能看图后才发
    env.set_config(|c| c.thumbnails = Some(Thumbnails::Auto));
    env.probe().await;
    let (job, _) = env.run_job().await;
    let requests = env.server.analysis_requests();
    assert_eq!(image_count(&requests.last().unwrap().body), 1);
    assert_eq!(job.images, 1);
}

#[tokio::test]
async fn a_model_that_cannot_see_images_degrades_to_text() {
    let Some(env) = media_env(Scenario::NoVision, 240, &[(0, 60), (180, 240)], Some(120)).await
    else {
        return;
    };
    env.server.set_analyst(opening);

    // on 但服务拒收图片：这一窗去掉图再问
    env.set_config(|c| c.thumbnails = Some(Thumbnails::On));
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    let requests = env.server.analysis_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(image_count(&requests[0].body), 1);
    assert_eq!(image_count(&requests[1].body), 0);
    assert!(!user_text(&requests[1].body).contains("[画面]"));
    assert_eq!(job.images, 0);
    assert_eq!(job.models.as_ref().unwrap()["thumbnails"], false);
    assert!(
        job.warnings.iter().any(|w| w.contains("拒收图片")),
        "{:?}",
        job.warnings
    );
    assert_eq!(
        suggestions::list(&env.pool, env.session)
            .await
            .unwrap()
            .len(),
        1
    );

    // auto：连通性测试测出不能看图，就不发并说明
    env.set_config(|c| c.thumbnails = Some(Thumbnails::Auto));
    env.probe().await;
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(image_count(&env.server.analysis_requests()[2].body), 0);
    assert!(
        job.warnings.iter().any(|w| w.contains("不能看图")),
        "{:?}",
        job.warnings
    );
}

#[tokio::test]
async fn a_session_without_danmaku_is_analysed_from_speech() {
    let Some(env) = media_env(Scenario::Ok, 240, &[(0, 60), (180, 240)], None).await else {
        return;
    };
    env.server.set_analyst(opening);
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    let text = user_text(&env.server.analysis_requests()[0].body);
    assert!(text.contains("[弹幕] 本场无弹幕记录"), "{text}");
    assert!(env.files().load_danmaku().await.is_none());
    assert_eq!(
        suggestions::list(&env.pool, env.session)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn unusable_replies_and_fatal_errors_fail_the_job() {
    let Some(env) = media_env(Scenario::Ok, 240, &[(0, 60), (180, 240)], Some(120)).await else {
        return;
    };
    env.server
        .set_analyst(|_, _| Ok("抱歉，这段直播我没法判断。".to_string()));
    let (job, outcome) = env.run_job().await;
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(error.contains("没有一窗得到可用的回复"), "{error}");
    assert!(error.contains("找不到 JSON"), "{error}");
    assert_eq!(job.state, JobState::Failed);
    assert!(
        suggestions::list(&env.pool, env.session)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(env.pins().await, 0);

    env.server
        .set_analyst(|_, _| Err((401, "Incorrect API key provided".to_string())));
    let (_, outcome) = env.run_job().await;
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        error.contains("chat 调用失败") && error.contains("API key"),
        "{error}"
    );

    // 缺 chat 模型：转写之前就停下，不花转写的钱
    let before = env.server.transcriptions().len();
    env.set_config(|c| c.chat_model = None);
    let (_, outcome) = env.run_job().await;
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(error.contains("chat 接口"), "{error}");
    assert_eq!(env.server.transcriptions().len(), before);
}

#[tokio::test]
async fn the_chat_token_limit_stops_before_calling() {
    let Some(env) = media_env(Scenario::Ok, 240, &[(0, 60), (180, 240)], Some(120)).await else {
        return;
    };
    env.server.set_analyst(opening);
    env.set_config(|c| c.max_chat_tokens = Some(500));
    let (job, outcome) = env.run_job().await;
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        error.contains("超过每场上限 500 token") && error.contains("没有调用 chat"),
        "{error}"
    );
    assert!(env.server.analysis_requests().is_empty());
    assert!(
        !env.server.transcriptions().is_empty(),
        "转写照常做完，下次沿用"
    );
    assert_eq!((job.tokens_in, job.tokens_out), (0, 0));

    let settings = env.config.read().unwrap().auto_clip.clone().unwrap();
    let estimate = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert_eq!(estimate.chat.basis, ChatBasis::Transcript);
    assert!(estimate.chat_over_limit);
    assert!(estimate.chat_message.unwrap().contains("max_chat_tokens"));

    // 调高上限后沿用转写，只做候选生成
    let sent = env.server.transcriptions().len();
    env.set_config(|c| c.max_chat_tokens = None);
    let (_, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(env.server.transcriptions().len(), sent);
    assert_eq!(env.server.analysis_requests().len(), 1);
}

#[tokio::test]
async fn cancel_stops_the_analysis() {
    let Some(env) = media_env(Scenario::Delay { millis: 3_000 }, 60, &[(0, 30)], Some(20)).await
    else {
        return;
    };
    env.server.set_analyst(opening);
    let job = env.enqueue(true).await;
    let runner = env.runner();
    let task = tokio::spawn(async move { runner.run_next().await });
    wait_until("分析请求发出", async || {
        !env.server.analysis_requests().is_empty()
    })
    .await;
    assert_eq!(env.job(job.id).await.stage, Some(Stage::Analyze));
    jobs::cancel(&env.pool, env.session, now_ms())
        .await
        .unwrap();
    let (_, outcome) = task.await.unwrap().unwrap().unwrap();
    assert_eq!(outcome, Outcome::Canceled);
    assert_eq!(env.pins().await, 0);
    assert!(
        suggestions::list(&env.pool, env.session)
            .await
            .unwrap()
            .is_empty()
    );
}

/// 34 分钟：两窗 0:00:00–0:30:00、0:28:00–0:34:00。
async fn long_env() -> Option<Env> {
    media_env(Scenario::Ok, 34 * 60, &[(0, 60), (1700, 1760)], None).await
}

#[tokio::test]
async fn long_sessions_split_resume_and_truncate() {
    let Some(env) = long_env().await else {
        return;
    };
    // 第一窗超上下文 → 对半；后半窗回的不是 JSON；最后一窗正常
    env.server.set_analyst(|body, _| match window_of(body).as_str() {
        "0:00:00–0:30:00" => Err((
            400,
            "This model's maximum context length is 8192 tokens. However, your messages resulted in 9000 tokens.".into(),
        )),
        "0:00:00–0:16:00" => reply(json!([
            {"start": "0:00:12", "end": "0:00:38", "title": "开场", "confidence": 0.7}
        ])),
        "0:14:00–0:30:00" => Ok("我不太确定。".into()),
        "0:28:00–0:34:00" => reply(json!([
            {"start": "0:28:25", "end": "0:29:05", "title": "结尾", "confidence": 0.6}
        ])),
        other => panic!("没想到的窗：{other}"),
    });
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    let windows: Vec<String> = env
        .server
        .analysis_requests()
        .iter()
        .map(|seen| window_of(&seen.body))
        .collect();
    assert_eq!(
        windows,
        vec![
            "0:00:00–0:30:00",
            "0:00:00–0:16:00",
            "0:14:00–0:30:00",
            "0:28:00–0:34:00"
        ]
    );
    assert_eq!((job.progress_done, job.progress_total), (3, 3));
    assert!(
        job.warnings.iter().any(|w| w.contains("上下文长度")),
        "{:?}",
        job.warnings
    );
    assert!(
        job.warnings
            .iter()
            .any(|w| w.contains("有 1 窗没有得到可用的回复") && w.contains("0:14:00–0:30:00")),
        "{:?}",
        job.warnings
    );
    let titles = |list: Vec<suggestions::Suggestion>| -> Vec<String> {
        list.into_iter().map(|s| s.title).collect()
    };
    assert_eq!(
        titles(suggestions::list(&env.pool, env.session).await.unwrap()),
        vec!["开场", "结尾"]
    );
    let spent = (job.tokens_in, job.tokens_out);

    // 服务重启续跑：问过的窗读缓存，不再花钱（超上下文的那次没有结果，再问一次）
    sqlx::query("UPDATE auto_clip_jobs SET state = 'queued', finished_at = NULL WHERE id = ?")
        .bind(job.id)
        .execute(&env.pool)
        .await
        .unwrap();
    let (id, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!((id, outcome), (job.id, Outcome::Done));
    assert_eq!(env.server.analysis_requests().len(), 5);
    let resumed = env.job(job.id).await;
    assert_eq!((resumed.tokens_in, resumed.tokens_out), spent);
    assert_eq!(
        titles(suggestions::list(&env.pool, env.session).await.unwrap()),
        vec!["开场", "结尾"]
    );

    // 实际用量眼看超上限：第一窗回复很长，第二窗不问了，已有的候选照常落库
    let settings = env.config.read().unwrap().auto_clip.clone().unwrap();
    let estimate = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert_eq!(estimate.chat.basis, ChatBasis::Transcript);
    assert_eq!(estimate.chat.windows, 2);
    let limit = estimate.chat.tokens as u64 + 500;
    env.set_config(|c| c.max_chat_tokens = Some(limit));
    let padding = "很长的理由".repeat(1_500);
    env.server.set_analyst(move |body, _| {
        if window_of(body).starts_with("0:00:00") {
            reply(json!([
                {"start": "0:00:12", "end": "0:00:38", "title": "新的开场", "reason": padding, "confidence": 0.8}
            ]))
        } else {
            panic!("超上限后不该再问");
        }
    });
    let (job, outcome) = env.run_job().await;
    assert_eq!(outcome, Outcome::Done, "{job:?}");
    assert_eq!(env.server.analysis_requests().len(), 6);
    assert!(
        job.warnings
            .iter()
            .any(|w| w.contains(&format!("每场上限 {limit} token")) && w.contains("剩下 1 窗")),
        "{:?}",
        job.warnings
    );
    assert!(job.tokens_in + job.tokens_out <= limit as i64 + 8_000);
    let list = suggestions::list(&env.pool, env.session).await.unwrap();
    assert_eq!(titles(list.clone()), vec!["新的开场"], "上一轮没处理的换掉");
    assert_eq!(list[0].reason.chars().count(), candidates::MAX_REASON_CHARS);
}
