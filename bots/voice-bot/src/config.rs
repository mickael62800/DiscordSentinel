use serenity::model::id::ChannelId;

pub struct Config {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
    pub guild_id: u64,
    pub public_creator_channel_id: ChannelId,
    pub private_creator_channel_id: ChannelId,
    pub log_channel_id: Option<ChannelId>,
}

impl Config {
    pub fn from_env() -> Self {
        let discord_token =
            std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN manquant dans .env");
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let api_key = std::env::var("API_KEY").unwrap_or_default();

        let guild_id: u64 = std::env::var("GUILD_ID")
            .expect("GUILD_ID manquant dans .env")
            .parse()
            .expect("GUILD_ID doit etre un nombre");

        let public_creator: u64 = std::env::var("PUBLIC_CREATOR_CHANNEL_ID")
            .expect("PUBLIC_CREATOR_CHANNEL_ID manquant dans .env")
            .parse()
            .expect("PUBLIC_CREATOR_CHANNEL_ID doit etre un nombre");

        let private_creator: u64 = std::env::var("PRIVATE_CREATOR_CHANNEL_ID")
            .expect("PRIVATE_CREATOR_CHANNEL_ID manquant dans .env")
            .parse()
            .expect("PRIVATE_CREATOR_CHANNEL_ID doit etre un nombre");

        let log_channel_id: Option<u64> = std::env::var("LOG_CHANNEL_ID")
            .ok()
            .and_then(|v| v.parse().ok());

        Self {
            discord_token,
            api_base_url,
            api_key,
            guild_id,
            public_creator_channel_id: ChannelId::new(public_creator),
            private_creator_channel_id: ChannelId::new(private_creator),
            log_channel_id: log_channel_id.map(ChannelId::new),
        }
    }
}
