use super::*;
use crate::server::api::access::require_permission;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::permissions::{Permission, Role, required_permission};
use crate::server::infrastructure::users::{Backend, Credentials};
use crate::server::workbench::renders::{EffectType, MaskRegion, RenderSpec};
use axum::Router;
use axum::extract::FromRef;
use axum::http::{Method, Request as HttpRequest};
use axum::middleware::from_fn;
use axum::routing::post;
use axum_login::AuthManagerLayerBuilder;
use serde_json::{Value, json};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone, FromRef)]
struct TestState {
    pool: ConnectionPool,
    renders: Arc<RenderJobs>,
    publisher: Arc<ClipPublisher>,
}

struct Fixture {
    dir: tempfile::TempDir,
    pool: ConnectionPool,
    jobs: Arc<RenderJobs>,
    app: Router,
    ended: i64,
    live: i64,
    op: String,
    viewer: String,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
        .await
        .unwrap();
    let mut ids = vec![];
    for ended in [Some(1000i64), None] {
        ids.push(sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at)
            VALUES('主播','https://www.douyu.com/1','录后测试','2026-10-09 00:00:00','',1,?) RETURNING id")
            .bind(ended).fetch_one(&pool).await.unwrap());
    }
    let jobs = Arc::new(RenderJobs::new(pool.clone(), dir.path().join("renders")));
    jobs.initialize().await.unwrap();
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let backend = Backend::new(pool.clone());
    backend
        .bootstrap_admin(Credentials {
            username: "admin".into(),
            password: "admin-password".into(),
            next: None,
        })
        .await
        .unwrap();
    backend
        .create_user("op", "operator-password".into(), Role::Operator)
        .await
        .unwrap();
    backend
        .create_user("viewer", "viewer-password".into(), Role::Viewer)
        .await
        .unwrap();
    let layer = AuthManagerLayerBuilder::new(
        backend,
        SessionManagerLayer::new(session_store).with_secure(false),
    )
    .build();
    let state = TestState {
        pool: pool.clone(),
        renders: jobs.clone(),
        publisher: Arc::new(ClipPublisher::new(
            pool.clone(),
            Arc::new(crate::server::workbench::clips::export::ClipExports::new(
                pool.clone(),
                dir.path().join("clips"),
            )),
            Arc::new(crate::server::workbench::clips::publish::queue::Offline),
        )),
    };
    let app = Router::new()
        .route(
            "/v1/sessions/{id}/render-recipe",
            get(get_render_recipe).put(put_render_recipe),
        )
        .route("/v1/sessions/{id}/render-assets", render_assets_route())
        .route(
            "/v1/render-assets/{aid}",
            get(get_render_asset).delete(delete_render_asset),
        )
        .route(
            "/v1/sessions/{id}/renders",
            get(list_renders).post(create_render),
        )
        .route("/v1/renders/{jid}", get(get_render).delete(cancel_render))
        .route("/v1/renders/{jid}/cancel", post(cancel_render))
        .route("/v1/renders/{jid}/retry", post(retry_render))
        .route("/v1/renders/{jid}/download", get(download_render))
        .route("/v1/sessions/{id}/render-preview", post(preview_render))
        .with_state(state)
        .route_layer(from_fn(require_permission))
        .merge(crate::server::api::auth::router())
        .layer(layer);
    let op = login(&app, "op", "operator-password").await;
    let viewer = login(&app, "viewer", "viewer-password").await;
    Fixture {
        dir,
        pool,
        jobs,
        app,
        ended: ids[0],
        live: ids[1],
        op,
        viewer,
    }
}

async fn call(
    app: &Router,
    cookie: Option<&str>,
    method: &str,
    uri: &str,
    mime: Option<&str>,
    body: impl Into<Body>,
) -> Response {
    let mut request = HttpRequest::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(mime) = mime {
        request = request.header(header::CONTENT_TYPE, mime);
    }
    app.clone()
        .oneshot(request.body(body.into()).unwrap())
        .await
        .unwrap()
}

async fn json_call(
    f: &Fixture,
    cookie: Option<&str>,
    method: &str,
    uri: &str,
    body: Value,
) -> Response {
    call(
        &f.app,
        cookie,
        method,
        uri,
        Some("application/json"),
        body.to_string(),
    )
    .await
}

