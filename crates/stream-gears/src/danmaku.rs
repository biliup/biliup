use async_trait::async_trait;
use biliup_cli::server::core::downloader::{DownloadStatus, Downloader, SegmentEvent};
use biliup_cli::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;

/// DanmakuClient provides Rust bindings to the Python DanmakuClient
/// for recording live chat (danmaku) alongside video streams.
pub struct DanmakuClient {
    py_client: Arc<Py<PyAny>>,
}

impl DanmakuClient {
    /// Creates a new DanmakuClient instance
    pub fn new(py_client: Arc<Py<PyAny>>) -> Self {
        Self { py_client }
    }
}

#[async_trait]
impl Downloader for DanmakuClient {
    /// Starts danmaku recording and manages lifecycle
    async fn download(
        &self,
        _callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus> {
        let py_client = self.py_client.clone();
        tokio::task::spawn_blocking(move || {
            Python::attach(|py| {
                let py_client = py_client.bind(py);
                py_client.call_method0("start")?;
                Ok::<_, PyErr>(())
            })
        })
        .await
        .change_context(AppError::Unknown)?
        .change_context(AppError::Unknown)?;

        // Start the danmaku recording
        // self.start()
        //     .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // Return downloading status - the actual recording runs in the background
        // The Python DanmakuClient handles the recording lifecycle internally
        Ok(DownloadStatus::Downloading)
    }

    /// Stops the danmaku recording
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let py_client = self.py_client.clone();
        // Call the DanmakuClient's stop method (not the trait method)
        tokio::task::spawn_blocking(move || {
            Python::attach(|py| {
                let py_client = py_client.bind(py);
                py_client.call_method0("stop")?;
                Ok::<_, PyErr>(())
            })
        })
        .await??;
        Ok(())
    }

    /// Saves current recording and starts new file (rolling)
    fn rolling(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Forward to Python client.save() - this saves current recording
        // and the Python client handles starting a new recording file
        let py_client = self.py_client.clone();
        let file_name = file_name.to_string();
        Python::attach(|py| {
            let py_client = py_client.bind(py);
            py_client.call_method1("save", (file_name,))?;
            Ok::<_, PyErr>(())
        })?;
        Ok(())
    }
}
