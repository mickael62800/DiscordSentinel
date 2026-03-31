use serenity::model::id::ChannelId;
use sentinel_shared::config::{BaseConfig, BotConfig};

#[derive(Clone)]
pub struct Config {
    base: BaseConfig,
    pub guild_id: u64,
    pub public_creator_channel_id: ChannelId,
    pub private_creator_channel_id: ChannelId,
    pub log_channel_id: Option<ChannelId>,
}

impl Config {
    pub fn from_env() -> Self {
        let base = BaseConfig::from_env("VOICE_DISCORD_TOKEN");

        let guild_id: u64 = std::env::var("VOICE_GUILD_ID")
            .expect("VOICE_GUILD_ID manquant dans .env")
            .parse()
            .expect("VOICE_GUILD_ID doit etre un nombre");

        let public_creator: u64 = std::env::var("VOICE_PUBLIC_CREATOR_CHANNEL_ID")
            .expect("VOICE_PUBLIC_CREATOR_CHANNEL_ID manquant dans .env")
            .parse()
            .expect("VOICE_PUBLIC_CREATOR_CHANNEL_ID doit etre un nombre");

        let private_creator: u64 = std::env::var("VOICE_PRIVATE_CREATOR_CHANNEL_ID")
            .expect("VOICE_PRIVATE_CREATOR_CHANNEL_ID manquant dans .env")
            .parse()
            .expect("VOICE_PRIVATE_CREATOR_CHANNEL_ID doit etre un nombre");

        let log_channel_id: Option<u64> = std::env::var("VOICE_LOG_CHANNEL_ID")
            .ok()
            .and_then(|v| v.parse().ok());

        Self {
            base,
            guild_id,
            public_creator_channel_id: ChannelId::new(public_creator),
            private_creator_channel_id: ChannelId::new(private_creator),
            log_channel_id: log_channel_id.map(ChannelId::new),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
