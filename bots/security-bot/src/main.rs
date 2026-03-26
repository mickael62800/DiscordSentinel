mod account_checker;
mod api_client;
mod config;
mod handler;
mod raid_detector;

use serenity::prelude::*;
use tracing::info;

use crate::account_checker::AccountChecker;
use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{AccountCheckerKey, ApiClientKey, Handler, RaidDetectorKey};
use crate::raid_detector::RaidDetector;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(
        api_url = %config.api_base_url,
        raid_threshold = config.raid_join_threshold,
        raid_window = config.raid_join_window_secs,
        min_account_age = config.min_account_age_secs,
        "Démarrage du security bot"
    );

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur création du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(ApiClient::new(&config));
        data.insert::<RaidDetectorKey>(RaidDetector::new(
            config.raid_join_threshold,
            config.raid_join_window_secs,
        ));
        data.insert::<AccountCheckerKey>(AccountChecker::new(config.min_account_age_secs));
    }

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
