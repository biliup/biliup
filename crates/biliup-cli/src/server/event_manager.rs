use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info};

use crate::server::core::live_streamers::LiveStreamerDto;

/// 事件类型枚举，对应Python版本的事件
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    PreDownload,
    Download,
    Downloaded,
    Upload,
    Uploaded,
}

/// 事件数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiliUpEvent {
    PreDownload { name: String, url: String },
    Download { name: String, url: String },
    Downloaded { stream_info: StreamInfo },
    Upload { stream_info: StreamInfo },
    Uploaded { files: Vec<FileInfo> },
}

/// 流信息结构，对应Python版本的stream_info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub name: String,
    pub url: String,
    pub title: Option<String>,
    pub date: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub live_cover_path: Option<String>,
    pub is_download: bool,
    pub platform: String,
    pub database_row_id: Option<i64>,
}

/// 文件信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub video: String,
    pub danmaku: Option<String>,
}

/// 事件处理器trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: BiliUpEvent) -> Result<Option<BiliUpEvent>>;
    fn event_type(&self) -> EventType;
}

/// 事件管理器上下文，对应Python版本的context
#[derive(Debug, Default)]
pub struct EventContext {
    pub url_upload_count: HashMap<String, i32>,
    pub upload_filename: Vec<String>,
    pub file_upload_count: HashMap<String, i32>,
    pub url_status: HashMap<String, i32>, // 0: idle, 1: downloading, 2: stopped
}

/// 事件管理器，对应Python版本的EventManager
pub struct EventManager {
    context: Arc<RwLock<EventContext>>,
    handlers: Arc<RwLock<HashMap<EventType, Vec<Box<dyn EventHandler>>>>>,
    event_tx: mpsc::UnboundedSender<BiliUpEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BiliUpEvent>>>>,
}

impl EventManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            context: Arc::new(RwLock::new(EventContext::default())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        }
    }

    /// 注册事件处理器
    pub async fn register_handler(&self, handler: Box<dyn EventHandler>) {
        let event_type = handler.event_type();
        let mut handlers = self.handlers.write().await;
        handlers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// 发送事件
    pub async fn send_event(&self, event: BiliUpEvent) -> Result<()> {
        self.event_tx
            .send(event)
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))?;
        Ok(())
    }

    /// 获取上下文的只读访问
    pub async fn get_context(&self) -> tokio::sync::RwLockReadGuard<EventContext> {
        self.context.read().await
    }

    /// 获取上下文的写访问
    pub async fn get_context_mut(&self) -> tokio::sync::RwLockWriteGuard<EventContext> {
        self.context.write().await
    }

    /// 启动事件循环
    pub async fn start(&self) -> Result<()> {
        let mut event_rx = {
            let mut rx_guard = self.event_rx.write().await;
            rx_guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("Event manager already started"))?
        };

        info!("Event manager started");

        while let Some(event) = event_rx.recv().await {
            if let Err(e) = self.process_event(event).await {
                error!("Failed to process event: {}", e);
            }
        }

        Ok(())
    }

    /// 处理单个事件
    async fn process_event(&self, event: BiliUpEvent) -> Result<()> {
        let event_type = match &event {
            BiliUpEvent::PreDownload { .. } => EventType::PreDownload,
            BiliUpEvent::Download { .. } => EventType::Download,
            BiliUpEvent::Downloaded { .. } => EventType::Downloaded,
            BiliUpEvent::Upload { .. } => EventType::Upload,
            BiliUpEvent::Uploaded { .. } => EventType::Uploaded,
        };

        debug!("Processing event: {:?}", event_type);

        let handlers = self.handlers.read().await;
        if let Some(event_handlers) = handlers.get(&event_type) {
            for handler in event_handlers {
                match handler.handle(event.clone()).await {
                    Ok(Some(next_event)) => {
                        // 如果处理器返回了新事件，继续发送
                        if let Err(e) = self.send_event(next_event).await {
                            error!("Failed to send follow-up event: {}", e);
                        }
                    }
                    Ok(None) => {
                        // 事件处理完成，无后续事件
                    }
                    Err(e) => {
                        error!("Handler failed to process event: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 更新URL状态
    pub async fn update_url_status(&self, url: &str, status: i32) {
        let mut context = self.context.write().await;
        context.url_status.insert(url.to_string(), status);
    }

    /// 获取URL状态
    pub async fn get_url_status(&self, url: &str) -> i32 {
        let context = self.context.read().await;
        context.url_status.get(url).copied().unwrap_or(0)
    }

    /// 增加上传计数
    pub async fn increment_upload_count(&self, url: &str) {
        let mut context = self.context.write().await;
        *context.url_upload_count.entry(url.to_string()).or_insert(0) += 1;
    }

    /// 减少上传计数
    pub async fn decrement_upload_count(&self, url: &str) {
        let mut context = self.context.write().await;
        if let Some(count) = context.url_upload_count.get_mut(url) {
            *count = (*count - 1).max(0);
        }
    }

    /// 检查是否正在上传
    pub async fn is_uploading(&self, url: &str) -> bool {
        let context = self.context.read().await;
        context.url_upload_count.get(url).copied().unwrap_or(0) > 0
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局事件管理器实例
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    struct TestHandler {
        event_type: EventType,
    }

    #[async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, event: BiliUpEvent) -> Result<Option<BiliUpEvent>> {
            println!("Handling event: {:?}", event);
            Ok(None)
        }

        fn event_type(&self) -> EventType {
            self.event_type.clone()
        }
    }

    #[tokio::test]
    async fn test_event_manager() {
        let event_manager = EventManager::new();

        // 注册处理器
        let handler = Box::new(TestHandler {
            event_type: EventType::PreDownload,
        });
        event_manager.register_handler(handler).await;

        // 发送事件
        let event = BiliUpEvent::PreDownload {
            name: "test".to_string(),
            url: "http://test.com".to_string(),
        };

        event_manager.send_event(event).await.unwrap();

        // 等待处理
        sleep(Duration::from_millis(100)).await;
    }
}
