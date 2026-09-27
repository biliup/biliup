//! 配对中本机 HTTP 接口上的增删改：成功之后马上扫一遍、发给对端，不等下一轮定时扫描（ha-pair 方案 H2）。
//!
//! 挂在控制面进程与节点进程的路由上（[`crate::server::fleet::Fleet::guard`]），单机不挂。
//! 没有配对时只读一次原子变量就放行，不读请求体、不碰响应。
//!
//! 节点上新建的主播（`POST /v1/streamers`）加入配对：按响应里的行 id 交给 [`super::member::Member::join_room`]。

use super::member::member_for;
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use serde_json::Value;

/// 新建主播的响应体最多读这么多（一行主播的 JSON）
const CREATED_LIMIT: usize = 1 << 20;

/// 会改动同步集合的请求：账号、主播（含暂停）、投稿模板的增删改
fn touches_pair(method: &Method, path: &str) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::DELETE)
        && ["/v1/users", "/v1/streamers", "/v1/upload/streamers"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
        && !(path.starts_with("/v1/streamers/") && path.ends_with("/live"))
}

fn creates_streamer(method: &Method, path: &str) -> bool {
    *method == Method::POST && path == "/v1/streamers"
}

pub async fn capture(
    State(services): State<ServiceRegister>,
    request: Request,
    next: Next,
) -> Response {
    let (method, path) = (request.method().clone(), request.uri().path().to_string());
    if !touches_pair(&method, &path) {
        return next.run(request).await;
    }
    let Some(member) = member_for(&services) else {
        return next.run(request).await;
    };
    let response = next.run(request).await;
    if !response.status().is_success() {
        return response;
    }
    if !creates_streamer(&method, &path) {
        member.scan().await;
        return response;
    }
    let (parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, CREATED_LIMIT).await else {
        member.scan().await;
        return Response::from_parts(parts, Body::empty());
    };
    let id = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|created| created.get("id").and_then(Value::as_i64));
    match id {
        Some(id) => member.join_room(id).await,
        None => member.scan().await,
    }
    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::member::Member;
    use crate::server::fleet::ha::member::tests::{credential, services};
    use crate::server::fleet::ha::sync::{Side, account_key};
    use crate::server::infrastructure::repositories::register_bilibili_cookie;
    use axum::Router;
    use axum::routing::post;
    use tower::ServiceExt;

    #[test]
    fn only_changes_to_synced_settings_are_captured() {
        assert!(touches_pair(&Method::POST, "/v1/users"));
        assert!(touches_pair(&Method::DELETE, "/v1/users/3"));
        assert!(touches_pair(&Method::PUT, "/v1/streamers"));
        assert!(touches_pair(&Method::PUT, "/v1/streamers/3/pause"));
        assert!(touches_pair(&Method::DELETE, "/v1/upload/streamers/2"));
        assert!(!touches_pair(&Method::GET, "/v1/users"));
        assert!(!touches_pair(&Method::PUT, "/v1/configuration"));
        assert!(
            !touches_pair(&Method::POST, "/v1/streamers/3/live"),
            "手动开录停录不是设置"
        );
        assert!(creates_streamer(&Method::POST, "/v1/streamers"));
        assert!(!creates_streamer(&Method::PUT, "/v1/streamers"));
    }

    /// 登记账号的请求一成功就排进队列，不等定时扫描；没有配对时原样放行
    #[tokio::test]
    async fn a_successful_account_request_is_queued_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(dir.path()).await;
        let file = dir.path().join("77.json");
        std::fs::write(&file, credential(77, "x")).unwrap();
        let pool = services.pool.clone();
        let app = Router::new()
            .route(
                "/v1/users",
                post(move || async move {
                    register_bilibili_cookie(&pool, &file).await.unwrap();
                    "ok"
                }),
            )
            .layer(axum::middleware::from_fn_with_state(
                services.clone(),
                capture,
            ));
        let send = || {
            app.clone().oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/users")
                    .body(Body::empty())
                    .unwrap(),
            )
        };
        assert!(send().await.unwrap().status().is_success());

        let member = Member::start(
            Side::Node,
            dir.path(),
            "peer",
            services.clone(),
            Side::Controller,
        )
        .await;
        let before = member.book().await.get(&account_key(77)).cloned();
        assert!(before.is_some());
        // 让定时扫描的第一拍过去，下一拍在几秒之后
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::fs::write(dir.path().join("77.json"), credential(77, "y")).unwrap();
        assert!(send().await.unwrap().status().is_success());
        assert_ne!(member.book().await.get(&account_key(77)).cloned(), before);
        assert!(member.pending().await > 0);
        member.dissolve();
    }
}
