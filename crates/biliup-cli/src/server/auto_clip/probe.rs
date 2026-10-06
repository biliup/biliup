//! 连通性测试：chat 能不能回 JSON、模型能不能看图、转写端点能不能用且有没有分句时间戳。
//!
//! 结果存进 `configuration` 表 `key = 'auto_clip_probe'` 的一行（不含 key），缩图设成 `auto`
//! 时按其中「能不能看图」决定开不开；只认与当前配置同一地址、同一模型的结果。

use super::model::{
    AudioFile, ChatRequest, ClientOptions, Endpoint, ErrorKind, ImageInput, ModelClient,
    ModelError, TranscribeOptions,
};
use super::settings::AutoClipConfig;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const PROBE_KEY: &str = "auto_clip_probe";
/// 测试不必等满正式调用的超时
const MAX_CHAT_WAIT: Duration = Duration::from_secs(30);
const MAX_ASR_WAIT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    /// 能用，但有需要知道的限制（不支持 JSON 模式、不能看图、没有分句时间戳）
    Warning,
    Failed,
    Skipped,
}

/// 一项测试的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub status: CheckStatus,
    pub elapsed_ms: Option<u64>,
    /// 给用户看的一句话
    pub message: String,
    /// chat：能否用 JSON 模式；看图：能否看图；转写：有没有分句时间戳。测不出来时为空
    pub capable: Option<bool>,
    pub http_status: Option<u16>,
}

impl Check {
    fn skipped(message: impl Into<String>) -> Self {
        Check {
            status: CheckStatus::Skipped,
            elapsed_ms: None,
            message: message.into(),
            capable: None,
            http_status: None,
        }
    }

    fn done(status: CheckStatus, started: Instant, message: impl Into<String>) -> Self {
        Check {
            status,
            elapsed_ms: Some(started.elapsed().as_millis() as u64),
            message: message.into(),
            capable: None,
            http_status: None,
        }
    }

    fn failed(started: Instant, error: &ModelError) -> Self {
        Check {
            http_status: error.status,
            ..Check::done(CheckStatus::Failed, started, error.to_string())
        }
    }

    fn capable(mut self, capable: Option<bool>) -> Self {
        self.capable = capable;
        self
    }

    fn passed(&self) -> bool {
        matches!(self.status, CheckStatus::Ok | CheckStatus::Warning)
    }
}

/// 一次连通性测试的结果与测的是谁（不含 key）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    /// Unix 毫秒
    pub tested_at: i64,
    pub chat_base_url: Option<String>,
    pub chat_model: Option<String>,
    pub asr_base_url: Option<String>,
    pub asr_model: Option<String>,
    pub chat: Check,
    pub vision: Check,
    pub asr: Check,
}

impl ProbeReport {
    /// 这份结果测的是不是当前配置的 chat 模型。
    pub fn matches_chat(&self, config: &AutoClipConfig) -> bool {
        self.chat_base_url == config.base_url && self.chat_model == config.chat_model
    }

    pub fn matches_asr(&self, config: &AutoClipConfig) -> bool {
        self.asr_base_url.as_deref() == config.asr_base_url() && self.asr_model == config.asr_model
    }
}

/// 要测的两个端点，由配置和环境变量里的 key 组成。
pub struct Targets {
    chat: Option<Endpoint>,
    asr: Option<Endpoint>,
    asr_options: TranscribeOptions,
}

impl Targets {
    pub fn from_config(config: &AutoClipConfig, env_key: Option<String>) -> Self {
        let chat = config
            .base_url
            .as_ref()
            .zip(config.chat_model.as_ref())
            .map(|(url, model)| {
                Endpoint::new(url, model.clone())
                    .with_key(config.chat_key_with(env_key.clone()).map(|(key, _)| key))
            });
        let asr = config
            .asr_base_url()
            .zip(config.asr_model.as_ref())
            .map(|(url, model)| {
                Endpoint::new(url, model.clone()).with_key(config.asr_key_with(env_key.clone()))
            });
        Targets {
            chat,
            asr,
            asr_options: TranscribeOptions::builder()
                .maybe_language(config.asr_language.clone())
                .maybe_prompt(config.asr_prompt.clone())
                .build(),
        }
    }
}

