//! OpenAI 兼容接口的客户端：`/chat/completions` 与 `/audio/transcriptions`。
//!
//! 429 / 5xx / 超时 / 连接断开按退避重试（遵守 `Retry-After`），400 / 401 / 403 / 404 / 413
//! 不重试。失败时给一句能照着做的中文说明，附 HTTP 状态和响应体开头（其中出现的 key 换成掩码）；
//! 请求头不进错误信息，日志里也不打 key。

use super::settings::{display_host, mask};
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;
use std::time::Duration;
use tracing::warn;

/// 错误信息里最多带多少字的响应体。
const DETAIL_CHARS: usize = 300;

/// 一个 OpenAI 兼容服务上的一个模型。
#[derive(Clone, PartialEq, Eq)]
pub struct Endpoint {
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl Endpoint {
    pub fn new(base_url: impl AsRef<str>, model: impl Into<String>) -> Self {
        Endpoint {
            base_url: super::settings::normalize_url(base_url.as_ref()),
            api_key: None,
            model: model.into(),
        }
    }

    pub fn with_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key.filter(|key| !key.is_empty());
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// 把响应体里原样出现的 key 换成掩码，有的服务会在 401 里回显 key。
    fn scrub(&self, text: &str) -> String {
        match &self.api_key {
            Some(key) if key.len() >= 4 => text.replace(key.as_str(), &mask(key)),
            _ => text.to_string(),
        }
    }
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_deref().map(mask))
            .field("model", &self.model)
            .finish()
    }
}

/// 客户端参数。重试次数 = `backoff` 的长度。
#[derive(bon::Builder, Debug, Clone)]
pub struct ClientOptions {
    #[builder(default = Duration::from_secs(10))]
    pub connect_timeout: Duration,
    #[builder(default = Duration::from_secs(super::settings::DEFAULT_CHAT_TIMEOUT_SECS))]
    pub chat_timeout: Duration,
    #[builder(default = Duration::from_secs(super::settings::DEFAULT_ASR_TIMEOUT_SECS))]
    pub asr_timeout: Duration,
    /// 第 n 次重试前至少等这么久；服务给的 `Retry-After` 更长时按它
    #[builder(default = vec![Duration::from_secs(5), Duration::from_secs(20), Duration::from_secs(60)])]
    pub backoff: Vec<Duration>,
    /// `Retry-After` 超过这个值就不等了，直接报限流
    #[builder(default = Duration::from_secs(120))]
    pub max_retry_after: Duration,
}

impl Default for ClientOptions {
    fn default() -> Self {
        ClientOptions::builder().build()
    }
}

/// 出错的是哪一类请求，决定提示里说哪个端点、哪个配置项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Call {
    Chat,
    Transcription,
}

