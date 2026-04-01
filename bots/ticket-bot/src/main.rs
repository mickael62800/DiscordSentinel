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

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<config::ConfigKey>(config.clone());
        data.insert::<SlaTrackerKey>(SlaTracker::new());
    }

    spawn_heartbeat(api.clone());

    // Background task: escalade automatique (toutes les 5 min)
    let data_for_escalation = Arc::clone(&client.data);
    let http_for_escalation = Arc::clone(&client.http);
    let api_for_escalation = Arc::clone(&api);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            check_escalations(
                &data_for_escalation,
                &http_for_escalation,
                &api_for_escalation,
            )
            .await;
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}

/// Verifie les tickets sans reponse et les escalade si necessaire.
async fn check_escalations(
    data: &Arc<RwLock<TypeMap>>,
    http: &Arc<serenity::http::Http>,
    api: &Arc<BaseApiClient>,
) {
    let data_lock = data.read().await;
    let sla_tracker = match data_lock.get::<SlaTrackerKey>() {
        Some(s) => s,
        None => return,
    };

    let api_client = ApiClient::new(api.clone());
    let tickets = match api_client.list_tickets().await {
        Ok(t) => t,
        Err(_) => return,
    };

    for ticket in &tickets {
        if ticket.status == "closed" {
            continue;
        }

        let guild_config = api.get_guild_config(&ticket.server).await.unwrap_or_default();
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
                let _ = channel.say(http, &msg).await;
            }
        }

        info!(ticket_id = %ticket.id, "Ticket escalade automatiquement (SLA breach)");
    }
}
