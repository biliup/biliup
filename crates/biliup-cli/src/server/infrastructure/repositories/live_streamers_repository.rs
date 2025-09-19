use crate::server::infrastructure::connection_pool::ConnectionPool;

#[derive(Clone)]
pub struct SqliteLiveStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteLiveStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