impl Call {
    fn path(self) -> &'static str {
        match self {
            Call::Chat => "/chat/completions",
            Call::Transcription => "/audio/transcriptions",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    PayloadTooLarge,
    RateLimited,
    Server,
    Timeout,
    Connect,
    /// 2xx 但内容不是预期的 JSON
    BadResponse,
    /// 转写结果没有分句时间戳
    NoSegments,
}

impl ErrorKind {
    fn retryable(self) -> bool {
        matches!(
            self,
            ErrorKind::RateLimited | ErrorKind::Server | ErrorKind::Timeout | ErrorKind::Connect
        )
    }
}

/// 调用失败的原因。`Display` 就是给用户看的那句话。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelError {
    pub kind: ErrorKind,
    pub call: Call,
    /// HTTP 状态；超时、连不上时没有
    pub status: Option<u16>,
    /// 服务要求多少秒后再试（429 / 503 时）
    pub retry_after_secs: Option<u64>,
    /// 能照着做的中文说明
    pub message: String,
    /// 响应体开头（已去掉 key），或底层错误
    pub detail: Option<String>,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(status) = self.status {
            write!(f, "（HTTP {status}）")?;
        }
        if let Some(detail) = &self.detail {
            write!(f, "：{detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ModelError {}

impl ModelError {
    fn new(kind: ErrorKind, call: Call, message: String) -> Self {
        ModelError {
            kind,
            call,
            status: None,
            retry_after_secs: None,
            message,
            detail: None,
        }
    }

    fn with_detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail.filter(|text| !text.trim().is_empty());
        self
    }

    /// 按 HTTP 状态翻译。
    fn from_status(
        call: Call,
        endpoint: &Endpoint,
        status: StatusCode,
        retry_after: Option<Duration>,
        body: &str,
    ) -> Self {
        let code = status.as_u16();
        let path = call.path();
        let (kind, message) = match code {
            401 => (
                ErrorKind::Unauthorized,
                "API key 无效或已过期：检查 key 有没有填错、是不是这个服务的 key".to_string(),
            ),
            403 => (
                ErrorKind::Forbidden,
                format!(
                    "这个 key 没有权限使用模型 {}：到服务商后台确认权限、余额或地区限制",
                    endpoint.model
                ),
            ),
            404 => (
                ErrorKind::NotFound,
                format!(
                    "地址或模型不存在：确认接口地址一般以 /v1 结尾（请求的是 {}），模型名 {} 拼写正确，且这个服务提供 {path}",
                    endpoint.url(path),
                    endpoint.model
                ),
            ),
            413 => (
                ErrorKind::PayloadTooLarge,
                match call {
                    Call::Transcription => {
                        "音频太大，服务拒收：换支持更大文件的转写服务，或调小每块音频的长度"
                            .to_string()
                    }
                    Call::Chat => {
                        "请求太大，服务拒收：减少一次送去的内容（例如关掉缩图）".to_string()
                    }
                },
            ),
            429 => (
                ErrorKind::RateLimited,
                match retry_after {
                    Some(wait) => format!(
                        "请求被限流或额度用完，服务要求 {} 秒后再试；经常出现时到服务商后台提高配额",
                        wait.as_secs()
                    ),
                    None => {
                        "请求被限流或额度用完：稍后再试；经常出现时到服务商后台提高配额".to_string()
                    }
                },
            ),
            500..=599 => (
                ErrorKind::Server,
                "服务端出错：稍后再试；一直这样就联系服务商或换一个接口地址".to_string(),
            ),
            _ => (
                ErrorKind::BadRequest,
                format!(
                    "服务拒绝了这个请求：按下面的返回内容检查模型 {} 是否支持这种调用",
                    endpoint.model
                ),
            ),
        };
        let mut error =
            ModelError::new(kind, call, message).with_detail(Some(snippet(&endpoint.scrub(body))));
        error.status = Some(code);
        error.retry_after_secs = retry_after.map(|wait| wait.as_secs());
        error
    }

    fn from_transport(
        call: Call,
        endpoint: &Endpoint,
        timeout: Duration,
        error: &reqwest::Error,
    ) -> Self {
        let host = display_host(&endpoint.base_url).unwrap_or_else(|| endpoint.base_url.clone());
        let (kind, message) = if error.is_timeout() {
            (
                ErrorKind::Timeout,
                format!(
                    "请求超时（{} 秒内没有完成）：检查网络和接口地址，或调大超时",
                    timeout.as_secs()
                ),
            )
        } else if error.is_builder() {
            (
                ErrorKind::BadRequest,
                format!(
                    "接口地址不对：{}（应形如 https://api.example.com/v1）",
                    endpoint.base_url
                ),
            )
        } else {
            (
                ErrorKind::Connect,
                format!("连不上 {host}：检查接口地址、网络和代理"),
            )
        };
        ModelError::new(kind, call, message).with_detail(Some(endpoint.scrub(&error_chain(error))))
    }

    fn bad_response(call: Call, endpoint: &Endpoint, what: &str, body: &str) -> Self {
        ModelError::new(
            ErrorKind::BadResponse,
            call,
            format!("返回的内容不是预期的格式（{what}）：确认接口地址指向 OpenAI 兼容接口"),
        )
        .with_detail(Some(snippet(&endpoint.scrub(body))))
    }

    /// 转写结果里没有 `segments`。
    pub fn no_segments(model: &str) -> Self {
        ModelError::new(
            ErrorKind::NoSegments,
            Call::Transcription,
            format!(
                "转写模型 {model} 没有返回分句时间戳（segments）：候选的边界会粗一些；要精确边界就换 whisper-1 或自建的 whisper 服务"
            ),
        )
    }
}

/// chat 的一条用户消息里的一张图（data URL 或 http 地址）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInput {
    pub url: String,
}

impl ImageInput {
    pub fn jpeg(bytes: &[u8]) -> Self {
        Self::data("image/jpeg", bytes)
    }

