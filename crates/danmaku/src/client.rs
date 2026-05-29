//! Core danmaku client implementation.
//!
//! The [`DanmakuRecorder`] manages WebSocket connections, heartbeats,
//! message processing, and XML output for recording live stream chat.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::interval;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tracing::{debug, error, info, warn};

use crate::error::{DanmakuError, Result};
use crate::output::xml::{XmlWriter, XmlWriterConfig};
use crate::protocols::{
    HeartbeatData, Platform, PlatformContext, RegistrationData, create_platform,
};

/// Configuration for the danmaku recorder.
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    /// The live stream URL.
    pub url: String,
    /// Output file path template.
    pub output_file: PathBuf,
    /// Platform-specific context.
    pub context: PlatformContext,
    /// Whether to save raw message data.
    pub save_raw: bool,
    /// Whether to save detailed info.
    pub save_detail: bool,
}

impl RecorderConfig {
    /// Create a new recorder config.
    pub fn new(url: impl Into<String>, output_file: impl AsRef<Path>) -> Self {
        Self {
            url: url.into(),
            output_file: output_file.as_ref().to_path_buf(),
            context: PlatformContext::new(),
            save_raw: false,
            save_detail: false,
        }
    }

    /// Set the platform context.
    pub fn with_context(mut self, context: PlatformContext) -> Self {
        self.context = context;
        self
    }

    /// Enable raw data saving.
    pub fn with_raw(mut self, save_raw: bool) -> Self {
        self.save_raw = save_raw;
        self
    }

    /// Enable detailed info saving.
    pub fn with_detail(mut self, save_detail: bool) -> Self {
        self.save_detail = save_detail;
        self
    }
}

/// Commands that can be sent to the recorder.
#[derive(Debug)]
enum RecorderCommand {
    /// Save current file and optionally rename.
    Rolling {
        new_file_name: Option<PathBuf>,
        done: oneshot::Sender<Result<()>>,
    },
    /// Stop recording.
    Stop,
}

/// Handle for controlling a running recorder.
#[derive(Clone)]
pub struct RecorderHandle {
    cmd_tx: mpsc::Sender<RecorderCommand>,
    stop_tx: watch::Sender<bool>,
}

impl RecorderHandle {
    /// Stop the recorder.
    pub async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        let _ = self.cmd_tx.send(RecorderCommand::Stop).await;
        Ok(())
    }

    /// Save current recording and optionally rename the file.
    pub async fn rolling(&self, new_file_name: Option<PathBuf>) -> Result<()> {
        let (done, rx) = oneshot::channel();
        self.cmd_tx
            .send(RecorderCommand::Rolling {
                new_file_name,
                done,
            })
            .await
            .map_err(|_| DanmakuError::ChannelSend)?;
        rx.await.map_err(|_| DanmakuError::ChannelSend)??;
        Ok(())
    }
}

/// Danmaku recorder that manages the recording lifecycle.
pub struct DanmakuRecorder {
    config: RecorderConfig,
    platform: Arc<dyn Platform>,
}

impl DanmakuRecorder {
    /// Create a new recorder for the given URL.
    pub fn new(config: RecorderConfig) -> Result<Self> {
        let platform = create_platform(&config.url)?;
        Ok(Self {
            config,
            platform: Arc::from(platform),
        })
    }

    /// Start recording in a background task.
    ///
    /// Returns a handle that can be used to control the recorder.
    pub fn start(self) -> RecorderHandle {
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        let (stop_tx, stop_rx) = watch::channel(false);

        let handle = RecorderHandle {
            cmd_tx,
            stop_tx: stop_tx.clone(),
        };

        tokio::spawn(async move {
            if let Err(e) = self.run(cmd_rx, stop_rx).await {
                error!("Recorder error: {}", e);
            }
        });

        handle
    }

