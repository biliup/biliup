use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    RuntimeOptions, StreamlinkOptions, StreamlinkPlatform,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

pub struct Niconico {
    re: Regex,
}

impl Default for Niconico {
    fn default() -> Self {
        Self::new()
    }
}

impl Niconico {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:(?:www|m|live)\.)?nicovideo\.jp").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Niconico {
    fn name(&self) -> &'static str {
        "Niconico"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        NiconicoLive::new(request).check_stream().await
    }
}

struct NiconicoLive {
    client: reqwest::Client,
    url: String,
    name: String,
    email: Option<String>,
    password: Option<String>,
    user_session: Option<String>,
    purge_credentials: Option<String>,
}

impl NiconicoLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            email: request.credentials.niconico_email,
            password: request.credentials.niconico_password,
            user_session: request.credentials.niconico_user_session,
            purge_credentials: request.credentials.niconico_purge_credentials,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        if !self.streamlink_available().await? {
            return Ok(LiveStatus::Offline);
        }
        let title = self.title().await;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: String::new(),
                raw_stream_url: self.url.clone(),
                platform: "niconico".to_string(),
                stream_headers: HashMap::new(),
                suffix: "ts".to_string(),
                danmaku: None,
                downloader_hint: DownloaderHint::Streamlink,
                runtime_options: Some(RuntimeOptions::Streamlink(StreamlinkOptions {
                    url: Some(self.url.clone()),
                    platform: StreamlinkPlatform::Niconico {
                        email: self.email.clone(),
                        password: self.password.clone(),
                        user_session: self.user_session.clone(),
                        purge_credentials: self.purge_credentials.clone(),
                    },
                })),
            }),
        })
    }

    async fn streamlink_available(&self) -> LiveResult<bool> {
        let mut command = Command::new("streamlink");
        command.stdin(Stdio::null()).arg("--stream-url");
        self.apply_streamlink_args(&mut command);
        let output = command
            .arg(&self.url)
            .arg("best")
            .output()
            .await
            .map_err(|err| {
                LiveError::custom(format!(
                    "运行 Niconico streamlink 失败，请确认已安装并在 PATH 中: {err}"
                ))
            })?;

        Ok(output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty())
    }

    fn apply_streamlink_args(&self, command: &mut Command) {
        if let Some(email) = self.email.as_deref().filter(|value| !value.is_empty()) {
            command.arg("--niconico-email").arg(email);
        }
        if let Some(password) = self.password.as_deref().filter(|value| !value.is_empty()) {
            command.arg("--niconico-password").arg(password);
        }
        if let Some(user_session) = self
            .user_session
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            command.arg("--niconico-user-session").arg(user_session);
        }
        if let Some(purge_credentials) = self
            .purge_credentials
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            command
                .arg("--niconico-purge-credentials")
                .arg(purge_credentials);
        }
    }

    async fn title(&self) -> String {
        let Ok(response) = self.client.get(&self.url).send().await else {
            return self.name.clone();
        };
        let Ok(text) = response.text().await else {
            return self.name.clone();
        };
        if let Some(title) = Regex::new(r#""name":"(.*?)","description":"(.*?)""#)
            .unwrap()
            .captures(&text)
            .map(|captures| captures[1].to_string())
        {
            return title;
        }
        extract_json_string(&text, "name").unwrap_or_else(|| self.name.clone())
    }
}

fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    value.get(key).and_then(Value::as_str).map(str::to_string)
}
