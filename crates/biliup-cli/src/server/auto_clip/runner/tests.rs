//! 用 ffmpeg lavfi 合成的音视频（纯音 + 静音交替，没有真实录像）和假 OpenAI 兼容服务跑整个任务。
//! 没有 ffmpeg 的环境跳过需要抽音频的用例。

use super::*;
use crate::server::auto_clip::fake::{FakeServer, Scenario};
use crate::server::auto_clip::jobs::NewJob;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use std::sync::atomic::AtomicI64;

const KEY: &str = "sk-test-0123456789abcdef";

/// 场次 id 从一个大数起，免得和同一进程里别的用例登记的「正在录」的场次撞上。
static NEXT_SESSION: AtomicI64 = AtomicI64::new(7_300_001);

/// 合成的一段：纯音在每 20 秒的前 `tone_secs` 秒，其余是静音；`tone_secs = 0` 表示没有音轨。
struct Piece {
    name: &'static str,
    start_ms: i64,
    secs: i64,
    tone_secs: i64,
}

/// 四段，编码参数各不相同：FLV（AAC 44.1 kHz 立体声）→ TS（48 kHz 单声道）→ 只有画面的 FLV →
/// 断流 30 秒 → fMP4（22.05 kHz 立体声）。
const PIECES: [Piece; 4] = [
    Piece {
        name: "a.flv",
        start_ms: 0,
        secs: 120,
        tone_secs: 8,
    },
    Piece {
        name: "b.ts",
        start_ms: 120_000,
        secs: 40,
        tone_secs: 6,
    },
    Piece {
        name: "c.flv",
        start_ms: 160_000,
        secs: 10,
        tone_secs: 0,
    },
    Piece {
        name: "d.mp4",
        start_ms: 200_000,
        secs: 40,
        tone_secs: 8,
    },
];

fn ffmpeg_available() -> bool {
    std::process::Command::new(crate::tools::ffmpeg())
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn synthesize(dir: &Path, piece: &Piece) -> PathBuf {
    let path = dir.join(piece.name);
    let video = format!("color=c=black:s=64x36:r=5:d={}", piece.secs);
    let (rate, channels, vcodec, hz) = match piece.name {
        "b.ts" => ("48000", "1", "mpeg2video", 660),
        "d.mp4" => ("22050", "2", "mpeg4", 330),
        _ => ("44100", "2", "flv1", 440),
    };
    let tone = format!(
        "aevalsrc=0.4*sin(2*PI*{hz}*t)*lt(mod(t\\,20)\\,{}):s={rate}:d={}",
        piece.tone_secs, piece.secs
    );
    let mut command = std::process::Command::new(crate::tools::ffmpeg());
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
    ]);
    command.arg(&video);
    if piece.tone_secs > 0 {
        command.args(["-f", "lavfi", "-i"]).arg(&tone);
        command.args(["-c:a", "aac", "-ac", channels, "-shortest"]);
    }
    command.args(["-c:v", vcodec]);
    if piece.name.ends_with(".mp4") {
        command.args(["-movflags", "frag_keyframe+empty_moov"]);
    }
    let output = command.arg(&path).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}

struct Env {
    dir: tempfile::TempDir,
    pool: ConnectionPool,
    config: Arc<RwLock<Config>>,
    server: FakeServer,
    session: i64,
    segments: Vec<i64>,
}

impl Env {
    fn root(&self) -> PathBuf {
        self.dir.path().join(AUTO_CLIP_DIR)
    }

    fn files(&self) -> SessionFiles {
        SessionFiles::new(&self.root(), self.session)
    }

    fn runner(&self) -> Arc<Runner> {
        Runner::new(
            self.pool.clone(),
            self.config.clone(),
            RunnerOptions::builder()
                .root(self.root())
                .backoff(vec![Duration::from_millis(10)])
                .cancel_poll(Duration::from_millis(50))
                .build(),
        )
    }

    async fn enqueue(&self, reuse_transcript: bool) -> Job {
        let job = NewJob::builder()
            .session_id(self.session)
            .trigger(Trigger::Manual)
            .not_before(0)
            .reuse_transcript(reuse_transcript)
            .created_at(now_ms())
            .build();
        jobs::insert(&self.pool, &job).await.unwrap().unwrap()
    }

    async fn job(&self, id: i64) -> Job {
        jobs::get(&self.pool, id).await.unwrap().unwrap()
    }

