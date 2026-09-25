use super::*;
use crate::server::workbench::clips::tests::flv_session;
use crate::server::workbench::clips::{NewClip, insert};
use std::collections::{BTreeSet, VecDeque};
use tempfile::TempDir;

/// 假的 B 站：按脚本返回失败，记下上传了什么、投了什么。
#[derive(Default)]
struct Fake {
    uploads: Mutex<Vec<PathBuf>>,
    covers: Mutex<Vec<PathBuf>>,
    submitted: Mutex<Vec<Studio>>,
    /// 下一次上传 / 投稿返回的失败（先进先出）。
    upload_failures: Mutex<VecDeque<Failure>>,
    submit_failures: Mutex<VecDeque<Failure>>,
    delay: Duration,
}

struct FakeConnection(Arc<Fake>);

#[async_trait]
impl Bilibili for Arc<Fake> {
    async fn connect(&self, template: &UploadStreamer) -> Result<Box<dyn Connection>, Failure> {
        assert_eq!(template.template_name, "切片模板");
        Ok(Box::new(FakeConnection(self.clone())))
    }
}

#[async_trait]
impl Connection for FakeConnection {
    async fn upload(
        &self,
        path: &Path,
        progress: &(dyn Fn(usize) + Send + Sync),
    ) -> Result<Video, Failure> {
        let len = std::fs::metadata(path).unwrap().len() as usize;
        progress(len / 2);
        tokio::time::sleep(self.0.delay).await;
        if let Some(failure) = lock(&self.0.upload_failures).pop_front() {
            return Err(failure);
        }
        progress(len - len / 2);
        lock(&self.0.uploads).push(path.to_path_buf());
        let n = lock(&self.0.uploads).len();
        Ok(Video::new(&format!("n{n}")))
    }

    async fn cover(&self, path: &Path) -> Result<String, Failure> {
        lock(&self.0.covers).push(path.to_path_buf());
        Ok("https://i0.hdslb.com/cover.jpg".into())
    }

    async fn submit(&self, studio: &Studio) -> Result<Submitted, Failure> {
        tokio::time::sleep(self.0.delay).await;
        if let Some(failure) = lock(&self.0.submit_failures).pop_front() {
            return Err(failure);
        }
        lock(&self.0.submitted)
            .push(serde_json::from_value(serde_json::to_value(studio).unwrap()).unwrap());
        let n = lock(&self.0.submitted).len();
        Ok(Submitted {
            bvid: format!("BV1fake{n}"),
        })
    }
}

struct Env {
    _dir: TempDir,
    pool: ConnectionPool,
    session: i64,
    fake: Arc<Fake>,
    publisher: Arc<ClipPublisher>,
}

/// 录完的一场（三段 FLV），主播绑定了一个选了「自制」的模板。
async fn env() -> Env {
    let (dir, pool, session, _) = flv_session().await;
    let template: i64 = sqlx::query_scalar(
        r#"INSERT INTO uploadstreamers (template_name, title, tid, copyright, tags, user_cookie, extra_fields)
           VALUES ('切片模板', '{streamer} 录播', 171, 1, '["直播"]', 'cookies.json', '{"copyright":1}')
           RETURNING id"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let streamer: i64 = sqlx::query_scalar(
        "INSERT INTO livestreamers (url, remark, upload_streamers_id) VALUES ('https://a', 'a', ?)
         RETURNING id",
    )
    .bind(template)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE stream_sessions SET streamer_id = ? WHERE id = ?")
        .bind(streamer)
        .bind(session)
        .execute(&pool)
        .await
        .unwrap();
    let exports = Arc::new(ClipExports::new(pool.clone(), dir.path().join("clips")));
    let fake = Arc::new(Fake {
        delay: Duration::from_millis(30),
        ..Default::default()
    });
    let publisher = Arc::new(ClipPublisher::new(
        pool.clone(),
        exports,
        Arc::new(fake.clone()),
    ));
    publisher.spawn();
    Env {
        _dir: dir,
        pool,
        session,
        fake,
        publisher,
    }
}

