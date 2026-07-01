//! Config globale du worker unifie.
//!
//! Chaque domaine ajoute ses propres champs ici (intervalles, retentions,
//! flags). Les valeurs viennent par defaut du code, peuvent etre override
//! par variables d'env, et finalement par la table `bot_guild_config`
//! (cle `sentinel-worker`). Pattern aligne sur les anciens workers.

use std::collections::HashMap;

use crate::common::{
    config_or_env, load_api_url, load_database_url, load_env, load_env_bool, load_redis_url,
    SECS_PER_HOUR, SECS_PER_MINUTE,
};

// ── Defauts cleanup ──
const DEFAULT_VOICE_SESSIONS_RETENTION_DAYS: i64 = 90;
const DEFAULT_LOGS_RETENTION_DAYS: i64 = 30;
const DEFAULT_CLOSED_TICKETS_RETENTION_DAYS: i64 = 180;
const DEFAULT_CLEANUP_INTERVAL_HOURS: u64 = 1;
const DEFAULT_VACUUM_INTERVAL_HOURS: u64 = 24;

// ── Defauts cache (warm Redis) ──
const DEFAULT_ANALYTICS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
const DEFAULT_DASHBOARD_REFRESH_SECS: u64 = 10 * SECS_PER_MINUTE;
const DEFAULT_VOICE_STATS_REFRESH_SECS: u64 = SECS_PER_HOUR;
const DEFAULT_LEADERBOARDS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
const DEFAULT_USER_CACHE_SYNC_SECS: u64 = 15 * SECS_PER_MINUTE;
const DEFAULT_PARTITION_MANAGER_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts audit_cache ──
const DEFAULT_AUDIT_CACHE_REFRESH_SECS: u64 = 60;
/// Limite du snapshot watched_users pousse en Redis (global, non per-guild).
const DEFAULT_WATCHED_USERS_QUERY_LIMIT: i64 = 10_000;

// ── Defauts blackjack ──
const DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS: u64 = 60;

// ── Defauts monitoring ──
const DEFAULT_MONITOR_CHECK_INTERVAL_SECS: u64 = 30;

// ── Defauts analytics ──
const DEFAULT_DAILY_SNAPSHOT_HOURS: u64 = 1;
const DEFAULT_HOURLY_SNAPSHOT_MINUTES: u64 = 60;
/// Retention announcement_runs : 1x/jour.
const DEFAULT_ANNOUNCEMENTS_RETENTION_SECS: u64 = 24 * SECS_PER_HOUR;
/// Retention cleanup tick : 1x/jour est largement suffisant pour purger
/// les snapshots > data_retention_days (cote API).
const DEFAULT_RETENTION_CLEANUP_SECS: u64 = 24 * SECS_PER_HOUR;
/// Top users publish tick : 1x/heure. L'API ne publie reellement que si
/// `top_users_publish_interval_days` est ecoule depuis le dernier post.
const DEFAULT_TOP_USERS_PUBLISH_CHECK_SECS: u64 = SECS_PER_HOUR;

// ── Defauts temp_roles ──
const DEFAULT_TEMP_ROLES_SCAN_SECS: u64 = SECS_PER_MINUTE;

// ── Defauts appeal_sla ──
const DEFAULT_APPEAL_SLA_SCAN_SECS: u64 = 120;

// ── Defauts export ──
const DEFAULT_EXPORT_SCAN_SECS: u64 = 5;

// ── Defauts discord_audit_sync ──
const DEFAULT_AUDIT_SYNC_SECS: u64 = 300;

// ── Defauts ai ──
const DEFAULT_AI_POLL_SECS: u64 = 2;
const DEFAULT_AI_JOB_TIMEOUT_SECS: u64 = 2 * SECS_PER_MINUTE;

// ── Defauts moderation ──
const DEFAULT_BAN_CLEANUP_MINUTES: u64 = 1;
const DEFAULT_SEND_REMINDERS_SECS: u64 = 30;

