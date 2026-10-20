//! Legacy postprocessor webhook: notify once per upload attempt, before touching files.
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::hook_step::HookStep;
use error_stack::ResultExt;
use serde_json::{Value, json};
use std::time::Duration;

fn payload(ctx: &Context, caught_error: Option<&str>) -> AppResult<Value> {
    let mut info = serde_json::to_value(ctx.streamer_info()).change_context(AppError::Unknown)?;
    let fields = info.as_object_mut().expect("StreamerInfo is an object");
    if let Some(template) = ctx.upload_config() {
        let template_fields = serde_json::to_value(template).change_context(AppError::Unknown)?;
        fields.extend(
            template_fields
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, value)| key.as_str() != "id" && !value.is_null())
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        let mut recorder = ctx.recorder(ctx.streamer_info().clone());
        recorder.filename_prefix = template.title.clone();
        fields.insert("format_title".into(), json!(recorder.format_title()));
        fields.insert(
            "format_desc".into(),
            json!(recorder.format(template.description.as_deref().unwrap_or(""))),
        );
    }
    let room = serde_json::to_value(ctx.live_streamer()).change_context(AppError::Unknown)?;
    fields.extend(
        room.as_object()
            .unwrap()
            .iter()
            .filter(|(key, value)| key.as_str() != "id" && !value.is_null())
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    // Python keyed streamers by remark, not the broadcaster's platform nickname.
    fields.insert("name".into(), json!(ctx.live_streamer().remark));
    Ok(json!({ "stream_info": info, "caught_error": caught_error.unwrap_or("") }))
}

pub async fn notify(ctx: &Context, caught_error: Option<&str>) -> AppResult<()> {
    let hooks: Vec<_> = ctx
        .live_streamer()
        .postprocessor
        .iter()
        .flatten()
        .filter_map(|step| {
            if let HookStep::Webhook { webhook } = step {
                Some(webhook)
            } else {
                None
            }
        })
        .collect();
    if hooks.is_empty() {
        return Ok(());
    }
    let body = payload(ctx, caught_error)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .change_context(AppError::Unknown)?;
    for endpoint in hooks {
        let url = reqwest::Url::parse(endpoint.trim())
            .map_err(|_| AppError::Custom("webhook 地址无效".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Custom("webhook 仅支持 HTTP 或 HTTPS 地址".into()).into());
        }
        let mut response = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .change_context(AppError::Custom("webhook 请求失败，保留文件".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Custom(format!(
                "webhook 返回 HTTP {}，保留文件",
                response.status()
            ))
            .into());
        }
        // Only a tiny acknowledgement is expected; never buffer an unbounded response.
        let mut answer = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(reqwest::Error::without_url)
            .change_context(AppError::Custom("webhook 响应读取失败，保留文件".into()))?
        {
            if answer.len() + chunk.len() > 128 {
                return Err(AppError::Custom("webhook 未返回 success，保留文件".into()).into());
            }
            answer.extend_from_slice(&chunk);
        }
        if answer != b"success" {
            return Err(AppError::Custom("webhook 未返回 success，保留文件".into()).into());
        }
    }
    Ok(())
}

