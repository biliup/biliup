//! 弹幕密度曲线接口：读场次分段的弹幕 XML，不调用模型。

use super::*;

async fn add_session(pool: &ConnectionPool) -> i64 {
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, started_at, ended_at)
         VALUES ('a', 'https://a', 't', '2026-09-24 00:00:00', '', 1000, 601000) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments (session_id, path, container, state, start_ms, end_ms, gap_before_ms)
         VALUES (?, '/nonexistent/a.flv', 'flv', 'finished', 0, 600000, 0)",
    )
    .bind(session)
    .execute(pool)
    .await
    .unwrap();
    session
}

#[test]
fn the_density_route_is_declared_in_the_policy() {
    let route = "/v1/sessions/{id}/danmaku-density";
    assert_eq!(
        RouteRequirement::of(&Method::GET, route, route),
        RouteRequirement::Permission(Permission::FileView)
    );
}

#[tokio::test]
async fn danmaku_density_is_read_from_the_xml() {
    let f = fixture(None).await;
    let session = add_session(&f.pool).await;
    let uri = format!("/v1/sessions/{session}/danmaku-density");
    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri, None).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body, json!({"density": null}), "没有弹幕文件");

    let xml = f._dir.path().join("a.xml");
    let comments: String = (0..30)
        .map(|i| format!("<d p=\"{}.5,1,25,0,1750000000,0,0,0\">哈哈</d>", 20 + i % 3))
        .collect();
    std::fs::write(&xml, format!("<i>{comments}</i>")).unwrap();
    sqlx::query("UPDATE segments SET danmaku_path = ? WHERE session_id = ?")
        .bind(xml.to_string_lossy().into_owned())
        .bind(session)
        .execute(&f.pool)
        .await
        .unwrap();
    let body = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri, None).await,
        StatusCode::OK,
    )
    .await;
    let density = &body["density"];
    assert_eq!(density["bucket_ms"], 10_000);
    assert_eq!(density["total"], 30);
    assert_eq!(density["counts"].as_array().unwrap().len(), 60);
    assert_eq!(density["counts"][2], 30);
    assert_eq!(density["peaks"][0]["samples"][0]["text"], "哈哈");

    let response = call(
        &f.app,
        Some(&f.viewer),
        "GET",
        "/v1/sessions/9999/danmaku-density",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
