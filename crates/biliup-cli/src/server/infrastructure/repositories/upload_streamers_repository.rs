use crate::server::core::upload_streamers::{StudioEntity, UploadStreamersRepository};
use crate::server::infrastructure::connection_pool::ConnectionPool;

use anyhow::Context;
use async_trait::async_trait;
use sqlx::query_as;

#[derive(Clone)]
pub struct SqliteUploadStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUploadStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UploadStreamersRepository for SqliteUploadStreamersRepository {
    async fn create_streamer(&self, studio: StudioEntity) -> anyhow::Result<StudioEntity> {
        query_as!(
            StudioEntity,
            r#"insert into upload_streamers(
                template_name,
                user,
                copyright,
                source,
                tid,
                cover,
                title,
                'desc',
                dynamic,
                tag,
                dtime,
                interactive,
                mission_id,
                dolby,
                lossless_music,
                no_reprint,
                open_elec,
                up_selection_reply,
                up_close_reply,
                up_close_danmu
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            returning
                id,
                template_name as "template_name!",
                user as "user!",
                copyright as "copyright!: u8",
                source as "source!",
                tid as "tid!: u16",
                cover as "cover!",
                title as "title!",
                "desc" as "desc!",
                dynamic as "dynamic!",
                tag as "tag!",
                dtime as "dtime: u32",
                interactive as "interactive!: u8",
                mission_id as "mission_id: u32",
                dolby as "dolby!: u8",
                lossless_music as "lossless_music!: u8",
                no_reprint as "no_reprint!: u8",
                open_elec as "open_elec!: u8",
                up_selection_reply as "up_selection_reply!: bool",
                up_close_reply as "up_close_reply!: bool",
                up_close_danmu as "up_close_danmu!: bool""#,
            studio.template_name,
            studio.user,
            studio.copyright,
            studio.source,
            studio.tid,
            studio.cover,
            studio.title,
            studio.desc,
            studio.dynamic,
            studio.tag,
            studio.dtime,
            studio.interactive,
            studio.mission_id,
            studio.dolby,
            studio.lossless_music,
            studio.no_reprint,
            studio.open_elec,
            studio.up_selection_reply,
            studio.up_close_reply,
            studio.up_close_danmu,
        )
        .fetch_one(&self.pool)
        .await
        .context("an unexpected error occurred while creating the streamer")
    }

    async fn delete_streamer(&self, id: i64) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_streamer(&self, studio: StudioEntity) -> anyhow::Result<StudioEntity> {
        todo!()
    }

    async fn get_streamers(&self) -> anyhow::Result<Vec<StudioEntity>> {
        query_as!(
            StudioEntity,
            r#"select id,
            template_name      as "template_name!",
            user,
            copyright          as "copyright!: u8",
            "source"           as "source!",
            tid                as "tid!: u16",
            cover              as "cover!",
            title              as "title!",
            "desc"             as "desc!",
            "dynamic"          as "dynamic!",
            tag                as "tag!",
            dtime              as "dtime: u32",
            interactive        as "interactive!: u8",
            mission_id         as "mission_id: u32",
            dolby              as "dolby!: u8",
            lossless_music     as "lossless_music!: u8",
            no_reprint         as "no_reprint!: u8",
            open_elec          as "open_elec!: u8",
            up_selection_reply as "up_selection_reply!: bool",
            up_close_reply     as "up_close_reply!: bool",
            up_close_danmu     as "up_close_danmu!: bool"
     from upload_streamers"#
        )
        .fetch_all(&self.pool)
        .await
        .context("an unexpected error occurred retrieving streamers")
    }

    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<StudioEntity> {
        query_as!(
            StudioEntity,
            r#"
       select
            id, template_name as "template_name!", user, copyright as "copyright!: u8", source as "source!", tid as "tid!: u16", cover as "cover!", title as "title!", desc as "desc!", dynamic as "dynamic!", tag as "tag!", dtime as "dtime: u32", interactive as "interactive!: u8", mission_id as "mission_id: u32", dolby as "dolby!: u8", lossless_music as "lossless_music!: u8", no_reprint as "no_reprint!: u8", open_elec as "open_elec!: u8", up_selection_reply as "up_selection_reply!: bool", up_close_reply as "up_close_reply!: bool", up_close_danmu as "up_close_danmu!: bool"
       from upload_streamers
       where
            id = $1
            "#,
            id
        )
            .fetch_one(&self.pool)
            .await
            .context("an unexpected error occurred retrieving streamers")
    }
}
