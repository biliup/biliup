use crate::server::core::users::{User, UsersRepository};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use anyhow::Context;
use async_trait::async_trait;
use sqlx::{query, query_as};

#[derive(Clone)]
pub struct SqliteUsersStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUsersStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UsersRepository for SqliteUsersStreamersRepository {
    async fn create_user(&self, user: User) -> anyhow::Result<User> {
        query_as!(
            User,
            r#"
        insert into users (name, value, platform)
        values ($1, $2, $3)
        returning id, name as "name!", value as "value!", platform as "platform!"
            "#,
            user.name,
            user.value,
            user.platform
        )
        .fetch_one(&self.pool)
        .await
        .context("an unexpected error occured while creating the user")
    }

    async fn get_users(&self) -> anyhow::Result<Vec<User>> {
        query_as!(
            User,
            r#"
        select *
        from users
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("unexpected error while querying for users")
    }

    async fn delete_user(&self, id: i64) -> anyhow::Result<()> {
        query!(
            r#"
       delete from users
       where id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .context("an unexpected error occurred while deleting user")?;
        Ok(())
    }

    async fn update_user(&self, _user: User) -> anyhow::Result<User> {
        todo!()
    }

    async fn get_user_by_id(&self, _id: i64) -> anyhow::Result<User> {
        todo!()
    }
}
