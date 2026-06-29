use crate::domain::entities::community::voice_channel::VoiceChannelConfig;
use crate::domain::errors::DomainError;

use super::ManageVoiceChannelsService;

impl ManageVoiceChannelsService {
    // ── Config voice-bot par guild ──

    pub(super) async fn get_voice_config_impl(
        &self,
        guild_id: &str,
    ) -> Result<VoiceChannelConfig, DomainError> {
        let entries = self
            .bot_config_repo
            .get_config(guild_id, "voice-bot")
            .await?;
        let pairs: Vec<(String, String)> =
            crate::application::coude::guild_settings::config_map(entries);
        Ok(VoiceChannelConfig::from_kv_pairs(&pairs))
    }
}
