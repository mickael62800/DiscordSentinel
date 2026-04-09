mod api_client;
mod handler;
mod template;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::{BotConfig, SimpleConfig};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "welcome_bot=info".into()),
        )
        .init();

    let config = SimpleConfig::from_env("WELCOME_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du welcome bot");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "welcome-bot"));

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(base_api.clone());
    }

    spawn_heartbeat(base_api.clone());

    info!("Demarrage welcome-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
