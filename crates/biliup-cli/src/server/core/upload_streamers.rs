use async_trait::async_trait;
use biliup::uploader::bilibili::Studio;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

pub type DynUploadStreamersRepository = Arc<dyn UploadStreamersRepository + Send + Sync>;
pub type DynUploadRecordsRepository = Arc<dyn UploadRecordsRepository + Send + Sync>;

#[async_trait]
pub trait UploadStreamersRepository {
    async fn create_streamer(&self, studio: StudioEntity) -> anyhow::Result<StudioEntity>;
    async fn delete_streamer(&self, id: i64) -> anyhow::Result<()>;
    async fn update_streamer(&self, studio: StudioEntity) -> anyhow::Result<StudioEntity>;
    async fn get_streamers(&self) -> anyhow::Result<Vec<StudioEntity>>;
    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<StudioEntity>;
}

#[async_trait]
pub trait UploadRecordsRepository {
    async fn create(&self, entity: UploadRecords) -> anyhow::Result<UploadRecords>;
    async fn delete(&self, id: i64) -> anyhow::Result<()>;
    // async fn update(&self, entity: UploadRecords) -> anyhow::Result<UploadRecords>;
    async fn get_all(&self) -> anyhow::Result<Vec<UploadRecords>>;
    async fn get_by_id(&self, id: i64) -> anyhow::Result<UploadRecords>;
}

// #[serde(default)]
#[derive(FromRow, Serialize, Deserialize)]
pub struct StudioEntity {
    #[serde(default)]
    pub id: i64,
    pub template_name: String,
    pub user: i64,
    pub copyright: u8,
    pub source: String,
    pub tid: u16,
    pub cover: String,
    pub title: String,
    pub desc: String,
    pub dynamic: String,
    pub tag: String,
    pub dtime: Option<u32>,
    pub interactive: u8,
    pub mission_id: Option<u32>,
    pub dolby: u8,
    pub lossless_music: u8,
    pub no_reprint: u8,
    pub charging_pay: u8,
    pub up_selection_reply: bool,
    pub up_close_reply: bool,
    pub up_close_danmu: bool,
}

impl StudioEntity {
    pub fn into_dto(self) -> Studio {
        Studio {
            copyright: self.copyright,
            source: self.source,
            tid: self.tid,
            cover: self.cover,
            title: self.title,
            desc_format_id: 0,
            desc: self.desc,
            dynamic: self.dynamic,
            subtitle: Default::default(),
            tag: self.tag,
            videos: vec![],
            desc_v2: None,
            dtime: self.dtime,
            open_subtitle: false,
            interactive: self.interactive,
            mission_id: self.mission_id,
            dolby: self.dolby,
            lossless_music: self.lossless_music,
            no_reprint: self.no_reprint,
            charging_pay: self.charging_pay,
            aid: None,
            up_selection_reply: self.up_selection_reply,
            up_close_reply: self.up_close_reply,
            up_close_danmu: self.up_close_danmu,
            extra_fields: None,
        }
    }

    // pub fn into_profile(self, following: bool) -> ProfileDto {
    //     ProfileDto {
    //         username: self.username,
    //         bio: self.bio,
    //         image: self.image,
    //         following,
    //     }
    // }
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct UploadRecords {
    pub id: i64,
    pub identity: String,
    pub status: String,
}

impl Default for StudioEntity {
    fn default() -> Self {
        StudioEntity {
            id: 0,
            template_name: "".to_string(),
            user: 0,
            copyright: 1,
            source: "".to_string(),
            tid: 0,
            cover: "".to_string(),
            title: "".to_string(),
            desc: "".to_string(),
            dynamic: "".to_string(),
            tag: "".to_string(),
            dtime: None,
            interactive: 0,
            mission_id: None,
            dolby: 0,
            lossless_music: 0,
            no_reprint: 0,
            charging_pay: 0,
            up_selection_reply: false,
            up_close_reply: false,
            up_close_danmu: false,
        }
    }
}