async fn login(app: &Router, username: &str, password: &str) -> String {
    let response = call(
        app,
        None,
        "POST",
        "/v1/users/login",
        Some("application/json"),
        json!({ "username": username, "password": password }).to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn body(response: Response) -> Bytes {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
}

async fn json_of(response: Response, status: StatusCode) -> Value {
    assert_eq!(response.status(), status);
    serde_json::from_slice(&body(response).await).unwrap()
}

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(8, 6, image::Rgba([255, 0, 0, 128]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

async fn upload(f: &Fixture, sid: i64) -> Value {
    json_of(
        call(
            &f.app,
            Some(&f.op),
            "POST",
            &format!("/v1/sessions/{sid}/render-assets"),
            Some("image/png"),
            png(),
        )
        .await,
        StatusCode::CREATED,
    )
    .await
}

#[tokio::test]
async fn authorization_recipe_validation_and_session_bound_assets() {
    let f = fixture().await;
    let uri = format!("/v1/sessions/{}/render-recipe", f.ended);
    assert_eq!(
        call(&f.app, None, "GET", &uri, None, Body::empty())
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let recipe = json_of(
        call(&f.app, Some(&f.viewer), "GET", &uri, None, Body::empty()).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        recipe,
        serde_json::to_value(RenderRecipe::default()).unwrap()
    );
    assert_eq!(
        json_call(&f, Some(&f.viewer), "PUT", &uri, recipe.clone())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut invalid = recipe.clone();
    invalid["danmaku"]["offset_ms"] = json!(3_600_001);
    assert_eq!(
        json_call(&f, Some(&f.op), "PUT", &uri, invalid)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let asset = upload(&f, f.live).await;
    let mut cross = RenderRecipe::default();
    cross.regions.push(MaskRegion {
        id: "cover".into(),
        effect_type: EffectType::Image,
        asset_id: Some(asset["id"].as_i64().unwrap()),
        ..Default::default()
    });
    assert_eq!(
        json_call(
            &f,
            Some(&f.op),
            "PUT",
            &uri,
            serde_json::to_value(cross).unwrap()
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let mut valid = RenderRecipe::default();
    valid.danmaku.font_size = 45.;
    let saved = json_of(
        json_call(
            &f,
            Some(&f.op),
            "PUT",
            &uri,
            serde_json::to_value(&valid).unwrap(),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved["danmaku"]["font_size"], 45.);
    assert_eq!(store::recipe(&f.pool, f.ended).await.unwrap(), valid);
    assert_eq!(
        call(
            &f.app,
            Some(&f.op),
            "GET",
            "/v1/sessions/999999/render-recipe",
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn images_are_decoded_bounded_immutable_and_reference_protected() {
    let f = fixture().await;
    let uri = format!("/v1/sessions/{}/render-assets", f.ended);
    for (mime, bytes, status) in [
        ("image/png", b"bad image".to_vec(), StatusCode::BAD_REQUEST),
        ("image/jpeg", png(), StatusCode::BAD_REQUEST),
        ("text/plain", png(), StatusCode::UNSUPPORTED_MEDIA_TYPE),
        (
            "image/png",
            vec![0; assets::MAX_ASSET_BYTES + 1],
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        assert_eq!(
            call(&f.app, Some(&f.op), "POST", &uri, Some(mime), bytes)
                .await
                .status(),
            status
        );
    }
    let uploaded = upload(&f, f.ended).await;
    assert_eq!(uploaded["width"], 8);
    assert_eq!(uploaded["height"], 6);
    assert!(uploaded.get("path").is_none());
    let aid = uploaded["id"].as_i64().unwrap();
    let asset_url = format!("/v1/render-assets/{aid}");
    let response = call(
        &f.app,
        Some(&f.viewer),
        "GET",
        &asset_url,
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(body(response).await, png());
    let head = call(
        &f.app,
        Some(&f.viewer),
        "HEAD",
        &asset_url,
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(body(head).await.is_empty());
    assert_eq!(
        call(
            &f.app,
            Some(&f.viewer),
            "DELETE",
            &asset_url,
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let mut recipe = RenderRecipe::default();
    recipe.regions.push(MaskRegion {
        id: "picture".into(),
        effect_type: EffectType::Image,
        asset_id: Some(aid),
        ..Default::default()
    });
    let settings = format!("/v1/sessions/{}/render-recipe", f.ended);
    assert_eq!(
        json_call(
            &f,
            Some(&f.op),
            "PUT",
            &settings,
            serde_json::to_value(recipe).unwrap()
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            &f.app,
            Some(&f.op),
            "DELETE",
            &asset_url,
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        json_call(&f, Some(&f.op), "PUT", &settings, json!({}))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            &f.app,
            Some(&f.op),
            "DELETE",
            &asset_url,
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &asset_url,
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    // Reusing an ID would make the immutable browser cache show an old image.
    assert!(upload(&f, f.ended).await["id"].as_i64().unwrap() > aid);
}

#[tokio::test]
async fn ready_products_support_range_head_and_attachment() {
    let f = fixture().await;
    let root = f.jobs.root();
    tokio::fs::create_dir_all(root).await.unwrap();
    let path = root.join("product.mp4");
    tokio::fs::write(&path, b"0123456789").await.unwrap();
    let spec = RenderSpec {
        session_id: f.ended,
        clip_id: None,
        in_ms: 0,
        out_ms: 1000,
        recipe: RenderRecipe::default(),
        sources: vec![],
        assets: vec![],
        font_path: None,
    };
    let job = store::insert(&f.pool, &spec, recorder::now_ms())
        .await
        .unwrap();
    let jid = job.view.id;
    let uri = format!("/v1/renders/{jid}/download");
    assert_eq!(
        call(&f.app, Some(&f.viewer), "GET", &uri, None, Body::empty())
            .await
            .status(),
        StatusCode::CONFLICT
    );
    store::running(&f.pool, jid, recorder::now_ms())
        .await
        .unwrap();
    store::finish(
        &f.pool,
        jid,
        path.to_str().unwrap(),
        10,
        1000,
        recorder::now_ms(),
    )
    .await
    .unwrap();
    let request = HttpRequest::builder()
        .uri(&uri)
        .header(header::COOKIE, &f.viewer)
        .header(header::RANGE, "bytes=2-5")
        .body(Body::empty())
        .unwrap();
    let partial = f.app.clone().oneshot(request).await.unwrap();
    assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(partial.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
    assert_eq!(body(partial).await, "2345");
    let head = call(&f.app, Some(&f.viewer), "HEAD", &uri, None, Body::empty()).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()[header::CONTENT_LENGTH], "10");
    assert!(body(head).await.is_empty());
    let attachment = call(
        &f.app,
        Some(&f.viewer),
        "GET",
        &format!("{uri}?attachment=true"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(attachment.status(), StatusCode::OK);
    assert!(
        attachment.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment;")
    );
    let view = json_of(
        call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &format!("/v1/renders/{jid}"),
            None,
            Body::empty(),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(view["state"], "ready");
    assert!(view.get("spec").is_none());
    assert!(view.get("output_path").is_none());
    assert_eq!(
        call(
            &f.app,
            Some(&f.op),
            "POST",
            &format!("/v1/renders/{jid}/cancel"),
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn live_whole_invalid_range_and_missing_job_fail_cleanly() {
    let f = fixture().await;
    let live = format!("/v1/sessions/{}/renders", f.live);
    assert_eq!(
        json_call(&f, Some(&f.op), "POST", &live, json!({}))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        json_call(&f, Some(&f.viewer), "POST", &live, json!({}))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let preview = format!("/v1/sessions/{}/render-preview", f.ended);
    assert_eq!(
        json_call(
            &f,
            Some(&f.viewer),
            "POST",
            &preview,
            json!({ "recipe": {}, "from_ms": -1, "to_ms": 1 })
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    for route in ["/v1/renders/99999", "/v1/renders/99999/download"] {
        assert_eq!(
            call(&f.app, Some(&f.viewer), "GET", route, None, Body::empty())
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    for action in ["cancel", "retry"] {
        assert_eq!(
            call(
                &f.app,
                Some(&f.op),
                "POST",
                &format!("/v1/renders/99999/{action}"),
                None,
                Body::empty()
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
}

#[test]
fn read_and_edit_routes_have_explicit_permissions() {
    for route in [
        "/v1/sessions/{id}/render-recipe",
        "/v1/sessions/{id}/render-settings",
        "/v1/sessions/{id}/render-assets",
        "/v1/sessions/{id}/renders",
        "/v1/renders/{jid}",
        "/v1/renders/{jid}/download",
        "/v1/render-assets/{aid}",
        "/v1/render-fonts/{id}",
    ] {
        for method in [Method::GET, Method::HEAD] {
            assert_eq!(
                required_permission(&method, route, route),
                Some(Permission::FileView)
            );
        }
    }
    for (method, route) in [
        (Method::PUT, "/v1/sessions/{id}/render-recipe"),
        (Method::POST, "/v1/sessions/{id}/renders"),
        (Method::POST, "/v1/sessions/{id}/render-assets"),
        (Method::DELETE, "/v1/render-assets/{aid}"),
        (Method::POST, "/v1/renders/{jid}/cancel"),
        (Method::POST, "/v1/renders/{jid}/retry"),
    ] {
        assert_eq!(
            required_permission(&method, route, route),
            Some(Permission::ClipEdit)
        );
    }
    assert_eq!(
        required_permission(&Method::POST, "/v1/sessions/{id}/render-preview", ""),
        Some(Permission::FileView)
    );
}

#[tokio::test]
async fn queued_whole_jobs_fix_recipe_snapshot_cancel_and_retry_original() {
    let f = fixture().await;
    // Keep work queued while exercising the HTTP lifecycle. The recorder would
    // supply real video bytes; FFmpeg never reads this cancelled fixture.
    let permit = crate::server::workbench::transcode::acquire(&CancellationToken::new())
        .await
        .unwrap();
    let source = f.dir.path().join("long.flv");
    tokio::fs::write(&source, b"fixture bytes never decoded")
        .await
        .unwrap();
    let duration = 26 * 3_600_000i64;
    sqlx::query(
        "INSERT INTO segments(session_id,path,container,state,start_ms,end_ms)
        VALUES(?,?,'flv','finished',0,?)",
    )
    .bind(f.ended)
    .bind(source.to_str().unwrap())
    .bind(duration)
    .execute(&f.pool)
    .await
    .unwrap();
    let uri = format!("/v1/sessions/{}/renders", f.ended);
    let mut recipe = RenderRecipe::default();
    recipe.danmaku.font_size = 50.;
    let queued = json_of(
        json_call(&f, Some(&f.op), "POST", &uri, json!({ "recipe": recipe })).await,
        StatusCode::ACCEPTED,
    )
    .await;
    let jid = queued["id"].as_i64().unwrap();
    assert_eq!(queued["state"], "queued");
    let before = store::get(&f.pool, jid).await.unwrap().unwrap();
    assert_eq!(before.spec.out_ms, duration);
    assert_eq!(before.spec.recipe.danmaku.font_size, 50.);
    let settings = format!("/v1/sessions/{}/render-recipe", f.ended);
    assert_eq!(
        json_call(
            &f,
            Some(&f.op),
            "PUT",
            &settings,
            json!({ "danmaku": { "font_size": 65 } })
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        store::get(&f.pool, jid)
            .await
            .unwrap()
            .unwrap()
            .spec
            .recipe
            .danmaku
            .font_size,
        50.
    );
    let pins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segment_pins WHERE owner=?")
        .bind(store::pin_owner(jid))
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, 1);
    let cancel = format!("/v1/renders/{jid}/cancel");
    assert_eq!(
        call(
            &f.app,
            Some(&f.viewer),
            "POST",
            &cancel,
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&f.app, Some(&f.op), "POST", &cancel, None, Body::empty())
            .await
            .status(),
        StatusCode::ACCEPTED
    );
    assert_eq!(
        store::get(&f.pool, jid).await.unwrap().unwrap().view.state,
        "cancelled"
    );
    let pins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segment_pins WHERE owner=?")
        .bind(store::pin_owner(jid))
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(pins, 0);
    let retried = json_of(
        call(
            &f.app,
            Some(&f.op),
            "POST",
            &format!("/v1/renders/{jid}/retry"),
            None,
            Body::empty(),
        )
        .await,
        StatusCode::ACCEPTED,
    )
    .await;
    let retry_id = retried["id"].as_i64().unwrap();
    assert_ne!(jid, retry_id);
    assert_eq!(
        store::get(&f.pool, retry_id)
            .await
            .unwrap()
            .unwrap()
            .spec
            .recipe
            .danmaku
            .font_size,
        50.
    );
    assert_eq!(
        call(
            &f.app,
            Some(&f.op),
            "POST",
            &format!("/v1/renders/{retry_id}/cancel"),
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::ACCEPTED
    );
    drop(permit);
    f.jobs.shutdown().await;
    assert_eq!(
        tokio::fs::read(source).await.unwrap(),
        b"fixture bytes never decoded"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn asset_symlink_cannot_escape_managed_directory() {
    let f = fixture().await;
    let uploaded = upload(&f, f.ended).await;
    let aid = uploaded["id"].as_i64().unwrap();
    let asset = assets::get(&f.pool, aid).await.unwrap().unwrap();
    let outside = f.dir.path().join("private.txt");
    tokio::fs::write(&outside, b"private data").await.unwrap();
    tokio::fs::remove_file(&asset.render.path).await.unwrap();
    std::os::unix::fs::symlink(&outside, &asset.render.path).unwrap();
    assert_eq!(
        call(
            &f.app,
            Some(&f.viewer),
            "GET",
            &format!("/v1/render-assets/{aid}"),
            None,
            Body::empty()
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
}