/// Notify failed uploads too, without treating an acknowledgement as a successful
/// upload or replacing the original error. No file cleanup runs on this path.
pub async fn on_failure<T>(ctx: &Context, result: AppResult<T>) -> AppResult<T> {
    if let Err(error) = &result {
        if let Err(callback_error) = notify(ctx, Some(&format!("{error:#}"))).await {
            tracing::warn!(error = %callback_error, "上传失败后的 webhook 回调失败");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::common::upload::{execute_postprocessor, process_with_upload};
    use crate::server::config::Config;
    use crate::server::infrastructure::context::Worker;
    use axum::{Json, Router, http::StatusCode, routing::post};
    use futures::StreamExt;
    use std::{
        path::PathBuf,
        sync::{Arc, RwLock},
    };
    use tokio::sync::mpsc;

    async fn context(endpoint: &str) -> Context {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let room = serde_json::from_value(json!({
            "id": 1, "url": "https://example.invalid/live", "remark": "录播备注",
            "postprocessor": ["rm", {"webhook": endpoint}]
        }))
        .unwrap();
        let template = serde_json::from_value(json!({
            "id": 2, "template_name": "测试模板", "title": "投稿 {title}",
            "description": "来自 {streamer}", "tags": ["录播"], "tid": 171,
            "user_cookie": "__webhook_test_missing_cookies__.json"
        }))
        .unwrap();
        let worker = Arc::new(Worker::new(
            room,
            Some(template),
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        ));
        let live = serde_json::from_value(json!({
            "name": "主播昵称", "url": "https://example.invalid/live", "title": "直播标题",
            "date": chrono::Utc::now(), "live_cover_url": "", "raw_stream_url": "",
            "platform": "test", "stream_headers": {}, "suffix": "flv",
            "danmaku": null, "downloader_hint": "StreamGears", "runtime_options": null
        }))
        .unwrap();
        Context::new(1, worker, pool, live)
    }

    async fn receiver(
        status: StatusCode,
        response: String,
        file: PathBuf,
    ) -> (
        String,
        mpsc::UnboundedReceiver<(Value, bool)>,
        tokio::task::JoinHandle<()>,
    ) {
        let (send, receive) = mpsc::unbounded_channel();
        let app = Router::new().route(
            "/hook",
            post(move |Json(body): Json<Value>| {
                let response = response.clone();
                let _ = send.send((body, file.exists()));
                async move { (status, response) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/hook", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (endpoint, receive, server)
    }

    #[test]
    fn legacy_webhook_config_round_trips() {
        let config = json!([{"webhook": "http://localhost/hook"}, "rm"]);
        let steps: Vec<HookStep> = serde_json::from_value(config.clone()).unwrap();
        assert!(
            matches!(&steps[0], HookStep::Webhook { webhook } if webhook == "http://localhost/hook")
        );
        assert_eq!(serde_json::to_value(steps).unwrap(), config);
    }

    #[tokio::test]
    async fn webhook_acknowledgement_gates_even_an_earlier_rm() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("recording.flv");
        std::fs::write(&file, b"video").unwrap();
        let (url, mut messages, server) =
            receiver(StatusCode::OK, "success".into(), file.clone()).await;
        let ctx = context(&url).await;
        execute_postprocessor(vec![file.clone()], &ctx)
            .await
            .unwrap();
        let (body, existed) = messages.recv().await.unwrap();
        assert!(
            existed,
            "callback must run before rm, even when listed after it"
        );
        assert!(!file.exists());
        assert_eq!(body["caught_error"], "");
        assert_eq!(body["stream_info"]["name"], "录播备注");
        assert_eq!(body["stream_info"]["format_title"], "投稿 直播标题");
        assert_eq!(body["stream_info"]["format_desc"], "来自 主播昵称");
        assert_eq!(body["stream_info"]["tid"], 171);
        assert!(
            messages.try_recv().is_err(),
            "only one callback per attempt"
        );
        server.abort();
    }

    #[tokio::test]
    async fn webhook_rejection_and_http_errors_keep_recordings() {
        for (status, answer) in [
            (StatusCode::OK, "rejected".into()),
            (StatusCode::BAD_GATEWAY, "success".into()),
            (StatusCode::OK, "x".repeat(129)),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("recording.flv");
            std::fs::write(&file, b"video").unwrap();
            let (url, mut messages, server) = receiver(status, answer, file.clone()).await;
            let ctx = context(&url).await;
            assert!(
                execute_postprocessor(vec![file.clone()], &ctx)
                    .await
                    .is_err()
            );
            assert!(file.exists());
            assert!(messages.recv().await.unwrap().1);
            assert!(messages.try_recv().is_err());
            server.abort();
        }
    }

    #[tokio::test]
    async fn webhook_reports_real_upload_initialization_failure_without_erasing_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("recording.flv");
        std::fs::write(&file, b"video").unwrap();
        let (url, mut messages, server) =
            receiver(StatusCode::OK, "success".into(), file.clone()).await;
        let ctx = context(&url).await;
        let (tx, rx) = async_channel::unbounded();
        drop(tx);
        let error = process_with_upload(
            rx.inspect(|_| {}),
            &ctx,
            ctx.upload_config().as_ref().unwrap(),
        )
        .await
        .unwrap_err();
        let (body, existed) = messages.recv().await.unwrap();
        assert!(format!("{error:#}").contains("cookies"));
        assert!(body["caught_error"].as_str().unwrap().contains("cookies"));
        assert!(existed && file.exists());
        assert!(messages.try_recv().is_err());
        server.abort();
    }

    #[tokio::test]
    async fn webhook_failure_does_not_replace_the_upload_error() {
        let (url, mut messages, server) =
            receiver(StatusCode::SERVICE_UNAVAILABLE, "".into(), PathBuf::new()).await;
        let ctx = context(&url).await;
        let error = on_failure::<()>(&ctx, Err(AppError::Custom("投稿失败".into()).into()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("投稿失败"));
        assert_eq!(messages.recv().await.unwrap().0["caught_error"], "投稿失败");
        server.abort();
    }

    #[tokio::test]
    async fn webhook_rejects_non_http_endpoints_and_non_upload_stages() {
        let ctx = context("file:///tmp/hook").await;
        assert!(
            notify(&ctx, None)
                .await
                .unwrap_err()
                .to_string()
                .contains("HTTP")
        );
        let hook = HookStep::Webhook {
            webhook: "https://example.invalid".into(),
        };
        assert!(hook.execute(&[]).await.is_err());
        assert!(hook.execute_with(b"{}").await.is_err());
    }
}