    pub fn png(bytes: &[u8]) -> Self {
        Self::data("image/png", bytes)
    }

    fn data(mime: &str, bytes: &[u8]) -> Self {
        use base64::Engine;
        ImageInput {
            url: format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
        }
    }
}

#[derive(bon::Builder, Debug, Clone)]
pub struct ChatRequest {
    #[builder(into)]
    pub system: String,
    #[builder(into)]
    pub user: String,
    #[builder(default)]
    pub images: Vec<ImageInput>,
    /// 要求 `response_format = json_object`；服务不支持时自动去掉再试一次
    #[builder(default)]
    pub json: bool,
    #[builder(default = 0.2)]
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatReply {
    pub content: String,
    pub usage: Usage,
    /// 这次是否用上了 JSON 模式（服务不支持时为 `false`）
    pub json_mode: bool,
}

impl ChatReply {
    /// 回复里的 JSON：整段是 JSON 就用整段，否则取第一个 `{…}` 块。
    pub fn json(&self) -> Option<Value> {
        extract_json(&self.content)
    }
}

/// 一块要转写的音频。
#[derive(Debug, Clone)]
pub struct AudioFile {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub mime: String,
}

#[derive(bon::Builder, Debug, Clone, Default)]
pub struct TranscribeOptions {
    pub language: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub text: String,
    /// 没有分句时间戳的模型为 `None`
    pub segments: Option<Vec<TranscriptSegment>>,
}

impl Transcript {
    pub fn require_segments(&self, model: &str) -> Result<&[TranscriptSegment], ModelError> {
        self.segments
            .as_deref()
            .ok_or_else(|| ModelError::no_segments(model))
    }
}

pub struct ModelClient {
    http: reqwest::Client,
    options: ClientOptions,
}

impl ModelClient {
    pub fn new(options: ClientOptions) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(options.connect_timeout)
            .build()
            .unwrap_or_default();
        ModelClient { http, options }
    }

    pub async fn chat(
        &self,
        endpoint: &Endpoint,
        request: &ChatRequest,
    ) -> Result<ChatReply, ModelError> {
        match self.chat_once(endpoint, request, request.json).await {
            Err(error)
                if request.json && rejects_option(&error, &["response_format", "json_object"]) =>
            {
                self.chat_once(endpoint, request, false).await
            }
            result => result,
        }
    }