    /// Run the recorder loop.
    async fn run(
        self,
        mut cmd_rx: mpsc::Receiver<RecorderCommand>,
        mut stop_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let platform_name = self.platform.name();
        info!(
            "Starting danmaku recording for {} - {}",
            platform_name, self.config.url
        );

        // Create XML writer
        let xml_config = XmlWriterConfig {
            save_raw: self.config.save_raw,
            save_detail: self.config.save_detail,
            save_interval: if self.config.save_raw { 300 } else { 10 },
        };

        let output_path = format_output_path(&self.config.output_file);
        let mut xml_writer = XmlWriter::new(&output_path, xml_config.clone())?;

        // Main loop with reconnection
        if is_polling_url(&self.config.url) {
            match self
                .poll_and_run(&mut cmd_rx, &mut stop_rx, &mut xml_writer, &xml_config)
                .await
            {
                Ok(()) | Err(DanmakuError::Stopped) => {}
                Err(e) => return Err(e),
            }
        } else {
            loop {
                if *stop_rx.borrow() {
                    break;
                }

                match self
                    .connect_and_run(&mut cmd_rx, &mut stop_rx, &mut xml_writer, &xml_config)
                    .await
                {
                    Ok(()) | Err(DanmakuError::Stopped) => break,
                    Err(e) => {
                        warn!(
                            "{}: Connection error: {}. Reconnecting in 30s...",
                            platform_name, e
                        );

                        let mut reconnect_sleep =
                            Box::pin(tokio::time::sleep(Duration::from_secs(30)));
                        loop {
                            tokio::select! {
                                _ = &mut reconnect_sleep => break,
                                _ = stop_rx.changed() => {
                                    if *stop_rx.borrow() {
                                        break;
                                    }
                                }
                                Some(command) = cmd_rx.recv() => {
                                    if handle_command(command, &self.config.output_file, &mut xml_writer, &xml_config)? {
                                        break;
                                    }
                                }
                            }

                            if *stop_rx.borrow() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Finish XML file
        let has_messages = xml_writer.has_messages();
        let final_path = xml_writer.finish()?;
        if !has_messages {
            let _ = fs::remove_file(&final_path);
        }
        info!(
            "{}: Recording finished. Output: {:?}",
            platform_name, final_path
        );

        Ok(())
    }

    /// Connect to WebSocket and process messages.
    async fn poll_and_run(
        &self,
        cmd_rx: &mut mpsc::Receiver<RecorderCommand>,
        stop_rx: &mut watch::Receiver<bool>,
        xml_writer: &mut XmlWriter,
        xml_config: &XmlWriterConfig,
    ) -> Result<()> {
        let platform_name = self.platform.name();
        let mut context = self.config.context.clone();
        let conn_info = self
            .platform
            .get_connection_info(&self.config.url, &context)
            .await?;
        let continuation = conn_info
            .ws_url
            .strip_prefix("poll://youtube?continuation=")
            .ok_or_else(|| DanmakuError::Decode("Invalid polling connection info".to_string()))?;
        context
            .extra
            .insert("continuation".to_string(), continuation.to_string());

        let mut ticker = interval(self.platform.poll_interval());
        info!("{}: Started polling danmaku", platform_name);

        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        return Err(DanmakuError::Stopped);
                    }
                }

                Some(command) = cmd_rx.recv() => {
                    if handle_command(command, &self.config.output_file, xml_writer, xml_config)? {
                        return Err(DanmakuError::Stopped);
                    }
                }

                _ = ticker.tick() => {
                    let events = self.platform.poll_messages(&self.config.url, &mut context).await?;
                    for event in events {
                        if let Err(e) = xml_writer.write_event(&event) {
                            warn!("Failed to write event: {}", e);
                        }
                    }
                }
            }
        }
    }

    /// Connect to WebSocket and process messages.
    async fn connect_and_run(
        &self,
        cmd_rx: &mut mpsc::Receiver<RecorderCommand>,
        stop_rx: &mut watch::Receiver<bool>,
        xml_writer: &mut XmlWriter,
        xml_config: &XmlWriterConfig,
    ) -> Result<()> {
        let platform_name = self.platform.name();

        // Get connection info
        let conn_info = self
            .platform
            .get_connection_info(&self.config.url, &self.config.context)
            .await?;

        debug!("{}: Connecting to {}", platform_name, conn_info.ws_url);

        // Build WebSocket request from the URL first so tungstenite generates
        // the required handshake headers such as Sec-WebSocket-Key.
        let mut request = conn_info
            .ws_url
            .as_str()
            .into_client_request()
            .map_err(|e| DanmakuError::Decode(e.to_string()))?;

        for (key, value) in conn_info.headers.iter() {
            request.headers_mut().insert(key.clone(), value.clone());
        }

        // Connect
        let (ws_stream, _) = connect_async(request).await?;
        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        info!("{}: Connected to WebSocket", platform_name);

        // Send registration data
        for reg_data in &conn_info.registration_data {
            let msg = match reg_data {
                RegistrationData::Text(text) => Message::Text(text.clone()),
                RegistrationData::Binary(data) => Message::Binary(data.clone().into()),
            };
            ws_sink.send(msg).await?;
        }

        // Get heartbeat config
        let heartbeat_config = self.platform.heartbeat_config();

        // Create heartbeat receiver
        let mut heartbeat_rx = if let Some(ref hb_data) = heartbeat_config.data {
            let hb_data = hb_data.clone();
            let interval_duration = heartbeat_config.interval;
            let (hb_tx, hb_rx) = mpsc::channel::<Message>(1);

            tokio::spawn(async move {
                let mut ticker = interval(interval_duration);
                loop {
                    ticker.tick().await;
                    let msg = match &hb_data {
                        HeartbeatData::Text(text) => Message::Text(text.clone()),
                        HeartbeatData::Binary(data) => Message::Binary(data.clone().into()),
                    };
                    if hb_tx.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            Some(hb_rx)
        } else {
            None
        };

        // Main message loop
        loop {
            tokio::select! {
                // Check stop signal
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        return Err(DanmakuError::Stopped);
                    }
                }

                // Handle heartbeat
                hb_msg = async {
                    match heartbeat_rx.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => futures::future::pending().await,
                    }
                } => {
                    if let Some(msg) = hb_msg {
                        ws_sink.send(msg).await?;
                    }
                }

                // Handle recorder commands
                Some(command) = cmd_rx.recv() => {
                    if handle_command(command, &self.config.output_file, xml_writer, xml_config)? {
                        return Err(DanmakuError::Stopped);
                    }
                }

                // Handle WebSocket message
                ws_msg = ws_stream.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            let data = match msg {
                                Message::Text(text) => text.into_bytes(),
                                Message::Binary(data) => data.to_vec(),
                                Message::Ping(data) => {
                                    ws_sink.send(Message::Pong(data)).await?;
                                    continue;
                                }
                                Message::Pong(_) => continue,
                                Message::Close(_) => {
                                    return Err(DanmakuError::ConnectionClosed);
                                }
                                _ => continue,
                            };

                            // Decode message
                            match self.platform.decode_message(&data) {
                                Ok(result) => {
                                    // Write decoded events
                                    for event in result.events {
                                        if let Err(e) = xml_writer.write_event(&event) {
                                            warn!("Failed to write event: {}", e);
                                        }
                                    }

                                    // Send ack if needed
                                    if let Some(ack) = result.ack {
                                        ws_sink.send(Message::Binary(ack.into())).await?;
                                    }
                                }
                                Err(e) => {
                                    debug!("{}: Decode error: {}", platform_name, e);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Err(DanmakuError::WebSocket(e));
                        }
                        None => {
                            return Err(DanmakuError::ConnectionClosed);
                        }
                    }
                }
            }
        }
    }
}

