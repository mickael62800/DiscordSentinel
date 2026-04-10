// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système (jemalloc ne compile
// pas dans ce target). Gain typique : -15 % RAM résidente sur les processus
// long-running grâce à une meilleure gestion de la fragmentation mémoire.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing::{error, info};

use sentinel_api::adapters::inbound::http::{router, state::AppState};
use sentinel_api::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::adapters::outbound::postgres::{
    PgBotConfigRepository, PgConductRepository, PgCoudeBetRepository, PgCoudeCombatRepository, PgCoudeEconomyRepository, PgCoudeInventoryRepository, PgCoudePlayerRepository, PgCoudeSocialRepository, PgGuildRepository, PgInfractionRepository,
    PgMemberRepository, PgModerationRepository, PgRuleRepository, PgSecurityEventRepository, PgStatsRepository,
    PgAnalyticsRepository, PgBlackjackRepository, PgDailyActivityRepository, PgDiscordRoleRepository, PgIaConfigRepository, PgLevelRepository, PgNotesRepository, PgReminderRepository, PgRolePanelRepository, PgStrikeRepository, PgTicketRepository, PgVoiceChannelRepository, PgWalletRepository, PgWatchedUserRepository,
};
use sentinel_api::adapters::outbound::batching::{
    BatchWriterConfig, BatchedPgAuditLogRepository, BatchedPgLogRepository,
};
use sentinel_api::adapters::outbound::job_client::JobClient;
use sentinel_api::adapters::outbound::redis_cache::RedisCache;
use sentinel_api::application::{
    AnalyzeImageService, AnalyzeMessageService, ManageConductService, ManageCoudeBetsService, ManageCoudeCombatsService, ManageCoudeEconomyService, ManageCoudeInventoryService, ManageCoudePlayersService, ManageCoudeSocialService, ManageInfractionsService,
    ManageModerationService, ManageRulesService, ManageSecurityService, ManageStatsService,
    BlackjackService, ManageAuditLogsService, ManageLevelsService, ManageMembersService, ManageNotesService, ManageRemindersService, ManageRolePanelsService, ManageStrikesService, ManageTicketsService, ManageVoiceChannelsService, ManageWatchedUsersService,
};
use sentinel_api::domain::services::{DiscordApiService, InferenceService, TextTokenizer};
use sentinel_api::config::AppConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "sentinel_api=info,tower_http=debug".into());

    // JSON structuré en production, format lisible en dev
    let json_logs = std::env::var("LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(false);

    if json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_span_list(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .init();
    }

    // Phase 0 — Observabilité : installe le recorder Prometheus AVANT toute
    // émission de métriques. Doit être appelé avant `Router::build`.
    sentinel_api::adapters::inbound::http::metrics::init_prometheus();

    // Échantillonnage du runtime tokio toutes les 10s → gauges Prometheus
    // (workers_count, busy_ratio, queue_depth, ...).
    sentinel_api::adapters::inbound::http::metrics::spawn_tokio_runtime_sampler();

    let config = AppConfig::from_env();

    info!(
        addr = %config.bind_addr(),
        rate_limit = config.rate_limit_per_sec,
        max_body = config.max_body_size,
        "Démarrage de Sentinel API"
    );

    // ── Connexions infrastructure ──
    let pg_pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .expect("Impossible de se connecter à PostgreSQL");

    // Exécuter les migrations
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Erreur lors des migrations");

    info!("Migrations appliquées");

    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("URL Redis invalide");

    // Vérifier la connexion Redis au démarrage
    match redis_client.get_multiplexed_async_connection().await {
        Ok(_) => info!("Redis connecté"),
        Err(e) => error!("Redis indisponible au démarrage: {e} — le cache sera désactivé"),
    }

    // ── Adapters sortants ──
    let rule_repo = Arc::new(PgRuleRepository::new(pg_pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pg_pool.clone()));
    let ticket_repo = Arc::new(PgTicketRepository::new(pg_pool.clone()));
    let security_repo = Arc::new(PgSecurityEventRepository::new(pg_pool.clone()));
    let moderation_repo = Arc::new(PgModerationRepository::new(pg_pool.clone()));
    let stats_repo = Arc::new(PgStatsRepository::new(pg_pool.clone()));
    let voice_channel_repo = Arc::new(PgVoiceChannelRepository::new(pg_pool.clone()));
    let bot_config_repo = Arc::new(PgBotConfigRepository::new(pg_pool.clone()));
    let conduct_repo = Arc::new(PgConductRepository::new(pg_pool.clone()));
    let guild_repo = Arc::new(PgGuildRepository::new(pg_pool.clone()));
    // Phase 5C — Batch writes : BatchedPgLogRepository bufferise les inserts et
    // flush via multi-row INSERT toutes les 500ms ou 100 entries.
    let log_repo = Arc::new(BatchedPgLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let ia_config_repo = Arc::new(PgIaConfigRepository::new(pg_pool.clone()));
    let notes_repo = Arc::new(PgNotesRepository::new(pg_pool.clone()));
    let reminder_repo = Arc::new(PgReminderRepository::new(pg_pool.clone()));
    let strike_repo = Arc::new(PgStrikeRepository::new(pg_pool.clone()));
    let cache = Arc::new(
        RedisCache::new(redis_client.clone())
            .await
            .expect("Impossible d'etablir la connexion Redis pour le cache"),
    );

    // ── Event broadcaster (Redis pub/sub → gateway WebSocket) ──
    let redis_channel = std::env::var("REDIS_CHANNEL")
        .unwrap_or_else(|_| "sentinel:events".to_string());
    let broadcaster = Arc::new(
        EventBroadcaster::new()
            .with_redis(redis_client.clone(), redis_channel)
    );

    // ── Inference ONNX ──
    let vision_model_path = std::env::var("VISION_MODEL_PATH").ok();
    let text_model_path = std::env::var("TEXT_MODEL_PATH").ok();
    let tokenizer_path = std::env::var("TEXT_TOKENIZER_PATH").ok();
    let text_max_length: usize = std::env::var("TEXT_MAX_LENGTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);

    let inference = Arc::new(InferenceService::new(
        vision_model_path.as_deref(),
        text_model_path.as_deref(),
    ));
    let tokenizer = Arc::new(TextTokenizer::new(
        tokenizer_path.as_deref(),
        text_max_length,
    ));

    // ── Inference rate limiter ──
    let inference_max_concurrent: usize = std::env::var("INFERENCE_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let inference_max_per_sec: u64 = std::env::var("INFERENCE_MAX_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let inference_limiter = Arc::new(
        sentinel_api::domain::services::InferenceRateLimiter::new(inference_max_concurrent, inference_max_per_sec)
    );

    info!(
        max_concurrent = inference_max_concurrent,
        max_per_sec = inference_max_per_sec,
        "Inference rate limiter configuré"
    );

    // ── Services applicatifs ──
    let conduct_uc = Arc::new(ManageConductService::new(conduct_repo.clone(), infraction_repo.clone(), broadcaster.clone(), config.discord_bot_token.clone()));

    let analyze_uc = Arc::new(
        AnalyzeMessageService::new(
            rule_repo.clone(),
            infraction_repo.clone(),
            cache.clone(),
            conduct_uc.clone(),
            ia_config_repo.clone(),
            inference_limiter.clone(),
        )
        .with_text_inference(inference.clone(), tokenizer)
    );
    let analyze_image_uc = Arc::new(AnalyzeImageService::new(
        inference.clone(),
        rule_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        conduct_uc.clone(),
        ia_config_repo.clone(),
        inference_limiter.clone(),
    ));
    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(ticket_repo.clone(), cache.clone()));
    let security_uc = Arc::new(ManageSecurityService::new(security_repo.clone(), cache.clone()));
    let moderation_uc = Arc::new(ManageModerationService::new(moderation_repo.clone(), cache.clone(), conduct_uc.clone()));
    let stats_uc = Arc::new(ManageStatsService::new(stats_repo.clone(), infraction_repo.clone(), cache.clone(), redis_client.clone()));
    let voice_channels_uc = Arc::new(ManageVoiceChannelsService::new(voice_channel_repo.clone(), cache.clone()));
    // Phase 5C — Batch writes : idem que log_repo, pour les audit events.
    let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let audit_logs_uc = Arc::new(ManageAuditLogsService::new(audit_log_repo));
    let role_panel_repo = Arc::new(PgRolePanelRepository::new(pg_pool.clone()));
    let role_panels_uc = Arc::new(ManageRolePanelsService::new(role_panel_repo));
    let analytics_repo = Arc::new(PgAnalyticsRepository::new(pg_pool.clone()));
    let daily_activity_repo = Arc::new(PgDailyActivityRepository::new(pg_pool.clone()));
    let level_repo = Arc::new(PgLevelRepository::new(pg_pool.clone()));
    let levels_uc = Arc::new(ManageLevelsService::new(level_repo));
    let watched_user_repo = Arc::new(PgWatchedUserRepository::new(pg_pool.clone()));
    let notes_uc = Arc::new(ManageNotesService::new(notes_repo));
    let reminders_uc = Arc::new(ManageRemindersService::new(reminder_repo));
    let strikes_uc = Arc::new(ManageStrikesService::new(strike_repo));
    let member_repo = Arc::new(PgMemberRepository::new(pg_pool.clone()));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pg_pool.clone()));
    let blackjack_repo = Arc::new(PgBlackjackRepository::new(pg_pool.clone()));
    let blackjack_svc = Arc::new(BlackjackService::new(blackjack_repo, wallet_repo.clone()));
    let coude_player_repo = Arc::new(PgCoudePlayerRepository::new(pg_pool.clone()));
    let coude_players_uc = Arc::new(ManageCoudePlayersService::new(coude_player_repo));
    let coude_combat_repo = Arc::new(PgCoudeCombatRepository::new(pg_pool.clone()));
    let coude_combats_uc: Arc<dyn sentinel_api::ports::inbound::ManageCoudeCombatsUseCase> =
        Arc::new(ManageCoudeCombatsService::new(coude_combat_repo));
    let coude_bet_repo = Arc::new(PgCoudeBetRepository::new(pg_pool.clone()));
    let coude_bets_uc = Arc::new(ManageCoudeBetsService::new(coude_bet_repo, coude_combats_uc.clone()));
    let coude_economy_repo = Arc::new(PgCoudeEconomyRepository::new(pg_pool.clone()));
    let coude_economy_uc = Arc::new(ManageCoudeEconomyService::new(coude_economy_repo));
    let coude_inventory_repo = Arc::new(PgCoudeInventoryRepository::new(pg_pool.clone()));
    let coude_inventory_uc = Arc::new(ManageCoudeInventoryService::new(coude_inventory_repo));
    let coude_social_repo = Arc::new(PgCoudeSocialRepository::new(pg_pool.clone()));
    let coude_social_uc = Arc::new(ManageCoudeSocialService::new(coude_social_repo));
    let watched_users_uc = Arc::new(ManageWatchedUsersService::new(
        watched_user_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        security_uc.clone(),
        conduct_uc.clone(),
        notes_uc.clone(),
    ));

    let members_uc = Arc::new(ManageMembersService::new(
        member_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        conduct_uc.clone(),
        stats_uc.clone(),
    ));

    // ── Discord API service ──
    let discord_api = Arc::new(DiscordApiService::new(config.discord_bot_token.clone()));

    // ── Job client (queue Redis → worker) ──
    let queue_key = std::env::var("REDIS_QUEUE_KEY")
        .unwrap_or_else(|_| "sentinel:jobs".to_string());
    let job_client = JobClient::new(redis_client.clone(), queue_key);

    // ── State & Router ──
    let state = AppState {
        analyze_uc,
        analyze_image_uc,
        rules_uc,
        infractions_uc,
        tickets_uc,
        security_uc,
        moderation_uc,
        stats_uc,
        voice_channels_uc,
        conduct_uc,
        watched_users_uc,
        audit_logs_uc,
        levels_uc,
        role_panels_uc,
        notes_uc,
        reminders_uc,
        strikes_uc,
        members_uc,
        analytics_repo,
        daily_activity_repo,
        log_repo,
        guild_repo,
        bot_config_repo,
        ia_config_repo,
        discord_role_repo,
        wallet_repo,
        blackjack_svc,
        coude_players_uc,
        coude_combats_uc,
        coude_bets_uc,
        coude_economy_uc,
        coude_inventory_uc,
        coude_social_uc,
        broadcaster,
        job_client,
        discord_api,
        inference: inference.clone(),
        api_key: config.api_key.clone(),
        discord_bot_token: config.discord_bot_token.clone(),
        pg_pool: pg_pool.clone(),
        redis_client: redis_client.clone(),
        cache: Some(cache.clone()),
    };

    let api_log_repo = state.log_repo.clone();
    let app = router::build(state, config.max_body_size, config.rate_limit_per_sec, &config.allowed_origins);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

    // Log demarrage en BDD
    {
        let entry = sentinel_api::domain::entities::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "info".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: format!("API demarree sur {}", config.bind_addr()),
            category: "api".into(),
            details: serde_json::json!({"event": "startup", "bind": config.bind_addr()}),
        };
        if let Err(e) = api_log_repo.save(&entry).await {
            tracing::warn!(error = %e, "Echec sauvegarde log API");
        }
    }

    info!("Sentinel API prêt (WebSocket sur /ws)");

    // ── Graceful shutdown ──
    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Erreur serveur");

    // Attendre que les connexions en cours se terminent
    info!(timeout_secs = config.shutdown_timeout_secs, "Arrêt en cours, attente des requêtes en vol...");
    tokio::time::sleep(shutdown_timeout).await;

    // Log arret en BDD
    {
        let entry = sentinel_api::domain::entities::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "warn".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: "API en cours d'arret".into(),
            category: "api".into(),
            details: serde_json::json!({"event": "shutdown"}),
        };
        if let Err(e) = api_log_repo.save(&entry).await {
            tracing::warn!(error = %e, "Echec sauvegarde log API");
        }
    }

    pg_pool.close().await;
    info!("Sentinel API arrêté proprement");
}

/// Écoute SIGTERM (Docker/K8s) et Ctrl+C (dev local)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Impossible d'écouter Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Impossible d'écouter SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C reçu"),
        _ = terminate => info!("Signal SIGTERM reçu"),
    }
}
