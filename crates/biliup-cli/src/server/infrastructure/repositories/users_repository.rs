use crate::server::infrastructure::connection_pool::ConnectionPool;

#[derive(Clone)]
pub struct SqliteUsersStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUsersStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
