use crate::server::api::bilibili_endpoints::{
    archive_pre_endpoint, get_myinfo_endpoint, get_proxy_endpoint,
};
use crate::server::api::endpoints::{
    add_upload_streamer_endpoint, add_user_endpoint, delete_streamers_endpoint,
    delete_template_endpoint, delete_user_endpoint, get_configuration, get_qrcode,
    get_streamer_info, get_streamers_endpoint, get_upload_streamer_endpoint,
    get_upload_streamers_endpoint, get_users_endpoint, get_videos, login_by_qrcode,
    post_streamers_endpoint, put_configuration, put_streamers_endpoint,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Router;
use axum::routing::{delete, get, post};

/// 创建应用程序路由
pub fn router(service_register: ServiceRegister) -> Router<()> {
    Router::new()
        // 主播管理相关路由
        .route(
            "/v1/streamers",
            get(get_streamers_endpoint)      // 获取主播列表
                .post(post_streamers_endpoint)   // 添加主播
                .put(put_streamers_endpoint),    // 更新主播
        )
        .route("/v1/streamers/{id}", delete(delete_streamers_endpoint)) // 删除主播
        
        // 配置管理路由
        .route(
            "/v1/configuration",
            get(get_configuration).put(put_configuration), // 获取/更新配置
        )
        
        // 主播信息路由
        .route("/v1/streamer-info", get(get_streamer_info)) // 获取主播信息
        
        // 上传模板管理路由
        .route("/v1/upload/streamers", get(get_upload_streamers_endpoint)) // 获取上传模板列表
        .route(
            "/v1/upload/streamers/{id}",
            delete(delete_template_endpoint)     // 删除上传模板
                .get(get_upload_streamer_endpoint), // 获取单个上传模板
        )
        .route("/v1/upload/streamers", post(add_upload_streamer_endpoint)) // 添加上传模板
        
        // 用户管理路由
        .route("/v1/users", get(get_users_endpoint).post(add_user_endpoint)) // 获取用户列表/添加用户
        .route("/v1/users/{id}", delete(delete_user_endpoint)) // 删除用户
        
        // B站API代理路由
        .route("/bili/archive/pre", get(archive_pre_endpoint))   // 投稿预处理
        .route("/bili/space/myinfo", get(get_myinfo_endpoint))   // 获取用户信息
        .route("/bili/proxy", get(get_proxy_endpoint))           // 代理请求
        
        // 认证相关路由
        .route("/v1/get_qrcode", get(get_qrcode))               // 获取二维码
        .route("/v1/login_by_qrcode", post(login_by_qrcode))    // 二维码登录
        
        // 视频文件管理路由
        .route("/v1/videos", get(get_videos))                   // 获取视频列表
        
        .with_state(service_register) // 注入服务注册器状态
}
