//! Scheduler central : enregistre tous les jobs periodiques avec leur
//! intervalle et delegue l'execution a `spawn_periodic` (impl commune
//! qui gere shutdown, panic catch, log lifecycle, metrics).
//!
//! Lecture de ce fichier = inventaire complet de ce que fait le worker.
//! Ajouter un job = ajouter une section ici + creer le module dans
//! `domains/{domain}/{job}.rs`.
//!
//! Note sur le `worker_name` passe a `spawn_periodic` et a
//! `is_worker_enabled` : on conserve les **noms d'origine par feature**
//! (cache-worker, audit-cache-worker, ...) plutot que de tout mettre
//! "sentinel-worker". Raison : les toggles `bot_guild_config` existants
//! sont indexes sur ces noms. Les changer obligerait a une migration DB
//! et casserait les configs guild deja en place.

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use sentinel_worker_common::spawn_periodic;

use crate::config::{CleanupConfig, WorkerConfig};
use crate::domains;

const WORKER_NAME: &str = "sentinel-worker";

pub fn start(
    config: &WorkerConfig,
    pool: PgPool,
    redis_client: redis::Client,
    shutdown: watch::Receiver<bool>,
) {
    let api_url = config.api_url.clone();

    // ─────────────────────────────────────────────────────────────
    // Domaine : cleanup (porte de l'ancien cleanup-worker)
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = CleanupConfig::from(config);
        spawn_periodic(
            "cleanup_old_data",
            config.cleanup_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            WORKER_NAME,
            move |pool| {
                let cfg = cfg.clone();
                Box::pin(async move { domains::cleanup::cleanup_old_data::run(&pool, &cfg).await })
            },
        );

        if config.vacuum_enabled {
            spawn_periodic(
                "vacuum_tables",
                config.vacuum_interval_secs,
                pool.clone(),
                shutdown.clone(),
                api_url.clone(),
                WORKER_NAME,
                |pool| Box::pin(async move { domains::cleanup::vacuum_tables::run(&pool).await }),
            );
        } else {
            info!("VACUUM desactive par configuration");
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : cache (warm Redis pour analytics, dashboard, voice)
    // Porte de l'ancien cache-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "warm_analytics",
            config.analytics_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_analytics::run(&pool, &redis).await })
            },
        );

        let redis = redis_client.clone();
        spawn_periodic(
            "warm_dashboard",
            config.dashboard_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_dashboard::run(&pool, &redis).await })
            },
        );

        let redis = redis_client.clone();
        spawn_periodic(
            "warm_voice_stats",
            config.voice_stats_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_voice_stats::run(&pool, &redis).await })
            },
        );

        spawn_periodic(
            "refresh_leaderboards",
            config.leaderboards_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            |pool| {
                Box::pin(async move {
                    domains::cache::refresh_leaderboards::run(&pool).await
                })
            },
        );

        spawn_periodic(
            "sync_user_cache",
            config.user_cache_sync_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            |pool| Box::pin(async move { domains::cache::sync_user_cache::run(&pool).await }),
        );

        spawn_periodic(
            "manage_partitions",
            config.partition_manager_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache-worker",
            |pool| {
                Box::pin(async move { domains::cache::manage_partitions::run(&pool).await })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : audit_cache (refresh watched_users en Redis)
    // Porte de l'ancien audit-cache-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "refresh_watched_users",
            config.audit_cache_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "audit-cache-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::audit_cache::refresh_watched_users::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : blackjack (cleanup AFK tables)
    // Porte de l'ancien blackjack-cleanup-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client;
        spawn_periodic(
            "cleanup_afk_tables",
            config.blackjack_scan_interval_secs,
            pool,
            shutdown,
            api_url,
            "blackjack-cleanup-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::blackjack::cleanup_afk_tables::run(&pool, &redis).await
                })
            },
        );
    }

    // Phases suivantes : moderation, coude, analytics, temp-roles,
    // appeal-sla, announcement, export, game-portal, discord-audit-sync,
    // monitoring, ai, + nouveaux jobs migres depuis le bot.
}
