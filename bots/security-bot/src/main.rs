mod account_checker;
mod api_client;
mod captcha;
mod config;
mod handler;
mod quarantine;
mod raid_detector;
mod slowmode;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use crate::account_checker::AccountChecker;
use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{
    AccountCheckerKey, ApiClientKey, ConfigKey, Handler, QuarantineKey, RaidDetectorKey,
    SlowmodeKey,
};
use crate::quarantine::QuarantineManager;
use crate::raid_detector::RaidDetector;
use crate::slowmode::SlowmodeManager;

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
        quarantine = config.quarantine_enabled,
        captcha = config.captcha_enabled,
        slowmode = config.slowmode_seconds,
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
        data.insert::<QuarantineKey>(QuarantineManager::new());
        data.insert::<SlowmodeKey>(SlowmodeManager::new());
        data.insert::<ConfigKey>(config.clone());
    }

    // Heartbeat task
    let api_for_heartbeat = ApiClient::new(&config);
    tokio::spawn(async move {
        loop {
            if let Err(e) = api_for_heartbeat.heartbeat("security-bot").await {
                tracing::warn!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    // Background task: kick des utilisateurs en quarantaine qui n'ont pas passé le captcha
    let data_for_timeout = Arc::clone(&client.data);
    let http_for_timeout = Arc::clone(&client.http);
    let captcha_timeout = config.captcha_timeout_secs;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            let data = data_for_timeout.read().await;
            let quarantine = match data.get::<QuarantineKey>() {
                Some(q) => q,
                None => continue,
            };

            let expired = quarantine.expired_users(captcha_timeout);
            for (guild_id, user_id) in expired {
                // Kick l'utilisateur qui n'a pas vérifié
                if let Err(e) = guild_id.kick(&*http_for_timeout, user_id).await {
                    tracing::warn!(
                        error = %e,
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Impossible de kick l'utilisateur en quarantaine expirée"
                    );
                } else {
                    info!(
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Utilisateur kick (captcha timeout)"
                    );
                }
                quarantine.remove_tracking(guild_id, user_id);
            }
        }
    });

    // Background task: revert slowmode après expiration
    let data_for_slowmode = Arc::clone(&client.data);
    let http_for_slowmode = Arc::clone(&client.http);
    let cache_for_slowmode = Arc::clone(&client.cache);
    let slowmode_duration = config.slowmode_duration_secs;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            let data = data_for_slowmode.read().await;
            let slowmode = match data.get::<SlowmodeKey>() {
                Some(s) => s,
                None => continue,
            };

            let expired = slowmode.expired_guilds(slowmode_duration);
            for guild_id in expired {
                // Construire un faux Context pour passer à deactivate
                // On utilise directement les channels via HTTP
                let fake_ctx = serenity::client::Context {
                    data: Arc::clone(&data_for_slowmode),
                    shard: serenity::gateway::ShardMessenger::new(tokio::sync::mpsc::unbounded_channel().0),
                    shard_id: serenity::model::id::ShardId(0),
                    http: Arc::clone(&http_for_slowmode),
                    cache: Arc::clone(&cache_for_slowmode),
                };

                slowmode.deactivate(&fake_ctx, guild_id).await;
            }
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
