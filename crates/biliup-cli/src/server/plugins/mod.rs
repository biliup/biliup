// 插件系统模块
//
// 提供分段处理插件的注册和管理功能

pub(crate) mod audio;
pub mod mosaic;
pub mod plugin_api;

pub use plugin_api::{Plugin, PluginRegistry, ProcessResult, SegmentProcessorPlugin};

use once_cell::sync::Lazy;
use std::sync::Mutex;

/// 全局插件注册表
///
/// 使用 Lazy + Mutex 实现线程安全的单例模式
static GLOBAL_REGISTRY: Lazy<Mutex<PluginRegistry>> = Lazy::new(|| {
    let mut registry = PluginRegistry::new();

    // 注册内置插件
    register_builtin_plugins(&mut registry);

    Mutex::new(registry)
});

/// 注册内置插件
fn register_builtin_plugins(registry: &mut PluginRegistry) {
    use std::sync::Arc;

    // 注册马赛克插件
    registry.register(Arc::new(mosaic::MosaicPlugin::new()));

    tracing::info!("已注册 {} 个内置插件", registry.plugins().len());
}

/// 获取全局插件注册表
pub fn global_registry() -> &'static Mutex<PluginRegistry> {
    &GLOBAL_REGISTRY
}

/// 获取所有已注册的插件
pub fn get_all_plugins() -> Vec<Plugin> {
    GLOBAL_REGISTRY.lock().unwrap().plugins().to_vec()
}

/// 按名称查找插件
pub fn find_plugin(name: &str) -> Option<Plugin> {
    GLOBAL_REGISTRY.lock().unwrap().find(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_registry() {
        let plugins = get_all_plugins();
        assert!(!plugins.is_empty(), "应该至少有一个内置插件");

        let mosaic = find_plugin("mosaic");
        assert!(mosaic.is_some(), "应该能找到 mosaic 插件");
    }
}