    async fn pins(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM segment_pins")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    fn sent_ms(&self) -> i64 {
        self.server
            .requests()
            .iter()
            .filter_map(|seen| seen.audio_ms)
            .sum()
    }

    fn set_config(&self, update: impl FnOnce(&mut AutoClipConfig)) {
        let mut config = self.config.write().unwrap();
        update(config.auto_clip.as_mut().unwrap());
    }
}

fn auto_clip(base_url: &str) -> AutoClipConfig {
    AutoClipConfig::builder()
        .enabled(true)
        .base_url(base_url.into())
        .api_key(KEY.into())
        .chat_model("chat-model".into())
        .asr_model("whisper-1".into())
        .asr_timeout_secs(5)
        .build()
}

async fn database(dir: &Path) -> ConnectionPool {
    ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap()
}

async fn add_session(pool: &ConnectionPool, ended_at: Option<i64>) -> i64 {
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    sqlx::query(
        "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES (?, 'a', 'https://a', 't', '2026-09-24 00:00:00', '', 1000, ?)",
    )
    .bind(id)
    .bind(ended_at)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn add_segment(
    pool: &ConnectionPool,
    session: i64,
    path: &Path,
    start: i64,
    end: i64,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, ?, ?, 'finished', ?, ?, 0) RETURNING id",
    )
    .bind(session)
    .bind(path.to_string_lossy().into_owned())
    .bind(store::container_of(path).unwrap())
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 建好库、合成录像、起假服务；没有 ffmpeg 时返回 `None`。
async fn env(scenario: Scenario) -> Option<Env> {
    if !ffmpeg_available() {
        eprintln!("没有 ffmpeg，跳过");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let pool = database(dir.path()).await;
    let server = FakeServer::start(scenario).await;
    let config = Arc::new(RwLock::new(Config {
        auto_clip: Some(auto_clip(server.base_url())),
        ..Config::default()
    }));
    let session = add_session(&pool, Some(250_000)).await;
    let media = dir.path().join("media");
    std::fs::create_dir_all(&media).unwrap();
    let mut segments = Vec::new();
    for piece in &PIECES {
        let path = synthesize(&media, piece);
        let end = piece.start_ms + piece.secs * 1000;
        segments.push(add_segment(&pool, session, &path, piece.start_ms, end).await);
    }
    Some(Env {
        dir,
        pool,
        config,
        server,
        session,
        segments,
    })
}

/// 每句的起点都要落在某一段的纯音里（静音两边各留 0.3 秒余量，再加 AAC 的编码延迟）。
fn assert_lines_fall_on_tones(lines: &[Line]) {
    assert!(!lines.is_empty());
    let slack = audio::PAD_MS + 100;
    for line in lines {
        let hit = PIECES.iter().filter(|p| p.tone_secs > 0).any(|piece| {
            let local = line.from_ms - piece.start_ms;
            let phase = local % 20_000;
            (0..piece.secs * 1000).contains(&local)
                && (phase <= piece.tone_secs * 1000 + slack || phase >= 20_000 - slack)
        });
        assert!(hit, "句子落在静音里：{line:?}");
        assert!(line.to_ms > line.from_ms, "{line:?}");
    }
}

fn distinct_chunks(lines: &[Line]) -> Vec<String> {
    let mut chunks: Vec<String> = lines.iter().map(|l| l.chunk.clone()).collect();
    chunks.dedup();
    chunks
}

#[tokio::test]
async fn transcribes_mixed_segments_and_skips_silence() {
    let Some(env) = env(Scenario::Ok).await else {
        return;
    };
    let job = env.enqueue(true).await;
    let (id, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!((id, outcome), (job.id, Outcome::Done));

    let job = env.job(job.id).await;
    assert_eq!(job.state, JobState::Done);
    assert_eq!(job.stage, Some(Stage::Asr));
    assert_eq!((job.progress_done, job.progress_total), (3, 3));
    assert!(job.error.is_none());
    assert!(job.finished_at.is_some());
    assert_eq!(job.warnings.len(), 1, "{:?}", job.warnings);
    assert!(job.warnings[0].contains("c.flv") && job.warnings[0].contains("没有音频"));
    assert_eq!(job.models.as_ref().unwrap()["asr_model"], "whisper-1");

    let recorded_ms: i64 = PIECES.iter().map(|p| p.secs * 1000).sum();
    let sent_ms = env.sent_ms();
    let requests = env.server.requests();
    assert_eq!(requests.len(), 3, "每段一块");
    assert!(
        requests
            .iter()
            .all(|seen| seen.authorization.as_deref() == Some(&format!("Bearer {KEY}")))
    );
    // 纯音一共 8×6 + 6×2 + 8×2 = 76 秒，加上两边的余量
    assert!((76_000..=84_000).contains(&sent_ms), "送转写 {sent_ms} ms");
    assert!((job.asr_seconds - sent_ms / 1000).abs() <= 3, "{job:?}");
    // 计费秒数按块向上取整再相加，与整场一起取整最多差块数
    assert!(
        (job.asr_planned_seconds.unwrap() - job.asr_seconds).abs() <= 3,
        "{job:?}"
    );
    eprintln!(
        "静音跳过：录像 {:.2} 分钟，送转写 {:.2} 分钟（{:.0}%），{} 次请求",
        recorded_ms as f64 / 60_000.0,
        sent_ms as f64 / 60_000.0,
        sent_ms as f64 * 100.0 / recorded_ms as f64,
        requests.len()
    );

    let lines = env.files().lines().await;
    assert_lines_fall_on_tones(&lines);
    assert_eq!(distinct_chunks(&lines).len(), 3);
    // 第二段第一句（块内 0 秒）换算回场次时间就是第二段的起点
    let first_b = lines
        .iter()
        .find(|l| l.chunk.starts_with(&format!("{}-", env.segments[1])))
        .unwrap();
    assert_eq!(first_b.from_ms, PIECES[1].start_ms);
    // 断流之后的一段也对得上
    let first_d = lines
        .iter()
        .find(|l| l.chunk.starts_with(&format!("{}-", env.segments[3])))
        .unwrap();
    assert_eq!(first_d.from_ms, PIECES[3].start_ms);

    assert!(!env.files().audio_dir().exists(), "抽出来的音频转写完就删");
    assert_eq!(env.pins().await, 0, "任务结束撤销素材引用");

    // 预估这时按静音表算，已经全部转写过
    let settings = env.config.read().unwrap().auto_clip.clone().unwrap();
    let estimate = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert_eq!(estimate.basis, Basis::Silence);
    assert_eq!(estimate.recorded_seconds, recorded_ms / 1000);
    assert_eq!(estimate.asr_seconds, 0);
    assert!((estimate.transcribed_seconds - sent_ms / 1000).abs() <= 1);
    let fresh = estimate_in(&env.pool, &env.root(), &settings, env.session, false)
        .await
        .unwrap();
    assert!((fresh.asr_seconds - sent_ms / 1000).abs() <= 1, "{fresh:?}");
}

#[tokio::test]
async fn over_the_per_session_limit_nothing_is_sent() {
    let Some(env) = env(Scenario::Ok).await else {
        return;
    };
    env.set_config(|c| c.max_asr_minutes = Some(1));
    let settings = env.config.read().unwrap().auto_clip.clone().unwrap();
    let before = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert_eq!(before.basis, Basis::Duration);
    assert!(before.over_limit);
    assert_eq!(before.asr_seconds, 210, "没抽过音频时按有效分段的时长估");

    let job = env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(error.contains("超过每场上限 1 分钟"), "{error}");
    assert!(error.contains("没有调用转写"), "{error}");
    assert!(env.server.requests().is_empty(), "超上限不调用转写");
    let job = env.job(job.id).await;
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.error.as_deref(), Some(error.as_str()));
    assert_eq!(job.asr_seconds, 0);
    assert!(job.asr_planned_seconds.unwrap() > 60);

    let after = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert_eq!(after.basis, Basis::Silence);
    assert!(after.over_limit);
    assert_eq!(Some(after.asr_seconds), job.asr_planned_seconds);
    assert!(after.message.unwrap().contains("max_asr_minutes"));
}

#[tokio::test]
async fn a_rejected_key_fails_at_once_and_is_masked() {
    let Some(env) = env(Scenario::Unauthorized).await else {
        return;
    };
    let job = env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(env.server.requests().len(), 1, "401 不重试");
    assert!(!error.contains(KEY), "{error}");
    let job = env.job(job.id).await;
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.asr_seconds, 0);
    assert!(env.files().lines().await.is_empty());
    assert_eq!(env.pins().await, 0);
}