/// 测试用的客户端：不重试（限流时把服务要求的等待时间告诉用户），超时比正式调用短。
pub fn client_for(config: &AutoClipConfig) -> ModelClient {
    ModelClient::new(
        ClientOptions::builder()
            .chat_timeout(config.chat_timeout().min(MAX_CHAT_WAIT))
            .asr_timeout(config.asr_timeout().min(MAX_ASR_WAIT))
            .backoff(Vec::new())
            .build(),
    )
}

pub async fn run(client: &ModelClient, targets: &Targets) -> ProbeReport {
    let chat = match &targets.chat {
        Some(endpoint) => check_chat(client, endpoint).await,
        None => Check::skipped("没填接口地址或 chat 模型，跳过"),
    };
    let vision = match &targets.chat {
        Some(endpoint) if chat.passed() => check_vision(client, endpoint).await,
        _ => Check::skipped("chat 不通，跳过看图测试"),
    };
    let asr = match &targets.asr {
        Some(endpoint) => check_asr(client, endpoint, &targets.asr_options).await,
        None => Check::skipped("没填转写模型（asr_model），跳过；不转写时只能靠弹幕和画面出候选"),
    };
    ProbeReport {
        tested_at: chrono::Utc::now().timestamp_millis(),
        chat_base_url: targets.chat.as_ref().map(|e| e.base_url().to_string()),
        chat_model: targets.chat.as_ref().map(|e| e.model().to_string()),
        asr_base_url: targets.asr.as_ref().map(|e| e.base_url().to_string()),
        asr_model: targets.asr.as_ref().map(|e| e.model().to_string()),
        chat,
        vision,
        asr,
    }
}

async fn check_chat(client: &ModelClient, endpoint: &Endpoint) -> Check {
    let started = Instant::now();
    let request = ChatRequest::builder()
        .system("你是连通性测试助手，只输出 JSON，不要任何解释。")
        .user("原样回复这个 JSON：{\"ok\": true}")
        .json(true)
        .max_tokens(50)
        .build();
    match client.chat(endpoint, &request).await {
        Ok(reply) if reply.json().is_some() => {
            let message = if reply.json_mode {
                "chat 可用，支持 JSON 模式"
            } else {
                "chat 可用；这个服务不支持 JSON 模式，会从回复里提取 JSON"
            };
            Check::done(CheckStatus::Ok, started, message).capable(Some(reply.json_mode))
        }
        Ok(reply) => Check::done(
            CheckStatus::Warning,
            started,
            "chat 能连通，但回复里没有 JSON：这个模型可能不擅长按格式输出，生成候选时容易失败，建议换一个模型",
        )
        .capable(Some(reply.json_mode)),
        Err(error) => Check::failed(started, &error),
    }
}

async fn check_vision(client: &ModelClient, endpoint: &Endpoint) -> Check {
    let started = Instant::now();
    let request = ChatRequest::builder()
        .system("你是图像识别助手，只输出 JSON。")
        .user("图片是什么颜色？用 JSON 回答，例如 {\"color\": \"红色\"}；只能从 红色、绿色、蓝色、其它 里选。")
        .images(vec![ImageInput::png(&test_image())])
        .json(true)
        .max_tokens(50)
        .build();
    let unsupported = "模型不支持看图：缩图设为「自动」时不会发截图";
    match client.chat(endpoint, &request).await {
        Ok(reply) => {
            let answer = reply
                .json()
                .and_then(|value| {
                    value
                        .get("color")
                        .and_then(|c| c.as_str())
                        .map(str::to_string)
                })
                .unwrap_or(reply.content);
            let answer = answer.to_lowercase();
            if answer.contains('红') || answer.contains("red") {
                Check::done(
                    CheckStatus::Ok,
                    started,
                    "模型能看图：缩图设为「自动」时会随分析一起发截图",
                )
                .capable(Some(true))
            } else {
                Check::done(
                    CheckStatus::Warning,
                    started,
                    format!("{unsupported}（发了一张红色图片，模型没认出来）"),
                )
                .capable(Some(false))
            }
        }
        // 带图才被拒：说明是不认图片，不是连不通
        Err(error) if matches!(error.status, Some(400) | Some(415) | Some(422)) => Check {
            http_status: error.status,
            ..Check::done(
                CheckStatus::Warning,
                started,
                format!(
                    "{unsupported}（服务拒收图片：{}）",
                    error.detail.as_deref().unwrap_or("无说明")
                ),
            )
        }
        .capable(Some(false)),
        Err(error) => Check::failed(started, &error),
    }
}

