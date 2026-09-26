//! 测试用的假 OpenAI 兼容服务：按场景返回成功、各类错误、限流、慢响应、没有分句时间戳等。
//!
//! 测试里用 [`FakeServer::start`] 固定一个场景；手工点界面时用 [`Scenario::ByModel`]，
//! 按请求里的模型名选场景（见 [`Scenario::from_model`]），一个服务就能演示所有情况：
//! `FAKE_OPENAI_ADDR=127.0.0.1:18080 cargo test -p biliup-cli serve_fake_openai -- --ignored`

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Ok,
    Unauthorized,
    NotFound,
    TooLarge,
    /// 第一次 429 + `Retry-After`，之后正常
    RateLimitedOnce {
        retry_after: u64,
    },
    ServerError,
    Slow {
        secs: u64,
    },
    /// 每个请求都等这么久再正常返回
    Delay {
        millis: u64,
    },
    /// 前 `ok` 个请求正常返回，之后都是 500
    FailAfter {
        ok: usize,
    },
    /// 转写只给 `text`，没有 `segments`
    NoSegments,
    /// 转写不认 `verbose_json`（400），`json` 可以
    NoVerboseJson,
    /// chat 不认 `response_format`（400），回复把 JSON 包在说明文字里
    NoJsonMode,
    /// chat 带图片时 400
    NoVision,
    /// 按请求里的模型名选场景
    ByModel,
}

impl Scenario {
    pub fn from_model(model: &str) -> Scenario {
        match model {
            "unauthorized" => Scenario::Unauthorized,
            "not-found" => Scenario::NotFound,
            "too-large" => Scenario::TooLarge,
            "rate-limited" => Scenario::RateLimitedOnce { retry_after: 2 },
            "rate-limited-long" => Scenario::RateLimitedOnce { retry_after: 600 },
            "server-error" => Scenario::ServerError,
            "slow" => Scenario::Slow { secs: 40 },
            "no-segments" => Scenario::NoSegments,
            "no-verbose-json" => Scenario::NoVerboseJson,
            "no-json-mode" => Scenario::NoJsonMode,
            "no-vision" => Scenario::NoVision,
            _ => Scenario::Ok,
        }
    }
}

/// 服务收到的一个请求。
#[derive(Debug, Clone)]
pub struct Seen {
    pub path: String,
    pub authorization: Option<String>,
    /// chat 的 JSON 请求体
    pub body: Value,
    /// transcriptions 的表单字段；`file` 记的是文件名
    pub form: HashMap<String, String>,
    /// 上传的 FLAC 的时长（毫秒），读不出来为空
    pub audio_ms: Option<i64>,
}

#[derive(Clone)]
struct Shared {
    scenario: Scenario,
    seen: Arc<Mutex<Vec<Seen>>>,
}

pub struct FakeServer {
    base_url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    task: tokio::task::JoinHandle<()>,
}

impl FakeServer {
    pub async fn start(scenario: Scenario) -> FakeServer {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        Self::serve(listener, scenario)
    }

    fn serve(listener: tokio::net::TcpListener, scenario: Scenario) -> FakeServer {
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/v1/chat/completions", post(chat))
            .route("/v1/audio/transcriptions", post(transcriptions))
            .with_state(Shared {
                scenario,
                seen: seen.clone(),
            });
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        FakeServer {
            base_url: format!("http://{addr}/v1"),
            seen,
            task,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn chat(State(shared): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let model = body["model"].as_str().unwrap_or_default().to_string();
    let has_image = body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|message| message["content"].is_array());
    let json_mode = body.get("response_format").is_some();
    let (scenario, attempt) = record(
        &shared,
        "/v1/chat/completions",
        &headers,
        body,
        HashMap::new(),
        None,
        &model,
    );
    if let Some(response) = common_failure(scenario, attempt, &headers).await {
        return response;
    }
    match scenario {
        Scenario::NoJsonMode if json_mode => {
            return error(
                StatusCode::BAD_REQUEST,
                "response_format json_object is not supported by this model",
            );
        }
        Scenario::NoVision if has_image => {
            return error(
                StatusCode::BAD_REQUEST,
                "Invalid content type. image_url is only supported by certain models.",
            );
        }
        _ => {}
    }
    let content = if has_image {
        "{\"color\": \"红色\"}".to_string()
    } else if json_mode {
        "{\"ok\": true}".to_string()
    } else {
        "好的：\n```json\n{\"ok\": true}\n```".to_string()
    };
    axum::Json(json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion",
        "model": model,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 12, "completion_tokens": 5, "total_tokens": 17},
    }))
    .into_response()
}

async fn transcriptions(State(shared): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let form = parse_multipart(&headers, &body);
    let model = form.get("model").cloned().unwrap_or_default();
    let verbose = form.get("response_format").map(String::as_str) == Some("verbose_json");
    let audio_ms =
        file_bytes(&headers, &body).and_then(|file| super::audio::flac_duration_ms(&file));
    let (scenario, attempt) = record(
        &shared,
        "/v1/audio/transcriptions",
        &headers,
        Value::Null,
        form,
        audio_ms,
        &model,
    );
    if let Some(response) = common_failure(scenario, attempt, &headers).await {
        return response;
    }
    match scenario {
        Scenario::NoVerboseJson if verbose => error(
            StatusCode::BAD_REQUEST,
            "response_format 'verbose_json' is not compatible with this model",
        ),
        Scenario::NoSegments | Scenario::NoVerboseJson => {
            axum::Json(json!({"text": "测试"})).into_response()
        }
        _ if verbose && audio_ms.is_some() => {
            axum::Json(sentences(audio_ms.unwrap_or_default())).into_response()
        }
        _ if verbose => axum::Json(json!({
            "task": "transcribe",
            "language": "chinese",
            "duration": 2.0,
            "text": "测试",
            "segments": [{"id": 0, "start": 0.0, "end": 2.0, "text": "测试"}],
        }))
        .into_response(),
        _ => axum::Json(json!({"text": "测试"})).into_response(),
    }
}