#[tokio::test]
async fn rate_limits_are_waited_out() {
    let Some(env) = env(Scenario::RateLimitedOnce { retry_after: 1 }).await else {
        return;
    };
    env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(
        env.server.requests().len(),
        4,
        "第一次 429，重试后三块都成功"
    );
    assert_eq!(distinct_chunks(&env.files().lines().await).len(), 3);
}

#[tokio::test]
async fn timeouts_fail_the_job_after_retrying() {
    let Some(env) = env(Scenario::Slow { secs: 30 }).await else {
        return;
    };
    env.set_config(|c| c.asr_timeout_secs = Some(1));
    let job = env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    let Outcome::Failed(error) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(error.contains("转写失败"), "{error}");
    assert_eq!(env.server.requests().len(), 2, "首次 + 重试一次");
    assert_eq!(env.job(job.id).await.state, JobState::Failed);
}

#[tokio::test]
async fn a_failed_job_keeps_finished_chunks_for_the_next_run() {
    let Some(env) = env(Scenario::FailAfter { ok: 1 }).await else {
        return;
    };
    let first = env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    // 第一块成功，第二块首次 + 重试一次都是 500
    assert_eq!(env.server.requests().len(), 3);
    let failed = env.job(first.id).await;
    assert_eq!(failed.progress_done, 1);
    let kept = env.files().lines().await;
    assert_eq!(distinct_chunks(&kept).len(), 1);
    let first_chunk_ms = env.server.requests()[0].audio_ms.unwrap();

    // 服务恢复后再跑一次：沿用已转写的块，只送剩下的
    let server = FakeServer::start(Scenario::Ok).await;
    env.set_config(|c| c.base_url = Some(server.base_url().into()));
    let settings = env.config.read().unwrap().auto_clip.clone().unwrap();
    let estimate = estimate_in(&env.pool, &env.root(), &settings, env.session, true)
        .await
        .unwrap();
    assert!((estimate.transcribed_seconds - first_chunk_ms / 1000).abs() <= 1);

    let second = env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(server.requests().len(), 2);
    let lines = env.files().lines().await;
    assert_eq!(distinct_chunks(&lines).len(), 3);
    assert_eq!(&lines[..kept.len()], &kept[..]);
    let second = env.job(second.id).await;
    assert!(
        (second.asr_seconds - estimate.asr_seconds).abs() <= 2,
        "只记这次送的：{second:?} {estimate:?}"
    );

    // 不沿用：从头转写三块
    env.enqueue(false).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(server.requests().len(), 5);
    assert_eq!(env.files().lines().await.len(), lines.len());
}

