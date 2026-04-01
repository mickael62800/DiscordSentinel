mod api_client;
mod commands;
mod config;
mod handler;
mod security;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{
    AccountCheckerKey, AltDetectorKey, CaptchaPendingKey, ConfigKey, Handler,
    LockdownKey, QuarantineKey, RaidDetectorKey, RecentJoinsKey, SecurityApiKey,
    SlowmodeKey,
};
use crate::security::account_checker::AccountChecker;
use crate::security::alt_detector::AltDetector;
use crate::security::captcha::CaptchaPending;
use crate::security::lockdown::LockdownManager;
use crate::security::quarantine::QuarantineManager;
use crate::security::raid_analyzer::RecentJoinsTracker;
use crate::security::raid_detector::RaidDetector;
use crate::security::slowmode::SlowmodeManager;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(
        api_url = %config.base().api_base_url,
        raid_threshold = config.raid_join_threshold,
        raid_window = config.raid_join_window_secs,
        min_account_age = config.min_account_age_secs,
        quarantine = config.quarantine_enabled,
        captcha = config.captcha_enabled,
        slowmode = config.slowmode_seconds,
        "Demarrage du security bot"
    );

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "security-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<SecurityApiKey>(ApiClient::new(Arc::clone(&base_api)));
        data.insert::<RaidDetectorKey>(RaidDetector::new(
            config.raid_join_threshold,
            config.raid_join_window_secs,
        ));
        data.insert::<AccountCheckerKey>(AccountChecker::new(config.min_account_age_secs));
        data.insert::<QuarantineKey>(QuarantineManager::new());
        data.insert::<SlowmodeKey>(SlowmodeManager::new());
        data.insert::<LockdownKey>(LockdownManager::new());
        data.insert::<RecentJoinsKey>(RecentJoinsTracker::new(config.raid_join_window_secs));
        data.insert::<CaptchaPendingKey>(CaptchaPending::new());
        data.insert::<AltDetectorKey>(AltDetector::new(
            config.alt_retention_secs,
            config.alt_name_distance,
            3600, // 1h cluster pour creation dates
        ));
        data.insert::<ConfigKey>(config.clone());
    }

    // Heartbeat task
    spawn_heartbeat(Arc::clone(&base_api));

    // Background task: kick des utilisateurs en quarantaine qui n'ont pas passe le captcha
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
                // Kick l'utilisateur qui n'a pas verifie
                if let Err(e) = guild_id.kick(&*http_for_timeout, user_id).await {
                    tracing::warn!(
                        error = %e,
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Impossible de kick l'utilisateur en quarantaine expiree"
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

    // Background task: revert slowmode apres expiration
    let data_for_slowmode = Arc::clone(&client.data);
    let http_for_slowmode = Arc::clone(&client.http);
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
                slowmode.deactivate_with_http(&http_for_slowmode, guild_id).await;
            }
        }
    });

    // Background task: revert lockdown apres expiration
    let data_for_lockdown = Arc::clone(&client.data);
    let http_for_lockdown = Arc::clone(&client.http);
    let lockdown_duration = config.lockdown_duration_secs;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            let data = data_for_lockdown.read().await;
            let lockdown = match data.get::<LockdownKey>() {
                Some(l) => l,
                None => continue,
            };

            let expired = lockdown.expired_guilds(lockdown_duration);
            for guild_id in expired {
                lockdown.deactivate_with_http(&http_for_lockdown, guild_id).await;
            }
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