    async fn chat_once(
        &self,
        endpoint: &Endpoint,
        request: &ChatRequest,
        json_mode: bool,
    ) -> Result<ChatReply, ModelError> {
        let content = if request.images.is_empty() {
            json!(request.user)
        } else {
            let mut parts = vec![json!({"type": "text", "text": request.user})];
            parts.extend(request.images.iter().map(|image| {
                json!({"type": "image_url", "image_url": {"url": image.url, "detail": "low"}})
            }));
            Value::Array(parts)
        };
        let mut body = json!({
            "model": endpoint.model,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": content},
            ],
            "temperature": request.temperature,
        });
        if json_mode {
            body["response_format"] = json!({"type": "json_object"});
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        let text = self
            .send(Call::Chat, endpoint, self.options.chat_timeout, || {
                Ok(self.http.post(endpoint.url(Call::Chat.path())).json(&body))
            })
            .await?;

        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
            #[serde(default)]
            usage: Option<Usage>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }
        #[derive(Deserialize)]
        struct Message {
            #[serde(default)]
            content: Option<String>,
        }
        let parsed: Response = serde_json::from_str(&text)
            .map_err(|_| ModelError::bad_response(Call::Chat, endpoint, "没有 choices", &text))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .ok_or_else(|| {
                ModelError::bad_response(Call::Chat, endpoint, "choices 为空或没有文字", &text)
            })?;
        Ok(ChatReply {
            content,
            usage: parsed.usage.unwrap_or_default(),
            json_mode,
        })
    }

    /// 转写一块音频。优先要 `verbose_json` 的分句时间戳；服务不认这个格式时退回 `json`，
    /// 此时 [`Transcript::segments`] 为 `None`。
    pub async fn transcribe(
        &self,
        endpoint: &Endpoint,
        audio: &AudioFile,
        options: &TranscribeOptions,
    ) -> Result<Transcript, ModelError> {
        match self.transcribe_once(endpoint, audio, options, true).await {
            Err(error)
                if rejects_option(
                    &error,
                    &["response_format", "verbose_json", "timestamp_granularities"],
                ) =>
            {
                self.transcribe_once(endpoint, audio, options, false).await
            }
            result => result,
        }
    }

    async fn transcribe_once(
        &self,
        endpoint: &Endpoint,
        audio: &AudioFile,
        options: &TranscribeOptions,
        verbose: bool,
    ) -> Result<Transcript, ModelError> {
        let text = self
            .send(
                Call::Transcription,
                endpoint,
                self.options.asr_timeout,
                || {
                    let file = Part::bytes(audio.bytes.clone())
                        .file_name(audio.file_name.clone())
                        .mime_str(&audio.mime)
                        .map_err(|_| {
                            ModelError::new(
                                ErrorKind::BadRequest,
                                Call::Transcription,
                                format!("音频类型 {} 不合法", audio.mime),
                            )
                        })?;
                    let mut form = Form::new()
                        .part("file", file)
                        .text("model", endpoint.model.clone());
                    form = if verbose {
                        form.text("response_format", "verbose_json")
                            .text("timestamp_granularities[]", "segment")
                    } else {
                        form.text("response_format", "json")
                    };
                    if let Some(language) = &options.language {
                        form = form.text("language", language.clone());
                    }
                    if let Some(prompt) = &options.prompt {
                        form = form.text("prompt", prompt.clone());
                    }
                    Ok(self
                        .http
                        .post(endpoint.url(Call::Transcription.path()))
                        .multipart(form))
                },
            )
            .await?;

        #[derive(Deserialize)]
        struct Response {
            text: String,
            #[serde(default)]
            segments: Option<Vec<TranscriptSegment>>,
        }
        let parsed: Response = serde_json::from_str(&text).map_err(|_| {
            ModelError::bad_response(Call::Transcription, endpoint, "没有 text", &text)
        })?;
        Ok(Transcript {
            text: parsed.text,
            segments: parsed.segments,
        })
    }

