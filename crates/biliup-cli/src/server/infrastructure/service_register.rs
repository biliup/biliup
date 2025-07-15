use crate::server::core::live_streamers::{
    DynDownloadRecordsRepository, DynLiveStreamersRepository, DynLiveStreamersService,
    DynVideosRepository,
};
use crate::server::core::upload_streamers::{
    DynUploadRecordsRepository, DynUploadStreamersRepository,
};
use crate::server::core::users::DynUsersRepository;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::live_streamers_service::ConduitLiveStreamersService;
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
    pub streamers_service: DynLiveStreamersService,
    pub live_streamers_repository: DynLiveStreamersRepository,
    pub upload_streamers_repository: DynUploadStreamersRepository,
    pub users_repository: DynUsersRepository,
    pub upload_records_repository: DynUploadRecordsRepository,
    pub videos_repository: DynVideosRepository,
    pub download_records_repository: DynDownloadRecordsRepository,
}

/// A simple service container responsible for managing the various services our API endpoints will pull from through axum extensions.
impl ServiceRegister {
    pub fn new(pool: ConnectionPool) -> Self {
        info!("initializing utility services...");

        info!("utility services initialized, building feature services...");
        let streamers_repository = Arc::new(SqliteLiveStreamersRepository::new(pool.clone()))
            as DynLiveStreamersRepository;
        let upload_streamers_repository =
            Arc::new(SqliteUploadStreamersRepository::new(pool.clone()))
                as DynUploadStreamersRepository;

        let users_repository =
            Arc::new(SqliteUsersStreamersRepository::new(pool.clone())) as DynUsersRepository;

        let upload_records_repository = Arc::new(SqliteUploadRecordsRepository::new(pool.clone()))
            as DynUploadRecordsRepository;

        let videos_repository =
            Arc::new(SqliteVideosRepository::new(pool.clone())) as DynVideosRepository;

        let download_records_repository =
            Arc::new(SqliteDownloadRecordsRepository::new(pool)) as DynDownloadRecordsRepository;

        let streamers_service = Arc::new(ConduitLiveStreamersService::new(
            streamers_repository.clone(),
            upload_streamers_repository.clone(),
        )) as DynLiveStreamersService;
        info!("feature services successfully initialized!");

        ServiceRegister {
            streamers_service,
            live_streamers_repository: streamers_repository,
            upload_streamers_repository,
            users_repository,
            upload_records_repository,
            videos_repository,
            download_records_repository,
        }
    }
}

impl FromRef<ServiceRegister> for DynLiveStreamersRepository {
    fn from_ref(app_state: &ServiceRegister) -> DynLiveStreamersRepository {
        app_state.live_streamers_repository.clone()
    }
}

impl FromRef<ServiceRegister> for DynUsersRepository {
    fn from_ref(app_state: &ServiceRegister) -> DynUsersRepository {
        app_state.users_repository.clone()
    }
}
