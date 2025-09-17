use crate::server::core::live_streamers::{
    DynDownloadRecordsRepository, DynLiveStreamersRepository, DynLiveStreamersService,
    DynVideosRepository,
};
use crate::server::core::upload_streamers::{
    DynUploadRecordsRepository, DynUploadStreamersRepository,
};
use crate::server::core::users::DynUsersRepository;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::repositories::download_records_repository::SqliteDownloadRecordsRepository;
use crate::server::infrastructure::repositories::live_streamers_repository::SqliteLiveStreamersRepository;
use crate::server::infrastructure::repositories::upload_records_repository::SqliteUploadRecordsRepository;
use crate::server::infrastructure::repositories::upload_streamers_repository::SqliteUploadStreamersRepository;
use crate::server::infrastructure::repositories::users_repository::SqliteUsersStreamersRepository;
use crate::server::infrastructure::repositories::videos_repository::SqliteVideosRepository;
use axum::extract::FromRef;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct ServiceRegister {
    pub pool: ConnectionPool,
    pub upload_records_repository: DynUploadRecordsRepository,
    pub videos_repository: DynVideosRepository,
    pub download_records_repository: DynDownloadRecordsRepository,
}

/// A simple service container responsible for managing the various services our API endpoints will pull from through axum extensions.
impl ServiceRegister {
    pub fn new(pool: ConnectionPool) -> Self {
        info!("initializing utility services...");

        info!("utility services initialized, building feature services...");

        let upload_records_repository = Arc::new(SqliteUploadRecordsRepository::new(pool.clone()))
            as DynUploadRecordsRepository;

        let videos_repository =
            Arc::new(SqliteVideosRepository::new(pool.clone())) as DynVideosRepository;

        let download_records_repository =
            Arc::new(SqliteDownloadRecordsRepository::new(pool.clone()))
                as DynDownloadRecordsRepository;

        info!("feature services successfully initialized!");

        ServiceRegister {
            pool,
            upload_records_repository,
            videos_repository,
            download_records_repository,
        }
    }
}

impl FromRef<ServiceRegister> for ConnectionPool {
    fn from_ref(app_state: &ServiceRegister) -> ConnectionPool {
        app_state.pool.clone()
    }
}