    /// 发请求并按规则重试，返回 2xx 的响应体。`build` 每次重试都会重新调用（multipart 不能复用）。
    async fn send(
        &self,
        call: Call,
        endpoint: &Endpoint,
        timeout: Duration,
        build: impl Fn() -> Result<reqwest::RequestBuilder, ModelError>,
    ) -> Result<String, ModelError> {
        let mut attempt = 0;
        loop {
            let mut request = build()?.timeout(timeout);
            if let Some(key) = &endpoint.api_key {
                request = request.bearer_auth(key);
            }
            let (error, retry_after) = match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    let retry_after = retry_after(response.headers());
                    match response.text().await {
                        Ok(body) if status.is_success() => return Ok(body),
                        Ok(body) => (
                            ModelError::from_status(call, endpoint, status, retry_after, &body),
                            retry_after,
                        ),
                        Err(error) => (
                            ModelError::from_transport(call, endpoint, timeout, &error),
                            retry_after,
                        ),
                    }
                }
                Err(error) => (
                    ModelError::from_transport(call, endpoint, timeout, &error),
                    None,
                ),
            };
            let Some(backoff) = self.options.backoff.get(attempt).copied() else {
                return Err(error);
            };
            if !error.kind.retryable() {
                return Err(error);
            }
            if retry_after.is_some_and(|wait| wait > self.options.max_retry_after) {
                return Err(error);
            }
            let wait = retry_after.map_or(backoff, |wait| wait.max(backoff));
            warn!(
                base_url = %endpoint.base_url,
                model = %endpoint.model,
                status = ?error.status,
                kind = ?error.kind,
                wait_secs = wait.as_secs_f32(),
                "模型接口调用失败，稍后重试"
            );
            tokio::time::sleep(wait).await;
            attempt += 1;
        }
    }
}

/// 400 / 422 且响应里提到了这些选项：说明是服务不认这个参数，而不是别的问题。
fn rejects_option(error: &ModelError, words: &[&str]) -> bool {
    matches!(error.status, Some(400) | Some(422))
        && error
            .detail
            .as_deref()
            .is_some_and(|detail| words.iter().any(|word| detail.contains(word)))
}

/// `Retry-After`：秒数或 HTTP 日期。
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let at = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let secs = (at.timestamp() - chrono::Utc::now().timestamp()).max(0);
    Some(Duration::from_secs(secs as u64))
}

fn snippet(text: &str) -> String {
    let text = text.trim();
    let mut out: String = text.chars().take(DETAIL_CHARS).collect();
    if text.chars().count() > DETAIL_CHARS {
        out.push('…');
    }
    out
}

