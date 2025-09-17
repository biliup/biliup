use crate::server::errors::{AppError, AppResult, report_to_response};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::Configuration;
use axum::extract::{Query, State};
use axum::response::Response;
use axum::{Extension, Json};
use biliup::client::StatelessClient;
use biliup::uploader::credential::login_by_cookies;
use bytes::Bytes;
use error_stack::{Report, ResultExt};
use ormlite::Model;
use std::collections::HashMap;
use tracing::info;

pub async fn archive_pre_endpoint(
    Query(params): Query<HashMap<String, String>>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<serde_json::Value>, Response> {
    let configurations = Configuration::select()
        .where_("key = 'bilibili-cookies'")
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    for cookies in configurations {
        if let Ok(bili) = login_by_cookies(cookies.value, None).await {
            return Ok(Json(
                bili.archive_pre()
                    .await
                    .change_context(AppError::Unknown)
                    .map_err(report_to_response)?,
            ));
        }
    }
    Err(report_to_response(Report::from(AppError::Custom(
        "无可用 cookie 文件".to_string(),
    ))))
}

pub async fn get_myinfo_endpoint(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, Response> {
    let bili = login_by_cookies(&params["user"], None)
        .await
        .change_context(AppError::Custom(params["user"].to_string()))
        .map_err(report_to_response)?;
    Ok(Json(
        bili.my_info()
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?,
    ))
}

pub async fn get_proxy_endpoint(
    Extension(client): Extension<StatelessClient>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Bytes, Response> {
    info!(params = &params["url"]);
    // let bili = login_by_cookies(&params["user"]).await?;
    Ok(client
        .client
        .get(&params["url"])
        .send()
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?
        .bytes()
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?)
}