#[tokio::test]
async fn a_new_segment_only_sends_the_new_audio() {
    let Some(env) = env(Scenario::Ok).await else {
        return;
    };
    env.enqueue(true).await;
    assert_eq!(
        env.runner().run_next().await.unwrap().unwrap().1,
        Outcome::Done
    );
    let before = env.files().lines().await;
    // 这一场接着录了一段（复用第二段的文件），再生成一次
    let extra = add_segment(
        &env.pool,
        env.session,
        &env.dir.path().join("media").join("b.ts"),
        300_000,
        340_000,
    )
    .await;
    env.enqueue(true).await;
    assert_eq!(
        env.runner().run_next().await.unwrap().unwrap().1,
        Outcome::Done
    );
    let requests = env.server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[3].audio_ms, requests[1].audio_ms);
    let lines = env.files().lines().await;
    assert_eq!(&lines[..before.len()], &before[..]);
    assert!(
        lines[before.len()..]
            .iter()
            .all(|l| l.chunk.starts_with(&format!("{extra}-")) && l.from_ms >= 300_000)
    );
}

#[tokio::test]
async fn an_oversized_upload_is_split_in_halves() {
    let Some(env) = env(Scenario::TooLarge).await else {
        return;
    };
    env.enqueue(true).await;
    let (_, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    let sizes: Vec<i64> = env
        .server
        .requests()
        .iter()
        .map(|seen| seen.audio_ms.unwrap())
        .collect();
    assert_eq!(sizes.len(), 1 + MAX_SPLITS as usize, "{sizes:?}");
    assert!(sizes.windows(2).all(|w| w[1] < w[0]), "{sizes:?}");
}

async fn wait_until(what: &str, mut check: impl AsyncFnMut() -> bool) {
    for _ in 0..400 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("等不到：{what}");
}

#[tokio::test]
async fn cancel_stops_a_running_job() {
    let Some(env) = env(Scenario::Delay { millis: 5_000 }).await else {
        return;
    };
    let job = env.enqueue(true).await;
    let runner = env.runner();
    let task = tokio::spawn(async move { runner.run_next().await });
    wait_until("转写请求发出", async || {
        !env.server.requests().is_empty()
    })
    .await;
    assert_eq!(env.pins().await, 1, "运行中引用整场素材");

    let canceled = jobs::cancel(&env.pool, env.session, now_ms())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(canceled.state, JobState::Canceled);
    let started = std::time::Instant::now();
    let (_, outcome) = task.await.unwrap().unwrap().unwrap();
    assert_eq!(outcome, Outcome::Canceled);
    assert!(started.elapsed() < Duration::from_secs(2), "不等在途的请求");

    let job = env.job(job.id).await;
    assert_eq!(job.state, JobState::Canceled);
    assert!(job.finished_at.is_some());
    assert!(!env.files().audio_dir().exists());
    assert_eq!(env.pins().await, 0);
    assert!(
        jobs::cancel(&env.pool, env.session, now_ms())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_restart_resumes_without_resending_finished_chunks() {
    let Some(env) = env(Scenario::Delay { millis: 400 }).await else {
        return;
    };
    let job = env.enqueue(true).await;
    let runner = env.runner();
    let task = tokio::spawn(async move { runner.run_next().await });
    let files = env.files();
    wait_until("第一块转写完", async || {
        !files.done_chunks().await.is_empty()
    })
    .await;
    // 进程在第二块的请求途中被杀
    task.abort();
    let _ = task.await;
    let sent_before = env.server.requests().len();
    assert_eq!(env.job(job.id).await.state, JobState::Running);

    assert_eq!(jobs::recover(&env.pool).await.unwrap(), 1);
    let resumed = env.job(job.id).await;
    assert_eq!(resumed.state, JobState::Queued);
    assert_eq!(resumed.stage, Some(Stage::Asr));
    let (id, outcome) = env.runner().run_next().await.unwrap().unwrap();
    assert_eq!((id, outcome), (job.id, Outcome::Done));

    let requests = env.server.requests();
    let resent = requests.len() - sent_before;
    // 断在第二块途中：续跑送第二、三块，第一块不重传
    assert_eq!(resent, 2, "{} 次请求，断之前 {sent_before}", requests.len());
    let lines = env.files().lines().await;
    assert_eq!(distinct_chunks(&lines).len(), 3);
    let mut keys = distinct_chunks(&lines);
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 3, "没有重复的块");
    assert_lines_fall_on_tones(&lines);
    let job = env.job(job.id).await;
    assert_eq!(job.state, JobState::Done);
    assert_eq!(job.progress_done, 3);
    assert_eq!(env.pins().await, 0);
}

#[tokio::test]
async fn auto_jobs_wait_for_the_merge_window_and_drop_when_recording_resumes() {
    let dir = tempfile::tempdir().unwrap();
    let pool = database(dir.path()).await;
    let session = add_session(&pool, Some(5_000)).await;
    let now = now_ms();
    let job = schedule_after_live(&pool, 10, session, now)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.trigger, Trigger::Auto);
    assert_eq!(job.not_before, now + 11 * 60_000);
    // 又一次下播：推迟同一条，不再加一条
    let later = schedule_after_live(&pool, 10, session, now + 60_000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(later.id, job.id);
    assert_eq!(later.not_before, now + 12 * 60_000);

    let config = Arc::new(RwLock::new(Config {
        auto_clip: Some(auto_clip("http://127.0.0.1:9/v1")),
        ..Config::default()
    }));
    let runner = Runner::new(
        pool.clone(),
        config,
        RunnerOptions::builder().root(dir.path().join("ac")).build(),
    );
    assert_eq!(runner.run_next().await.unwrap(), None, "没到点不跑");

    sqlx::query("UPDATE auto_clip_jobs SET not_before = 0")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stream_sessions SET ended_at = NULL WHERE id = ?")
        .bind(session)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        runner.run_next().await.unwrap(),
        Some((job.id, Outcome::Dropped))
    );
    assert!(jobs::get(&pool, job.id).await.unwrap().is_none());
    assert!(!dir.path().join("ac").exists());
}

#[tokio::test]
async fn a_manual_job_on_a_live_session_fails_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let pool = database(dir.path()).await;
    let session = add_session(&pool, None).await;
    let job = NewJob::builder()
        .session_id(session)
        .trigger(Trigger::Manual)
        .not_before(0)
        .created_at(now_ms())
        .build();
    let job = jobs::insert(&pool, &job).await.unwrap().unwrap();
    assert!(
        jobs::insert(
            &pool,
            &NewJob::builder()
                .session_id(session)
                .trigger(Trigger::Manual)
                .not_before(0)
                .created_at(0)
                .build()
        )
        .await
        .unwrap()
        .is_none(),
        "一场同时只有一条排队或运行中的任务"
    );
    let config = Arc::new(RwLock::new(Config {
        auto_clip: Some(auto_clip("http://127.0.0.1:9/v1")),
        ..Config::default()
    }));
    let runner = Runner::new(pool.clone(), config, RunnerOptions::default());
    assert_eq!(
        runner.run_next().await.unwrap(),
        Some((job.id, Outcome::Failed("这一场还在录，下播后再生成".into())))
    );
}

