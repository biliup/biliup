use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::server::event_manager::StreamInfo;

/// 插件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum PluginType {
    Downloader,
    Uploader,
}

/// 插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub plugin_type: PluginType,
    pub valid_url_pattern: String,
    pub priority: i32,
}

/// 下载器插件trait，对应Python版本的DownloadBase
#[async_trait]
pub trait DownloaderPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn valid_url_pattern(&self) -> &str;
    fn priority(&self) -> i32 {
        0
    }

    /// 检查流状态，对应Python的acheck_stream
    async fn check_stream(&self, url: &str, is_check: bool) -> Result<bool>;

    /// 获取流信息
    async fn get_stream_info(&self, url: &str) -> Result<Option<StreamInfo>>;

    /// 开始下载，对应Python的start方法
    async fn start_download(
        &self,
        name: &str,
        url: &str,
        kwargs: HashMap<String, String>,
    ) -> Result<StreamInfo>;

    /// 检查是否应该录制，对应Python的should_record
    fn should_record(&self, stream_info: &StreamInfo) -> bool {
        // TODO: 实现关键词和时间范围检查
        true
    }
}

/// 上传器插件trait，对应Python版本的UploadBase
#[async_trait]
pub trait UploaderPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 {
        0
    }

    /// 上传文件列表
    async fn upload(&self, files: Vec<String>, stream_info: &StreamInfo) -> Result<Vec<String>>;

    /// 获取上传配置
    fn get_upload_config(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Python插件包装器，用于调用Python插件
#[cfg(feature = "python-bridge")]
pub struct PythonPluginWrapper {
    name: String,
    plugin_type: PluginType,
    module_path: String,
    class_name: String,
    // Python对象引用将在实际实现时添加
}

#[cfg(feature = "python-bridge")]
impl PythonPluginWrapper {
    pub fn new(
        name: String,
        plugin_type: PluginType,
        module_path: String,
        class_name: String,
    ) -> Self {
        Self {
            name,
            plugin_type,
            module_path,
            class_name,
        }
    }
}

#[cfg(feature = "python-bridge")]
#[async_trait]
impl DownloaderPlugin for PythonPluginWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn valid_url_pattern(&self) -> &str {
        // TODO: 从Python模块获取VALID_URL_BASE
        ""
    }

    async fn check_stream(&self, url: &str, is_check: bool) -> Result<bool> {
        // TODO: 使用PyO3调用Python插件的acheck_stream方法
        Ok(false)
    }

    async fn get_stream_info(&self, url: &str) -> Result<Option<StreamInfo>> {
        // TODO: 从Python插件获取流信息
        Ok(None)
    }

    async fn start_download(
        &self,
        name: &str,
        url: &str,
        kwargs: HashMap<String, String>,
    ) -> Result<StreamInfo> {
        // TODO: 调用Python插件的start方法
        Err(anyhow::anyhow!("Not implemented"))
    }
}

/// 插件注册表
pub struct PluginRegistry {
    downloaders: Arc<RwLock<HashMap<String, Box<dyn DownloaderPlugin>>>>,
    uploaders: Arc<RwLock<HashMap<String, Box<dyn UploaderPlugin>>>>,
    url_patterns: Arc<RwLock<Vec<(regex::Regex, String)>>>, // 正则表达式和插件名的映射
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            downloaders: Arc::new(RwLock::new(HashMap::new())),
            uploaders: Arc::new(RwLock::new(HashMap::new())),
            url_patterns: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册下载器插件
    pub async fn register_downloader(&self, plugin: Box<dyn DownloaderPlugin>) -> Result<()> {
        let name = plugin.name().to_string();
        let pattern = plugin.valid_url_pattern().to_string();

        info!("Registering downloader plugin: {}", name);

        // 编译正则表达式
        if let Ok(regex) = regex::Regex::new(&pattern) {
            let mut patterns = self.url_patterns.write().await;
            patterns.push((regex, name.clone()));
        } else {
            warn!("Invalid URL pattern for plugin {}: {}", name, pattern);
        }

        let mut downloaders = self.downloaders.write().await;
        downloaders.insert(name, plugin);

        Ok(())
    }

    /// 注册上传器插件
    pub async fn register_uploader(&self, plugin: Box<dyn UploaderPlugin>) -> Result<()> {
        let name = plugin.name().to_string();
        info!("Registering uploader plugin: {}", name);

        let mut uploaders = self.uploaders.write().await;
        uploaders.insert(name, plugin);

        Ok(())
    }

    /// 根据URL找到对应的下载器插件
    pub async fn find_downloader(&self, url: &str) -> Option<String> {
        let patterns = self.url_patterns.read().await;

        for (regex, plugin_name) in patterns.iter() {
            if regex.is_match(url) {
                debug!("Found downloader plugin {} for URL: {}", plugin_name, url);
                return Some(plugin_name.clone());
            }
        }

        None
    }