fn error_chain(error: &reqwest::Error) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(inner) = source {
        text.push('：');
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

/// 整段是 JSON 就用整段；否则取第一个配平的 `{…}` 块（模型常在 JSON 外面包 ```json 或说明文字）。
pub fn extract_json(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(text.trim()) {
        return Some(value);
    }
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(offset) = text[start..].find('{') {
        let open = start + offset;
        let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
        for (index, &byte) in bytes.iter().enumerate().skip(open) {
            if in_string {
                match byte {
                    _ if escaped => escaped = false,
                    b'\\' => escaped = true,
                    b'"' => in_string = false,
                    _ => {}
                }
                continue;
            }
            match byte {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Ok(value) = serde_json::from_str(&text[open..=index]) {
                            return Some(value);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
        start = open + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::fake::{FakeServer, Scenario};
    use super::*;

    const KEY: &str = "sk-test-0123456789abcdef";

    fn quick() -> ClientOptions {
        ClientOptions::builder()
            .chat_timeout(Duration::from_millis(800))
            .asr_timeout(Duration::from_millis(800))
            .backoff(vec![Duration::from_millis(10), Duration::from_millis(10)])
            .max_retry_after(Duration::from_secs(5))
            .build()
    }

    fn ask() -> ChatRequest {
        ChatRequest::builder()
            .system("只输出 JSON")
            .user("回复 {\"ok\": true}")
            .json(true)
            .build()
    }

    fn audio() -> AudioFile {
        AudioFile {
            bytes: super::super::probe::silent_wav(Duration::from_millis(200)),
            file_name: "probe.wav".into(),
            mime: "audio/wav".into(),
        }
    }

    async fn server(scenario: Scenario) -> (FakeServer, Endpoint) {
        let server = FakeServer::start(scenario).await;
        let endpoint = Endpoint::new(server.base_url(), "test-model").with_key(Some(KEY.into()));
        (server, endpoint)
    }

    #[tokio::test]
    async fn chat_success_sends_the_key_and_reads_the_reply() {
        let (server, endpoint) = server(Scenario::Ok).await;
        let reply = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap();
        assert_eq!(reply.json().unwrap()["ok"], true);
        assert!(reply.json_mode);
        assert_eq!(reply.usage.prompt_tokens, 12);
        let seen = server.requests();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].path, "/v1/chat/completions");
        assert_eq!(
            seen[0].authorization.as_deref(),
            Some(&*format!("Bearer {KEY}"))
        );
        assert_eq!(seen[0].body["response_format"]["type"], "json_object");
    }

    #[tokio::test]
    async fn errors_that_retrying_cannot_fix_fail_at_once_with_advice() {
        for (scenario, kind, status, advice) in [
            (
                Scenario::Unauthorized,
                ErrorKind::Unauthorized,
                401,
                "API key 无效",
            ),
            (Scenario::NotFound, ErrorKind::NotFound, 404, "/v1 结尾"),
            (Scenario::TooLarge, ErrorKind::PayloadTooLarge, 413, "太大"),
        ] {
            let (server, endpoint) = server(scenario).await;
            let error = ModelClient::new(quick())
                .chat(&endpoint, &ask())
                .await
                .unwrap_err();
            assert_eq!(error.kind, kind, "{error}");
            assert_eq!(error.status, Some(status));
            assert!(error.message.contains(advice), "{error}");
            assert_eq!(server.requests().len(), 1, "{scenario:?} 不应重试");
            let text = error.to_string();
            assert!(!text.contains(KEY), "key 出现在错误里：{text}");
        }
    }

    #[tokio::test]
    async fn a_key_echoed_by_the_server_is_masked() {
        let (_server, endpoint) = server(Scenario::Unauthorized).await;
        let error = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap_err();
        let text = error.to_string();
        assert!(text.contains("sk-…cdef"), "{text}");
        assert!(!text.contains(KEY), "{text}");
        assert!(!format!("{endpoint:?}").contains(KEY));
    }

    #[tokio::test]
    async fn rate_limits_wait_for_retry_after_then_succeed() {
        let (server, endpoint) = server(Scenario::RateLimitedOnce { retry_after: 1 }).await;
        let started = std::time::Instant::now();
        let reply = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap();
        assert_eq!(reply.json().unwrap()["ok"], true);
        assert_eq!(server.requests().len(), 2);
        assert!(
            started.elapsed() >= Duration::from_millis(950),
            "应等满 Retry-After，实际 {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn a_retry_after_beyond_the_cap_is_reported_instead_of_waited() {
        let (server, endpoint) = server(Scenario::RateLimitedOnce { retry_after: 600 }).await;
        let error = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::RateLimited);
        assert_eq!(error.retry_after_secs, Some(600));
        assert!(error.message.contains("600 秒后再试"), "{error}");
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn server_errors_are_retried_a_limited_number_of_times() {
        let (server, endpoint) = server(Scenario::ServerError).await;
        let error = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Server);
        assert_eq!(error.status, Some(500));
        assert!(error.message.contains("稍后再试"), "{error}");
        // 首次 + backoff 两次
        assert_eq!(server.requests().len(), 3);
    }

    #[tokio::test]
    async fn slow_servers_time_out() {
        let (server, endpoint) = server(Scenario::Slow { secs: 5 }).await;
        let options = ClientOptions {
            backoff: vec![Duration::from_millis(10)],
            ..quick()
        };
        let error = ModelClient::new(options)
            .chat(&endpoint, &ask())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Timeout, "{error}");
        assert!(error.message.contains("超时"), "{error}");
        assert_eq!(server.requests().len(), 2, "超时要重试");
    }

    #[tokio::test]
    async fn an_unreachable_host_is_a_connect_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let endpoint = Endpoint::new(format!("http://127.0.0.1:{port}/v1"), "m");
        let options = ClientOptions {
            backoff: vec![],
            ..quick()
        };
        let error = ModelClient::new(options)
            .chat(&endpoint, &ask())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Connect, "{error}");
        assert!(
            error.message.contains(&format!("连不上 127.0.0.1:{port}")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn json_mode_is_dropped_when_the_server_rejects_it() {
        let (server, endpoint) = server(Scenario::NoJsonMode).await;
        let reply = ModelClient::new(quick())
            .chat(&endpoint, &ask())
            .await
            .unwrap();
        assert!(!reply.json_mode);
        assert_eq!(reply.json().unwrap()["ok"], true, "从包裹文字里取出 JSON");
        let seen = server.requests();
        assert_eq!(seen.len(), 2);
        assert!(seen[1].body.get("response_format").is_none());
    }

    #[tokio::test]
    async fn transcription_returns_segments() {
        let (server, endpoint) = server(Scenario::Ok).await;
        let options = TranscribeOptions::builder().language("zh".into()).build();
        let transcript = ModelClient::new(quick())
            .transcribe(&endpoint, &audio(), &options)
            .await
            .unwrap();
        let segments = transcript.require_segments("test-model").unwrap();
        assert_eq!(segments.len(), 1);
        let seen = server.requests();
        assert_eq!(seen[0].path, "/v1/audio/transcriptions");
        assert_eq!(
            seen[0].form.get("response_format").map(String::as_str),
            Some("verbose_json")
        );
        assert_eq!(seen[0].form.get("language").map(String::as_str), Some("zh"));
        assert_eq!(
            seen[0].form.get("file").map(String::as_str),
            Some("probe.wav")
        );
    }

    #[tokio::test]
    async fn a_transcript_without_segments_explains_the_consequence() {
        let (_server, endpoint) = server(Scenario::NoSegments).await;
        let transcript = ModelClient::new(quick())
            .transcribe(&endpoint, &audio(), &TranscribeOptions::default())
            .await
            .unwrap();
        assert_eq!(transcript.segments, None);
        let error = transcript.require_segments("test-model").unwrap_err();
        assert_eq!(error.kind, ErrorKind::NoSegments);
        assert!(error.message.contains("分句时间戳"), "{error}");
    }

    #[tokio::test]
    async fn verbose_json_falls_back_to_json_when_rejected() {
        let (server, endpoint) = server(Scenario::NoVerboseJson).await;
        let transcript = ModelClient::new(quick())
            .transcribe(&endpoint, &audio(), &TranscribeOptions::default())
            .await
            .unwrap();
        assert_eq!(transcript.segments, None);
        let seen = server.requests();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[1].form.get("response_format").map(String::as_str),
            Some("json")
        );
    }

    #[test]
    fn retry_after_accepts_seconds_and_dates() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, "7".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(7)));
        let later = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc2822();
        headers.insert(RETRY_AFTER, later.parse().unwrap());
        let secs = retry_after(&headers).unwrap().as_secs();
        assert!((28..=30).contains(&secs), "{secs}");
        headers.insert(RETRY_AFTER, "soon".parse().unwrap());
        assert_eq!(retry_after(&headers), None);
    }

    #[test]
    fn json_is_found_inside_wrapping_text() {
        assert_eq!(extract_json("{\"a\":1}").unwrap()["a"], 1);
        let wrapped = "好的：\n```json\n{\"a\": \"}{\", \"b\": {\"c\": 2}}\n```";
        assert_eq!(extract_json(wrapped).unwrap()["b"]["c"], 2);
        assert_eq!(
            extract_json("{坏的} 然后 {\"ok\":true}").unwrap()["ok"],
            true
        );
        assert_eq!(extract_json("没有 JSON"), None);
    }
}
