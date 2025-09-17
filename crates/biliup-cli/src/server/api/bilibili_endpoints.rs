use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::repositories::models::Configuration;
use axum::extract::{Query, State};
use axum::{Extension, Json};
use biliup::client::StatelessClient;
use biliup::uploader::credential::login_by_cookies;
use bytes::Bytes;
use ormlite::Model;
use std::collections::HashMap;

pub async fn archive_pre_endpoint(
    Query(params): Query<HashMap<String, String>>,
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<serde_json::Value>> {
    let configurations = Configuration::select()
        .where_("key = 'bilibili-cookies'")
        .fetch_all(&pool)
        .await?;
    for cookies in configurations {
        if let Ok(bili) = login_by_cookies(cookies.value, None).await {
            return Ok(Json(bili.archive_pre().await?));
        }
    }
    Err(AppError::InternalServerErrorWithContext(
        "无可用 cookie 文件".to_string(),
    ))
}

pub async fn get_myinfo_endpoint(
    Query(params): Query<HashMap<String, String>>,
) -> AppResult<Json<serde_json::Value>> {
    println!("{}", &params["user"]);
    let bili = login_by_cookies(&params["user"], None).await?;
    Ok(Json(bili.my_info().await?))
}

pub async fn get_proxy_endpoint(
    Extension(client): Extension<StatelessClient>,
    Query(params): Query<HashMap<String, String>>,
) -> AppResult<Bytes> {
    println!("{}", &params["url"]);
    // let bili = login_by_cookies(&params["user"]).await?;
    Ok(client
        .client
        .get(&params["url"])
        .send()
        .await?
        .bytes()
        .await?)
}
