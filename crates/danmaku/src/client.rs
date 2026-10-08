//! Core danmaku client implementation.
//!
//! The [`DanmakuRecorder`] manages WebSocket connections, heartbeats,
//! message processing, and XML output for recording live stream chat.

use std::fs;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{Connector, connect_async_tls_with_config};
use tracing::{debug, error, info, warn};

use crate::error::{DanmakuError, Result};
use crate::message::DanmakuEvent;
use crate::output::xml::{XmlWriter, XmlWriterConfig};
use crate::protocols::{
    ConnectionInfo, ConnectionTransport, DecodeResult, HeartbeatData, Platform, PlatformContext,
    RegistrationData, create_platform,
};

/// 捕获平台解码器的 panic，降级为解码错误：
/// 录制任务在 tokio::spawn 中运行，panic 会直接杀死任务且没有任何
/// 重启机制，一条畸形消息就会让弹幕录制静默永久停止。
fn decode_message_guarded(
    platform: &dyn Platform,
    data: &[u8],
    platform_name: &str,
) -> Result<DecodeResult> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        platform.decode_message(data)
    }))
    .unwrap_or_else(|_| {
        error!(
            "{}: decoder panicked on a {}-byte message, skipping it",
            platform_name,
            data.len()
        );
        Err(DanmakuError::Decode("decoder panicked".to_string()))
    })
}

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
    /// Optional live tee: every decoded event is also `send` to this broadcast channel
    /// (best effort — no subscribers or a full ring just drops the copy). Used by the
    /// web UI's live preview to overlay danmaku; never affects the XML recording.
    pub live_tx: Option<broadcast::Sender<DanmakuEvent>>,
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
            live_tx: None,
        }
    }

    /// Also broadcast every decoded event to `tx` (see [`RecorderConfig::live_tx`]).
    pub fn with_live_tx(mut self, tx: broadcast::Sender<DanmakuEvent>) -> Self {
        self.live_tx = Some(tx);
        self
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
        done: oneshot::Sender<Result<bool>>,
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
    pub async fn rolling(&self, new_file_name: Option<PathBuf>) -> Result<bool> {
        let (done, rx) = oneshot::channel();
        self.cmd_tx
            .send(RecorderCommand::Rolling {
                new_file_name,
                done,
            })
            .await
            .map_err(|_| DanmakuError::ChannelSend)?;
        rx.await.map_err(|_| DanmakuError::ChannelSend)?
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

    /// Tee one decoded event to the live broadcast, if configured. `send` never waits:
    /// with no receivers the copy is dropped, with a full ring the oldest is overwritten.
    fn emit_live(&self, event: &DanmakuEvent) {
        if let Some(tx) = &self.config.live_tx {
            let _ = tx.send(event.clone());
        }
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

        // Retain polling continuations across temporary connection failures.
        let mut polling_context = self.config.context.clone();
        loop {
            if stop_requested(&stop_rx) {
                break;
            }

            let result = if is_polling_url(&self.config.url) {
                self.poll_and_run(
                    &mut cmd_rx,
                    &mut stop_rx,
                    &mut xml_writer,
                    &xml_config,
                    &mut polling_context,
                )
                .await
            } else {
                self.connect_and_run(&mut cmd_rx, &mut stop_rx, &mut xml_writer, &xml_config)
                    .await
            };
            match result {
                Ok(()) | Err(DanmakuError::Stopped) => break,
                Err(e) => {
                    warn!(
                        "{}: Connection error: {}. Reconnecting in 30s...",
                        platform_name, e
                    );

                    let mut reconnect_sleep = Box::pin(tokio::time::sleep(Duration::from_secs(30)));
                    loop {
                        tokio::select! {
                            _ = &mut reconnect_sleep => break,
                            _ = stop_rx.changed() => {
                                if stop_requested(&stop_rx) {
                                    break;
                                }
                            }
                            Some(command) = cmd_rx.recv() => {
                                if handle_command(command, &self.config.output_file, &mut xml_writer, &xml_config)? {
                                    break;
                                }
                            }
                        }

                        if stop_requested(&stop_rx) {
                            break;
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
        context: &mut PlatformContext,
    ) -> Result<()> {
        let platform_name = self.platform.name();
        let conn_info = until_stopped(
            self.platform.get_connection_info(&self.config.url, context),
            stop_rx,
        )
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
                    if stop_requested(stop_rx) {
                        return Err(DanmakuError::Stopped);
                    }
                }

                Some(command) = cmd_rx.recv() => {
                    if handle_command(command, &self.config.output_file, xml_writer, xml_config)? {
                        return Err(DanmakuError::Stopped);
                    }
                }

                _ = ticker.tick() => {
                    let events = until_stopped(
                        self.platform.poll_messages(&self.config.url, context),
                        stop_rx,
                    ).await?;
                    for event in events {
                        self.emit_live(&event);
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
        let conn_info = until_stopped(
            self.platform
                .get_connection_info(&self.config.url, &self.config.context),
            stop_rx,
        )
        .await?;

        match conn_info.transport {
            ConnectionTransport::WebSocket => {
                self.run_websocket_connection(
                    conn_info,
                    cmd_rx,
                    stop_rx,
                    xml_writer,
                    xml_config,
                    platform_name,
                )
                .await
            }
            ConnectionTransport::Tcp => {
                self.run_tcp_connection(
                    conn_info,
                    cmd_rx,
                    stop_rx,
                    xml_writer,
                    xml_config,
                    platform_name,
                )
                .await
            }
        }
    }

    async fn run_websocket_connection(
        &self,
        conn_info: ConnectionInfo,
        cmd_rx: &mut mpsc::Receiver<RecorderCommand>,
        stop_rx: &mut watch::Receiver<bool>,
        xml_writer: &mut XmlWriter,
        xml_config: &XmlWriterConfig,
        platform_name: &str,
    ) -> Result<()> {
        debug!("{}: Connecting to {}", platform_name, conn_info.ws_url);

        // Connect
        let ws_stream =
            until_stopped(connect_websocket(&conn_info, platform_name), stop_rx).await?;
        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        info!("{}: Connected to WebSocket", platform_name);

        // Send registration data
        for reg_data in &conn_info.registration_data {
            let msg = match reg_data {
                RegistrationData::Text(text) => Message::Text(text.clone().into()),
                RegistrationData::Binary(data) => Message::Binary(data.clone().into()),
            };
            until_stopped(async { Ok(ws_sink.send(msg).await?) }, stop_rx).await?;
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
                        HeartbeatData::Text(text) => Message::Text(text.clone().into()),
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

        let mut consecutive_decode_errors = 0u64;

        // Main message loop
        loop {
            tokio::select! {
                // Check stop signal
                _ = stop_rx.changed() => {
                    if stop_requested(stop_rx) {
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
                                Message::Text(text) => text.as_str().as_bytes().to_vec(),
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
                            match decode_message_guarded(self.platform.as_ref(), &data, platform_name) {
                                Ok(result) => {
                                    consecutive_decode_errors = 0;
                                    // Write decoded events
                                    for event in result.events {
                                        self.emit_live(&event);
                                        if let Err(e) = xml_writer.write_event(&event) {
                                            warn!("Failed to write event: {}", e);
                                        }
                                    }

                                    // Send ack if needed
                                    if let Some(ack) = result.ack {
                                        if result.ack_is_text {
                                            let text = String::from_utf8(ack)
                                                .map_err(|e| DanmakuError::Decode(e.to_string()))?;
                                            ws_sink.send(Message::Text(text.into())).await?;
                                        } else {
                                            ws_sink.send(Message::Binary(ack.into())).await?;
                                        }
                                    }
                                }
                                Err(e) => {
                                    consecutive_decode_errors += 1;
                                    if consecutive_decode_errors == 1
                                        || consecutive_decode_errors.is_multiple_of(100)
                                    {
                                        warn!(
                                            platform = platform_name,
                                            consecutive_decode_errors,
                                            error = %e,
                                            "Danmaku decode error"
                                        );
                                    } else {
                                        debug!("{}: Decode error: {}", platform_name, e);
                                    }
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

    async fn run_tcp_connection(
        &self,
        conn_info: ConnectionInfo,
        cmd_rx: &mut mpsc::Receiver<RecorderCommand>,
        stop_rx: &mut watch::Receiver<bool>,
        xml_writer: &mut XmlWriter,
        xml_config: &XmlWriterConfig,
        platform_name: &str,
    ) -> Result<()> {
        debug!("{}: Connecting to {}", platform_name, conn_info.ws_url);

        let mut tcp_stream = until_stopped(connect_tcp(&conn_info, platform_name), stop_rx).await?;
        info!("{}: Connected to TCP danmaku endpoint", platform_name);

        for reg_data in &conn_info.registration_data {
            match reg_data {
                RegistrationData::Text(text) => tcp_stream.write_all(text.as_bytes()).await?,
                RegistrationData::Binary(data) => tcp_stream.write_all(data).await?,
            }
        }

        let heartbeat_config = self.platform.heartbeat_config();
        let mut heartbeat_rx = if let Some(ref hb_data) = heartbeat_config.data {
            let hb_data = hb_data.clone();
            let interval_duration = heartbeat_config.interval;
            let (hb_tx, hb_rx) = mpsc::channel::<Vec<u8>>(1);

            tokio::spawn(async move {
                let mut ticker = interval(interval_duration);
                loop {
                    ticker.tick().await;
                    let data = match &hb_data {
                        HeartbeatData::Text(text) => text.as_bytes().to_vec(),
                        HeartbeatData::Binary(data) => data.clone(),
                    };
                    if hb_tx.send(data).await.is_err() {
                        break;
                    }
                }
            });

            Some(hb_rx)
        } else {
            None
        };

        let (mut tcp_reader, mut tcp_writer) = tcp_stream.into_split();
        let mut frame_reader = TcpFrameReader::default();

        let mut consecutive_decode_errors = 0u64;
        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if stop_requested(stop_rx) {
                        return Err(DanmakuError::Stopped);
                    }
                }

                hb_data = async {
                    match heartbeat_rx.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => futures::future::pending().await,
                    }
                } => {
                    if let Some(data) = hb_data {
                        tcp_writer.write_all(&data).await?;
                    }
                }

                Some(command) = cmd_rx.recv() => {
                    if handle_command(command, &self.config.output_file, xml_writer, xml_config)? {
                        return Err(DanmakuError::Stopped);
                    }
                }

                frame = frame_reader.read_frame(&mut tcp_reader) => {
                    let frame = frame?;
                    match decode_message_guarded(self.platform.as_ref(), &frame, platform_name) {
                        Ok(result) => {
                            consecutive_decode_errors = 0;
                            for event in result.events {
                                self.emit_live(&event);
                                if let Err(e) = xml_writer.write_event(&event) {
                                    warn!("Failed to write event: {}", e);
                                }
                            }

                            if let Some(ack) = result.ack {
                                tcp_writer.write_all(&ack).await?;
                            }
                        }
                        Err(e) => {
                            consecutive_decode_errors += 1;
                            if consecutive_decode_errors == 1
                                || consecutive_decode_errors.is_multiple_of(100)
                            {
                                warn!(
                                    platform = platform_name,
                                    consecutive_decode_errors,
                                    error = %e,
                                    "Danmaku decode error"
                                );
                            } else {
                                debug!("{}: Decode error: {}", platform_name, e);
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn connect_tcp(conn_info: &ConnectionInfo, platform_name: &str) -> Result<TcpStream> {
    let urls = std::iter::once(&conn_info.ws_url).chain(conn_info.fallback_ws_urls.iter());
    let mut last_error = None;

    for (index, tcp_url) in urls.enumerate() {
        let addr = parse_tcp_addr(tcp_url)?;
        debug!("{}: Connecting to {}", platform_name, tcp_url);

        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                if index > 0 {
                    warn!(
                        "{}: Primary TCP endpoint failed, fell back to {}",
                        platform_name, tcp_url
                    );
                }
                return Ok(stream);
            }
            Err(err) => {
                warn!(
                    "{}: TCP connect to {} failed: {}",
                    platform_name, tcp_url, err
                );
                last_error = Some(err);
            }
        }
    }

    Err(DanmakuError::Io(last_error.unwrap_or_else(|| {
        std::io::Error::other("no TCP endpoints configured")
    })))
}

fn parse_tcp_addr(url: &str) -> Result<String> {
    url.strip_prefix("tcp://")
        .filter(|addr| !addr.is_empty())
        .map(str::to_string)
        .ok_or_else(|| DanmakuError::Decode(format!("Invalid TCP endpoint: {url}")))
}

/// TCP 帧头：`u32 LE 长度 | u32 LE 长度 | u32 LE 类型`，长度不含自身的 4 字节。
const TCP_FRAME_HEADER_LEN: usize = 12;

/// 单帧长度上限。真实的斗鱼消息只有几百字节到几十 KB；长度字段是对端给的 u32，
/// 流一旦错位就是任意值，不能照着它缓冲/分配（最大可达 4 GiB）。
const MAX_TCP_FRAME_LEN: usize = 1 << 20;

/// 从 TCP 字节流中切出完整帧。
///
/// 读帧在 `select!` 里与心跳、命令分支竞争，随时可能被丢弃。`read_exact` 不是
/// 取消安全的：被抢先时已读进局部缓冲的半帧会丢失，之后的字节被当成帧头，整个
/// 流错位。这里把已读字节留在 `buf` 里，只在取消安全的 `read_buf` 上等待。
#[derive(Default)]
struct TcpFrameReader {
    buf: Vec<u8>,
}

impl TcpFrameReader {
    async fn read_frame<R: AsyncRead + Unpin>(&mut self, reader: &mut R) -> Result<Vec<u8>> {
        loop {
            if let Some(frame) = self.take_frame()? {
                return Ok(frame);
            }
            self.buf.reserve(8 * 1024);
            if reader.read_buf(&mut self.buf).await? == 0 {
                return Err(DanmakuError::ConnectionClosed);
            }
        }
    }

    /// 缓冲区里已有完整帧时取出它；帧还没收全时返回 `None`。
    fn take_frame(&mut self) -> Result<Option<Vec<u8>>> {
        if self.buf.len() < TCP_FRAME_HEADER_LEN {
            return Ok(None);
        }

        let length = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
        let length = length as usize;
        if !(8..=MAX_TCP_FRAME_LEN).contains(&length) {
            return Err(DanmakuError::Decode(format!(
                "Invalid TCP frame length: {length}"
            )));
        }

        let total = 4 + length;
        if self.buf.len() < total {
            return Ok(None);
        }
        let rest = self.buf.split_off(total);
        Ok(Some(std::mem::replace(&mut self.buf, rest)))
    }
}

/// 是否应当停止录制。所有 [`RecorderHandle`] 都被丢弃（例如持有它的任务被 abort）
/// 后，`stop_rx.changed()` 每次都立即返回 `Err`，而 `borrow()` 仍是 `false`；
/// 只看 `borrow()` 会让 `select!` 循环在该分支上空转、永不让出线程。没有句柄就
/// 再也无法停止或滚动这个录制，按停止处理。
fn stop_requested(stop_rx: &watch::Receiver<bool>) -> bool {
    *stop_rx.borrow() || stop_rx.has_changed().is_err()
}

async fn until_stopped<T>(
    operation: impl Future<Output = Result<T>>,
    stop_rx: &mut watch::Receiver<bool>,
) -> Result<T> {
    if stop_requested(stop_rx) {
        return Err(DanmakuError::Stopped);
    }
    tokio::select! {
        biased;
        _ = stop_rx.changed() => Err(DanmakuError::Stopped),
        result = operation => result,
    }
}

fn platform_tls_connector() -> Result<Connector> {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let config = rustls::ClientConfig::builder_with_provider(provider.into())
        .with_safe_default_protocol_versions()
        .map_err(|e| DanmakuError::Decode(e.to_string()))?
        .with_platform_verifier()
        .map_err(|e| DanmakuError::Decode(e.to_string()))?
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

async fn connect_websocket(
    conn_info: &ConnectionInfo,
    platform_name: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
> {
    let urls = std::iter::once(&conn_info.ws_url).chain(conn_info.fallback_ws_urls.iter());
    let mut last_error = None;

    for (index, ws_url) in urls.enumerate() {
        debug!("{}: Connecting to {}", platform_name, ws_url);

        let mut request = ws_url
            .as_str()
            .into_client_request()
            .map_err(|e| DanmakuError::Decode(e.to_string()))?;

        for (key, value) in conn_info.headers.iter() {
            request.headers_mut().insert(key.clone(), value.clone());
        }

        let connector = if ws_url.starts_with("wss://") {
            Some(platform_tls_connector()?)
        } else {
            None
        };

        match connect_async_tls_with_config(request, None, false, connector).await {
            Ok((ws_stream, _)) => {
                if index > 0 {
                    warn!(
                        "{}: Primary WebSocket failed, fell back to {}",
                        platform_name, ws_url
                    );
                }
                return Ok(ws_stream);
            }
            Err(err) => {
                warn!(
                    "{}: WebSocket connect to {} failed: {}",
                    platform_name, ws_url, err
                );
                last_error = Some(err);
            }
        }
    }

    Err(DanmakuError::WebSocket(last_error.unwrap_or_else(|| {
        tokio_tungstenite::tungstenite::Error::Io(std::io::Error::other(
            "no WebSocket endpoints configured",
        ))
    })))
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
) -> Result<bool> {
    let current_path = xml_writer.file_path().to_path_buf();
    xml_writer.finalize()?;
    let current_exists = current_path.exists();
    *xml_writer = XmlWriter::new(next_output_path(template), xml_config.clone())?;

    if !current_exists {
        return Ok(false);
    }

    if let Some(new_path) = new_file_name
        && !is_same_file(&current_path, &new_path)
    {
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if new_path.exists() {
            fs::remove_file(&new_path)?;
        }
        fs::rename(current_path, new_path)?;
    }

    Ok(true)
}

/// `Path` 的相等比较保留开头的 `.`，`./x.xml` 和 `x.xml` 会被当成两个文件；
/// 这时 `roll_writer` 会把目标（其实就是当前文件）删掉，再改名就失败了。
fn is_same_file(a: &Path, b: &Path) -> bool {
    fn lexical(path: &Path) -> PathBuf {
        path.components()
            .filter(|component| !matches!(component, Component::CurDir))
            .collect()
    }

    if lexical(a) == lexical(b) {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
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
    fn rolling_without_messages_renames_current_xml() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-roll-empty-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("danmaku");
        let new_path = dir.join("segment.xml");
        let config = XmlWriterConfig::default();
        let mut writer = XmlWriter::new(format_output_path(&template), config.clone()).unwrap();
        let current_path = writer.file_path().to_path_buf();

        assert!(roll_writer(&mut writer, &template, &config, Some(new_path.clone())).unwrap());

        assert!(!current_path.exists());
        assert!(new_path.exists());
        let content = std::fs::read_to_string(&new_path).unwrap();
        assert!(content.contains("<i>"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rolling_missing_current_xml_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-roll-missing-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("danmaku");
        let new_path = dir.join("segment.xml");
        let config = XmlWriterConfig::default();
        let mut writer = XmlWriter::new(format_output_path(&template), config.clone()).unwrap();
        let current_path = writer.file_path().to_path_buf();
        std::fs::remove_file(&current_path).unwrap();

        assert!(roll_writer(&mut writer, &template, &config, Some(new_path.clone())).is_ok());

        assert!(!new_path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn dot_prefixed_path_is_the_same_file() {
        assert!(is_same_file(Path::new("./x.xml"), Path::new("x.xml")));
        assert!(is_same_file(Path::new("x.xml"), Path::new("./x.xml")));
        assert!(is_same_file(Path::new("./a/x.xml"), Path::new("a/x.xml")));
        assert!(!is_same_file(Path::new("./x.xml"), Path::new("y.xml")));
        assert!(!is_same_file(Path::new("a/x.xml"), Path::new("b/x.xml")));
    }

    fn write_one_chat(writer: &mut XmlWriter) {
        let chat = crate::message::ChatMessage::new("hello".to_string()).with_name("user");
        writer.write_event(&DanmakuEvent::Chat(chat)).unwrap();
    }

    /// 视频分段路径带 `./`、当前 XML 路径不带时，分段不能把当前 XML 删掉。
    #[test]
    fn rolling_onto_dot_prefixed_current_path_keeps_the_xml() {
        // 相对路径才能复现：绝对路径中间的 `.` 在 `Path` 比较时本来就会被忽略
        let dir = PathBuf::from(format!(
            "danmaku-roll-dot-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("danmaku");
        let config = XmlWriterConfig::default();
        let mut writer = XmlWriter::new(format_output_path(&template), config.clone()).unwrap();
        write_one_chat(&mut writer);
        let current_path = writer.file_path().to_path_buf();
        let dotted = Path::new(".").join(&current_path);
        assert_ne!(current_path, dotted);

        let rolled = roll_writer(&mut writer, &template, &config, Some(dotted.clone()));

        let content = std::fs::read_to_string(&current_path);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(rolled.unwrap());
        let content = content.unwrap();
        assert!(content.contains("hello"));
        assert!(content.trim_end().ends_with("</i>"));
    }

    #[test]
    fn rolling_onto_another_spelling_of_the_current_path_keeps_the_xml() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-roll-alias-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let template = dir.join("danmaku");
        let config = XmlWriterConfig::default();
        let mut writer = XmlWriter::new(format_output_path(&template), config.clone()).unwrap();
        write_one_chat(&mut writer);
        let current_path = writer.file_path().to_path_buf();
        let alias = dir
            .join("sub")
            .join("..")
            .join(current_path.file_name().unwrap());

        assert!(roll_writer(&mut writer, &template, &config, Some(alias)).unwrap());

        let content = std::fs::read_to_string(&current_path).unwrap();
        assert!(content.contains("hello"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rolling_replaces_a_different_existing_target() {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-roll-replace-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("danmaku");
        let new_path = dir.join("segment.xml");
        std::fs::write(&new_path, "stale").unwrap();
        let config = XmlWriterConfig::default();
        let mut writer = XmlWriter::new(format_output_path(&template), config.clone()).unwrap();
        write_one_chat(&mut writer);
        let current_path = writer.file_path().to_path_buf();

        assert!(roll_writer(&mut writer, &template, &config, Some(new_path.clone())).unwrap());

        assert!(!current_path.exists());
        let content = std::fs::read_to_string(&new_path).unwrap();
        assert!(content.contains("hello"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_format_output_path() {
        let template = PathBuf::from("/tmp/test_%Y%m%d");
        let result = format_output_path(&template);
        assert!(result.to_string_lossy().contains("/tmp/test_"));
        assert!(result.extension().map(|e| e == "xml").unwrap_or(false));
    }

    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "danmaku-{tag}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct PendingPlatform {
        entered: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl Platform for PendingPlatform {
        fn name(&self) -> &'static str {
            "Pending"
        }

        async fn get_connection_info(
            &self,
            _url: &str,
            _context: &PlatformContext,
        ) -> Result<ConnectionInfo> {
            self.entered.notify_one();
            futures::future::pending().await
        }

        fn heartbeat_config(&self) -> crate::protocols::HeartbeatConfig {
            crate::protocols::HeartbeatConfig::none()
        }

        fn decode_message(&self, _data: &[u8]) -> Result<DecodeResult> {
            Ok(DecodeResult::empty())
        }
    }

    #[tokio::test]
    async fn stop_cancels_pending_connection_setup_and_finalizes_output() {
        let dir = test_dir("pending-stop");
        let entered = Arc::new(tokio::sync::Notify::new());
        let recorder = DanmakuRecorder {
            config: RecorderConfig::new("https://example.invalid/room", dir.join("danmaku")),
            platform: Arc::new(PendingPlatform {
                entered: entered.clone(),
            }),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        let (stop_tx, stop_rx) = watch::channel(false);
        let recording = tokio::spawn(recorder.run(cmd_rx, stop_rx));
        entered.notified().await;
        stop_tx.send(true).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), recording)
                .await
                .unwrap()
                .unwrap()
                .is_ok()
        );
        assert!(!dir.join("danmaku.xml").exists());
        drop(cmd_tx);
        let _ = fs::remove_dir_all(dir);
    }

    struct FlakyPollingPlatform {
        attempts: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Platform for FlakyPollingPlatform {
        fn name(&self) -> &'static str {
            "FlakyPolling"
        }

        async fn get_connection_info(
            &self,
            _url: &str,
            context: &PlatformContext,
        ) -> Result<ConnectionInfo> {
            let continuation = context
                .extra
                .get("continuation")
                .map_or("initial", String::as_str);
            Ok(ConnectionInfo::new(format!(
                "poll://youtube?continuation={continuation}"
            )))
        }

        fn heartbeat_config(&self) -> crate::protocols::HeartbeatConfig {
            crate::protocols::HeartbeatConfig::none()
        }

        fn decode_message(&self, _data: &[u8]) -> Result<DecodeResult> {
            Ok(DecodeResult::empty())
        }

        async fn poll_messages(
            &self,
            _url: &str,
            context: &mut PlatformContext,
        ) -> Result<Vec<DanmakuEvent>> {
            let attempt = self
                .attempts
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if attempt == 1 {
                return Err(DanmakuError::ConnectionClosed);
            }
            if attempt > 1 {
                assert_eq!(
                    context.extra.get("continuation").map(String::as_str),
                    Some("next")
                );
            }
            context
                .extra
                .insert("continuation".to_string(), "next".to_string());
            Ok(vec![DanmakuEvent::Chat(crate::message::ChatMessage::new(
                format!("message-{attempt}"),
            ))])
        }
    }

    #[tokio::test(start_paused = true)]
    async fn polling_reconnects_after_failure_without_losing_continuation() {
        let dir = test_dir("polling-reconnect");
        let (live_tx, mut live_rx) = broadcast::channel(16);
        let recorder = DanmakuRecorder {
            config: RecorderConfig::new("https://youtube.com/watch?v=test", dir.join("danmaku"))
                .with_live_tx(live_tx),
            platform: Arc::new(FlakyPollingPlatform {
                attempts: std::sync::atomic::AtomicUsize::new(0),
            }),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        let (stop_tx, stop_rx) = watch::channel(false);
        let recording = tokio::spawn(recorder.run(cmd_rx, stop_rx));
        for expected in ["message-0", "message-2"] {
            let event = tokio::time::timeout(Duration::from_secs(60), live_rx.recv())
                .await
                .unwrap()
                .unwrap();
            let DanmakuEvent::Chat(chat) = event else {
                panic!("expected chat")
            };
            assert_eq!(chat.content, expected);
        }
        stop_tx.send(true).unwrap();
        assert!(recording.await.unwrap().is_ok());
        let content = fs::read_to_string(dir.join("danmaku.xml")).unwrap();
        assert!(content.trim_end().ends_with("</i>"));
        drop(cmd_tx);
        let _ = fs::remove_dir_all(dir);
    }

    /// 连接总是失败的平台：录制器会一直停在 30s 重连等待里。
    struct UnreachablePlatform;

    #[async_trait::async_trait]
    impl Platform for UnreachablePlatform {
        fn name(&self) -> &'static str {
            "Unreachable"
        }

        async fn get_connection_info(
            &self,
            _url: &str,
            _context: &PlatformContext,
        ) -> Result<ConnectionInfo> {
            Err(DanmakuError::ConnectionClosed)
        }

        fn heartbeat_config(&self) -> crate::protocols::HeartbeatConfig {
            crate::protocols::HeartbeatConfig::none()
        }

        fn decode_message(&self, _data: &[u8]) -> Result<DecodeResult> {
            Ok(DecodeResult::empty())
        }
    }

    /// 所有 RecorderHandle 被丢弃（例如持有它的下载任务被 abort）后，
    /// `stop_rx.changed()` 每次都立即返回 Err。修复前只看 `borrow()`（仍为 false），
    /// `select!` 在这个分支上空转：任务永不让出工作线程、永不结束。
    #[test]
    fn dropping_every_handle_stops_the_recorder() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let dir = test_dir("handle-dropped");
        let recorder = DanmakuRecorder {
            config: RecorderConfig::new("https://example.invalid/room", dir.join("danmaku")),
            platform: Arc::new(UnreachablePlatform),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        rt.spawn(async move {
            let result = recorder.run(cmd_rx, stop_rx).await;
            let _ = done_tx.send(result.is_ok());
        });

        // 等录制器进入重连等待，再丢掉全部句柄
        std::thread::sleep(Duration::from_millis(200));
        drop(cmd_tx);
        drop(stop_tx);

        let finished = done_rx.recv_timeout(Duration::from_secs(5));
        // 修复前任务在空转，不能等它让出线程
        rt.shutdown_background();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            finished,
            Ok(true),
            "recorder kept running after every handle was dropped"
        );
    }

    /// 斗鱼式 TCP 帧：`len | len | type | body`，len = 8 + body.len()（小端）。
    fn tcp_frame(body: &[u8]) -> Vec<u8> {
        let len = (8 + body.len()) as u32;
        let mut frame = Vec::new();
        frame.extend_from_slice(&len.to_le_bytes());
        frame.extend_from_slice(&len.to_le_bytes());
        frame.extend_from_slice(&690u32.to_le_bytes());
        frame.extend_from_slice(body);
        frame
    }

    #[tokio::test]
    async fn tcp_frame_reader_splits_coalesced_frames_and_survives_cancellation() {
        let (mut server, mut client) = tokio::io::duplex(1024);
        let first = tcp_frame(b"first-body");
        let second = tcp_frame(b"second");
        let mut reader = TcpFrameReader::default();

        // 半帧到达后读 future 被丢弃（模拟 select! 里其它分支抢先）
        server.write_all(&first[..15]).await.unwrap();
        let cancelled =
            tokio::time::timeout(Duration::from_millis(20), reader.read_frame(&mut client)).await;
        assert!(cancelled.is_err());

        // 剩余部分与下一帧粘在一起到达
        let mut rest = first[15..].to_vec();
        rest.extend_from_slice(&second);
        server.write_all(&rest).await.unwrap();
        assert_eq!(reader.read_frame(&mut client).await.unwrap(), first);
        assert_eq!(reader.read_frame(&mut client).await.unwrap(), second);

        drop(server);
        assert!(matches!(
            reader.read_frame(&mut client).await,
            Err(DanmakuError::ConnectionClosed)
        ));
    }

    #[test]
    fn tcp_frame_reader_rejects_bogus_lengths_without_buffering_them() {
        for length in [0u32, 7, u32::MAX] {
            let mut header = Vec::new();
            header.extend_from_slice(&length.to_le_bytes());
            header.extend_from_slice(&length.to_le_bytes());
            header.extend_from_slice(&690u32.to_le_bytes());
            let mut reader = TcpFrameReader { buf: header };
            assert!(
                matches!(reader.take_frame(), Err(DanmakuError::Decode(_))),
                "length {length} accepted"
            );
        }
    }

    /// 走 TCP 传输、心跳极频繁的平台；每帧解出一条正文为帧体的弹幕。
    struct TcpEchoPlatform {
        addr: String,
    }

    #[async_trait::async_trait]
    impl Platform for TcpEchoPlatform {
        fn name(&self) -> &'static str {
            "TcpEcho"
        }

        async fn get_connection_info(
            &self,
            _url: &str,
            _context: &PlatformContext,
        ) -> Result<ConnectionInfo> {
            Ok(ConnectionInfo::new(format!("tcp://{}", self.addr)).with_tcp_transport())
        }

        fn heartbeat_config(&self) -> crate::protocols::HeartbeatConfig {
            crate::protocols::HeartbeatConfig::binary(b"hb".to_vec(), Duration::from_millis(5))
        }

        fn decode_message(&self, data: &[u8]) -> Result<DecodeResult> {
            let body = String::from_utf8_lossy(&data[12..]).into_owned();
            Ok(DecodeResult::with_events(vec![DanmakuEvent::Chat(
                crate::message::ChatMessage::new(body),
            )]))
        }
    }

    /// 一帧分两次到达、中间心跳分支抢先完成时，读帧 future 被 `select!` 丢弃。
    /// 修复前用 `read_exact` 读进局部缓冲的半帧随之丢失，后续字节被当成帧头，
    /// 流整体错位（这里错位后的“长度”为 1，连接报错并进入 30s 重连）。
    #[tokio::test]
    async fn tcp_frame_split_across_heartbeats_is_not_lost() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let first_body = b"AA\x01\x00\x00\x00-first";
        let first = tcp_frame(first_body);
        let second = tcp_frame(b"second");
        let server = {
            let (first, second) = (first.clone(), second.clone());
            tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                // 帧头 + 2 字节正文先到，剩余部分在若干次心跳之后才到
                socket.write_all(&first[..14]).await.unwrap();
                tokio::time::sleep(Duration::from_millis(150)).await;
                socket.write_all(&first[14..]).await.unwrap();
                socket.write_all(&second).await.unwrap();
                // 保持连接直到测试结束
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
        };

        let dir = test_dir("tcp-split");
        let (live_tx, mut live_rx) = broadcast::channel(16);
        let recorder = DanmakuRecorder {
            config: RecorderConfig::new("tcp-echo", dir.join("danmaku")).with_live_tx(live_tx),
            platform: Arc::new(TcpEchoPlatform { addr }),
        };
        let handle = recorder.start();

        let mut received = Vec::new();
        for _ in 0..2 {
            match tokio::time::timeout(Duration::from_secs(5), live_rx.recv()).await {
                Ok(Ok(DanmakuEvent::Chat(chat))) => received.push(chat.content),
                _ => break,
            }
        }

        handle.stop().await.unwrap();
        server.abort();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            received,
            vec![
                String::from_utf8(first_body.to_vec()).unwrap(),
                "second".to_string()
            ]
        );
    }
}
