//! 候选生成：转写之后同一个任务接着做。
//!
//! - `signals` 阶段：算弹幕密度（写 `danmaku.json`），按缩图开关采关键帧缩图（写 `thumbs/`）；
//! - `analyze` 阶段：30 分钟一窗问 chat，每窗的回复与用量写 `analysis.jsonl`（按请求内容的摘要查，
//!   服务重启续跑时已问过的窗不再花钱）；回复逐条校验、吸附关键帧，各窗合并去重、按置信度截断，
//!   换掉这一场上一轮没处理的候选。
//!
//! chat 用量有每场上限（`max_chat_tokens`，输入 + 输出，按这个任务累计）：开问之前按提示词估算，
//! 超了就不问、任务失败并说明；问的过程中实际用量眼看要超时停下，已有的候选照常落库并提示。
//! 超上下文的窗对半再问；服务拒收图片时那一窗去掉图再问，后面的窗不再附图。

use super::candidates::{self, Candidate, Context, Rejection, Snapper};
use super::danmaku::{self, Density};
use super::files::{CachedWindow, Line, SessionFiles};
use super::jobs::{self, Job, Stage};
use super::model::{
    ChatReply, ChatRequest, Endpoint, ErrorKind, ImageInput, ModelClient, ModelError,
};
use super::probe;
use super::prompt::{self, Prompt, PromptInput, SessionInfo, Window};
use super::settings::{AutoClipConfig, display_host};
use super::suggestions;
use super::thumbs::{self, Shot};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::recorder::now_ms;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, VecDeque};
use tracing::info;

/// 还没转写的语音按每分钟这么多 token 估（约 200 字，加每句时间戳约 +30%）。
pub const ASR_TOKENS_PER_MINUTE: i64 = 260;

fn db(error: sqlx::Error) -> String {
    format!("读写数据库出错：{error}")
}

fn io(error: std::io::Error) -> String {
    format!("写分析数据出错：{error}")
}

/// 提示词开头的场次信息：直播标题、主播备注名、直播间地址的主机名。
pub async fn session_info(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<SessionInfo> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT name, url, title FROM stream_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    let (name, url, title) = row.unwrap_or_default();
    Ok(SessionInfo {
        title,
        streamer: name,
        platform: display_host(&url).unwrap_or_default(),
    })
}