// ── Defauts coude ──
const DEFAULT_COMBAT_EXPIRY_CHECK_SECS: u64 = 86400; // 24h (a baisser plus tard)
const DEFAULT_BETTING_CHECK_SECS: u64 = 30;
const DEFAULT_HP_REGEN_TICK_SECS: u64 = 300;
const DEFAULT_CASHBOX_TICK_SECS: u64 = 3600;
const DEFAULT_CASHBOX_MIN_DAYS: u64 = 7;
// Phase 5 — Vol /voler : tick frequent (5s) car la fenetre de defense
// est de 60s, donc une latence max de 65s pour la resolution AFK.
const DEFAULT_STEAL_EXPIRY_CHECK_SECS: u64 = 5;

// Phase 5 — Tickets close inactifs : tick 30 min (meme cadence que
// l'ancienne boucle bot).
const DEFAULT_TICKETS_CLOSE_INACTIVE_SECS: u64 = 1800;

// Phase 5I — Tickets SLA escalation : tick 5 min.
const DEFAULT_TICKETS_SLA_CHECK_SECS: u64 = 300;

// Phase 5F — Quarantine kick : tick 15s (l'ancienne boucle etait a 30s
// mais avec une fenetre captcha typique de 5min, 15s = bonne reactivite).
const DEFAULT_QUARANTINE_KICK_CHECK_SECS: u64 = 15;

// Phase 5G — Lockdown expire : tick 15s (meme cadence que la boucle
// d'origine cote bot).
const DEFAULT_LOCKDOWN_EXPIRE_CHECK_SECS: u64 = 15;

// Phase 5H — Slowmode security expire : tick 15s.
const DEFAULT_SLOWMODE_EXPIRE_CHECK_SECS: u64 = 15;

// Tamagotchi : tick de cycle de vie (decroissance/maladie/mort). 300s par defaut.
const DEFAULT_TAMAGOTCHI_TICK_INTERVAL_SECS: u64 = 300;

// ── Defauts moderation age-unban ──
/// Auto-deban verification d'age : cadence mensuelle (30 j) par defaut.
const DEFAULT_AGE_UNBAN_INTERVAL_SECS: u64 = 30 * 24 * SECS_PER_HOUR;

// ── Defauts game-portal (intervals des jobs worker) ──
const DEFAULT_GAME_HEALTH_CHECK_SECS: u64 = 30;
const DEFAULT_GAME_IDLE_SHUTDOWN_CHECK_SECS: u64 = SECS_PER_HOUR;
const DEFAULT_GAME_RECONCILER_SECS: u64 = SECS_PER_HOUR;
const DEFAULT_GAME_IMAGE_CLEANUP_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts automod ──
/// Cloture des votes de moderation a echeance : tick 60s. CHEMIN CRITIQUE —
/// seule voie qui ferme les cartes de vote a leur deadline.
const DEFAULT_AUTOMOD_CLOSE_VOTES_SECS: u64 = SECS_PER_MINUTE;
/// Suppression des cartes closes vieilles de > 1 mois : tick 24h.
const DEFAULT_AUTOMOD_CLEANUP_CARDS_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts classement mensuel (analytics) ──
/// Check horaire ; l'API gate sur le passage de mois.
const DEFAULT_MONTHLY_RANKING_CHECK_SECS: u64 = SECS_PER_HOUR;

