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

use crate::common::spawn_periodic;

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

    // ─────────────────────────────────────────────────────────────
    // Domaine : automod — cloture des votes de moderation a echeance
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "automod_close_votes",
        60,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "automod-bot",
        move |pool| {
            Box::pin(async move { domains::automod::close_votes::run(&pool).await })
        },
    );

    {
        let redis = redis_client.clone();
        spawn_periodic(
            "warm_analytics",
            config.analytics_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
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
            "cache",
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
            "cache",
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
            "cache",
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
            "cache",
            |pool| Box::pin(async move { domains::cache::sync_user_cache::run(&pool).await }),
        );

        spawn_periodic(
            "manage_partitions",
            config.partition_manager_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
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
            "audit-bot",
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
            "blackjack-bot",
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
        "analytics",
        |pool| Box::pin(async move { domains::analytics::daily_snapshot::run(&pool).await }),
    );
    spawn_periodic(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::hourly_snapshot::run(&pool).await }),
    );
    spawn_periodic(
        "analytics_retention_cleanup",
        config.analytics_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::retention_cleanup::run(&pool).await }),
    );
    spawn_periodic(
        "publish_top_users",
        config.top_users_publish_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::publish_top_users::run(&pool).await }),
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
            "temp_roles",
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
            "ticket-bot",
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
        "export",
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
            "audit-bot",
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
            "ai",
            move |pool| {
                let redis = redis.clone();
                let api = api.clone();
                Box::pin(async move {
                    domains::ai::drain_ai_jobs::run(&pool, &redis, &api, timeout).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : announcements (publication horaire alignee)
    // Porte de l'ancien announcement-worker. Structure custom (boucle
    // alignee sur HH:00:00 UTC).
    // ─────────────────────────────────────────────────────────────
    domains::announcements::publish_due::start(api_url.clone(), redis_client.clone());
    spawn_periodic(
        "announcements_retention_cleanup",
        config.announcements_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "announcements",
        |pool| Box::pin(async move { domains::announcements::retention_cleanup::run(&pool).await }),
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : game_portal (4 jobs HTTP-triggered en parallele)
    // Porte de l'ancien game-portal-worker.
    // ─────────────────────────────────────────────────────────────
    domains::game_portal::jobs::start(api_url.clone());

    // ─────────────────────────────────────────────────────────────
    // Domaine : moderation (conduit, bans, propositions, rappels)
    // Porte de l'ancien moderation-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "conduct_regen",
        config.conduct_regen_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-bot",
        |pool| Box::pin(async move { domains::moderation::conduct_regen::run(&pool).await }),
    );
    spawn_periodic(
        "cleanup_bans",
        config.ban_cleanup_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-bot",
        |pool| Box::pin(async move { domains::moderation::cleanup_bans::run(&pool).await }),
    );
    spawn_periodic(
        "sync_ban_proposals",
        config.sync_ban_proposals_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-bot",
        |pool| {
            Box::pin(async move { domains::moderation::sync_ban_proposals::run(&pool).await })
        },
    );
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "send_reminders",
            config.send_reminders_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "moderation-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::moderation::send_reminders::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : coude (6 jobs : combats, paris, hp_regen, tournament,
    // cashbox, daily_chaos). Porte de l'ancien coude-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "expire_combats",
        config.combat_expiry_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "coude-bot",
        |pool| Box::pin(async move { domains::coude::expire_combats::run(&pool).await }),
    );
    {
        let api = api_url.clone();
        let token = config.discord_bot_token.clone();
        spawn_periodic(
            "resolve_betting",
            config.betting_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "coude-bot",
            move |pool| {
                let api = api.clone();
                let token = token.clone();
                Box::pin(async move {
                    domains::coude::resolve_betting::run(&pool, &api, &token).await
                })
            },
        );
    }
    spawn_periodic(
        "hp_regen",
        config.hp_regen_tick_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "coude-bot",
        |pool| Box::pin(async move { domains::coude::hp_regen::run(&pool).await }),
    );
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "resolve_tournament",
            21_600, // 6h fixe (comme l'ancien scheduler)
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "coude-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::coude::resolve_tournament::run(&pool, &redis).await
                })
            },
        );
    }
    {
        let min_days = config.cashbox_min_days as i64;
        spawn_periodic(
            "redistribute_cashbox",
            config.cashbox_tick_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "coude-bot",
            move |pool| {
                Box::pin(async move {
                    domains::coude::redistribute_cashbox::run(&pool, min_days).await
                })
            },
        );
    }

    // Phase 5 — expire_steals : claim les /voler dont la fenetre de
    // defense est ecoulee + publie un event Redis pour le bot.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_steals",
            config.steal_expiry_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "coude-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::coude::expire_steals::run(&pool, &redis).await
                })
            },
        );
    }

    // daily_chaos : delai aleatoire 2-6h, pas un interval fixe -> on
    // spawn une task custom.
    {
        let pool = pool.clone();
        let api = api_url.clone();
        let token = config.discord_bot_token.clone();
        let mut rx = shutdown.clone();
        tokio::spawn(async move {
            use rand::Rng;
            loop {
                let delay_secs = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(7200..=21600)
                };
                info!(
                    delay_secs,
                    "daily_chaos: prochain tick dans {}h{}",
                    delay_secs / 3600,
                    (delay_secs % 3600) / 60
                );
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)) => {}
                    _ = rx.changed() => {
                        if *rx.borrow() { return; }
                    }
                }
                if let Err(e) = domains::coude::daily_chaos::run(&pool, &api, &token).await {
                    tracing::error!(error = %e, "daily_chaos job failed");
                }
            }
        });
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5I — Tickets SLA escalation (toutes categories sauf
    // appel_sanction qui est gere par appeal_sla::escalate_appeal_sla).
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "escalate_ticket_sla",
            config.tickets_sla_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ticket-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::tickets::escalate_sla::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5 — Domaine tickets : fermeture auto des tickets inactifs.
    // Avant : boucle 30min dans le bot. Maintenant : worker UPDATE
    // status='closed' + XADD event 'ticket_auto_closed' que le bot
    // consume pour le menage Discord (notification + delete channel).
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "close_inactive_tickets",
            config.tickets_close_inactive_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ticket-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::tickets::close_inactive::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5F — Domaine security : kick auto des quarantaines expirees
    // (captcha non valide). Le bot publie via API a chaque mise en
    // quarantaine, ce job claim les expirees et XADD quarantine_expired.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "kick_expired_quarantine",
            config.quarantine_kick_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::security::kick_expired_quarantine::run(&pool, &redis).await
                })
            },
        );
    }

    // Phase 5G — Lockdown auto-revert : worker scanne les expires
    // et publie un event avec le JSON des saved_states. Le bot
    // desserialise et restaure les permissions Discord.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_lockdown",
            config.lockdown_expire_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::security::expire_lockdown::run(&pool, &redis).await
                })
            },
        );
    }

    // Phase 5H — Slowmode security auto-revert.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_slowmode",
            config.slowmode_expire_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::security::expire_slowmode::run(&pool, &redis).await
                })
            },
        );
    }

    // Phases suivantes : slowmode automod (meme pattern, ~150 lignes).
    // voice-afk + progression voice tick + tickets SLA dependent
    // d'etat live populé par events Discord -> rester dans le bot.

    // Variables inutilisees a ce stade.
    let _ = (pool, shutdown, redis_client, api_url);
}

