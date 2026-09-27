//! 配对中本机 HTTP 接口上的增删改：成功之后马上扫一遍、发给对端，不等下一轮定时扫描（ha-pair 方案 H2）。
//!
//! 挂在控制面进程与节点进程的路由上（[`crate::server::fleet::Fleet::guard`]），单机不挂。
//! 没有配对时只读一次原子变量就放行，不读请求体、不碰响应。

use super::member::member_for;
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;

/// 会改动同步集合的请求：账号的登记与删除
fn touches_pair(method: &Method, path: &str) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::DELETE) && path.starts_with("/v1/users")
}

pub async fn capture(
    State(services): State<ServiceRegister>,
    request: Request,
    next: Next,
) -> Response {
    if !touches_pair(request.method(), request.uri().path()) {
        return next.run(request).await;
    }
    let Some(member) = member_for(&services) else {
        return next.run(request).await;
    };
    let response = next.run(request).await;
    if response.status().is_success() {
        member.scan().await;
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::member::Member;
    use crate::server::fleet::ha::member::tests::{credential, services};
    use crate::server::fleet::ha::sync::{Side, account_key};
    use crate::server::infrastructure::repositories::register_bilibili_cookie;
    use axum::Router;
    use axum::body::Body;
    use axum::routing::post;
    use tower::ServiceExt;

    #[test]
    fn only_account_changes_are_captured() {
        assert!(touches_pair(&Method::POST, "/v1/users"));
        assert!(touches_pair(&Method::DELETE, "/v1/users/3"));
        assert!(!touches_pair(&Method::GET, "/v1/users"));
        assert!(!touches_pair(&Method::PUT, "/v1/configuration"));
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
