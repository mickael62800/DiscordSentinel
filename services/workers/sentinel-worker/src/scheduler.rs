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
        let redis = redis_client.clone();
        spawn_periodic(
            "cleanup_afk_tables",
            config.blackjack_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "blackjack-cleanup-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::blackjack::cleanup_afk_tables::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : monitoring (surveillance bots/workers online)
    // Porte de l'ancien monitoring-worker. Structure differente :
    // boucle stateful (track previous_online), pas un simple
    // spawn_periodic. On delegue a son propre `start()`.
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = domains::monitoring::MonitorConfig {
            api_url: api_url.clone(),
            api_key: config.api_key.clone(),
            check_interval_secs: config.monitor_check_interval_secs,
        };
        domains::monitoring::check_services::start(redis_client.clone(), cfg);
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : analytics (snapshots quotidien + horaire)
    // Porte de l'ancien analytics-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics-worker",
        |pool| Box::pin(async move { domains::analytics::daily_snapshot::run(&pool).await }),
    );
    spawn_periodic(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics-worker",
        |pool| Box::pin(async move { domains::analytics::hourly_snapshot::run(&pool).await }),
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : temp_roles (expiration des roles temporaires)
    // Porte de l'ancien temp-roles-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_temp_roles",
            config.temp_roles_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "temp-roles-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::temp_roles::expire_temp_roles::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : appeal_sla (escalade des appels de sanction)
    // Porte de l'ancien appeal-sla-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "escalate_appeal_sla",
            config.appeal_sla_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "appeal-sla-worker",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::appeal_sla::escalate_appeal_sla::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : export (drain export_jobs)
    // Porte de l'ancien export-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "drain_export_jobs",
        config.export_scan_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "export-worker",
        |pool| Box::pin(async move { domains::export::drain_export_jobs::run(&pool).await }),
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : discord_audit_sync (poll Discord audit-logs API)
    // Porte de l'ancien discord-audit-sync-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let token = config.discord_bot_token.clone();
        spawn_periodic(
            "sync_discord_audit_logs",
            config.audit_sync_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "discord-audit-sync-worker",
            move |pool| {
                let token = token.clone();
                Box::pin(async move {
                    domains::discord_audit_sync::sync_discord_audit_logs::run(&pool, &token).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : ai (drain ai_jobs)
    // Porte de l'ancien ai-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        let api = api_url.clone();
        let timeout = config.ai_job_timeout_secs;
        spawn_periodic(
            "drain_ai_jobs",
            config.ai_poll_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ai-worker",
            move |pool| {
                let redis = redis.clone();
                let api = api.clone();
                Box::pin(async move {
                    domains::ai::drain_ai_jobs::run(&pool, &redis, &api, timeout).await
                })
            },
        );
    }

    // Phases suivantes : moderation, coude, announcement, game-portal,
    // + nouveaux jobs migres depuis le bot.

    // Variables inutilisees a ce stade (les phases suivantes les
    // consommeront).
    let _ = (pool, shutdown, redis_client);
}

