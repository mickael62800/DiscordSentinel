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
                    shard_total,
                    "SHARD_ID >= SHARD_TOTAL, fallback sur single shard"
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

/// Parse la config shard depuis l'environnement — expose pour tests +
/// diagnostics (ex: endpoint /health qui affiche la config courante).
#[derive(Debug, Clone, PartialEq)]
pub enum ShardMode {
    Single,
    Auto,
    Manual { shard_id: u32, shard_total: u32 },
}

impl ShardMode {
    pub fn from_env() -> Self {
        parse_shard_mode(
            std::env::var("SHARD_MODE").ok().as_deref(),
            std::env::var("SHARD_ID").ok().as_deref(),
            std::env::var("SHARD_TOTAL").ok().as_deref(),
        )
    }
}

fn parse_shard_mode(
    mode: Option<&str>,
    shard_id: Option<&str>,
    shard_total: Option<&str>,
) -> ShardMode {
    let mode = mode.unwrap_or("single").to_lowercase();
    match mode.as_str() {
        "auto" => ShardMode::Auto,
        "manual" => {
            let id: u32 = shard_id.and_then(|s| s.parse().ok()).unwrap_or(0);
            let total: u32 = shard_total.and_then(|s| s.parse().ok()).unwrap_or(1);
            if total == 0 || id >= total {
                ShardMode::Single
            } else {
                ShardMode::Manual {
                    shard_id: id,
                    shard_total: total,
                }
            }
        }
        _ => ShardMode::Single,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_is_single() {
        assert_eq!(parse_shard_mode(None, None, None), ShardMode::Single);
        assert_eq!(parse_shard_mode(Some(""), None, None), ShardMode::Single);
    }

    #[test]
    fn parse_single_explicit() {
        assert_eq!(
            parse_shard_mode(Some("single"), None, None),
            ShardMode::Single
        );
        // Case-insensitive
        assert_eq!(
            parse_shard_mode(Some("SINGLE"), None, None),
            ShardMode::Single
        );
    }

    #[test]
    fn parse_auto() {
        assert_eq!(parse_shard_mode(Some("auto"), None, None), ShardMode::Auto);
        assert_eq!(parse_shard_mode(Some("AUTO"), None, None), ShardMode::Auto);
    }

    #[test]
    fn parse_manual_valid() {
        assert_eq!(
            parse_shard_mode(Some("manual"), Some("0"), Some("4")),
            ShardMode::Manual {
                shard_id: 0,
                shard_total: 4
            }
        );
        assert_eq!(
            parse_shard_mode(Some("manual"), Some("3"), Some("4")),
            ShardMode::Manual {
                shard_id: 3,
                shard_total: 4
            }
        );
    }

    #[test]
    fn parse_manual_total_zero_fallback() {
        assert_eq!(
            parse_shard_mode(Some("manual"), Some("0"), Some("0")),
            ShardMode::Single
        );
    }

    #[test]
    fn parse_manual_id_out_of_range_fallback() {
        // id >= total -> fallback single
        assert_eq!(
            parse_shard_mode(Some("manual"), Some("5"), Some("4")),
            ShardMode::Single
        );
        assert_eq!(
            parse_shard_mode(Some("manual"), Some("4"), Some("4")),
            ShardMode::Single
        );
    }

    #[test]
    fn parse_manual_missing_defaults() {
        // manual sans id/total -> id=0, total=1 -> 0 < 1 donc valid Manual
        assert_eq!(
            parse_shard_mode(Some("manual"), None, None),
            ShardMode::Manual {
                shard_id: 0,
                shard_total: 1
            }
        );
    }

    #[test]
    fn parse_unknown_mode_fallback() {
        assert_eq!(
            parse_shard_mode(Some("cluster"), None, None),
            ShardMode::Single
        );
        assert_eq!(
            parse_shard_mode(Some("nonsense"), Some("0"), Some("4")),
            ShardMode::Single
        );
    }
}