/// 记下请求，返回这次生效的场景和这是同一场景的第几次请求（从 0 数）。
fn record(
    shared: &Shared,
    path: &str,
    headers: &HeaderMap,
    body: Value,
    form: HashMap<String, String>,
    audio_ms: Option<i64>,
    model: &str,
) -> (Scenario, usize) {
    let scenario = match shared.scenario {
        Scenario::ByModel => Scenario::from_model(model),
        fixed => fixed,
    };
    let mut seen = shared.seen.lock().unwrap();
    let attempt = seen
        .iter()
        .filter(|earlier| {
            earlier.body["model"]
                .as_str()
                .or(earlier.form.get("model").map(String::as_str))
                == Some(model)
        })
        .count();
    seen.push(Seen {
        path: path.to_string(),
        authorization: headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        body,
        form,
        audio_ms,
    });
    (scenario, attempt)
}

async fn common_failure(
    scenario: Scenario,
    attempt: usize,
    headers: &HeaderMap,
) -> Option<Response> {
    Some(match scenario {
        Scenario::Unauthorized => {
            // 像 OpenAI 一样在错误里回显 key，客户端要把它换成掩码
            let key = headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .unwrap_or_default()
                .to_string();
            error(
                StatusCode::UNAUTHORIZED,
                &format!("Incorrect API key provided: {key}"),
            )
        }
        Scenario::NotFound => error(StatusCode::NOT_FOUND, "The model does not exist"),
        Scenario::TooLarge => error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Maximum content size limit exceeded",
        ),
        Scenario::RateLimitedOnce { retry_after } if attempt == 0 => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry_after.to_string())],
            axum::Json(json!({"error": {"message": "Rate limit reached", "type": "requests"}})),
        )
            .into_response(),
        Scenario::ServerError => {
            error(StatusCode::INTERNAL_SERVER_ERROR, "The server had an error")
        }
        Scenario::Slow { secs } => {
            tokio::time::sleep(Duration::from_secs(secs)).await;
            return None;
        }
        Scenario::Delay { millis } => {
            tokio::time::sleep(Duration::from_millis(millis)).await;
            return None;
        }
        Scenario::FailAfter { ok } if attempt >= ok => {
            error(StatusCode::INTERNAL_SERVER_ERROR, "The server had an error")
        }
        _ => return None,
    })
}

/// 按上传音频的时长每 2 秒给一句，文字是这句在块内的起点（秒），测试据此核对时间换算。
fn sentences(audio_ms: i64) -> Value {
    let mut segments = Vec::new();
    let mut start = 0;
    while start < audio_ms {
        let end = (start + 2000).min(audio_ms);
        segments.push(json!({
            "id": segments.len(),
            "start": start as f64 / 1000.0,
            "end": end as f64 / 1000.0,
            "text": format!("块内{}秒", start / 1000),
        }));
        start = end;
    }
    let text = segments
        .iter()
        .map(|segment| segment["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("");
    json!({
        "task": "transcribe",
        "language": "chinese",
        "duration": audio_ms as f64 / 1000.0,
        "text": text,
        "segments": segments,
    })
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(json!({"error": {"message": message}}))).into_response()
}

/// 够测试用的 multipart 解析：文本字段取值，文件字段记文件名。
fn parse_multipart(headers: &HeaderMap, body: &[u8]) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let Some(boundary) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split("boundary=").nth(1))
        .map(|boundary| format!("--{}", boundary.trim_matches('"')))
    else {
        return fields;
    };
    let text = String::from_utf8_lossy(body);
    for part in text.split(boundary.as_str()) {
        let Some((head, value)) = part.split_once("\r\n\r\n") else {
            continue;
        };
        let attribute = |name: &str| {
            head.split(&format!("{name}=\""))
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .map(str::to_string)
        };
        let Some(name) = attribute("name") else {
            continue;
        };
        let value = match attribute("filename") {
            Some(file_name) => file_name,
            None => value.trim_end_matches("\r\n").to_string(),
        };
        fields.insert(name, value);
    }
    fields
}

/// multipart 里文件字段的原始字节。
fn file_bytes(headers: &HeaderMap, body: &[u8]) -> Option<Vec<u8>> {
    let boundary = headers
        .get(header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split("boundary=")
        .nth(1)?
        .trim_matches('"')
        .to_string();
    let find = |haystack: &[u8], needle: &[u8], from: usize| {
        haystack
            .get(from..)?
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|at| at + from)
    };
    let name = find(body, b"filename=\"", 0)?;
    let start = find(body, b"\r\n\r\n", name)? + 4;
    let end = find(body, format!("\r\n--{boundary}").as_bytes(), start)?;
    Some(body[start..end].to_vec())
}

/// 手工点界面用：按模型名选场景，一直跑到进程被停掉。
#[tokio::test]
#[ignore = "手工测试用的常驻服务"]
async fn serve_fake_openai() {
    let addr = std::env::var("FAKE_OPENAI_ADDR").unwrap_or_else(|_| "127.0.0.1:18080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let server = FakeServer::serve(listener, Scenario::ByModel);
    eprintln!("fake OpenAI server on {}", server.base_url());
    std::future::pending::<()>().await;
}
