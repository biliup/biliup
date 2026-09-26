//! `GET /v1/sessions/{id}/danmaku-density`（`file.view`）：这一场的弹幕密度（10 秒一桶）、基线与高峰，
//! 给回看页画曲线。生成过候选的用当时存下的结果，没有就现读弹幕 XML 算；一个弹幕文件都读不到时为空。

use crate::server::auto_clip::danmaku::{self, Density};
use crate::server::auto_clip::files::SessionFiles;
use crate::server::auto_clip::runner;
use crate::server::errors::ApiError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::store;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DanmakuDensity {
    pub density: Option<Density>,
}

pub async fn get_danmaku_density(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<DanmakuDensity>, (StatusCode, Json<ApiError>)> {
    let internal = |error: sqlx::Error| {
        tracing::error!(%error, "读取弹幕密度失败");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(error.to_string())),
        )
    };
    if store::session(&pool, id).await.map_err(internal)?.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError::new("场次不存在".to_string())),
        ));
    }
    let density = match SessionFiles::new(&runner::root(), id).load_danmaku().await {
        Some(density) => Some(density),
        None => danmaku::session_density(&pool, id)
            .await
            .map_err(internal)?,
    };
    Ok(Json(DanmakuDensity { density }))
}
