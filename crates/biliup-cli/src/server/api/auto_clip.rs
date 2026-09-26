//! 自动切片的连通性测试与状态。
//!
//! - `POST /v1/auto-clip/test`（`config.edit`）：用表单里当前的值（未保存也能测）测 chat、
//!   看图与转写，结果存下来供缩图「自动」使用。表单里的 key 是掩码时换成已保存的 key，规则同保存。
//! - `GET /v1/auto-clip/status`（`file.view`）：是否启用、模型名、能否看图、上限，不含 key 和地址。

use crate::server::auto_clip::probe::{self, CheckStatus, ProbeReport, Targets};
use crate::server::auto_clip::settings::{
    self, API_KEY_ENV, AutoClipConfig, KeySource, Thumbnails, display_host,
};
use crate::server::config::Config;
use crate::server::errors::ApiError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

pub async fn test_auto_clip(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
    Json(form): Json<AutoClipConfig>,
) -> Result<Json<ProbeReport>, (StatusCode, Json<ApiError>)> {
    let stored = config.read().unwrap().auto_clip.clone();
    let form = form.normalized().unwrap_or_default();
    let form = settings::restore_masked_keys(stored.as_ref(), form)
        .map_err(|message| (StatusCode::BAD_REQUEST, Json(ApiError::new(message))))?;
    let targets = Targets::from_config(&form, std::env::var(API_KEY_ENV).ok());
    let report = probe::run(&probe::client_for(&form), &targets).await;
    info!(
        base_url = ?report.chat_base_url,
        chat_model = ?report.chat_model,
        asr_base_url = ?report.asr_base_url,
        asr_model = ?report.asr_model,
        chat = ?report.chat.status,
        vision = ?report.vision.status,
        asr = ?report.asr.status,
        "自动切片连通性测试"
    );
    if let Err(error) = probe::save(&pool, &report).await {
        warn!(%error, "保存自动切片连通性测试结果失败");
    }
    Ok(Json(report))
}

#[derive(Debug, Serialize, PartialEq)]
pub struct StatusView {
    pub enabled: bool,
    /// 填了接口地址和 chat 模型
    pub configured: bool,
    /// 接口地址的主机名，给隐私提示用
    pub api_host: Option<String>,
    pub chat_model: Option<String>,
    pub asr_host: Option<String>,
    pub asr_model: Option<String>,
    pub key_source: Option<KeySource>,
    pub thumbnails: Thumbnails,
    /// 缩图实际开不开：`auto` 时看连通性测试；还没测过当前模型为 `false`
    pub thumbnails_active: bool,
    /// 当前 chat 模型能否看图；没测过为空
    pub vision: Option<bool>,
    /// 当前转写模型有没有分句时间戳；没测过为空
    pub asr_segments: Option<bool>,
    pub max_asr_minutes: u64,
    pub max_chat_tokens: u64,
    pub last_test: Option<LastTest>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct LastTest {
    pub tested_at: i64,
    pub chat: CheckStatus,
    pub vision: CheckStatus,
    pub asr: CheckStatus,
}

pub async fn auto_clip_status(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
) -> Json<StatusView> {
    let current = config.read().unwrap().auto_clip.clone().unwrap_or_default();
    let report = probe::load(&pool).await.unwrap_or_else(|error| {
        warn!(%error, "读取自动切片连通性测试结果失败");
        None
    });
    Json(status_view(
        &current,
        report.as_ref(),
        std::env::var(API_KEY_ENV).ok(),
    ))
}

fn status_view(
    config: &AutoClipConfig,
    report: Option<&ProbeReport>,
    env_key: Option<String>,
) -> StatusView {
    let vision = report
        .filter(|report| report.matches_chat(config))
        .and_then(|report| report.vision.capable);
    let asr_segments = report
        .filter(|report| report.matches_asr(config))
        .and_then(|report| report.asr.capable);
    let thumbnails = config.thumbnails();
    StatusView {
        enabled: config.enabled,
        configured: config.base_url.is_some() && config.chat_model.is_some(),
        api_host: config.base_url.as_deref().and_then(display_host),
        chat_model: config.chat_model.clone(),
        asr_host: config.asr_base_url().and_then(display_host),
        asr_model: config.asr_model.clone(),
        key_source: config.chat_key_with(env_key).map(|(_, source)| source),
        thumbnails,
        thumbnails_active: match thumbnails {
            Thumbnails::On => true,
            Thumbnails::Off => false,
            Thumbnails::Auto => vision == Some(true),
        },
        vision,
        asr_segments,
        max_asr_minutes: config.max_asr_minutes(),
        max_chat_tokens: config.max_chat_tokens(),
        last_test: report.map(|report| LastTest {
            tested_at: report.tested_at,
            chat: report.chat.status,
            vision: report.vision.status,
            asr: report.asr.status,
        }),
    }
}
#[cfg(test)]
mod tests;
