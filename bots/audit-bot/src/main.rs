mod anomaly;
mod api_client;
mod audit_event;
mod commands;
mod config;
mod handler;
mod handlers;
mod message_cache;
mod permission_diff;
mod weekly_report;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::embeds::info_embed;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::anomaly::{AnomalyDetector, AnomalyThresholds};
use crate::config::Config;
use dashmap::DashSet;
use crate::handler::{AnomalyDetectorKey, ConfigKey, Handler, MessageCacheKey, WatchedUserIdsKey, WeeklyTrackerKey};
use crate::message_cache::MessageCache;
use crate::weekly_report::{WeeklyTracker, format_report};

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
    let api = Arc::new(BaseApiClient::new(&config, "audit-bot"));

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<MessageCacheKey>(MessageCache::new(config.message_cache_size));
        data.insert::<AnomalyDetectorKey>(AnomalyDetector::new(
            config.anomaly_window_secs,
            AnomalyThresholds {
                mass_ban: config.anomaly_mass_ban_threshold,
                mass_delete: config.anomaly_mass_delete_threshold,
                mass_role_change: config.anomaly_mass_role_threshold,
            },
        ));
        data.insert::<WeeklyTrackerKey>(WeeklyTracker::new());
        data.insert::<ConfigKey>(config.clone());
        data.insert::<WatchedUserIdsKey>(Arc::new(DashSet::new()));
    }

    spawn_heartbeat(api.clone());

    // Background task: rapport hebdomadaire (toutes les heures, check si lundi 8h-9h UTC)
    if config.weekly_report_enabled {
        let data_for_report = Arc::clone(&client.data);
        let http_for_report = Arc::clone(&client.http);
        let api_for_report = Arc::clone(&api);
        tokio::spawn(async move {
            let mut last_report_week: Option<u8> = None;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;

                let now = time::OffsetDateTime::now_utc();
                let weekday = now.weekday();
                let hour = now.hour();
                let week = now.iso_week();

                // Lundi entre 8h et 9h UTC, une seule fois par semaine
                if weekday != time::Weekday::Monday || hour != 8 {
                    continue;
                }
                if last_report_week == Some(week) {
                    continue;
                }
                last_report_week = Some(week);

                info!("Generation du rapport hebdomadaire...");

                let data = data_for_report.read().await;
                let tracker = match data.get::<WeeklyTrackerKey>() {
                    Some(t) => t,
                    None => continue,
                };

                let all_stats = tracker.take_all();
                drop(data);

                for (guild_id, stats) in &all_stats {
                    let guild_config = api_for_report
                        .get_guild_config(&guild_id.to_string())
                        .await
                        .unwrap_or_default();

                    let log_channel = guild_config
                        .get("log_channel_id")
                        .and_then(|v| v.parse::<u64>().ok());

                    if let Some(channel_id) = log_channel {
                        let channel = serenity::model::id::ChannelId::new(channel_id);
                        let report_text = format_report(stats);

                        let embed = info_embed("Rapport Hebdomadaire — Audit")
                            .description(report_text);
                        let builder = serenity::builder::CreateMessage::new().embed(embed);

                        if let Err(e) = channel.send_message(&http_for_report, builder).await {
                            tracing::warn!(
                                error = %e,
                                guild_id = %guild_id,
                                "Erreur envoi rapport hebdomadaire"
                            );
                        } else {
                            info!(guild_id = %guild_id, "Rapport hebdomadaire envoye");
                        }
                    }
                }
            }
        });
    }

    info!("Demarrage audit-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
