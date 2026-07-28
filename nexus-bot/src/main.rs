//! # nexus-bot — bot Discord de la plateforme jeux Nexus
//!
//! Scaffold serenity minimal, calque sur l'architecture de `sentinel-bot`
//! (adapters autour de `nexus-core`, pas d'acces DB direct).
//! Par defaut le binaire NE se connecte PAS a Discord : sans
//! `NEXUS_DISCORD_TOKEN`, il log et s'arrete proprement.

use serenity::prelude::*;

struct Handler;

impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("nexus-bot scaffold");

    let Ok(token) = std::env::var("NEXUS_DISCORD_TOKEN") else {
        tracing::info!("NEXUS_DISCORD_TOKEN absent — arret (scaffold, pas de connexion Discord)");
        return;
    };

    let mut client = Client::builder(&token, GatewayIntents::non_privileged())
        .event_handler(Handler)
        .await
        .expect("creation du client serenity");
    if let Err(e) = client.start().await {
        tracing::error!("erreur client nexus-bot: {e}");
    }
}
