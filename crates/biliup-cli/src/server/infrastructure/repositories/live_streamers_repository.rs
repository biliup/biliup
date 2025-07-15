use crate::server::core::live_streamers::{
    AddLiveStreamerDto, LiveStreamerEntity, LiveStreamersRepository,
};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use anyhow::Context;
use async_trait::async_trait;
use sqlx::query_as;

#[derive(Clone)]
pub struct SqliteLiveStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteLiveStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LiveStreamersRepository for SqliteLiveStreamersRepository {
    async fn create_streamer(&self, dto: AddLiveStreamerDto) -> anyhow::Result<LiveStreamerEntity> {
        let split_time = dto.split_time.map(|t| t as i64);
        let split_size = dto.split_size.map(|s| s as i64);
        query_as!(
            LiveStreamerEntity,
            r#"
        insert into live_streamers (url, remark, filename, split_time, split_size, upload_id)
        values ($1 , $2 , $3, $4 , $5, $6)
        returning id, url as "url!", remark as "remark!", filename as "filename!", split_time, split_size, upload_id
            "#,
            dto.url,
            dto.remark,
            dto.filename,
            split_time,
            split_size,
            dto.upload_id
        )
        .fetch_one(&self.pool)
        .await
        .context("an unexpected error occurred while creating the streamer")
    }

    async fn delete_streamer(&self, id: i64) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_streamer(
        &self,
        entity: LiveStreamerEntity,
    ) -> anyhow::Result<LiveStreamerEntity> {
        todo!()
    }

    async fn get_streamers(&self) -> anyhow::Result<Vec<LiveStreamerEntity>> {
        query_as!(
            LiveStreamerEntity,
            r#"
       select id, url as "url!", remark as "remark!", filename as "filename!", split_time, split_size, upload_id from live_streamers
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("an unexpected error occurred retrieving streamers")
    }

    async fn get_streamer_by_url(&self, url: &str) -> anyhow::Result<LiveStreamerEntity> {
        query_as!(
            LiveStreamerEntity,
            r#"
        select
            id, url as "url!", remark as "remark!", filename as "filename!", split_time, split_size, upload_id
        from
            live_streamers
        where
            url=$1
            "#,
            url
        )
        .fetch_one(&self.pool)
        .await
        .context("an unexpected error occurred while creating the streamer")
    }

    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<LiveStreamerEntity> {
        query_as!(
            LiveStreamerEntity,
            r#"
        select
            id, url as "url!", remark as "remark!", filename as "filename!", split_time, split_size, upload_id
        from
            live_streamers
        where
            id=$1
            "#,
            id
        )
            .fetch_one(&self.pool)
            .await
            .context("an unexpected error occurred while creating the streamer")
    }
}
