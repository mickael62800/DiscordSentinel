mod api_client;
mod config;
mod handler;

use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{ApiClientKey, Handler};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "audit_bot=info".into()),
        )
        .init();

    let config = Config::from_env();
    let api_client = ApiClient::new(&config);

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_BANS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api_client);
    }

    // Heartbeat toutes les 30 secondes
    let data = client.data.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let data = data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.heartbeat("audit-bot").await;
            }
        }
    });

    info!("Demarrage audit-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
