use crate::server::infrastructure::connection_pool::ConnectionPool;


#[derive(Clone)]
pub struct SqliteUploadStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUploadStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
