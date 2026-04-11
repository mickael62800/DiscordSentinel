// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod commands;
mod config;
mod faq;
mod handler;
mod satisfaction;
mod sla;
mod templates;
mod transcript;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, SlaTrackerKey};
use crate::sla::SlaTracker;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url(), "Demarrage du ticket bot");

    let api = Arc::new(BaseApiClient::new(&config, "ticket-bot"));

    // Phase 7A — gRPC interne (TicketsService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => panic!("SentinelGrpcClient: {e}"),
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::small())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<GrpcClientKey>(grpc.clone());
        data.insert::<config::ConfigKey>(config.clone());
        data.insert::<SlaTrackerKey>(SlaTracker::new());
    }

    spawn_heartbeat(api.clone());

    // Background task: escalade automatique (toutes les 5 min)
    let data_for_escalation = Arc::clone(&client.data);
    let http_for_escalation = Arc::clone(&client.http);
    let api_for_escalation = Arc::clone(&api);
    let grpc_for_escalation = Arc::clone(&grpc);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            check_escalations(
                &data_for_escalation,
                &http_for_escalation,
                &api_for_escalation,
                &grpc_for_escalation,
            )
            .await;
        }
    });

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}

/// Verifie les tickets sans reponse et les escalade si necessaire.
async fn check_escalations(
    data: &Arc<RwLock<TypeMap>>,
    http: &Arc<serenity::http::Http>,
    api: &Arc<BaseApiClient>,
    grpc: &Arc<SentinelGrpcClient>,
) {
    let data_lock = data.read().await;
    let sla_tracker = match data_lock.get::<SlaTrackerKey>() {
        Some(s) => s,
        None => return,
    };

    // Nettoyer les tickets orphelins > 48h
    sla_tracker.cleanup_stale();

    let api_client = ApiClient::new(api.clone(), grpc.clone());
    let tickets = match api_client.list_tickets().await {
        Ok(t) => t,
        Err(_) => return,
    };

    for ticket in &tickets {
        if ticket.status == "closed" {
            continue;
        }

        let guild_config = match api.get_guild_config(&ticket.server).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %ticket.server, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };
        let escalation_minutes = BaseApiClient::config_u64(&guild_config, "sla_escalation_minutes", 60);
        if escalation_minutes == 0 {
            continue;
        }

        let breached = sla_tracker.breached_tickets(escalation_minutes);
        if !breached.contains(&ticket.id) {
            continue;
        }

        if sla_tracker.is_escalated(&ticket.id) {
            continue;
        }

        // Escalader
        if let Err(e) = api_client.update_ticket_priority(&ticket.id, "high").await {
            warn!(error = %e, ticket_id = %ticket.id, "Erreur escalade ticket");
            continue;
        }

        sla_tracker.mark_escalated(&ticket.id);

        // Envoyer un message dans le salon si possible
        if let Some(ref channel_id_str) = ticket.channel_id {
            if let Ok(ch_id) = channel_id_str.parse::<u64>() {
                let channel = serenity::model::id::ChannelId::new(ch_id);
                let msg = format!(
                    "**\u{26a0}\u{fe0f} Escalade automatique** — Ce ticket n'a pas recu de reponse depuis {}min. La priorite a ete augmentee.",
                    escalation_minutes
                );
                if let Err(e) = channel.say(http, &msg).await {
                    warn!(error = %e, "Failed to send escalation message in channel");
                }
            }
        }

        info!(ticket_id = %ticket.id, "Ticket escalade automatiquement (SLA breach)");
    }
}