// ── Defauts tournoi coude ──
/// Tick horaire ; le job n'agit que dans la fenetre dimanche >= 23h UTC.
const DEFAULT_TOURNAMENT_CHECK_SECS: u64 = SECS_PER_HOUR;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,

    // ── Cleanup ──
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
    pub cleanup_interval_secs: u64,
    pub vacuum_enabled: bool,
    pub vacuum_interval_secs: u64,

    // ── Cache (warm Redis) ──
    pub analytics_refresh_secs: u64,
    pub dashboard_refresh_secs: u64,
    pub voice_stats_refresh_secs: u64,
    pub leaderboards_refresh_secs: u64,
    pub user_cache_sync_secs: u64,
    pub partition_manager_secs: u64,

    // ── Audit cache ──
    pub audit_cache_refresh_secs: u64,
    pub watched_users_query_limit: i64,

    // ── Blackjack ──
    pub blackjack_scan_interval_secs: u64,

    // ── Monitoring ──
    pub api_key: String,
    pub monitor_check_interval_secs: u64,

    // ── Analytics ──
    pub daily_snapshot_interval_secs: u64,
    pub hourly_snapshot_interval_secs: u64,
    pub analytics_retention_check_secs: u64,
    pub top_users_publish_check_secs: u64,
    pub announcements_retention_check_secs: u64,

    // ── Temp roles ──
    pub temp_roles_scan_interval_secs: u64,

    // ── Appeal SLA ──
    pub appeal_sla_scan_interval_secs: u64,

    // ── Export ──
    pub export_scan_interval_secs: u64,

    // ── Discord audit sync ──
    pub audit_sync_interval_secs: u64,
    pub discord_bot_token: String,

    // ── AI ──
    pub ai_poll_interval_secs: u64,
    pub ai_job_timeout_secs: u64,

    // ── Moderation ──
    pub ban_cleanup_interval_secs: u64,
    pub send_reminders_interval_secs: u64,
    pub age_unban_interval_secs: u64,

    // ── Coude ──
    pub combat_expiry_check_secs: u64,
    pub betting_check_secs: u64,
    pub hp_regen_tick_secs: u64,
    pub cashbox_tick_secs: u64,
    pub cashbox_min_days: u64,
    pub steal_expiry_check_secs: u64,

    // ── Tickets ──
    pub tickets_close_inactive_secs: u64,
    pub tickets_sla_check_secs: u64,

    // ── Security ──
    pub quarantine_kick_check_secs: u64,
    pub lockdown_expire_check_secs: u64,
    pub slowmode_expire_check_secs: u64,

    // ── Tamagotchi ──
    pub tamagotchi_tick_interval_secs: u64,

    // ── Game portal (intervals des jobs worker) ──
    pub game_health_check_interval_secs: u64,
    pub game_idle_shutdown_check_interval_secs: u64,
    pub game_reconciler_interval_secs: u64,
    pub game_image_cleanup_interval_secs: u64,

    // ── Automod ──
    pub automod_close_votes_secs: u64,
    pub automod_cleanup_cards_secs: u64,

    // ── Classement mensuel (analytics) ──
    pub monthly_ranking_check_secs: u64,

    // ── Tournoi (coude) ──
    pub tournament_check_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let cleanup_hours: u64 = load_env("CLEANUP_INTERVAL_HOURS", DEFAULT_CLEANUP_INTERVAL_HOURS);
        let vacuum_hours: u64 = load_env("VACUUM_INTERVAL_HOURS", DEFAULT_VACUUM_INTERVAL_HOURS);

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),

            // cleanup
            voice_sessions_retention_days: load_env(
                "VOICE_SESSIONS_RETENTION_DAYS",
                DEFAULT_VOICE_SESSIONS_RETENTION_DAYS,
            ),
            logs_retention_days: load_env("LOGS_RETENTION_DAYS", DEFAULT_LOGS_RETENTION_DAYS),
            closed_tickets_retention_days: load_env(
                "CLOSED_TICKETS_RETENTION_DAYS",
                DEFAULT_CLOSED_TICKETS_RETENTION_DAYS,
            ),
            cleanup_interval_secs: cleanup_hours * SECS_PER_HOUR,
            vacuum_enabled: load_env_bool("VACUUM_ENABLED", true),
            vacuum_interval_secs: vacuum_hours * SECS_PER_HOUR,

            // cache
            analytics_refresh_secs: load_env(
                "ANALYTICS_CACHE_REFRESH",
                DEFAULT_ANALYTICS_REFRESH_SECS,
            ),
            dashboard_refresh_secs: load_env(
                "DASHBOARD_CACHE_REFRESH",
                DEFAULT_DASHBOARD_REFRESH_SECS,
            ),
            voice_stats_refresh_secs: load_env(
                "VOICE_STATS_CACHE_REFRESH",
                DEFAULT_VOICE_STATS_REFRESH_SECS,
            ),
            leaderboards_refresh_secs: load_env(
                "LEADERBOARDS_REFRESH",
                DEFAULT_LEADERBOARDS_REFRESH_SECS,
            ),
            user_cache_sync_secs: load_env("USER_CACHE_SYNC", DEFAULT_USER_CACHE_SYNC_SECS),
            partition_manager_secs: load_env("PARTITION_MANAGER", DEFAULT_PARTITION_MANAGER_SECS),

            // audit_cache
            audit_cache_refresh_secs: load_env(
                "AUDIT_CACHE_REFRESH_INTERVAL",
                DEFAULT_AUDIT_CACHE_REFRESH_SECS,
            ),
            watched_users_query_limit: load_env(
                "WATCHED_USERS_QUERY_LIMIT",
                DEFAULT_WATCHED_USERS_QUERY_LIMIT,
            ),

            // blackjack
            blackjack_scan_interval_secs: load_env(
                "BLACKJACK_CLEANUP_SCAN_INTERVAL",
                DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS,
            ),

            // monitoring
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            monitor_check_interval_secs: load_env(
                "MONITOR_CHECK_INTERVAL",
                DEFAULT_MONITOR_CHECK_INTERVAL_SECS,
            ),

            // analytics
            daily_snapshot_interval_secs: load_env::<u64>(
                "DAILY_SNAPSHOT_INTERVAL",
                DEFAULT_DAILY_SNAPSHOT_HOURS,
            ) * SECS_PER_HOUR,
            hourly_snapshot_interval_secs: load_env::<u64>(
                "HOURLY_SNAPSHOT_INTERVAL",
                DEFAULT_HOURLY_SNAPSHOT_MINUTES,
            ) * SECS_PER_MINUTE,
            analytics_retention_check_secs: load_env(
                "ANALYTICS_RETENTION_CHECK",
                DEFAULT_RETENTION_CLEANUP_SECS,
            ),
            top_users_publish_check_secs: load_env(
                "TOP_USERS_PUBLISH_CHECK",
                DEFAULT_TOP_USERS_PUBLISH_CHECK_SECS,
            ),
            announcements_retention_check_secs: load_env(
                "ANNOUNCEMENTS_RETENTION_CHECK",
                DEFAULT_ANNOUNCEMENTS_RETENTION_SECS,
            ),

            // temp_roles
            temp_roles_scan_interval_secs: load_env(
                "TEMP_ROLES_SCAN_INTERVAL",
                DEFAULT_TEMP_ROLES_SCAN_SECS,
            ),

            // appeal_sla
            appeal_sla_scan_interval_secs: load_env(
                "APPEAL_SLA_SCAN_INTERVAL",
                DEFAULT_APPEAL_SLA_SCAN_SECS,
            ),

            // export
            export_scan_interval_secs: load_env("EXPORT_SCAN_INTERVAL", DEFAULT_EXPORT_SCAN_SECS),

            // discord_audit_sync
            audit_sync_interval_secs: load_env("AUDIT_SYNC_INTERVAL", DEFAULT_AUDIT_SYNC_SECS),
            discord_bot_token: std::env::var("SENTINEL_DISCORD_TOKEN")
                .or_else(|_| std::env::var("DISCORD_TOKEN"))
                .unwrap_or_default(),

            // ai
            ai_poll_interval_secs: load_env("AI_POLL_INTERVAL", DEFAULT_AI_POLL_SECS),
            ai_job_timeout_secs: load_env("AI_JOB_TIMEOUT", DEFAULT_AI_JOB_TIMEOUT_SECS),

            // moderation
            ban_cleanup_interval_secs: load_env::<u64>(
                "BAN_CLEANUP_INTERVAL",
                DEFAULT_BAN_CLEANUP_MINUTES,
            ) * SECS_PER_MINUTE,
            send_reminders_interval_secs: load_env(
                "SEND_REMINDERS_INTERVAL",
                DEFAULT_SEND_REMINDERS_SECS,
            ),
            age_unban_interval_secs: load_env(
                "AGE_UNBAN_INTERVAL",
                DEFAULT_AGE_UNBAN_INTERVAL_SECS,
            ),

            // coude
            combat_expiry_check_secs: load_env(
                "COMBAT_EXPIRY_CHECK_SECS",
                DEFAULT_COMBAT_EXPIRY_CHECK_SECS,
            ),
            betting_check_secs: load_env("BETTING_CHECK_SECS", DEFAULT_BETTING_CHECK_SECS),
            hp_regen_tick_secs: load_env("HP_REGEN_TICK_SECS", DEFAULT_HP_REGEN_TICK_SECS),
            cashbox_tick_secs: load_env("CASHBOX_TICK_SECS", DEFAULT_CASHBOX_TICK_SECS),
            cashbox_min_days: load_env("CASHBOX_MIN_DAYS", DEFAULT_CASHBOX_MIN_DAYS),
            steal_expiry_check_secs: load_env(
                "STEAL_EXPIRY_CHECK_SECS",
                DEFAULT_STEAL_EXPIRY_CHECK_SECS,
            ),

            // tickets
            tickets_close_inactive_secs: load_env(
                "TICKETS_CLOSE_INACTIVE_SECS",
                DEFAULT_TICKETS_CLOSE_INACTIVE_SECS,
            ),
            tickets_sla_check_secs: load_env(
                "TICKETS_SLA_CHECK_SECS",
                DEFAULT_TICKETS_SLA_CHECK_SECS,
            ),

            // security
            quarantine_kick_check_secs: load_env(
                "QUARANTINE_KICK_CHECK_SECS",
                DEFAULT_QUARANTINE_KICK_CHECK_SECS,
            ),
            lockdown_expire_check_secs: load_env(
                "LOCKDOWN_EXPIRE_CHECK_SECS",
                DEFAULT_LOCKDOWN_EXPIRE_CHECK_SECS,
            ),
            slowmode_expire_check_secs: load_env(
                "SLOWMODE_EXPIRE_CHECK_SECS",
                DEFAULT_SLOWMODE_EXPIRE_CHECK_SECS,
            ),

            // tamagotchi
            tamagotchi_tick_interval_secs: load_env(
                "TAMAGOTCHI_TICK_INTERVAL_SECS",
                DEFAULT_TAMAGOTCHI_TICK_INTERVAL_SECS,
            ),

            // game portal (intervals des jobs worker)
            game_health_check_interval_secs: load_env(
                "GAME_HEALTH_CHECK_INTERVAL_SECS",
                DEFAULT_GAME_HEALTH_CHECK_SECS,
            ),
            game_idle_shutdown_check_interval_secs: load_env(
                "GAME_IDLE_SHUTDOWN_CHECK_INTERVAL_SECS",
                DEFAULT_GAME_IDLE_SHUTDOWN_CHECK_SECS,
            ),
            game_reconciler_interval_secs: load_env(
                "GAME_RECONCILER_INTERVAL_SECS",
                DEFAULT_GAME_RECONCILER_SECS,
            ),
            game_image_cleanup_interval_secs: load_env(
                "GAME_IMAGE_CLEANUP_INTERVAL_SECS",
                DEFAULT_GAME_IMAGE_CLEANUP_SECS,
            ),

            // automod
            automod_close_votes_secs: load_env(
                "AUTOMOD_CLOSE_VOTES_SECS",
                DEFAULT_AUTOMOD_CLOSE_VOTES_SECS,
            ),
            automod_cleanup_cards_secs: load_env(
                "AUTOMOD_CLEANUP_CARDS_SECS",
                DEFAULT_AUTOMOD_CLEANUP_CARDS_SECS,
            ),

            // classement mensuel (analytics)
            monthly_ranking_check_secs: load_env(
                "MONTHLY_RANKING_CHECK_SECS",
                DEFAULT_MONTHLY_RANKING_CHECK_SECS,
            ),

            // tournoi (coude)
            tournament_check_secs: load_env("TOURNAMENT_CHECK_SECS", DEFAULT_TOURNAMENT_CHECK_SECS),
        }
    }

    /// Surcharge depuis la table `bot_guild_config` (cle `sentinel-worker`).
    pub fn apply_db_config(&mut self, db: &HashMap<String, String>) {
        // cleanup
        self.voice_sessions_retention_days = config_or_env(
            db,
            "voice_sessions_retention_days",
            "VOICE_SESSIONS_RETENTION_DAYS",
            DEFAULT_VOICE_SESSIONS_RETENTION_DAYS,
        );
        self.logs_retention_days = config_or_env(
            db,
            "logs_retention_days",
            "LOGS_RETENTION_DAYS",
            DEFAULT_LOGS_RETENTION_DAYS,
        );
        self.closed_tickets_retention_days = config_or_env(
            db,
            "closed_tickets_retention_days",
            "CLOSED_TICKETS_RETENTION_DAYS",
            DEFAULT_CLOSED_TICKETS_RETENTION_DAYS,
        );
        let cleanup_hours: u64 = config_or_env(
            db,
            "cleanup_interval_hours",
            "CLEANUP_INTERVAL_HOURS",
            DEFAULT_CLEANUP_INTERVAL_HOURS,
        );
        self.cleanup_interval_secs = cleanup_hours * SECS_PER_HOUR;
        let vacuum_hours: u64 = config_or_env(
            db,
            "vacuum_interval_hours",
            "VACUUM_INTERVAL_HOURS",
            DEFAULT_VACUUM_INTERVAL_HOURS,
        );
        self.vacuum_interval_secs = vacuum_hours * SECS_PER_HOUR;

        // cache
        self.analytics_refresh_secs = config_or_env(
            db,
            "analytics_cache_refresh",
            "ANALYTICS_CACHE_REFRESH",
            DEFAULT_ANALYTICS_REFRESH_SECS,
        );
        self.dashboard_refresh_secs = config_or_env(
            db,
            "dashboard_cache_refresh",
            "DASHBOARD_CACHE_REFRESH",
            DEFAULT_DASHBOARD_REFRESH_SECS,
        );
        self.voice_stats_refresh_secs = config_or_env(
            db,
            "voice_stats_cache_refresh",
            "VOICE_STATS_CACHE_REFRESH",
            DEFAULT_VOICE_STATS_REFRESH_SECS,
        );
        self.leaderboards_refresh_secs = config_or_env(
            db,
            "leaderboards_refresh",
            "LEADERBOARDS_REFRESH",
            DEFAULT_LEADERBOARDS_REFRESH_SECS,
        );
        self.user_cache_sync_secs = config_or_env(
            db,
            "user_cache_sync",
            "USER_CACHE_SYNC",
            DEFAULT_USER_CACHE_SYNC_SECS,
        );
        self.partition_manager_secs = config_or_env(
            db,
            "partition_manager",
            "PARTITION_MANAGER",
            DEFAULT_PARTITION_MANAGER_SECS,
        );

        // audit_cache
        self.audit_cache_refresh_secs = config_or_env(
            db,
            "audit_cache_refresh_interval",
            "AUDIT_CACHE_REFRESH_INTERVAL",
            DEFAULT_AUDIT_CACHE_REFRESH_SECS,
        );
        // Borne saine : 100..100000 (evite un LIMIT 0 ou une valeur absurde).
        let watched_limit: i64 = config_or_env(
            db,
            "watched_users_query_limit",
            "WATCHED_USERS_QUERY_LIMIT",
            DEFAULT_WATCHED_USERS_QUERY_LIMIT,
        );
        self.watched_users_query_limit = watched_limit.clamp(100, 100_000);

        // blackjack
        self.blackjack_scan_interval_secs = config_or_env(
            db,
            "blackjack_cleanup_scan_interval",
            "BLACKJACK_CLEANUP_SCAN_INTERVAL",
            DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS,
        );

        // monitoring
        self.monitor_check_interval_secs = config_or_env(
            db,
            "monitor_check_interval",
            "MONITOR_CHECK_INTERVAL",
            DEFAULT_MONITOR_CHECK_INTERVAL_SECS,
        );

        // analytics
        let daily_h: u64 = config_or_env(
            db,
            "daily_snapshot_interval",
            "DAILY_SNAPSHOT_INTERVAL",
            DEFAULT_DAILY_SNAPSHOT_HOURS,
        );
        self.daily_snapshot_interval_secs = daily_h * SECS_PER_HOUR;
        let hourly_m: u64 = config_or_env(
            db,
            "hourly_snapshot_interval",
            "HOURLY_SNAPSHOT_INTERVAL",
            DEFAULT_HOURLY_SNAPSHOT_MINUTES,
        );
        self.hourly_snapshot_interval_secs = hourly_m * SECS_PER_MINUTE;
        self.analytics_retention_check_secs = config_or_env(
            db,
            "analytics_retention_check",
            "ANALYTICS_RETENTION_CHECK",
            DEFAULT_RETENTION_CLEANUP_SECS,
        );
        self.top_users_publish_check_secs = config_or_env(
            db,
            "top_users_publish_check",
            "TOP_USERS_PUBLISH_CHECK",
            DEFAULT_TOP_USERS_PUBLISH_CHECK_SECS,
        );
        self.announcements_retention_check_secs = config_or_env(
            db,
            "announcements_retention_check",
            "ANNOUNCEMENTS_RETENTION_CHECK",
            DEFAULT_ANNOUNCEMENTS_RETENTION_SECS,
        );

        // temp_roles
        self.temp_roles_scan_interval_secs = config_or_env(
            db,
            "temp_roles_scan_interval",
            "TEMP_ROLES_SCAN_INTERVAL",
            DEFAULT_TEMP_ROLES_SCAN_SECS,
        );

        // appeal_sla
        self.appeal_sla_scan_interval_secs = config_or_env(
            db,
            "appeal_sla_scan_interval",
            "APPEAL_SLA_SCAN_INTERVAL",
            DEFAULT_APPEAL_SLA_SCAN_SECS,
        );

        // export
        self.export_scan_interval_secs = config_or_env(
            db,
            "export_scan_interval",
            "EXPORT_SCAN_INTERVAL",
            DEFAULT_EXPORT_SCAN_SECS,
        );

        // discord_audit_sync
        self.audit_sync_interval_secs = config_or_env(
            db,
            "audit_sync_interval",
            "AUDIT_SYNC_INTERVAL",
            DEFAULT_AUDIT_SYNC_SECS,
        );

        // ai
        self.ai_poll_interval_secs = config_or_env(
            db,
            "ai_poll_interval",
            "AI_POLL_INTERVAL",
            DEFAULT_AI_POLL_SECS,
        );
        self.ai_job_timeout_secs = config_or_env(
            db,
            "ai_job_timeout",
            "AI_JOB_TIMEOUT",
            DEFAULT_AI_JOB_TIMEOUT_SECS,
        );

        // moderation
        let cleanup_m: u64 = config_or_env(
            db,
            "ban_cleanup_interval",
            "BAN_CLEANUP_INTERVAL",
            DEFAULT_BAN_CLEANUP_MINUTES,
        );
        self.ban_cleanup_interval_secs = cleanup_m * SECS_PER_MINUTE;
        self.send_reminders_interval_secs = config_or_env(
            db,
            "send_reminders_interval",
            "SEND_REMINDERS_INTERVAL",
            DEFAULT_SEND_REMINDERS_SECS,
        );

        // coude
        self.combat_expiry_check_secs = config_or_env(
            db,
            "combat_expiry_check_secs",
            "COMBAT_EXPIRY_CHECK_SECS",
            DEFAULT_COMBAT_EXPIRY_CHECK_SECS,
        );
        self.betting_check_secs = config_or_env(
            db,
            "betting_check_secs",
            "BETTING_CHECK_SECS",
            DEFAULT_BETTING_CHECK_SECS,
        );
        self.hp_regen_tick_secs = config_or_env(
            db,
            "hp_regen_tick_secs",
            "HP_REGEN_TICK_SECS",
            DEFAULT_HP_REGEN_TICK_SECS,
        );
        self.cashbox_tick_secs = config_or_env(
            db,
            "cashbox_tick_secs",
            "CASHBOX_TICK_SECS",
            DEFAULT_CASHBOX_TICK_SECS,
        );
        self.cashbox_min_days = config_or_env(
            db,
            "cashbox_min_days",
            "CASHBOX_MIN_DAYS",
            DEFAULT_CASHBOX_MIN_DAYS,
        );
        self.steal_expiry_check_secs = config_or_env(
            db,
            "steal_expiry_check_secs",
            "STEAL_EXPIRY_CHECK_SECS",
            DEFAULT_STEAL_EXPIRY_CHECK_SECS,
        );

        // tickets
        self.tickets_close_inactive_secs = config_or_env(
            db,
            "tickets_close_inactive_secs",
            "TICKETS_CLOSE_INACTIVE_SECS",
            DEFAULT_TICKETS_CLOSE_INACTIVE_SECS,
        );
        self.tickets_sla_check_secs = config_or_env(
            db,
            "tickets_sla_check_secs",
            "TICKETS_SLA_CHECK_SECS",
            DEFAULT_TICKETS_SLA_CHECK_SECS,
        );

        // security
        self.quarantine_kick_check_secs = config_or_env(
            db,
            "quarantine_kick_check_secs",
            "QUARANTINE_KICK_CHECK_SECS",
            DEFAULT_QUARANTINE_KICK_CHECK_SECS,
        );
        self.lockdown_expire_check_secs = config_or_env(
            db,
            "lockdown_expire_check_secs",
            "LOCKDOWN_EXPIRE_CHECK_SECS",
            DEFAULT_LOCKDOWN_EXPIRE_CHECK_SECS,
        );
        self.slowmode_expire_check_secs = config_or_env(
            db,
            "slowmode_expire_check_secs",
            "SLOWMODE_EXPIRE_CHECK_SECS",
            DEFAULT_SLOWMODE_EXPIRE_CHECK_SECS,
        );

        // moderation (age-unban) — omis precedemment de apply_db_config.
        self.age_unban_interval_secs = config_or_env(
            db,
            "age_unban_interval_secs",
            "AGE_UNBAN_INTERVAL",
            DEFAULT_AGE_UNBAN_INTERVAL_SECS,
        );

        // tamagotchi — omis precedemment de apply_db_config.
        self.tamagotchi_tick_interval_secs = config_or_env(
            db,
            "tamagotchi_tick_interval_secs",
            "TAMAGOTCHI_TICK_INTERVAL_SECS",
            DEFAULT_TAMAGOTCHI_TICK_INTERVAL_SECS,
        );

        // game portal (intervals des jobs worker) — cles du schema game-portal.
        self.game_health_check_interval_secs = config_or_env(
            db,
            "health_check_interval_secs",
            "GAME_HEALTH_CHECK_INTERVAL_SECS",
            DEFAULT_GAME_HEALTH_CHECK_SECS,
        );
        self.game_idle_shutdown_check_interval_secs = config_or_env(
            db,
            "idle_shutdown_check_interval_secs",
            "GAME_IDLE_SHUTDOWN_CHECK_INTERVAL_SECS",
            DEFAULT_GAME_IDLE_SHUTDOWN_CHECK_SECS,
        );
        self.game_reconciler_interval_secs = config_or_env(
            db,
            "reconciler_interval_secs",
            "GAME_RECONCILER_INTERVAL_SECS",
            DEFAULT_GAME_RECONCILER_SECS,
        );
        self.game_image_cleanup_interval_secs = config_or_env(
            db,
            "image_cleanup_interval_secs",
            "GAME_IMAGE_CLEANUP_INTERVAL_SECS",
            DEFAULT_GAME_IMAGE_CLEANUP_SECS,
        );

        // automod — cles du schema automod-bot (module flattene).
        self.automod_close_votes_secs = config_or_env(
            db,
            "automod_close_votes_secs",
            "AUTOMOD_CLOSE_VOTES_SECS",
            DEFAULT_AUTOMOD_CLOSE_VOTES_SECS,
        );
        self.automod_cleanup_cards_secs = config_or_env(
            db,
            "automod_cleanup_cards_secs",
            "AUTOMOD_CLEANUP_CARDS_SECS",
            DEFAULT_AUTOMOD_CLEANUP_CARDS_SECS,
        );

        // classement mensuel — cle du schema analytics (module flattene).
        self.monthly_ranking_check_secs = config_or_env(
            db,
            "monthly_ranking_check_secs",
            "MONTHLY_RANKING_CHECK_SECS",
            DEFAULT_MONTHLY_RANKING_CHECK_SECS,
        );

        // tournoi — cle du schema coude-bot (module flattene).
        self.tournament_check_secs = config_or_env(
            db,
            "tournament_check_secs",
            "TOURNAMENT_CHECK_SECS",
            DEFAULT_TOURNAMENT_CHECK_SECS,
        );
    }
}

/// Sous-config passee aux jobs cleanup (pour ne pas leur donner toute la
/// WorkerConfig).
#[derive(Clone)]
pub struct CleanupConfig {
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
}

impl From<&WorkerConfig> for CleanupConfig {
    fn from(c: &WorkerConfig) -> Self {
        Self {
            voice_sessions_retention_days: c.voice_sessions_retention_days,
            logs_retention_days: c.logs_retention_days,
            closed_tickets_retention_days: c.closed_tickets_retention_days,
        }
    }
}
