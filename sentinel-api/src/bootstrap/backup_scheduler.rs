//! Planificateur de sauvegardes automatiques (guild-backup).
//!
//! Consomme la config `guild-backup-bot` (deja exposee dans la page Composants) :
//!   - `auto_backup_enabled` (bool)
//!   - `auto_backup_interval_hours` (nombre, defaut 24)
//! Pour chaque serveur eligible, si la sauvegarde la plus recente date de plus
//! de l'intervalle configure (ou s'il n'y en a aucune), publie l'event Redis
//! `guild_backup:capture_requested` — le bot execute la capture, la retention
//! (quota) evince les plus anciennes.
//!
//! Le check tourne a intervalle court (`BACKUP_SCHEDULER_INTERVAL_SECS`, defaut
//! 1800s) ; c'est la comparaison d'AGE de la derniere capture qui garantit
//! qu'on ne capture qu'une fois par `interval_hours`.

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::adapters::inbound::http::state::AppState;

pub fn spawn(state: AppState) {
    let check_secs: u64 = std::env::var("BACKUP_SCHEDULER_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(check_secs)).await;
            if let Err(e) = run_once(&state).await {
                tracing::warn!(error = %e, "backup_scheduler: passe echouee");
            }
        }
    });
}

async fn run_once(state: &AppState) -> Result<(), sqlx::Error> {
    let guilds: Vec<String> = sqlx::query_scalar("SELECT guild_id FROM guilds")
        .fetch_all(&state.pg_pool)
        .await?;

    for gid in guilds {
        let cfg = match state
            .bot_config_repo
            .get_config(&gid, "guild-backup-bot")
            .await
        {
            Ok(c) => c,
            Err(_) => continue,
        };
        let get = |key: &str| {
            cfg.iter()
                .find(|c| c.config_key == key)
                .map(|c| c.config_value.clone())
        };

        // Meme regle que le worker (`is_worker_enabled`) sur la meme ligne
        // `guild-backup-bot/enabled` : absent => actif, present => parse_bool_str.
        let enabled = sentinel_core::domain::entities::system::config_parsers::parse_enabled_flag(
            get("enabled").as_deref(),
        );
        let auto = get("auto_backup_enabled")
            .map(|v| sentinel_core::domain::entities::system::config_parsers::parse_bool_str(&v))
            .unwrap_or(false);
        if !enabled || !auto {
            continue;
        }
        let interval_hours: i64 = get("auto_backup_interval_hours")
            .and_then(|v| v.parse().ok())
            .filter(|h| *h > 0)
            .unwrap_or(24);

        // Age de la sauvegarde la plus recente.
        let snaps = match state
            .guild_backup
            .guild_snapshots_uc
            .list_snapshots(&gid)
            .await
        {
            Ok(s) => s,
            Err(_) => continue,
        };
        let newest = snaps
            .iter()
            .filter_map(|s| DateTime::parse_from_rfc3339(&s.created_at).ok())
            .map(|d| d.with_timezone(&Utc))
            .max();

        let due = match newest {
            Some(ts) => Utc::now() - ts >= chrono::Duration::hours(interval_hours),
            None => true, // aucune sauvegarde -> en creer une
        };
        if !due {
            continue;
        }

        state.broadcaster.broadcast(
            "guild_backup:capture_requested",
            serde_json::json!({
                "guild_id": gid,
                "label": "auto",
                "requested_by": "scheduler",
            }),
        );
        tracing::info!(guild = %gid, interval_hours, "backup_scheduler: capture auto declenchee");
    }
    Ok(())
}