fn is_polling_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

fn handle_command(
    command: RecorderCommand,
    template: &Path,
    xml_writer: &mut XmlWriter,
    xml_config: &XmlWriterConfig,
) -> Result<bool> {
    match command {
        RecorderCommand::Rolling {
            new_file_name,
            done,
        } => {
            let result = roll_writer(xml_writer, template, xml_config, new_file_name);
            let _ = done.send(result);
            Ok(false)
        }
        RecorderCommand::Stop => Ok(true),
    }
}

fn roll_writer(
    xml_writer: &mut XmlWriter,
    template: &Path,
    xml_config: &XmlWriterConfig,
    new_file_name: Option<PathBuf>,
) -> Result<()> {
    let current_path = xml_writer.file_path().to_path_buf();
    xml_writer.finalize()?;
    *xml_writer = XmlWriter::new(next_output_path(template), xml_config.clone())?;

    if let Some(new_path) = new_file_name {
        if current_path != new_path {
            if let Some(parent) = new_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if new_path.exists() {
                fs::remove_file(&new_path)?;
            }
            fs::rename(current_path, new_path)?;
        }
    }

    Ok(())
}

fn next_output_path(template: &Path) -> PathBuf {
    let output_path = format_output_path(template);
    if !output_path.exists() {
        return output_path;
    }

    let parent = output_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let stem = output_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "danmaku".to_string());

    for index in 1.. {
        let candidate = parent.join(format!("{stem}_{index}.xml"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!()
}

/// Format output path with timestamp substitution.
fn format_output_path(template: &Path) -> PathBuf {
    let now = chrono::Local::now();
    let path_str = template.to_string_lossy();

    // Replace strftime-like patterns
    let formatted = now.format(&path_str).to_string();

    PathBuf::from(formatted).with_extension("xml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_output_path() {
        let template = PathBuf::from("/tmp/test_%Y%m%d");
        let result = format_output_path(&template);
        assert!(result.to_string_lossy().contains("/tmp/test_"));
        assert!(result.extension().map(|e| e == "xml").unwrap_or(false));
    }
}