    /// 获取下载器插件
    pub async fn get_downloader(
        &self,
        name: &str,
    ) -> Option<tokio::sync::RwLockReadGuard<HashMap<String, Box<dyn DownloaderPlugin>>>> {
        let downloaders = self.downloaders.read().await;
        if downloaders.contains_key(name) {
            Some(downloaders)
        } else {
            None
        }
    }

    /// 获取上传器插件
    pub async fn get_uploader(
        &self,
        name: &str,
    ) -> Option<tokio::sync::RwLockReadGuard<HashMap<String, Box<dyn UploaderPlugin>>>> {
        let uploaders = self.uploaders.read().await;
        if uploaders.contains_key(name) {
            Some(uploaders)
        } else {
            None
        }
    }

    /// 列出所有插件
    pub async fn list_plugins(&self) -> (Vec<String>, Vec<String>) {
        let downloaders = self.downloaders.read().await;
        let uploaders = self.uploaders.read().await;

        (
            downloaders.keys().cloned().collect(),
            uploaders.keys().cloned().collect(),
        )
    }

    /// 从Python插件目录加载插件
    #[cfg(feature = "python-bridge")]
    pub async fn load_python_plugins(&self, plugin_dir: &str) -> Result<()> {
        use std::fs;
        use std::path::Path;

        info!("Loading Python plugins from: {}", plugin_dir);

        let plugin_path = Path::new(plugin_dir);
        if !plugin_path.exists() {
            return Err(anyhow::anyhow!(
                "Plugin directory does not exist: {}",
                plugin_dir
            ));
        }

        // 扫描Python文件
        for entry in fs::read_dir(plugin_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("py") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // 跳过特殊文件
                    if file_name.starts_with("__") {
                        continue;
                    }

                    // TODO: 实际的Python插件加载逻辑
                    info!("Found Python plugin file: {}", file_name);

                    // 创建Python插件包装器
                    let plugin = PythonPluginWrapper::new(
                        file_name.to_string(),
                        PluginType::Downloader,
                        format!("biliup.plugins.{}", file_name),
                        "Plugin".to_string(),
                    );

                    // self.register_downloader(Box::new(plugin)).await?;
                }
            }
        }

        Ok(())
    }

    /// 检查所有URL的流状态，对应Python的batch_check
    pub async fn batch_check(&self, urls: &[String]) -> Result<Vec<String>> {
        let mut online_urls = Vec::new();

        for url in urls {
            if let Some(plugin_name) = self.find_downloader(url).await {
                let downloaders = self.downloaders.read().await;
                if let Some(plugin) = downloaders.get(&plugin_name) {
                    match plugin.check_stream(url, true).await {
                        Ok(true) => {
                            debug!("Stream online: {}", url);
                            online_urls.push(url.clone());
                        }
                        Ok(false) => {
                            debug!("Stream offline: {}", url);
                        }
                        Err(e) => {
                            warn!("Failed to check stream {}: {}", url, e);
                        }
                    }
                }
            }
        }

        Ok(online_urls)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 全局插件注册表实例
/// 插件初始化函数，对应Python版本的插件发现机制
pub async fn initialize_plugins() -> Result<()> {
    // TODO: 注册内置的Rust插件
    info!("Registering built-in plugins...");

    // TODO: 加载Python插件（如果启用了python-bridge特性）
    #[cfg(feature = "python-bridge")]
    {
        info!("Loading Python plugins...");
        let python_plugin_dir =
            std::env::var("BILIUP_PLUGIN_DIR").unwrap_or_else(|_| "biliup/plugins".to_string());

        if let Err(e) = registry.load_python_plugins(&python_plugin_dir).await {
            warn!("Failed to load Python plugins: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDownloader {
        name: String,
    }

    #[async_trait]
    impl DownloaderPlugin for TestDownloader {
        fn name(&self) -> &str {
            &self.name
        }

        fn valid_url_pattern(&self) -> &str {
            r"https://test\.com/.*"
        }

        async fn check_stream(&self, _url: &str, _is_check: bool) -> Result<bool> {
            Ok(true)
        }

        async fn get_stream_info(&self, _url: &str) -> Result<Option<StreamInfo>> {
            Ok(None)
        }

        async fn start_download(
            &self,
            _name: &str,
            _url: &str,
            _kwargs: HashMap<String, String>,
        ) -> Result<StreamInfo> {
            Err(anyhow::anyhow!("Test plugin"))
        }
    }

    #[tokio::test]
    async fn test_plugin_registry() {
        let registry = PluginRegistry::new();

        let plugin = Box::new(TestDownloader {
            name: "test".to_string(),
        });

        registry.register_downloader(plugin).await.unwrap();

        let plugin_name = registry.find_downloader("https://test.com/stream").await;
        assert_eq!(plugin_name, Some("test".to_string()));

        let (downloaders, uploaders) = registry.list_plugins().await;
        assert_eq!(downloaders.len(), 1);
        assert_eq!(uploaders.len(), 0);
    }
}
