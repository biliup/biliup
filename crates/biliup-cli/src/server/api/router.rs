use biliup::client::StatelessClient;

use crate::server::api::endpoints::{
    add_streamer_endpoint, add_upload_streamer_endpoint, add_user_endpoint,
    delete_streamer_endpoint, delete_template_endpoint, delete_user_endpoint, get_configuration,
    get_streamer_endpoint, get_streamer_info, get_streamers_endpoint, get_upload_streamer_endpoint,
    get_upload_streamers_endpoint, get_users_endpoint, update_streamer_endpoint,
    update_template_endpoint,
};
use crate::server::core::download_actor::DownloadActorHandle;

use crate::server::infrastructure::service_register::ServiceRegister;

use anyhow::Context;
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router, http};

use crate::server::api::bilibili_endpoints::{
    archive_pre_endpoint, get_myinfo_endpoint, get_proxy_endpoint,
};
use crate::server::core::main_loop::spawn_main_loop;
use axum::http::HeaderValue;
use std::net::SocketAddr;

use crate::server::api::spa::static_handler;
use tower_http::cors::{AllowMethods, CorsLayer};
use tracing::info;

pub struct ApplicationController;

impl ApplicationController {
    pub async fn serve(addr: &SocketAddr, service_register: ServiceRegister) -> anyhow::Result<()> {
        let client = StatelessClient::default();
        // let vec = service_register.streamers_service.get_streamers().await?;
        let (main_loop, _) = spawn_main_loop();
        let actor_handle = DownloadActorHandle::new(vec![], client.clone());
        // build our application with a route
        let app = Router::new()
            // `GET /` goes to `root`
            .route("/v1/streamers", get(get_streamers_endpoint))
            .route("/v1/configuration", get(get_configuration))
            .route("/v1/streamer-info", get(get_streamer_info))
            .route("/v1/upload/streamers", get(get_upload_streamers_endpoint))
            .route(
                "/v1/upload/streamers/{id}",
                delete(delete_template_endpoint)
                    .put(update_template_endpoint)
                    .get(get_upload_streamer_endpoint),
            )
            .route("/v1/streamers/{id}", get(get_streamer_endpoint))
            .route("/v1/streamers/{id}", delete(delete_streamer_endpoint))
            .route("/v1/streamers/{id}", put(update_streamer_endpoint))
            .route("/v1/streamers", post(add_streamer_endpoint))
            // .route(
            //     "/v1/upload/streamers/:id",
            //     ,
            // )
            // .route("/v1/upload/streamers/:id", )
            .route("/v1/upload/streamers", post(add_upload_streamer_endpoint))
            .route("/v1/users", get(get_users_endpoint).post(add_user_endpoint))
            .route("/v1/users/{id}", delete(delete_user_endpoint))
            .route("/bili/archive/pre", get(archive_pre_endpoint))
            .route("/bili/space/myinfo", get(get_myinfo_endpoint))
            .route("/bili/proxy", get(get_proxy_endpoint))
            .fallback(static_handler)
            // .route("/", get(root))
            .layer(
                // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
                // for more details
                //
                // pay attention that for some request types like posting content-type: application/json
                // it is required to add ".allow_headers([http::header::CONTENT_TYPE])"
                // or see this issue https://github.com/tokio-rs/axum/issues/849
                CorsLayer::new()
                    .allow_headers([http::header::CONTENT_TYPE])
                    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                    .allow_methods(AllowMethods::any()),
            )
            .layer(Extension(actor_handle))
            // .layer(Extension(client.clone()))
            .layer(Extension(client.clone()))
            .with_state(service_register);
        // `POST /users` goes to `create_user`
        // .route("/users", post(create_user));
        // let mut test = Cycle::new(indexmap![
        //     "1".to_string()  => StreamStatus::Idle,
        //     "2".to_string() => StreamStatus::Idle,
        //     "3".to_string() => StreamStatus::Idle,
        //     "4".to_string() => StreamStatus::Idle,
        // ]);
        // let testget = test.clone();

        // tokio::spawn(async move {
        //     tokio::time::sleep(Duration::from_secs(31)).await;
        //     actor_handle.remove_streamer("https://www.huya.com/superrabbit");
        //     println!("yesssss")
        // });
        // let mut n = 0;
        // loop {
        //     let string = testget.get(&mut n);
        //     println!("yoooo {string:?}");
        //     tokio::time::sleep(Duration::from_secs(2)).await;
        // }
        // extensions.get().and_then(|actor| {
        //    actor
        // });
        // println!("nonono");
        // Ok::<_, anyhow::Error>(())

        // tokio::spawn(async move {
        //     tokio::time::sleep(Duration::from_secs(10)).await;
        //     test.replace(indexmap!["10".to_string()  => StreamStatus::Idle]);
        // });
        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        info!("routes initialized, listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(listener, app)
            .await
            .context("error while starting API server")?;

        Ok(())
    }
}
