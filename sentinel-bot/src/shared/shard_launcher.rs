//! Phase 7 C — Sharding Discord pour scaling horizontal.
//!
//! Tous les bots Sentinel utilisent historiquement `Client::start()` qui se
//! connecte comme un seul shard (shard 0 / 1). Ca fonctionne tant qu'on est
//! sous le seuil Discord de ~2500 guilds par shard, mais au-dela (ou pour
//! distribuer la charge), il faut du sharding.
//!
//! Ce helper wrap les 3 modes de demarrage Serenity selon `SHARD_MODE` :
//!
//! - **`single`** (defaut, backward-compat) : `Client::start()`, un seul shard.
//!   Utilisation : petits deploiements (< 2500 guilds), dev, tests.
//!
//! - **`auto`** : `Client::start_autosharded()`, qui appelle `/gateway/bot`
//!   sur Discord pour recuperer le nombre de shards recommande puis les
//!   spawn tous dans le process courant. Ideal pour un deploiement
//!   single-process qui veut scale automatiquement avec Discord.
//!
//! - **`manual`** : `Client::start_shard(id, total)` — spawn UN seul shard
//!   avec les id et total explicites. Permet le scaling multi-process :
//!   deployer N replicas du meme bot avec `SHARD_ID=0..N-1` et
//!   `SHARD_TOTAL=N`. Ideal pour Kubernetes StatefulSet ou docker-compose
//!   replicas.
//!
//! # Exemples d'env vars
//!
//! ```env
//! # Par defaut, aucun changement de comportement :
//! # (rien a definir — equivalent a SHARD_MODE=single)
//!
//! # Autoshard : le bot demande a Discord combien de shards il faut
//! SHARD_MODE=auto
//!
//! # Manuel : deployer 4 replicas avec SHARD_ID=0..3
//! SHARD_MODE=manual
//! SHARD_TOTAL=4
//! SHARD_ID=0  # (different par replica)
//! ```
//!
//! # Migration progressive
//!
//! Les bots appellent `start_bot(&mut client)` au lieu de `client.start()`.
//! Par defaut (`SHARD_MODE` absent), le comportement est strictement
//! identique a l'ancien `client.start()` — aucun changement en prod.

use serenity::prelude::Client;
use tracing::{info, warn};

/// Demarre un client Serenity selon la strategie de sharding definie par
/// l'environnement. Voir le module-level doc pour les details.
///
/// Retourne l'erreur Serenity si le client ne peut pas se connecter, comme
/// `Client::start()`.
pub async fn start_bot(client: &mut Client) -> Result<(), serenity::Error> {
    let mode = std::env::var("SHARD_MODE")
        .unwrap_or_else(|_| "single".to_string())
        .to_lowercase();

    match mode.as_str() {
        "auto" => {
            info!("Sharding: mode=auto (autoshard via Discord /gateway/bot)");
            client.start_autosharded().await
        }
        "manual" => {
            let shard_id: u32 = std::env::var("SHARD_ID")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let shard_total: u32 = std::env::var("SHARD_TOTAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);

            if shard_total == 0 {
                warn!("SHARD_TOTAL=0 invalide, fallback sur single shard");
                return client.start().await;
            }
            if shard_id >= shard_total {
                warn!(
                    shard_id,
                    shard_total, "SHARD_ID >= SHARD_TOTAL, fallback sur single shard"
                );
                return client.start().await;
            }

            info!(shard_id, shard_total, "Sharding: mode=manual");
            client.start_shard(shard_id, shard_total).await
        }
        "single" | "" => {
            info!("Sharding: mode=single (shard 0/1, backward compat)");
            client.start().await
        }
        other => {
            warn!(mode = %other, "SHARD_MODE inconnu, fallback sur single");
            client.start().await
        }
    }
}