impl Env {
    async fn clip(&self, in_ms: i64, out_ms: i64, title: &str) -> Clip {
        let new = NewClip {
            marker_id: None,
            in_ms,
            out_ms,
            title: title.into(),
            created_by: None,
            created_at: 1,
        };
        insert(&self.pool, self.session, &new).await.unwrap()
    }

    fn enqueue(&self, clips: &[Clip]) -> JobView {
        self.publisher
            .enqueue(
                self.session,
                clips,
                Settings::default(),
                Mode::Quick,
                Some(7),
            )
            .unwrap()
    }

    /// 等任务停在 `state`，顺便记下经过了哪些步骤。
    async fn wait(&self, id: u64, state: JobState) -> (JobView, BTreeSet<String>) {
        let mut seen = BTreeSet::new();
        for _ in 0..3000 {
            let job = self.publisher.job(id).expect("任务不见了");
            if let Some(step) = job.step {
                seen.insert(format!("{step:?}"));
            }
            if job.state == state {
                return (job, seen);
            }
            assert!(
                !(matches!(job.state, JobState::Failed | JobState::Done) && job.state != state),
                "任务停在 {:?}：{:?}",
                job.state,
                job.error
            );
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        panic!("任务没有到 {state:?}：{:?}", self.publisher.job(id));
    }

    async fn pins(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM segment_pins")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn one_clip_is_exported_uploaded_and_submitted_as_a_reprint() {
    let env = env().await;
    let clip = env.clip(1000, 2000, "名场面").await;
    assert_eq!(clip.state, ClipState::Draft);
    assert!(env.pins().await > 0);

    let job = env.enqueue(std::slice::from_ref(&clip));
    assert_eq!(job.state, JobState::Queued);
    assert_eq!(job.created_by, Some(7));
    let (done, steps) = env.wait(job.id, JobState::Done).await;
    assert_eq!(
        steps,
        ["Export", "Upload", "Submit"].map(String::from).into()
    );
    assert_eq!(done.bvid.as_deref(), Some("BV1fake1"));
    assert_eq!(done.title.as_deref(), Some("名场面"));
    assert_eq!(done.uploaded, 1);
    assert_eq!(done.error, None);

    let studio = lock(&env.fake.submitted).remove(0);
    assert_eq!(studio.copyright, 2, "模板选了自制，切片仍按转载投");
    assert_eq!(studio.source, "https://a", "来源是直播间地址");
    assert!(
        !studio
            .extra_fields
            .as_ref()
            .is_some_and(|e| e.contains_key("copyright"))
    );
    assert_eq!(studio.title, "名场面");
    assert_eq!(studio.tag, "直播");
    assert_eq!(studio.tid, 171);
    assert_eq!(studio.videos.len(), 1);
    assert_eq!(studio.videos[0].title.as_deref(), Some("名场面"));
    assert!(lock(&env.fake.covers).is_empty(), "模板没有封面");

    let clip = clips::get(&env.pool, clip.id).await.unwrap().unwrap();
    assert_eq!(clip.state, ClipState::Published);
    assert_eq!(clip.archive_bvid.as_deref(), Some("BV1fake1"));
    assert!(clip.published_at.is_some());
    assert!(clip.output_path.is_some(), "产物保留");
    assert_eq!(env.pins().await, 0, "发布后撤销对源录像的引用");

    // 已发布的不能再排
    assert!(matches!(
        env.publisher
            .enqueue(env.session, &[clip], Settings::default(), Mode::Quick, None),
        Err(EnqueueError::Conflict(_))
    ));
}

#[tokio::test]
async fn rate_limits_pause_the_whole_queue_until_resumed() {
    let env = env().await;
    let a = env.clip(1000, 2000, "a").await;
    let b = env.clip(3000, 4000, "b").await;
    lock(&env.fake.upload_failures).push_back(Failure::RateLimited(format!(
        "{RATE_LIMITED}（upload too frequently）"
    )));
    let first = env.enqueue(std::slice::from_ref(&a));
    let (paused, _) = env.wait(first.id, JobState::Paused).await;
    assert!(paused.error.as_deref().unwrap().starts_with(RATE_LIMITED));
    let view = env.publisher.view(Some(env.session));
    assert!(view.paused.as_deref().unwrap().starts_with(RATE_LIMITED));

    // 暂停时新任务只排队，不开始，也不自动重试
    let second = env.enqueue(std::slice::from_ref(&b));
    assert_eq!(second.detail, "排队中（队列已暂停）");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        env.publisher.job(second.id).unwrap().state,
        JobState::Queued
    );
    assert_eq!(env.publisher.job(first.id).unwrap().state, JobState::Paused);
    assert!(lock(&env.fake.uploads).is_empty());

    assert!(env.publisher.resume());
    assert_eq!(env.publisher.view(None).paused, None);
    env.wait(first.id, JobState::Done).await;
    env.wait(second.id, JobState::Done).await;
    assert_eq!(lock(&env.fake.submitted).len(), 2);
    assert!(!env.publisher.resume(), "没暂停时继续无事发生");
}

#[tokio::test]
async fn several_clips_can_become_one_multi_part_archive() {
    let env = env().await;
    let a = env.clip(1000, 2000, "开场").await;
    let b = env.clip(3000, 5000, "").await;
    let job = env.enqueue(&[a.clone(), b.clone()]);
    assert!(job.combine);
    let (done, _) = env.wait(job.id, JobState::Done).await;
    assert_eq!(done.uploaded, 2);

    let studio = lock(&env.fake.submitted).remove(0);
    assert_eq!(studio.videos.len(), 2);
    assert_eq!(studio.videos[0].title.as_deref(), Some("开场"));
    assert!(
        studio.videos[1]
            .title
            .as_deref()
            .unwrap()
            .starts_with("P2 ")
    );
    assert!(studio.title.starts_with("a 切片合集 "), "{}", studio.title);
    assert_eq!(studio.copyright, 2);
    for id in [a.id, b.id] {
        let clip = clips::get(&env.pool, id).await.unwrap().unwrap();
        assert_eq!(clip.archive_bvid.as_deref(), Some("BV1fake1"));
    }
    assert_eq!(env.pins().await, 0);
}

#[tokio::test]
async fn failures_keep_their_reason_and_retry_skips_uploaded_parts() {
    let env = env().await;
    let a = env.clip(1000, 2000, "a").await;
    let b = env.clip(3000, 4000, "b").await;
    lock(&env.fake.submit_failures).push_back(Failure::Other(
        "B 站拒绝了投稿：标题含有敏感词（code 21012）".into(),
    ));
    let job = env.enqueue(&[a.clone(), b.clone()]);
    let (failed, _) = env.wait(job.id, JobState::Failed).await;
    assert_eq!(
        failed.error.as_deref(),
        Some("投稿失败：B 站拒绝了投稿：标题含有敏感词（code 21012）")
    );
    assert_eq!(failed.uploaded, 2);
    assert_eq!(lock(&env.fake.uploads).len(), 2);
    let clip = clips::get(&env.pool, a.id).await.unwrap().unwrap();
    assert_eq!(clip.state, ClipState::Ready, "没投成的切片不算发布");
    assert!(env.pins().await > 0);

    // 失败的任务还在：同一个切片不能再排一次
    let again = env
        .publisher
        .enqueue(
            env.session,
            std::slice::from_ref(&a),
            Settings::default(),
            Mode::Quick,
            None,
        )
        .unwrap_err();
    assert!(matches!(again, EnqueueError::Conflict(ref m) if m.contains("重试")));
    assert_eq!(env.publisher.state_of(a.id), Some(JobState::Failed));

    // 改了 b 的范围：重试时 b 重新上传，a 不重传
    env.publisher.forget_upload(b.id);
    assert_eq!(env.publisher.job(job.id).unwrap().uploaded, 1);
    env.publisher.retry(job.id).unwrap();
    assert_eq!(
        env.publisher.retry(job.id).unwrap_err(),
        ActionError::Conflict("只有失败的任务可以重试".into())
    );
    env.wait(job.id, JobState::Done).await;
    assert_eq!(lock(&env.fake.uploads).len(), 3);
    assert_eq!(lock(&env.fake.submitted)[0].videos.len(), 2);
    assert_eq!(env.publisher.state_of(a.id), None);
}

#[tokio::test]
async fn upload_errors_name_the_part_and_missing_templates_fail_early() {
    let env = env().await;
    let a = env.clip(1000, 2000, "a").await;
    let b = env.clip(3000, 4000, "b").await;
    lock(&env.fake.upload_failures).push_back(Failure::Other("连接被重置".into()));
    lock(&env.fake.upload_failures).push_back(Failure::Other("连接被重置".into()));
    let job = env.enqueue(&[a, b]);
    let (failed, _) = env.wait(job.id, JobState::Failed).await;
    assert_eq!(failed.error.as_deref(), Some("上传 P1 失败：连接被重置"));

    env.publisher.remove(job.id).unwrap();
    assert!(env.publisher.job(job.id).is_none());
    assert_eq!(env.publisher.remove(job.id), Err(ActionError::NotFound));

    sqlx::query("UPDATE livestreamers SET upload_streamers_id = NULL")
        .execute(&env.pool)
        .await
        .unwrap();
    let c = env.clip(1000, 1500, "c").await;
    let job = env.enqueue(&[c]);
    let (failed, _) = env.wait(job.id, JobState::Failed).await;
    assert!(failed.error.unwrap().contains("没有绑定上传模板"));
}

#[tokio::test]
async fn queued_jobs_can_be_removed_and_running_uploads_are_stopped() {
    let env = env().await;
    let a = env.clip(1000, 2000, "a").await;
    let b = env.clip(3000, 4000, "b").await;
    let first = env.enqueue(std::slice::from_ref(&a));
    let second = env.enqueue(std::slice::from_ref(&b));
    assert!(matches!(
        env.publisher
            .enqueue(env.session, std::slice::from_ref(&a), Settings::default(), Mode::Quick, None),
        Err(EnqueueError::Conflict(ref m)) if m.contains("已经在发布队列里")
    ));
    env.publisher.remove(second.id).unwrap();
    // 等第一个开始上传再移出
    for _ in 0..3000 {
        if env.publisher.job(first.id).unwrap().step == Some(Step::Upload) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    env.publisher.remove(first.id).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(env.publisher.view(None).jobs.is_empty());
    assert!(lock(&env.fake.submitted).is_empty());
}

#[test]
fn rate_limits_and_bilibili_rejections_are_told_apart() {
    let report = Report::new(Kind::RateLimit {
        code: 601,
        message: "上传视频过快".into(),
    })
    .change_context(AppError::Custom("选择上传线路失败".into()));
    assert_eq!(
        describe(&report),
        Failure::RateLimited(format!("{RATE_LIMITED}（上传视频过快）"))
    );

    let report = Report::new(Kind::Custom(
        r#"ResponseData { code: 21070, data: None, message: "您投稿的频率过快", ttl: Some(1) }"#
            .into(),
    ))
    .change_context(AppError::Custom("提交视频失败".into()));
    let Failure::Other(message) = describe(&report) else {
        panic!()
    };
    assert_eq!(
        message,
        "提交视频失败：B 站拒绝了投稿：您投稿的频率过快（code 21070）"
    );
    assert_eq!(
        bilibili_message(
            r#"ResponseData { code: 21566, data: None, message: "封面格式不对", ttl: Some(1) }"#,
            "封面"
        ),
        "B 站拒绝了封面：封面格式不对（code 21566）"
    );
}
