pub mod cli;
pub mod downloader;
pub mod server;
pub mod upload_lock;
pub mod uploader;

// use crate::server::api::router::ApplicationController;
use crate::server::app::ApplicationController;
use crate::server::config::{Config, StreamerConfig};
use crate::server::core::download_manager::DownloadManager;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::models::live_streamer::InsertLiveStreamer;
use crate::server::infrastructure::models::upload_streamer::{
    InsertUploadStreamer, UploadStreamer, is_noop_uploader,
};
use crate::server::infrastructure::repositories;
use crate::server::infrastructure::service_register::ServiceRegister;
use clap::ValueEnum;
use error_stack::{Report, ResultExt};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing_subscriber::{EnvFilter, Registry, reload};

// 定义 Handle 的类型别名，简化代码
// EnvFilter: 我们使用的过滤器类型
// Registry: 基础的 Subscriber 类型
type LogHandle = reload::Handle<EnvFilter, Registry>;

pub async fn run(
    addr: (&str, u16),
    auth: bool,
    log_handle: LogHandle,
    config_path: Option<PathBuf>,
) -> AppResult<()> {
    // let config = Arc::new(AppConfig::parse());

    tracing::info!(
        "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
    );
    let conn_pool = ConnectionManager::new_pool("data/data.sqlite3")
        .await
        .expect("could not initialize the database connection pool");

    let loaded_config = if let Some(path) = config_path.as_deref() {
        let config = Config::load(path)?;
        tracing::info!(config = %path.display(), "loaded server config file");
        config
    } else {
        repositories::get_config(&conn_pool).await?
    };

    let config = Arc::new(RwLock::new(loaded_config));
    let download_manager = DownloadManager::new(
        config.read().unwrap().pool1_size,
        config.read().unwrap().pool2_size,
        conn_pool.clone(),
    );
    let service_register = ServiceRegister::new(
        conn_pool.clone(),
        config.clone(),
        download_manager,
        log_handle,
    )
    .await;

    if let Some(path) = config_path.as_deref() {
        import_config_streamers(path, &service_register).await?;
    } else {
        import_database_streamers(&service_register).await?;
    }

    tracing::info!("migrations successfully ran, initializing axum server...");
    let addr = addr
        .to_socket_addrs()
        .change_context(AppError::Unknown)?
        .next()
        .unwrap();
    ApplicationController::serve(&addr, auth, service_register)
        .await
        .attach("could not initialize application routes")?;
    Ok(())
}

