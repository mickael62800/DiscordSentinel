// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod analysis_queue;
mod api_client;
mod channel_thresholds;
mod commands;
mod config;
mod handler;
mod image_hash;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::embeds::moderate_embed;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
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

    // Phase 7A — gRPC interne (ImagesService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Erreur fatale: impossible d'initialiser SentinelGrpcClient: {e}");
            std::process::exit(1);
        }
    };

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::small())
        .await
        .expect("Erreur creation du client Discord");

    // Creer la queue d'analyse
    let (queue, mut rx) = AnalysisQueue::new(100);

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashMap::new()));
        data.insert::<MaxImageSizeKey>(config.max_image_size);
        data.insert::<HashCacheKey>(ImageHashCache::new(300));
        data.insert::<QueueSenderKey>(queue);
    }

    spawn_heartbeat(Arc::clone(&base_api));

    // Background task : consumer de la queue d'analyse
    let api_for_queue = Arc::clone(&base_api);
    let grpc_for_queue = Arc::clone(&grpc);
    let http_for_queue = Arc::clone(&client.http);
    tokio::spawn(async move {
        while let Some(queued) = rx.recv().await {
            let api_client = ApiClient::new(
                Arc::clone(&api_for_queue),
                Arc::clone(&grpc_for_queue),
                handler::DEFAULT_MAX_IMAGE_SIZE,
            );
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
                if let Err(e) = channel.delete_message(&http_for_queue, message).await {
                    warn!(error = %e, message_id = queued.message_id, "Queue: impossible de supprimer le message");
                }
                let embed = moderate_embed("Image supprimee (queue)")
                    .description("API indisponible apres plusieurs tentatives.");
                if let Err(e) = channel.send_message(
                    &http_for_queue,
                    serenity::builder::CreateMessage::new().embed(embed),
                ).await {
                    warn!(error = %e, "Queue: impossible d'envoyer l'embed de notification");
                }
                error!(message_id = queued.message_id, "Queue: suppression preventive apres {} retries", max_retries);
            }
        }
    });

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
