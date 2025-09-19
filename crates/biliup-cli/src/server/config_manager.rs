use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

/// 配置格式枚举
#[derive(Debug, Clone)]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
}

/// 流配置结构，对应Python版本的streamers配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preprocessor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_processor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postprocessor: Option<Vec<PostprocessorConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_processor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_args: Option<Vec<String>>,

    // 其他字段作为额外的键值对存储
    #[serde(flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

/// 后处理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostprocessorConfig {
    Simple(String),
    Complex {
        #[serde(skip_serializing_if = "Option::is_none")]
        rm: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mv: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        run: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        webhook: Option<String>,
    },
}

/// 用户配置结构，对应Python版本的user配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// 主配置结构，对应Python版本的config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiliUpConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streamers: Option<HashMap<String, StreamerConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool1_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool2_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_loop_interval: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_sourcecode: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_upload_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtering_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_live_cover: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_processor_parallel: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dolby: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hires: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_pay: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_reprint: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_api: Option<String>,

    // 日志配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<HashMap<String, serde_json::Value>>,

    // 其他字段作为额外的键值对存储
    #[serde(flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

impl Default for BiliUpConfig {
    fn default() -> Self {
        Self {
            streamers: None,
            user: None,
            threads: Some(2),
            lines: Some("AUTO".to_string()),
            pool1_size: Some(5),
            pool2_size: Some(3),
            event_loop_interval: Some(30),
            check_sourcecode: Some(15),
            delay: Some(0),
            max_upload_limit: Some(999),
            filtering_threshold: Some(0),
            downloader: Some("stream-gears".to_string()),
            segment_time: Some("01:00:00".to_string()),
            file_size: None,
            filename_prefix: None,
            use_live_cover: Some(false),
            segment_processor_parallel: Some(false),
            dolby: Some(0),
            hires: Some(0),
            charging_pay: Some(0),
            no_reprint: Some(0),
            submit_api: None,
            logging: None,
            extra_fields: HashMap::new(),
        }
    }
}