fn streamer(override_cfg: serde_json::Value) -> LiveStreamer {
    serde_json::from_value(json!({
        "id": 1,
        "url": "https://live.example/1",
        "remark": "t",
        "override": override_cfg,
    }))
    .unwrap()
}

#[test]
fn only_the_streamer_override_opts_in() {
    assert!(streamer_opted_in(&streamer(
        json!({"auto_clip_after_live": true})
    )));
    assert!(!streamer_opted_in(&streamer(
        json!({"auto_clip_after_live": false})
    )));
    assert!(!streamer_opted_in(&streamer(json!({}))));
    assert!(!streamer_opted_in(&streamer(serde_json::Value::Null)));
}

#[tokio::test]
async fn without_auto_clip_nothing_is_scheduled_or_started() {
    let dir = tempfile::tempdir().unwrap();
    let pool = database(dir.path()).await;
    let session = add_session(&pool, Some(5_000)).await;
    let opted_in = streamer(json!({"auto_clip_after_live": true}));
    let count = async || -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM auto_clip_jobs")
            .fetch_one(&pool)
            .await
            .unwrap()
    };

    // 没配置、配置了没打开、打开了但主播没开（全局配置里写了也不算）：下播都不排任务
    let global_flag: Config =
        serde_json::from_value(json!({"auto_clip_after_live": true})).unwrap();
    let disabled = Config {
        auto_clip: Some(AutoClipConfig::default()),
        ..Config::default()
    };
    let enabled_config = Config {
        auto_clip: Some(auto_clip("http://127.0.0.1:9/v1")),
        ..global_flag.clone()
    };
    session_finished(&pool, &Config::default(), &opted_in, session);
    session_finished(&pool, &global_flag, &opted_in, session);
    session_finished(&pool, &disabled, &opted_in, session);
    session_finished(&pool, &enabled_config, &streamer(json!({})), session);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(count().await, 0);

    // 调度器：没配置时不起调度任务，库里运行中的任务也不动
    let job = NewJob::builder()
        .session_id(session)
        .trigger(Trigger::Manual)
        .not_before(0)
        .created_at(0)
        .build();
    let job = jobs::insert(&pool, &job).await.unwrap().unwrap();
    jobs::claim(&pool, job.id, 1).await.unwrap().unwrap();
    let state = async || jobs::get(&pool, job.id).await.unwrap().unwrap().state;
    let runner_with = |config: Config| {
        Runner::new(
            pool.clone(),
            Arc::new(RwLock::new(config)),
            RunnerOptions::builder().root(dir.path().join("ac")).build(),
        )
    };
    let runner = runner_with(Config::default());
    runner.recover().await;
    runner.poke();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!runner.is_started());
    assert_eq!(state().await, JobState::Running, "没配置时不收尾上次的任务");

    // 配置了没打开：上次没跑完的放回排队，但不起调度任务、不跑
    let runner = runner_with(disabled);
    runner.recover().await;
    runner.poke();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!runner.is_started());
    assert_eq!(state().await, JobState::Queued);
    assert!(!dir.path().join("ac").exists());

    // 打开了、主播也开了：下播后排上自动任务
    let other = add_session(&pool, Some(5_000)).await;
    session_finished(&pool, &enabled_config, &opted_in, other);
    wait_until("排上自动任务", async || {
        jobs::latest(&pool, other)
            .await
            .unwrap()
            .is_some_and(|job| job.trigger == Trigger::Auto && job.state == JobState::Queued)
    })
    .await;
}
