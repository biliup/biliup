use crate::server::errors::{AppError, report_to_response};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::Configuration;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use biliup::uploader::bilibili::ArchivePage;
use biliup::uploader::credential::login_by_cookies;
use error_stack::{Report, ResultExt};
use ormlite::Model;
use serde::Deserialize;
use std::collections::HashMap;

/// B站投稿预处理端点
pub async fn archive_pre_endpoint(
    Query(_params): Query<HashMap<String, String>>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<serde_json::Value>, Response> {
    // 获取所有B站Cookie配置
    let configurations = Configuration::select()
        .where_("key = 'bilibili-cookies'")
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    // 尝试使用每个Cookie进行登录
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

    // 没有可用的Cookie
    Err(report_to_response(Report::from(AppError::Custom(
        "无可用 cookie 文件".to_string(),
    ))))
}

async fn get_registered_cookie(pool: &ConnectionPool, id: i64) -> Result<Configuration, Response> {
    Configuration::select()
        .where_("id = ? AND key = 'bilibili-cookies'")
        .bind(id)
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "B 站用户不存在").into_response())
}

/// 获取已登记 B 站用户的信息；客户端只能提交不透明的数据库 ID。
pub async fn get_user_profile_endpoint(
    Path(id): Path<i64>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<serde_json::Value>, Response> {
    let cookie = get_registered_cookie(&pool, id).await?;
    let bili = login_by_cookies(&cookie.value, None)
        .await
        .change_context(AppError::Custom("无法读取或刷新该用户凭据".into()))
        .map_err(report_to_response)?;

    // 获取用户信息
    Ok(Json(
        bili.my_info()
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?,
    ))
}

fn default_page() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct ArchivesQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default = "default_page")]
    from_page: u32,
    #[serde(default = "default_page")]
    max_pages: u32,
}

impl ArchivesQuery {
    fn validate(&self) -> Result<&'static str, Response> {
        if self.from_page == 0 {
            return Err((StatusCode::BAD_REQUEST, "from_page 必须大于等于 1").into_response());
        }
        if !(1..=20).contains(&self.max_pages) {
            return Err((StatusCode::BAD_REQUEST, "max_pages 必须在 1 到 20 之间").into_response());
        }
        match self.status.as_deref().unwrap_or("all") {
            "all" => Ok("is_pubing,pubed,not_pubed"),
            "is_pubing" => Ok("is_pubing"),
            "pubed" => Ok("pubed"),
            "not_pubed" => Ok("not_pubed"),
            _ => Err((StatusCode::BAD_REQUEST, "无效的稿件状态").into_response()),
        }
    }
}

/// 获取一个明确账号的远程 B 站稿件列表。
pub async fn get_user_archives_endpoint(
    Path(id): Path<i64>,
    State(pool): State<ConnectionPool>,
    Query(query): Query<ArchivesQuery>,
) -> Result<Json<ArchivePage>, Response> {
    let status = query.validate()?;
    let cookie = get_registered_cookie(&pool, id).await?;
    let bili = login_by_cookies(&cookie.value, None)
        .await
        .change_context(AppError::Custom("无法读取或刷新该用户凭据".into()))
        .map_err(report_to_response)?;
    let page = bili
        .recent_archives_page(status, query.from_page, Some(query.max_pages))
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(page))
}

#[cfg(test)]
mod tests {
    use super::{ArchivesQuery, get_registered_cookie};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use axum::http::StatusCode;

    #[test]
    fn archives_query_has_safe_bounds_and_known_statuses() {
        let default = ArchivesQuery {
            status: None,
            from_page: 1,
            max_pages: 1,
        };
        assert_eq!(default.validate().unwrap(), "is_pubing,pubed,not_pubed");

        for query in [
            ArchivesQuery {
                status: None,
                from_page: 0,
                max_pages: 1,
            },
            ArchivesQuery {
                status: None,
                from_page: 1,
                max_pages: 0,
            },
            ArchivesQuery {
                status: None,
                from_page: 1,
                max_pages: 21,
            },
            ArchivesQuery {
                status: Some("unknown".into()),
                from_page: 1,
                max_pages: 1,
            },
        ] {
            assert_eq!(
                query.validate().unwrap_err().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }

    #[tokio::test]
    async fn profile_and_archive_lookup_cannot_select_other_configuration_types() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let config = sqlx::query("INSERT INTO configuration (key, value) VALUES ('config', '{}')")
            .execute(&pool)
            .await
            .unwrap();
        let cookie = sqlx::query(
            "INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', 'cookie.json')",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            get_registered_cookie(&pool, config.last_insert_rowid())
                .await
                .unwrap_err()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get_registered_cookie(&pool, cookie.last_insert_rowid())
                .await
                .unwrap()
                .key,
            "bilibili-cookies"
        );
    }
}
