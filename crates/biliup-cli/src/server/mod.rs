pub mod errors;

pub mod api {
    pub mod bilibili_endpoints;
    pub mod endpoints;
    pub mod router;
    pub mod spa;
}

pub mod core;

pub mod infrastructure;

// 新增的模块
pub mod config_manager;
pub mod event_manager;
pub mod handlers;
pub mod integration;
pub mod plugin_registry;