/// 一窗请求内容的摘要：模型、两条消息、附图的时间都一样才算同一个请求。
pub fn cache_key(model: &str, prompt: &Prompt) -> String {
    let mut hasher = Sha256::new();
    for part in [model, &prompt.system, &prompt.user] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    for t_ms in &prompt.images {
        hasher.update(t_ms.to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 每场 chat 用量超上限时的说明。
pub fn chat_limit_message(tokens: i64, limit: u64) -> String {
    format!(
        "这一场的候选生成预计要用约 {tokens} token，超过每场上限 {limit} token，没有调用 chat：\
         可以在设置「自动切片（实验）」里调高每场 chat 上限（max_chat_tokens），或关掉缩图后重试"
    )
}

/// chat 用量预估。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatEstimate {
    /// 输入 + 输出 token
    pub tokens: i64,
    /// 其中缩图的张数
    pub images: i64,
    pub windows: i64,
    pub basis: ChatBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatBasis {
    /// 转写已经齐了，按实际的提示词算
    Transcript,
    /// 还有没转写的语音，按时长估
    Duration,
}

/// 按提示词估一场的 chat 用量：已有的转写、弹幕密度（有缓存用缓存，没有现算）、按开关算的缩图张数，
/// 加上还没转写的语音按时长估的部分。
pub async fn estimate_chat(
    pool: &ConnectionPool,
    files: &SessionFiles,
    config: &AutoClipConfig,
    session_id: i64,
    end_ms: i64,
    lines: &[Line],
    untranscribed_ms: i64,
) -> sqlx::Result<ChatEstimate> {
    let density = match files.load_danmaku().await {
        Some(density) => Some(density),
        None => danmaku::session_density(pool, session_id).await?,
    };
    let probe = probe::load(pool).await.ok().flatten();
    let shots = if thumbs::enabled(config, probe.as_ref()).0 {
        let peaks = density.as_ref().map_or(&[][..], |d| &d.peaks[..]);
        thumbs::plan(end_ms, peaks)
    } else {
        Vec::new()
    };
    let session = session_info(pool, session_id).await?;
    let (min_secs, max_secs) = config.clip_secs();
    let (mut tokens, mut images) = (0, 0);
    let windows = prompt::windows(end_ms);
    for window in &windows {
        let shots = thumbs::for_window(&shots, window.from_ms, window.to_ms);
        let prompt = prompt::build(
            &PromptInput::builder()
                .session(&session)
                .window(*window)
                .lines(lines)
                .maybe_density(density.as_ref())
                .shots(&shots)
                .min_clip_secs(min_secs)
                .max_clip_secs(max_secs)
                .build(),
        );
        tokens += prompt.estimate_tokens() + prompt::EXPECTED_OUTPUT_TOKENS;
        images += prompt.images.len() as i64;
    }
    tokens += untranscribed_ms.max(0) * ASR_TOKENS_PER_MINUTE / 60_000;
    Ok(ChatEstimate {
        tokens,
        images,
        windows: windows.len() as i64,
        basis: if untranscribed_ms > 0 {
            ChatBasis::Duration
        } else {
            ChatBasis::Transcript
        },
    })
}

/// 一次 chat 的结果分类。
enum Asked {
    Answered {
        reply: ChatReply,
        sent: Prompt,
    },
    /// 超上下文：对半再问
    TooLong(ModelError),
    /// key、权限、模型名不对：后面的窗也不会成功
    Fatal(ModelError),
    /// 这一窗跳过
    Skip(ModelError),
}

fn too_long(error: &ModelError) -> bool {
    if error.kind == ErrorKind::PayloadTooLarge {
        return true;
    }
    let detail = error.detail.as_deref().unwrap_or_default().to_lowercase();
    error.kind == ErrorKind::BadRequest
        && [
            "context",
            "maximum",
            "too long",
            "too many tokens",
            "上下文",
        ]
        .iter()
        .any(|needle| detail.contains(needle))
}

fn fatal(error: &ModelError) -> bool {
    matches!(
        error.kind,
        ErrorKind::Unauthorized | ErrorKind::Forbidden | ErrorKind::NotFound
    )
}

/// 候选生成一次要用的东西。
pub struct Analysis<'a> {
    pub pool: &'a ConnectionPool,
    pub job: &'a Job,
    pub files: &'a SessionFiles,
    pub config: &'a AutoClipConfig,
    pub client: &'a ModelClient,
    pub endpoint: &'a Endpoint,
    /// 场次时间轴上录像的终点
    pub end_ms: i64,
    pub models: &'a mut Value,
    pub warnings: &'a mut Vec<String>,
}

/// 跑完的结果，给日志和测试看。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Summary {
    pub windows: usize,
    pub answered: usize,
    pub rejected: BTreeMap<Rejection, usize>,
    pub suggestions: usize,
    pub truncated: bool,
}

impl Analysis<'_> {
    async fn warn(&mut self, message: String) -> Result<(), String> {
        info!(job = self.job.id, %message, "自动切片：提示");
        self.warnings.push(message);
        jobs::set_details(self.pool, self.job.id, self.models, self.warnings)
            .await
            .map_err(db)
    }

    pub async fn run(mut self) -> Result<Summary, String> {
        let (density, shots) = self.signals().await?;
        self.analyze(density.as_ref(), &shots).await
    }

    /// 弹幕密度与缩图。
    async fn signals(&mut self) -> Result<(Option<Density>, Vec<Shot>), String> {
        let (pool, id, session) = (self.pool, self.job.id, self.job.session_id);
        jobs::set_stage(pool, id, Stage::Signals, 0, 2)
            .await
            .map_err(db)?;
        let density = danmaku::session_density(pool, session).await.map_err(db)?;
        self.files
            .save_danmaku(density.as_ref())
            .await
            .map_err(io)?;
        if density.is_none() {
            self.warn("这一场没有弹幕记录（没开弹幕录制或文件已删），只按语音和画面分析".into())
                .await?;
        }
        jobs::set_progress(pool, id, 1).await.map_err(db)?;

        let probe = probe::load(pool).await.ok().flatten();
        let (enabled, note) = thumbs::enabled(self.config, probe.as_ref());
        self.models["thumbnails"] = Value::Bool(enabled);
        match note {
            Some(note) => self.warn(note).await?,
            None => jobs::set_details(pool, id, self.models, self.warnings)
                .await
                .map_err(db)?,
        }
        let shots = if enabled {
            let peaks = density.as_ref().map_or(&[][..], |d| &d.peaks[..]);
            let planned = thumbs::plan(self.end_ms, peaks);
            let snapped = thumbs::snap(pool, session, &planned).await;
            let (shots, failures) = thumbs::grab(pool, session, self.files, &snapped).await;
            if let Some(first) = failures.first() {
                self.warn(format!(
                    "有 {} 张缩图没取到，这些时刻不附图：{first}",
                    snapped.len() - shots.len()
                ))
                .await?;
            }
            shots
        } else {
            Vec::new()
        };
        jobs::set_progress(pool, id, 2).await.map_err(db)?;
        Ok((density, shots))
    }

    fn prompt(
        &self,
        session: &SessionInfo,
        window: Window,
        lines: &[Line],
        density: Option<&Density>,
        shots: &[Shot],
    ) -> Prompt {
        let (min_secs, max_secs) = self.config.clip_secs();
        let shots = thumbs::for_window(shots, window.from_ms, window.to_ms);
        prompt::build(
            &PromptInput::builder()
                .session(session)
                .window(window)
                .lines(lines)
                .maybe_density(density)
                .shots(&shots)
                .min_clip_secs(min_secs)
                .max_clip_secs(max_secs)
                .build(),
        )
    }

    async fn send(&self, prompt: &Prompt) -> Result<ChatReply, ModelError> {
        let mut images = Vec::new();
        for t_ms in &prompt.images {
            if let Ok(bytes) = tokio::fs::read(self.files.thumb(*t_ms)).await {
                images.push(ImageInput::jpeg(&bytes));
            }
        }
        let request = ChatRequest::builder()
            .system(prompt.system.clone())
            .user(prompt.user.clone())
            .images(images)
            .json(true)
            .max_tokens(prompt::MAX_OUTPUT_TOKENS)
            .build();
        self.client.chat(self.endpoint, &request).await
    }

    /// 问一窗；带图被拒时去掉图再问一次，并让后面的窗不再附图。
    async fn ask(&mut self, prompt: &Prompt, images_ok: &mut bool) -> Result<Asked, String> {
        let classify = |error: ModelError| {
            if too_long(&error) {
                Asked::TooLong(error)
            } else if fatal(&error) {
                Asked::Fatal(error)
            } else {
                Asked::Skip(error)
            }
        };
        match self.send(prompt).await {
            Ok(reply) => Ok(Asked::Answered {
                reply,
                sent: prompt.clone(),
            }),
            Err(error)
                if !prompt.images.is_empty()
                    && error.kind == ErrorKind::BadRequest
                    && !too_long(&error) =>
            {
                *images_ok = false;
                self.models["thumbnails"] = Value::Bool(false);
                self.warn(format!(
                    "chat 服务拒收图片，这一窗去掉图重问，后面的窗不再附图：{error}"
                ))
                .await?;
                let plain = prompt.without_images();
                Ok(match self.send(&plain).await {
                    Ok(reply) => Asked::Answered { reply, sent: plain },
                    Err(error) => classify(error),
                })
            }
            Err(error) => Ok(classify(error)),
        }
    }

    async fn analyze(
        &mut self,
        density: Option<&Density>,
        shots: &[Shot],
    ) -> Result<Summary, String> {
        let (pool, id, session_id) = (self.pool, self.job.id, self.job.session_id);
        let lines = self.files.lines().await;
        let session = session_info(pool, session_id).await.map_err(db)?;
        let snapper = Snapper::load(pool, session_id).await.map_err(db)?;
        let cache: HashMap<String, CachedWindow> = self.files.load_analysis().await;
        let model = self.endpoint.model().to_string();
        let limit = self.config.max_chat_tokens();
        let mut used = self.job.tokens_in + self.job.tokens_out;
        let mut images_ok = !shots.is_empty();

        let mut queue: VecDeque<Window> = prompt::windows(self.end_ms).into();
        let upcoming: i64 = queue
            .iter()
            .map(|w| self.prompt(&session, *w, &lines, density, shots))
            .filter(|p| !cache.contains_key(&cache_key(&model, p)))
            .map(|p| p.estimate_tokens() + prompt::EXPECTED_OUTPUT_TOKENS)
            .sum();
        if used + upcoming > limit as i64 {
            return Err(chat_limit_message(used + upcoming, limit));
        }

        let mut total = queue.len() as i64;
        jobs::set_stage(pool, id, Stage::Analyze, 0, total)
            .await
            .map_err(db)?;
        let (min_secs, max_secs) = self.config.clip_secs();
        let mut summary = Summary::default();
        let mut rejected: Vec<Rejection> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let mut found: Vec<Candidate> = Vec::new();
        let mut done = 0i64;
        let mut split_noted = false;
        while let Some(window) = queue.pop_front() {
            let full = self.prompt(&session, window, &lines, density, shots);
            let prompt = if images_ok {
                full.clone()
            } else {
                full.without_images()
            };
            let key = cache_key(&model, &full);
            let (content, sent_images) = match cache
                .get(&key)
                .or_else(|| cache.get(&cache_key(&model, &prompt)))
            {
                Some(hit) => (hit.content.clone(), hit.images),
                None => {
                    let need = prompt.estimate_tokens() + prompt::EXPECTED_OUTPUT_TOKENS;
                    if used + need > limit as i64 {
                        summary.truncated = true;
                        self.warn(format!(
                            "chat 已用约 {used} token，再问会超过每场上限 {limit} token，剩下 {} 窗（{} 起）没有分析",
                            queue.len() + 1,
                            prompt::clock(window.from_ms)
                        ))
                        .await?;
                        break;
                    }
                    match self.ask(&prompt, &mut images_ok).await? {
                        Asked::Answered { reply, sent } => {
                            let tokens_in = match reply.usage.prompt_tokens {
                                0 => sent.estimate_tokens(),
                                n => n as i64,
                            };
                            let tokens_out = match reply.usage.completion_tokens {
                                0 => prompt::estimate_text_tokens(&reply.content),
                                n => n as i64,
                            };
                            let images = sent.images.len() as i64;
                            used += tokens_in + tokens_out;
                            jobs::record_chat(pool, id, tokens_in, tokens_out, images)
                                .await
                                .map_err(db)?;
                            let cached = CachedWindow {
                                key: key.clone(),
                                from_ms: window.from_ms,
                                to_ms: window.to_ms,
                                content: reply.content.clone(),
                                tokens_in,
                                tokens_out,
                                images,
                            };
                            self.files.save_window(&cached).await.map_err(io)?;
                            (reply.content, images)
                        }
                        Asked::TooLong(error) => match window.halves() {
                            Some((left, right)) => {
                                queue.push_front(right);
                                queue.push_front(left);
                                total += 1;
                                jobs::set_stage(pool, id, Stage::Analyze, done, total)
                                    .await
                                    .map_err(db)?;
                                if !split_noted {
                                    split_noted = true;
                                    self.warn(format!(
                                        "有的窗超过了模型的上下文长度，对半切开再问：{error}"
                                    ))
                                    .await?;
                                }
                                continue;
                            }
                            None => {
                                failures.push(window_failure(window, &error.to_string()));
                                done += 1;
                                jobs::set_progress(pool, id, done).await.map_err(db)?;
                                continue;
                            }
                        },
                        Asked::Fatal(error) => return Err(format!("chat 调用失败：{error}")),
                        Asked::Skip(error) => {
                            failures.push(window_failure(window, &error.to_string()));
                            done += 1;
                            jobs::set_progress(pool, id, done).await.map_err(db)?;
                            continue;
                        }
                    }
                }
            };
            done += 1;
            summary.windows += 1;
            match candidates::parse(&content, &mut rejected) {
                Ok(proposed) => {
                    summary.answered += 1;
                    let images: Vec<i64> = if sent_images > 0 {
                        full.images.clone()
                    } else {
                        Vec::new()
                    };
                    let cx = Context {
                        window,
                        end_ms: self.end_ms,
                        min_ms: min_secs as i64 * 1000,
                        max_ms: max_secs as i64 * 1000,
                        lines: &lines,
                        density,
                        images: &images,
                    };
                    for p in proposed {
                        let evidence = match candidates::check(&p, &cx) {
                            Ok(evidence) => evidence,
                            Err(why) => {
                                rejected.push(why);
                                continue;
                            }
                        };
                        match snapper.snap(pool, session_id, p.start_ms, p.end_ms).await {
                            Some((in_ms, out_ms)) => found.push(Candidate {
                                in_ms,
                                out_ms,
                                title: p.title,
                                reason: p.reason,
                                confidence: p.confidence,
                                tags: p.tags,
                                evidence,
                            }),
                            None => rejected.push(Rejection::Unreadable),
                        }
                    }
                }
                Err(error) => failures.push(window_failure(
                    window,
                    match error {
                        candidates::ReplyError::NotJson => "回复里找不到 JSON",
                        candidates::ReplyError::NoCandidates => {
                            "回复的 JSON 里没有 candidates 数组"
                        }
                    },
                )),
            }
            jobs::set_progress(pool, id, done).await.map_err(db)?;
        }

        for why in &rejected {
            *summary.rejected.entry(*why).or_default() += 1;
        }
        if summary.answered == 0 {
            return Err(match (summary.truncated, failures.first()) {
                (_, Some(first)) => format!("没有一窗得到可用的回复：{first}"),
                (true, None) => chat_limit_message(used, limit),
                (false, None) => "没有可以分析的内容".into(),
            });
        }
        if !failures.is_empty() {
            self.warn(format!(
                "有 {} 窗没有得到可用的回复，跳过：{}",
                failures.len(),
                failures.join("；")
            ))
            .await?;
        }
        if !summary.rejected.is_empty() {
            let listed: Vec<String> = summary
                .rejected
                .iter()
                .map(|(why, n)| format!("{} {n} 条", why.describe()))
                .collect();
            self.warn(format!(
                "模型给的候选有 {} 条没通过校验，已丢掉：{}",
                rejected.len(),
                listed.join("，")
            ))
            .await?;
        }
        let merged = candidates::merge(found, self.config.max_candidates());
        let saved = suggestions::replace_pending(pool, session_id, Some(id), &merged, now_ms())
            .await
            .map_err(db)?;
        summary.suggestions = saved.len();
        info!(
            job = id,
            windows = summary.windows,
            suggestions = summary.suggestions,
            tokens = used,
            "自动切片：候选生成完成"
        );
        Ok(summary)
    }
}

fn window_failure(window: Window, why: &str) -> String {
    format!(
        "{}–{} {why}",
        prompt::clock(window.from_ms),
        prompt::clock(window.to_ms)
    )
}
