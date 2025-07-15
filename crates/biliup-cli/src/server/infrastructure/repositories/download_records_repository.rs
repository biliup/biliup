use crate::server::core::live_streamers::{DownloadRecords, DownloadRecordsRepository};

use crate::server::infrastructure::connection_pool::ConnectionPool;
use async_trait::async_trait;

pub struct SqliteDownloadRecordsRepository {
    pool: ConnectionPool,
}

impl SqliteDownloadRecordsRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DownloadRecordsRepository for SqliteDownloadRecordsRepository {
    async fn create(&self, entity: DownloadRecords) -> anyhow::Result<DownloadRecords> {
        todo!()
    }

    async fn get_all(&self) -> anyhow::Result<Vec<DownloadRecords>> {
        todo!()
    }
}
