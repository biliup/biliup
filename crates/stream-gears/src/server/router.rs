use crate::server::api::bilibili_endpoints::{
    archive_pre_endpoint, get_myinfo_endpoint, get_proxy_endpoint,
};
use crate::server::api::endpoints::{add_upload_streamer_endpoint, add_user_endpoint, delete_streamers_endpoint, delete_template_endpoint, delete_user_endpoint, get_configuration, get_qrcode, get_streamer_info, get_streamers_endpoint, get_upload_streamer_endpoint, get_upload_streamers_endpoint, get_users_endpoint, get_videos, login_by_qrcode, post_streamers_endpoint, put_streamers_endpoint};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Router;
use axum::routing::{delete, get, post};

pub fn router(service_register: ServiceRegister) -> Router<()> {
    Router::new()
        // `GET /` goes to `root`
        .route(
            "/v1/streamers",
            get(get_streamers_endpoint)
                .post(post_streamers_endpoint)
                .put(put_streamers_endpoint),
        )
        .route("/v1/streamers/{id}", delete(delete_streamers_endpoint))
        .route("/v1/configuration", get(get_configuration))
        .route("/v1/streamer-info", get(get_streamer_info))
        .route("/v1/upload/streamers", get(get_upload_streamers_endpoint))
        .route(
            "/v1/upload/streamers/{id}",
            delete(delete_template_endpoint).
            // .put(update_template_endpoint)
            get(get_upload_streamer_endpoint),
        )
        // .route("/v1/streamers/{id}", get(get_streamer_endpoint))
        // .route("/v1/streamers/{id}", delete(delete_streamer_endpoint))
        // .route("/v1/streamers/{id}", put(update_streamer_endpoint))
        // .route("/v1/streamers", post(add_streamer_endpoint))
        // .route(
        //     "/v1/upload/streamers/:id",
        //     ,
        // )
        // .route("/v1/upload/streamers/:id", )
        .route("/v1/upload/streamers", post(add_upload_streamer_endpoint))
        .route(
            "/v1/users",
            get(get_users_endpoint)
            .post(add_user_endpoint),
        )
        .route("/v1/users/{id}", delete(delete_user_endpoint))
        .route("/bili/archive/pre", get(archive_pre_endpoint))
        .route("/bili/space/myinfo", get(get_myinfo_endpoint))
        .route("/bili/proxy", get(get_proxy_endpoint))
        .route("/v1/get_qrcode", get(get_qrcode))
        .route("/v1/login_by_qrcode", post(login_by_qrcode))
        .route("/v1/videos", get(get_videos))
        // .layer(Extension(client.clone()))
        .with_state(service_register)
}
