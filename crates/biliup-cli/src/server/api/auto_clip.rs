//! 自动切片的连通性测试与状态。
//!
//! - `POST /v1/auto-clip/test`（`config.edit`）：用表单里当前的值（未保存也能测）测 chat、
//!   看图与转写，结果存下来供缩图「自动」使用。表单里的 key 是掩码时换成已保存的 key，规则同保存。
//! - `GET /v1/auto-clip/status`（`file.view`）：是否启用、模型名、能否看图、上限，不含 key 和地址。
//! - `GET /v1/sessions/{id}/auto-clip`（`file.view`）：这一场最近的任务与用量预估。
//! - `POST /v1/sessions/{id}/auto-clip`（`clip.edit`）：不带 `confirm` 只回预估；`confirm: true`
//!   入队（超过每场转写上限时拒绝）。
//! - `DELETE /v1/sessions/{id}/auto-clip`（`clip.edit`）：取消排队或运行中的任务。

use crate::server::api::access::Caller;
use crate::server::auto_clip::jobs::{self, Job, NewJob, Trigger};
use crate::server::auto_clip::probe::{self, CheckStatus, ProbeReport, Targets};
use crate::server::auto_clip::runner::{self, Basis, Estimate};
use crate::server::auto_clip::settings::{
    self, API_KEY_ENV, AutoClipConfig, KeySource, Thumbnails, display_host,
};
use crate::server::config::Config;
use crate::server::errors::ApiError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::recorder::now_ms;
use crate::server::workbench::{live, store};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, PartialEq)]
pub struct SessionAutoClip {
    /// 全局 `auto_clip.enabled`
    pub enabled: bool,
    /// 这一场最近的一条任务
    pub job: Option<Job>,
    /// 这次要送转写多少；没打开时为空
    pub estimate: Option<Estimate>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StartAutoClip {
    /// `false`（默认）只回预估，不入队
    pub confirm: bool,
    /// 沿用已转写的部分（默认）；`false` 从头转写
    pub reuse_transcript: bool,
}

impl Default for StartAutoClip {
    fn default() -> Self {
        StartAutoClip {
            confirm: false,
            reuse_transcript: true,
        }
    }
}

type Rejection = (StatusCode, Json<ApiError>);

fn reject(status: StatusCode, message: impl Into<String>) -> Rejection {
    (status, Json(ApiError::new(message.into())))
}

fn internal(error: sqlx::Error) -> Rejection {
    tracing::error!(%error, "读写自动切片任务失败");
    reject(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

async fn require_session(pool: &ConnectionPool, id: i64) -> Result<store::SessionRow, Rejection> {
    store::session(pool, id)
        .await
        .map_err(internal)?
        .ok_or_else(|| reject(StatusCode::NOT_FOUND, "场次不存在"))
}

pub async fn get_session_auto_clip(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<SessionAutoClip>, Rejection> {
    require_session(&pool, id).await?;
    let current = config.read().unwrap().auto_clip.clone();
    let enabled = current.as_ref().is_some_and(|c| c.enabled);
    let job = jobs::latest(&pool, id).await.map_err(internal)?;
    let estimate = match current.filter(|c| c.enabled) {
        Some(current) => Some(
            runner::estimate(&pool, &current, id, true)
                .await
                .map_err(internal)?,
        ),
        None => None,
    };
    Ok(Json(SessionAutoClip {
        enabled,
        job,
        estimate,
    }))
}

pub async fn start_session_auto_clip(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
    caller: Caller,
    Path(id): Path<i64>,
    body: Option<Json<StartAutoClip>>,
) -> Result<(StatusCode, Json<SessionAutoClip>), Rejection> {
    let request = body.map(|Json(body)| body).unwrap_or_default();
    let session = require_session(&pool, id).await?;
    let current = config
        .read()
        .unwrap()
        .auto_clip
        .clone()
        .filter(|c| c.enabled)
        .ok_or_else(|| {
            reject(
                StatusCode::CONFLICT,
                "自动切片没有开启：到设置页「自动切片（实验）」里打开并填好转写接口",
            )
        })?;
    if runner::asr_endpoint(&current).is_none() {
        return Err(reject(
            StatusCode::CONFLICT,
            "没有配置转写接口：到设置页「自动切片（实验）」填好转写的接口地址和模型",
        ));
    }
    if session.ended_at.is_none() || live::is_recording(id) {
        return Err(reject(StatusCode::CONFLICT, "这一场还在录，下播后再生成"));
    }
    let latest = jobs::latest(&pool, id).await.map_err(internal)?;
    if latest.as_ref().is_some_and(Job::is_active) {
        return Err(reject(
            StatusCode::CONFLICT,
            "这一场已经有排队或运行中的任务",
        ));
    }
    let estimate = runner::estimate(&pool, &current, id, request.reuse_transcript)
        .await
        .map_err(internal)?;
    if estimate.recorded_seconds == 0 {
        return Err(reject(
            StatusCode::CONFLICT,
            "这一场没有录完的分段，没有可以转写的音频",
        ));
    }
    if !request.confirm {
        return Ok((
            StatusCode::OK,
            Json(SessionAutoClip {
                enabled: true,
                job: latest,
                estimate: Some(estimate),
            }),
        ));
    }
    if estimate.over_limit && estimate.basis == Basis::Silence {
        return Err(reject(
            StatusCode::CONFLICT,
            estimate.message.clone().unwrap_or_default(),
        ));
    }
    let now = now_ms();
    let new_job = NewJob::builder()
        .session_id(id)
        .trigger(Trigger::Manual)
        .not_before(now)
        .reuse_transcript(request.reuse_transcript)
        .maybe_created_by(caller.subject.user_id)
        .created_at(now)
        .build();
    let job = jobs::insert(&pool, &new_job)
        .await
        .map_err(internal)?
        .ok_or_else(|| reject(StatusCode::CONFLICT, "这一场已经有排队或运行中的任务"))?;
    info!(
        session = id,
        job = job.id,
        asr_seconds = estimate.asr_seconds,
        "自动切片：手动生成候选入队"
    );
    runner::kick();
    Ok((
        StatusCode::CREATED,
        Json(SessionAutoClip {
            enabled: true,
            job: Some(job),
            estimate: Some(estimate),
        }),
    ))
}

pub async fn cancel_session_auto_clip(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<Job>, Rejection> {
    require_session(&pool, id).await?;
    let job = jobs::cancel(&pool, id, now_ms())
        .await
        .map_err(internal)?
        .ok_or_else(|| reject(StatusCode::NOT_FOUND, "这一场没有排队或运行中的任务"))?;
    info!(session = id, job = job.id, "自动切片：任务已取消");
    Ok(Json(job))
}

#[cfg(test)]
mod tests;
