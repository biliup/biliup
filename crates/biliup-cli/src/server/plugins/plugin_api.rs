// 插件 API 定义
//
// 提供 SegmentProcessorPlugin trait，允许在分段处理阶段注入自定义逻辑

use crate::server::config::{Config, ConfigPatch};
use crate::server::core::downloader::SegmentInfo;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// 分段处理结果
#[derive(Debug)]
pub enum ProcessResult {
    /// 处理完成，文件路径可能已更改
    Completed { new_path: Option<PathBuf> },

    /// 跳过处理（插件未启用或不适用）
    Skipped,

    /// 处理失败
    Failed {
        error: String,
        /// 是否保留原始文件
        preserve_original: bool,
    },
}

impl ProcessResult {
    /// 创建成功结果（路径未变）
    pub fn completed() -> Self {
        Self::Completed { new_path: None }
    }

    /// 创建成功结果（路径已变）
    pub fn completed_with_path(path: PathBuf) -> Self {
        Self::Completed {
            new_path: Some(path),
        }
    }

    /// 创建跳过结果
    pub fn skipped() -> Self {
        Self::Skipped
    }

    /// 创建失败结果（保留原始文件）
    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
            preserve_original: true,
        }
    }

    /// 创建失败结果（不保留原始文件）
    pub fn failed_no_preserve(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
            preserve_original: false,
        }
    }
}

/// 分段处理插件 trait
///
/// 实现此 trait 的插件可以在分段文件关闭后、上传前对其进行处理。
/// 典型用途包括：转码、添加水印、马赛克处理等。
#[async_trait]
pub trait SegmentProcessorPlugin: Send + Sync {
    /// 插件名称（用于日志和识别）
    fn name(&self) -> &'static str;

    /// 检查是否对指定直播间启用
    ///
    /// # 参数
    /// - `streamer_id`: 直播间 ID
    /// - `config`: 全局配置
    /// - `override_cfg`: 直播间覆写配置
    async fn is_enabled(
        &self,
        streamer_id: i64,
        config: &Config,
        override_cfg: &Option<ConfigPatch>,
    ) -> bool;

    /// 处理分段文件
    ///
    /// # 参数
    /// - `segment`: 分段信息
    /// - `config`: 全局配置
    /// - `override_cfg`: 直播间覆写配置
    ///
    /// # 返回
    /// - `Ok(ProcessResult)`: 处理结果
    /// - `Err`: 致命错误（会中断后续处理）
    async fn process_segment(
        &self,
        segment: &SegmentInfo,
        config: &Config,
        override_cfg: &Option<ConfigPatch>,
    ) -> crate::server::errors::AppResult<ProcessResult>;

    /// 清理失败的临时文件
    ///
    /// 在处理失败时调用，用于清理插件创建的临时文件。
    async fn cleanup(&self, segment: &SegmentInfo) -> crate::server::errors::AppResult<()> {
        // 默认实现：无需清理
        let _ = segment;
        Ok(())
    }
}

/// 插件类型别名
pub type Plugin = Arc<dyn SegmentProcessorPlugin>;

/// 插件注册表
///
/// 管理所有注册的分段处理插件
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Plugin>,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// 注册插件
    pub fn register(&mut self, plugin: Plugin) {
        self.plugins.push(plugin);
    }

    /// 获取所有插件
    pub fn plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// 按名称查找插件
    pub fn find(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_result_constructors() {
        let result = ProcessResult::completed();
        assert!(matches!(
            result,
            ProcessResult::Completed { new_path: None }
        ));

        let result = ProcessResult::completed_with_path(PathBuf::from("/test"));
        assert!(matches!(
            result,
            ProcessResult::Completed { new_path: Some(_) }
        ));

        let result = ProcessResult::skipped();
        assert!(matches!(result, ProcessResult::Skipped));

        let result = ProcessResult::failed("test error");
        assert!(matches!(
            result,
            ProcessResult::Failed {
                preserve_original: true,
                ..
            }
        ));
    }
}
