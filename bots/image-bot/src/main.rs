mod analysis_queue;
mod api_client;
mod channel_thresholds;
mod commands;
mod config;
mod handler;
mod image_hash;

use std::sync::Arc;

use dashmap::DashSet;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::embeds::moderate_embed;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::analysis_queue::AnalysisQueue;
use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, HashCacheKey, MaxImageSizeKey, ProcessedMessagesKey, QueueSenderKey};
use crate::image_hash::ImageHashCache;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(
        api_url = %config.base().api_base_url,
        max_image_size = config.max_image_size,
        "Demarrage de l'image bot"
    );

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let base_api = Arc::new(BaseApiClient::new(&config, "image-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    // Creer la queue d'analyse
    let (queue, mut rx) = AnalysisQueue::new(100);

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashSet::new()));
        data.insert::<MaxImageSizeKey>(config.max_image_size);
        data.insert::<HashCacheKey>(ImageHashCache::new(300));
        data.insert::<QueueSenderKey>(queue);
    }

    spawn_heartbeat(Arc::clone(&base_api));

    // Background task : consumer de la queue d'analyse
    let api_for_queue = Arc::clone(&base_api);
    let http_for_queue = Arc::clone(&client.http);
    tokio::spawn(async move {
        while let Some(queued) = rx.recv().await {
            let api_client = ApiClient::new(Arc::clone(&api_for_queue), 10 * 1024 * 1024);
            let max_retries = 3u32;

            let mut success = false;
            for attempt in 0..max_retries {
                match api_client.analyze_image(&queued.request).await {
                    Ok(response) => {
                        if response.action != crate::api_client::Action::None {
                            info!(
                                action = ?response.action,
                                message_id = queued.message_id,
                                "Queue: analyse terminee"
                            );
                        }
                        // Action executee par le handler via l'API
                        success = true;
                        break;
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            attempt = attempt + 1,
                            max = max_retries,
                            "Queue: erreur analyse, retry..."
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                }
            }

            if !success {
                // Suppression preventive apres echec de tous les retries
                let channel = serenity::model::id::ChannelId::new(queued.channel_id);
                let message = serenity::model::id::MessageId::new(queued.message_id);
                let _ = channel.delete_message(&http_for_queue, message).await;
                let embed = moderate_embed("Image supprimee (queue)")
                    .description("API indisponible apres plusieurs tentatives.");
                let _ = channel.send_message(
                    &http_for_queue,
                    serenity::builder::CreateMessage::new().embed(embed),
                ).await;
                error!(message_id = queued.message_id, "Queue: suppression preventive apres {} retries", max_retries);
            }
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
