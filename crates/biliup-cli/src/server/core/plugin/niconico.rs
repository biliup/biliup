use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{Report, ResultExt};
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Mutex;
use tokio::process::{Child, Command};
use tokio::time::{Duration, sleep};

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

impl DownloadPlugin for Niconico {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let user = ctx.config().user.clone().unwrap_or_default();
        Box::new(NiconicoDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            user.niconico_email,
            user.niconico_password,
            user.niconico_user_session,
            user.niconico_purge_credentials,
        ))
    }

    fn name(&self) -> &str {
        "Niconico"
    }
}

struct NiconicoDownloader {
    client: Client,
    url: String,
    name: String,
    email: Option<String>,
    password: Option<String>,
    user_session: Option<String>,
    purge_credentials: Option<String>,
    process: Mutex<Option<Child>>,
}

impl NiconicoDownloader {
    fn new(
        client: Client,
        url: String,
        name: String,
        email: Option<String>,
        password: Option<String>,
        user_session: Option<String>,
        purge_credentials: Option<String>,
    ) -> Self {
        Self {
            client,
            url,
            name,
            email,
            password,
            user_session,
            purge_credentials,
            process: Mutex::new(None),
        }
    }

    async fn title(&self) -> String {
        let Ok(response) = self.client.get(&self.url).send().await else {
            return self.name.clone();
        };
        let Ok(text) = response.text().await else {
            return self.name.clone();
        };
        Regex::new(r#""name":"(.*?)","description":"(.*?)""#)
            .unwrap()
            .captures(&text)
            .map(|captures| captures[1].to_string())
            .unwrap_or_else(|| self.name.clone())
    }

    fn stop_streamlink(&self) {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.start_kill();
        }
    }

    fn streamlink_command(&self, port: u16) -> Command {
        let mut command = Command::new("streamlink");
        command.stdin(Stdio::null()).stdout(Stdio::null());

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

        command.args([
            "--player-external-http",
            "--player-external-http-port",
            &port.to_string(),
            "--player-external-http-interface",
            "localhost",
            &self.url,
            "best",
        ]);
        command
    }

    async fn start_streamlink(&self) -> Result<Option<u16>, Report<AppError>> {
        self.stop_streamlink();
        let port = rand::thread_rng().gen_range(1025..=65535);
        let mut child = self
            .streamlink_command(port)
            .spawn()
            .change_context(AppError::Custom(
                "启动 Niconico streamlink 失败".to_string(),
            ))?;

        for _ in 0..5 {
            if child
                .try_wait()
                .change_context(AppError::Custom(
                    "检查 Niconico streamlink 状态失败".to_string(),
                ))?
                .is_some()
            {
                return Ok(None);
            }
            sleep(Duration::from_secs(1)).await;
        }

        *self.process.lock().unwrap() = Some(child);
        Ok(Some(port))
    }
}

#[async_trait]
impl DownloadBase for NiconicoDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let title = self.title().await;
        let Some(port) = self.start_streamlink().await? else {
            return Ok(StreamStatus::Offline);
        };

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: "flv".to_string(),
                raw_stream_url: format!("http://localhost:{port}"),
                platform: "niconico".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

impl Drop for NiconicoDownloader {
    fn drop(&mut self) {
        self.stop_streamlink();
    }
}
