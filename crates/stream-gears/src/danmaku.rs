use async_trait::async_trait;
use biliup_cli::server::core::downloader::DanmakuClient;
use biliup_cli::server::errors::{AppError, AppResult};
use danmaku_client::{DanmakuRecorder, RecorderConfig, RecorderHandle};
use error_stack::Report;
use std::path::PathBuf;
use std::sync::Mutex;

#[allow(dead_code)]
pub struct RustDanmakuClient {
    config: RecorderConfig,
    handle: Mutex<Option<RecorderHandle>>,
}

#[allow(dead_code)]
impl RustDanmakuClient {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            handle: Mutex::new(None),
        }
    }
}

#[async_trait]
impl DanmakuClient for RustDanmakuClient {
    async fn download(&self) -> AppResult<()> {
        let mut handle = self.handle.lock().unwrap();
        if handle.is_some() {
            return Ok(());
        }

        let recorder = DanmakuRecorder::new(self.config.clone())
            .map_err(|e| Report::new(AppError::Custom(e.to_string())))?;
        *handle = Some(recorder.start());
        Ok(())
    }

    async fn stop(&self) -> AppResult<()> {
        let handle = self.handle.lock().unwrap().take();
        if let Some(handle) = handle {
            handle
                .stop()
                .await
                .map_err(|e| Report::new(AppError::Custom(e.to_string())))?;
        }
        Ok(())
    }

    fn rolling(&self, file_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let handle = self
            .handle
            .lock()
            .map_err(|_| "danmaku handle lock poisoned")?
            .clone();
        if let Some(handle) = handle {
            return tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(handle.rolling(Some(PathBuf::from(file_name))))
            })
            .map_err(Into::into);
        }
        Ok(false)
    }
}
