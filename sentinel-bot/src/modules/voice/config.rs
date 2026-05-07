//! Config voice-bot — simplifiee pour le bot unifie.
//!
//! Les variables d'env VOICE_* sont conservees pour compatibilite. Si elles
//! sont absentes, le module voice est effectivement desactive (guild_id == 0).

use serenity::model::id::ChannelId;
use crate::shared::config::load_env_optional;

#[derive(Clone)]
pub struct Config {
    pub guild_id: u64,
    pub public_creator_channel_id: ChannelId,
    pub private_creator_channel_id: ChannelId,
    pub log_channel_id: Option<ChannelId>,
}

impl Config {
    pub fn from_env() -> Self {
        let guild_id: u64 = load_env_optional("VOICE_GUILD_ID").unwrap_or(0);
        let public_creator: u64 = load_env_optional("VOICE_PUBLIC_CREATOR_CHANNEL_ID").unwrap_or(0);
        let private_creator: u64 = load_env_optional("VOICE_PRIVATE_CREATOR_CHANNEL_ID").unwrap_or(0);
        let log_channel_id: Option<u64> = load_env_optional("VOICE_LOG_CHANNEL_ID");

        Self {
            guild_id,
            public_creator_channel_id: ChannelId::new(public_creator.max(1)),
            private_creator_channel_id: ChannelId::new(private_creator.max(1)),
            log_channel_id: log_channel_id.map(ChannelId::new),
        }
    }
}