async fn import_config_streamers(path: &Path, service_register: &ServiceRegister) -> AppResult<()> {
    let streamers: Vec<_> = {
        let config = service_register.config.read().unwrap();
        config
            .streamers
            .iter()
            .flat_map(|(remark, streamer)| {
                let global_uploader = config.uploader.clone();
                streamer
                    .url
                    .iter()
                    .map(|url| {
                        (
                            remark.clone(),
                            url.clone(),
                            streamer.clone(),
                            global_uploader.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };

    let mut imported = 0usize;
    for (remark, url, streamer, global_uploader) in streamers {
        let upload_config =
            import_upload_config(&remark, &streamer, global_uploader, service_register).await?;
        let live_streamer = repositories::upsert_live_streamer_by_url(
            &service_register.pool,
            to_live_streamer_insert(
                &remark,
                &url,
                &streamer,
                upload_config.as_ref().map(|cfg| cfg.id),
            )?,
        )
        .await?;
        service_register
            .managers
            .add_room(service_register.worker(live_streamer.clone(), upload_config))
            .await
            .ok_or_else(|| {
                Report::new(AppError::Custom(format!(
                    "not supported url in {}: {}",
                    path.display(),
                    live_streamer.url
                )))
            })?;
        imported += 1;
    }
    tracing::info!(config = %path.display(), imported, "imported config streamers");
    Ok(())
}

async fn import_database_streamers(service_register: &ServiceRegister) -> AppResult<()> {
    let streamers = repositories::get_all_streamer(&service_register.pool).await?;
    let mut imported = 0usize;
    for live_streamer in streamers {
        let upload_config =
            repositories::get_upload_config(&service_register.pool, live_streamer.id).await?;

        service_register
            .managers
            .add_room(service_register.worker(live_streamer.clone(), upload_config))
            .await
            .ok_or_else(|| {
                Report::new(AppError::Custom(format!(
                    "not supported url in database: {}",
                    live_streamer.url
                )))
            })?;
        imported += 1;
    }
    tracing::info!(imported, "imported database streamers");
    Ok(())
}

async fn import_upload_config(
    remark: &str,
    streamer: &StreamerConfig,
    global_uploader: Option<String>,
    service_register: &ServiceRegister,
) -> AppResult<Option<UploadStreamer>> {
    let Some(payload) = to_upload_streamer_insert(remark, streamer, global_uploader)? else {
        return Ok(None);
    };

    repositories::upsert_upload_streamer_by_template_name(&service_register.pool, payload)
        .await
        .map(Some)
}

fn to_upload_streamer_insert(
    remark: &str,
    streamer: &StreamerConfig,
    global_uploader: Option<String>,
) -> AppResult<Option<InsertUploadStreamer>> {
    let uploader = streamer.uploader.clone().or(global_uploader);
    if is_noop_uploader(uploader.as_deref()) {
        return Ok(None);
    }

    if !has_upload_config(streamer) && uploader.is_none() {
        return Ok(None);
    }

    Ok(Some(InsertUploadStreamer {
        id: None,
        template_name: format!("config:{remark}"),
        title: streamer.title.clone(),
        tid: streamer
            .tid
            .map(|value| {
                u16::try_from(value).map_err(|_| {
                    Report::new(AppError::Custom(format!(
                        "streamer {remark} tid 超出 u16 范围: {value}"
                    )))
                })
            })
            .transpose()?,
        copyright: streamer.copyright,
        copyright_source: streamer.copyright_source.clone(),
        cover_path: streamer
            .cover_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        description: streamer.description.clone(),
        dynamic: streamer.dynamic.clone(),
        dtime: streamer
            .dtime
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    Report::new(AppError::Custom(format!(
                        "streamer {remark} dtime 超出 u32 范围: {value}"
                    )))
                })
            })
            .transpose()?,
        dolby: streamer.dolby,
        hires: streamer.hires,
        charging_pay: streamer.charging_pay,
        no_reprint: streamer.no_reprint,
        uploader,
        user_cookie: streamer.user_cookie.clone(),
        tags: streamer.tags.clone().unwrap_or_default(),
        credits: streamer
            .credits
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .change_context(AppError::Unknown)?,
        up_selection_reply: streamer.up_selection_reply,
        up_close_reply: streamer.up_close_reply,
        up_close_danmu: streamer.up_close_danmu,
        extra_fields: streamer.extra_fields.clone(),
        is_only_self: streamer.is_only_self,
    }))
}

fn has_upload_config(streamer: &StreamerConfig) -> bool {
    streamer.title.is_some()
        || streamer.tid.is_some()
        || streamer.copyright.is_some()
        || streamer.copyright_source.is_some()
        || streamer.cover_path.is_some()
        || streamer.description.is_some()
        || streamer.credits.is_some()
        || streamer.dynamic.is_some()
        || streamer.dtime.is_some()
        || streamer.dolby.is_some()
        || streamer.hires.is_some()
        || streamer.charging_pay.is_some()
        || streamer.no_reprint.is_some()
        || streamer.up_selection_reply.is_some()
        || streamer.up_close_reply.is_some()
        || streamer.up_close_danmu.is_some()
        || streamer.is_only_self.is_some()
        || streamer.user_cookie.is_some()
        || streamer.tags.is_some()
        || streamer.extra_fields.is_some()
}

fn to_live_streamer_insert(
    remark: &str,
    url: &str,
    streamer: &StreamerConfig,
    upload_streamers_id: Option<i64>,
) -> AppResult<InsertLiveStreamer> {
    Ok(InsertLiveStreamer {
        url: url.to_string(),
        remark: remark.to_string(),
        filename_prefix: streamer.filename_prefix.clone(),
        time_range: streamer.time_range.clone(),
        upload_streamers_id,
        format: streamer.format.clone(),
        override_cfg: streamer
            .override_cfg
            .clone()
            .map(|cfg| serde_json::from_value(serde_json::Value::Object(cfg.into_iter().collect())))
            .transpose()
            .change_context(AppError::Unknown)?,
        preprocessor: streamer.preprocessor.clone(),
        segment_processor: streamer.segment_processor.clone(),
        downloaded_processor: streamer.downloaded_processor.clone(),
        postprocessor: streamer.postprocessor.clone(),
        opt_args: streamer
            .opt_args
            .as_ref()
            .map(|args| serde_json::Value::Array(args.iter().cloned().map(Into::into).collect())),
        excluded_keywords: streamer.excluded_keywords.as_ref().map(|keywords| {
            serde_json::Value::Array(keywords.iter().cloned().map(Into::into).collect())
        }),
    })
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum UploadLine {
    Bldsa,
    Cnbldsa,
    Andsa,
    Atdsa,
    Bda2,
    Cnbd,
    Anbd,
    Atbd,
    Tx,
    Cntx,
    Antx,
    Attx,
    Bda,
    Txa,
    Alia,
}
