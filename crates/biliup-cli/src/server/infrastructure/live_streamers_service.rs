use crate::server::core::live_streamers::{
    AddLiveStreamerDto, DynLiveStreamersRepository, LiveStreamerDto, LiveStreamerEntity,
    LiveStreamersService,
};
use crate::server::core::upload_streamers::DynUploadStreamersRepository;
use async_trait::async_trait;
use biliup::uploader::bilibili::Studio;

#[derive(Clone)]
pub struct ConduitLiveStreamersService {
    repository: DynLiveStreamersRepository,
    upload_streamers_repository: DynUploadStreamersRepository,
}

impl ConduitLiveStreamersService {
    pub fn new(
        repository: DynLiveStreamersRepository,
        upload_streamers_repository: DynUploadStreamersRepository,
    ) -> Self {
        Self {
            repository,
            upload_streamers_repository,
        }
    }
}

#[async_trait]
impl LiveStreamersService for ConduitLiveStreamersService {
    async fn add_streamer(&self, request: AddLiveStreamerDto) -> anyhow::Result<LiveStreamerDto> {
        Ok(self.repository.create_streamer(request).await?.into_dto())
    }

    async fn get_streamer_by_url(&self, url: &str) -> anyhow::Result<LiveStreamerDto> {
        Ok(self.repository.get_streamer_by_url(url).await?.into_dto())
    }

    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<LiveStreamerDto> {
        Ok(self.repository.get_streamer_by_id(id).await?.into_dto())
    }

    async fn get_streamers(&self) -> anyhow::Result<Vec<LiveStreamerDto>> {
        Ok(self
            .repository
            .get_streamers()
            .await?
            .into_iter()
            .map(|s| s.into_dto())
            .collect())
    }

    async fn get_studio_by_url(&self, url: &str) -> anyhow::Result<Option<Studio>> {
        let LiveStreamerEntity {
            upload_id: Some(upload_id),
            ..
        } = self.repository.get_streamer_by_url(url).await?
        else {
            return Ok(None);
        };
        Ok(Some(
            self.upload_streamers_repository
                .get_streamer_by_id(upload_id)
                .await?
                .into_dto(),
        ))
    }
}