async fn check_asr(
    client: &ModelClient,
    endpoint: &Endpoint,
    options: &TranscribeOptions,
) -> Check {
    let started = Instant::now();
    let audio = AudioFile {
        bytes: silent_wav(Duration::from_secs(2)),
        file_name: "biliup-probe.wav".into(),
        mime: "audio/wav".into(),
    };
    match client.transcribe(endpoint, &audio, options).await {
        Ok(transcript) => match transcript.require_segments(endpoint.model()) {
            Ok(_) => {
                Check::done(CheckStatus::Ok, started, "转写可用，带分句时间戳").capable(Some(true))
            }
            Err(error) => {
                debug_assert_eq!(error.kind, ErrorKind::NoSegments);
                Check::done(
                    CheckStatus::Warning,
                    started,
                    format!("转写可用，但{}", error.message),
                )
                .capable(Some(false))
            }
        },
        Err(error) => Check::failed(started, &error),
    }
}

/// 64×64 的纯红色 PNG。
fn test_image() -> Vec<u8> {
    let image = image::RgbImage::from_pixel(64, 64, image::Rgb([220, 30, 30]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("编码内存里的 PNG 不会失败");
    bytes.into_inner()
}

/// 16 kHz 单声道 16 位的静音 WAV。
pub fn silent_wav(duration: Duration) -> Vec<u8> {
    const RATE: u32 = 16_000;
    let samples = (RATE as u128 * duration.as_millis() / 1000) as u32;
    let data_len = samples * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // 单声道
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.resize(44 + data_len as usize, 0);
    wav
}

/// 覆盖上一次的结果。
pub async fn save(pool: &ConnectionPool, report: &ProbeReport) -> Result<(), sqlx::Error> {
    let value = serde_json::to_string(report).expect("测试结果总能序列化");
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM configuration WHERE key = ?1")
        .bind(PROBE_KEY)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO configuration (key, value) VALUES (?1, ?2)")
        .bind(PROBE_KEY)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

pub async fn load(pool: &ConnectionPool) -> Result<Option<ProbeReport>, sqlx::Error> {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value FROM configuration WHERE key = ?1 ORDER BY id DESC LIMIT 1",
    )
    .bind(PROBE_KEY)
    .fetch_optional(pool)
    .await?;
    Ok(value.and_then(|value| serde_json::from_str(&value).ok()))
}

#[cfg(test)]
mod tests {
    use super::super::fake::{FakeServer, Scenario};
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    const KEY: &str = "sk-probe-0123456789abcdef";

    fn config(base_url: &str) -> AutoClipConfig {
        AutoClipConfig::builder()
            .base_url(base_url.into())
            .api_key(KEY.into())
            .chat_model("chat-model".into())
            .asr_model("whisper-1".into())
            .chat_timeout_secs(1)
            .asr_timeout_secs(1)
            .build()
    }

    async fn probe(scenario: Scenario) -> ProbeReport {
        let server = FakeServer::start(scenario).await;
        let config = config(server.base_url());
        run(&client_for(&config), &Targets::from_config(&config, None)).await
    }

    #[tokio::test]
    async fn everything_passes_against_a_capable_service() {
        let report = probe(Scenario::Ok).await;
        assert_eq!(report.chat.status, CheckStatus::Ok, "{:?}", report.chat);
        assert_eq!(report.chat.capable, Some(true));
        assert_eq!(report.vision.status, CheckStatus::Ok, "{:?}", report.vision);
        assert_eq!(report.vision.capable, Some(true));
        assert_eq!(report.asr.status, CheckStatus::Ok, "{:?}", report.asr);
        assert_eq!(report.asr.capable, Some(true));
        assert_eq!(report.chat_model.as_deref(), Some("chat-model"));
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains(KEY), "{text}");
    }

    #[tokio::test]
    async fn a_model_that_cannot_see_images_is_recorded_as_such() {
        let report = probe(Scenario::NoVision).await;
        assert_eq!(report.chat.status, CheckStatus::Ok);
        assert_eq!(
            report.vision.status,
            CheckStatus::Warning,
            "{:?}",
            report.vision
        );
        assert_eq!(report.vision.capable, Some(false));
        assert_eq!(report.vision.http_status, Some(400));
        assert!(
            report.vision.message.contains("不支持看图"),
            "{}",
            report.vision.message
        );
    }

    #[tokio::test]
    async fn missing_segments_is_a_warning_with_advice() {
        let report = probe(Scenario::NoSegments).await;
        assert_eq!(report.asr.status, CheckStatus::Warning, "{:?}", report.asr);
        assert_eq!(report.asr.capable, Some(false));
        assert!(
            report.asr.message.contains("分句时间戳"),
            "{}",
            report.asr.message
        );
    }

    #[tokio::test]
    async fn failures_are_explained_and_later_checks_skipped() {
        for (scenario, status, advice) in [
            (Scenario::Unauthorized, Some(401), "API key 无效"),
            (Scenario::NotFound, Some(404), "/v1 结尾"),
            (Scenario::TooLarge, Some(413), "太大"),
            (
                Scenario::RateLimitedOnce { retry_after: 30 },
                Some(429),
                "30 秒后再试",
            ),
            (Scenario::ServerError, Some(500), "服务端出错"),
            (Scenario::Slow { secs: 5 }, None, "超时"),
        ] {
            let report = probe(scenario).await;
            assert_eq!(report.chat.status, CheckStatus::Failed, "{scenario:?}");
            assert_eq!(report.chat.http_status, status, "{scenario:?}");
            assert!(
                report.chat.message.contains(advice),
                "{scenario:?}: {}",
                report.chat.message
            );
            assert!(
                !report.chat.message.contains(KEY),
                "{}",
                report.chat.message
            );
            assert_eq!(report.vision.status, CheckStatus::Skipped, "{scenario:?}");
        }
    }

    #[tokio::test]
    async fn unconfigured_parts_are_skipped() {
        let config = AutoClipConfig::builder().enabled(true).build();
        let report = run(&client_for(&config), &Targets::from_config(&config, None)).await;
        assert_eq!(report.chat.status, CheckStatus::Skipped);
        assert_eq!(report.vision.status, CheckStatus::Skipped);
        assert_eq!(report.asr.status, CheckStatus::Skipped);
    }

    #[tokio::test]
    async fn the_asr_endpoint_can_be_a_separate_service() {
        let chat = FakeServer::start(Scenario::Ok).await;
        let asr = FakeServer::start(Scenario::Ok).await;
        let config = AutoClipConfig {
            asr_base_url: Some(asr.base_url().to_string()),
            asr_api_key: Some("asr-key-000000000000".into()),
            ..config(chat.base_url())
        };
        let report = run(
            &client_for(&config),
            &Targets::from_config(&config, Some("sk-env-key-0000000000".into())),
        )
        .await;
        assert_eq!(report.asr.status, CheckStatus::Ok);
        assert!(
            chat.requests()
                .iter()
                .all(|r| r.path.ends_with("/chat/completions"))
        );
        let asr_requests = asr.requests();
        assert_eq!(asr_requests.len(), 1);
        assert_eq!(
            asr_requests[0].authorization.as_deref(),
            Some("Bearer asr-key-000000000000")
        );
        // 环境变量覆盖 chat 的 key
        assert_eq!(
            chat.requests()[0].authorization.as_deref(),
            Some("Bearer sk-env-key-0000000000")
        );
        assert!(report.matches_asr(&config));
    }

    #[tokio::test]
    async fn results_are_saved_and_matched_to_the_configured_model() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(load(&pool).await.unwrap(), None);
        let server = FakeServer::start(Scenario::Ok).await;
        let config = config(server.base_url());
        let report = run(&client_for(&config), &Targets::from_config(&config, None)).await;
        save(&pool, &report).await.unwrap();
        save(&pool, &report).await.unwrap();
        let loaded = load(&pool).await.unwrap().unwrap();
        assert_eq!(loaded, report);
        assert!(loaded.matches_chat(&config));
        let other = AutoClipConfig {
            chat_model: Some("another".into()),
            ..config
        };
        assert!(!loaded.matches_chat(&other));
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'auto_clip_probe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_test_assets_are_valid() {
        let png = test_image();
        assert_eq!(&png[1..4], b"PNG");
        let decoded = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (64, 64));
        let wav = silent_wav(Duration::from_secs(2));
        assert_eq!(wav.len(), 44 + 64_000);
        assert_eq!(&wav[..4], b"RIFF");
    }
}