/// 配置管理器，对应Python版本的Config类
pub struct ConfigManager {
    config: BiliUpConfig,
    config_path: Option<PathBuf>,
    format: Option<ConfigFormat>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: BiliUpConfig::default(),
            config_path: None,
            format: None,
        }
    }

    /// 从文件加载配置，对应Python的load方法
    pub async fn load<P: AsRef<Path>>(&mut self, path: Option<P>) -> Result<()> {
        let config_path = if let Some(path) = path {
            path.as_ref().to_path_buf()
        } else {
            // 自动检测配置文件
            self.find_config_file().await?
        };

        info!("Loading config from: {:?}", config_path);

        let content = fs::read_to_string(&config_path).await?;
        let format = self.detect_format(&config_path)?;

        self.config = match format {
            ConfigFormat::Yaml => serde_yaml::from_str(&content)?,
            ConfigFormat::Toml => toml::from_str(&content)?,
            ConfigFormat::Json => serde_json::from_str(&content)?,
        };

        self.config_path = Some(config_path);
        self.format = Some(format);

        info!("Config loaded successfully");
        Ok(())
    }

    /// 保存配置到文件，对应Python的save方法
    pub async fn save(&self) -> Result<()> {
        let config_path = self
            .config_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config path set"))?;
        let format = self
            .format
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config format set"))?;

        let content = match format {
            ConfigFormat::Yaml => serde_yaml::to_string(&self.config)?,
            ConfigFormat::Toml => toml::to_string_pretty(&self.config)?,
            ConfigFormat::Json => serde_json::to_string_pretty(&self.config)?,
        };

        fs::write(config_path, content).await?;
        info!("Config saved to: {:?}", config_path);
        Ok(())
    }

    /// 保存到指定路径，对应Python的dump方法
    pub async fn dump<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let format = self.detect_format(path)?;

        // 创建备份
        if path.exists() {
            let backup_path = format!(
                "{}.backup.{}",
                path.to_string_lossy(),
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            );
            if let Err(e) = fs::copy(path, &backup_path).await {
                warn!("Failed to create backup: {}", e);
            } else {
                info!("Created backup: {}", backup_path);
            }
        }

        let content = match format {
            ConfigFormat::Yaml => serde_yaml::to_string(&self.config)?,
            ConfigFormat::Toml => toml::to_string_pretty(&self.config)?,
            ConfigFormat::Json => serde_json::to_string_pretty(&self.config)?,
        };

        fs::write(path, content).await?;
        info!("Config dumped to: {:?}", path);
        Ok(())
    }

    /// 自动查找配置文件
    async fn find_config_file(&self) -> Result<PathBuf> {
        let candidates = vec!["config.yaml", "config.yml", "config.toml", "config.json"];

        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(anyhow::anyhow!("No config file found"))
    }

    /// 检测配置文件格式
    fn detect_format<P: AsRef<Path>>(&self, path: P) -> Result<ConfigFormat> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Cannot determine file format"))?;

        match extension.to_lowercase().as_str() {
            "yaml" | "yml" => Ok(ConfigFormat::Yaml),
            "toml" => Ok(ConfigFormat::Toml),
            "json" => Ok(ConfigFormat::Json),
            _ => Err(anyhow::anyhow!("Unsupported config format: {}", extension)),
        }
    }

    /// 获取配置的只读引用
    pub fn get_config(&self) -> &BiliUpConfig {
        &self.config
    }

    /// 获取配置的可变引用
    pub fn get_config_mut(&mut self) -> &mut BiliUpConfig {
        &mut self.config
    }

    /// 获取流配置
    pub fn get_streamer_config(&self, name: &str) -> Option<&StreamerConfig> {
        self.config.streamers.as_ref()?.get(name)
    }

    /// 设置流配置
    pub fn set_streamer_config(&mut self, name: String, config: StreamerConfig) {
        if self.config.streamers.is_none() {
            self.config.streamers = Some(HashMap::new());
        }
        if let Some(ref mut streamers) = self.config.streamers {
            streamers.insert(name, config);
        }
    }

    /// 删除流配置
    pub fn remove_streamer_config(&mut self, name: &str) -> Option<StreamerConfig> {
        self.config.streamers.as_mut()?.remove(name)
    }

    /// 获取所有流名称
    pub fn get_streamer_names(&self) -> Vec<String> {
        self.config
            .streamers
            .as_ref()
            .map(|streamers| streamers.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// 获取配置值（通用方法）
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        // 这是一个简化版本，实际实现需要处理嵌套的键路径
        // 例如：get("streamers.streamer1.url")
        None
    }

    /// 从cookies文件加载，对应Python的load_cookies
    pub async fn load_cookies<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = fs::read_to_string(path).await?;
        let cookies_data: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(cookie_info) = cookies_data.get("cookie_info")
            && let Some(cookies) = cookie_info.get("cookies").and_then(|c| c.as_array()) {
                let mut cookie_map = HashMap::new();
                for cookie in cookies {
                    if let (Some(name), Some(value)) = (
                        cookie.get("name").and_then(|n| n.as_str()),
                        cookie.get("value").and_then(|v| v.as_str()),
                    ) {
                        cookie_map.insert(name.to_string(), value.to_string());
                    }
                }

                if self.config.user.is_none() {
                    self.config.user = Some(UserConfig {
                        cookies: None,
                        access_token: None,
                    });
                }
                if let Some(ref mut user) = self.config.user {
                    user.cookies = Some(cookie_map);
                }
            }

        if let Some(token_info) = cookies_data.get("token_info")
            && let Some(access_token) = token_info.get("access_token").and_then(|t| t.as_str())
                && let Some(ref mut user) = self.config.user {
                    user.access_token = Some(access_token.to_string());
                }

        Ok(())
    }

    /// 合并配置，用于处理覆盖字段
    pub fn merge(&mut self, other: BiliUpConfig) {
        // TODO: 实现深度合并逻辑
        if other.streamers.is_some() {
            self.config.streamers = other.streamers;
        }
        if other.user.is_some() {
            self.config.user = other.user;
        }
        // 合并其他字段...
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局配置管理器实例

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_config_yaml_loading() {
        let yaml_content = r#"
streamers:
  test_streamer:
    url: "https://live.bilibili.com/12345"
    format: "mp4"
    tags: ["测试", "直播"]
threads: 4
pool1_size: 8
"#;

        let mut temp_file = NamedTempFile::new().unwrap();

        let mut manager = ConfigManager::new();
        manager.load(Some(temp_file.path())).await.unwrap();

        let config = manager.get_config();
        assert_eq!(config.threads, Some(4));
        assert_eq!(config.pool1_size, Some(8));

        let streamer = config
            .streamers
            .as_ref()
            .unwrap()
            .get("test_streamer")
            .unwrap();
        assert_eq!(streamer.url, "https://live.bilibili.com/12345");
        assert_eq!(streamer.format, Some("mp4".to_string()));
    }

    #[tokio::test]
    async fn test_config_toml_loading() {
        let toml_content = r#"
threads = 4
pool1_size = 8

[streamers.test_streamer]
url = "https://live.bilibili.com/12345"
format = "mp4"
tags = ["测试", "直播"]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();

        let mut manager = ConfigManager::new();
        manager.load(Some(temp_file.path())).await.unwrap();

        let config = manager.get_config();
        assert_eq!(config.threads, Some(4));
        assert_eq!(config.pool1_size, Some(8));
    }
}
