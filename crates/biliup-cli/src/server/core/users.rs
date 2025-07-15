use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

pub type DynUsersRepository = Arc<dyn UsersRepository + Send + Sync>;

#[derive(FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub value: String,
    pub platform: String,
}

#[async_trait]
pub trait UsersRepository {
    async fn create_user(&self, user: User) -> anyhow::Result<User>;
    async fn get_users(&self) -> anyhow::Result<Vec<User>>;
    async fn delete_user(&self, id: i64) -> anyhow::Result<()>;
    async fn update_user(&self, user: User) -> anyhow::Result<User>;
    async fn get_user_by_id(&self, id: i64) -> anyhow::Result<User>;
}
