use std::sync::Arc;

use crate::domain::entities::{VoiceChannel, VoiceChannelDetail};
use crate::domain::ports::VoiceChannelRepository;

pub struct VoiceChannelsService {
    repo: Arc<dyn VoiceChannelRepository>,
}

impl VoiceChannelsService {
    pub fn new(repo: Arc<dyn VoiceChannelRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_channels(&self, guild_id: String) -> Result<Vec<VoiceChannel>, String> {
        self.repo.get_channels(guild_id).await
    }

    pub async fn get_channel_detail(&self, channel_id: String) -> Result<VoiceChannelDetail, String> {
        self.repo.get_channel_detail(channel_id).await
    }
}
