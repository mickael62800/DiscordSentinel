mod adapters;
mod application;
mod config;
mod domain;
mod ports;

use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing::{error, info};

use crate::adapters::inbound::http::{router, state::AppState};
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::postgres::{
    PgInfractionRepository, PgModerationRepository, PgRuleRepository, PgSecurityEventRepository,
    PgStatsRepository, PgTicketRepository,
};
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::{
    AnalyzeMessageService, ManageInfractionsService, ManageModerationService,
    ManageRulesService, ManageSecurityService, ManageStatsService, ManageTicketsService,
};
use crate::config::AppConfig;

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
    let cache = Arc::new(RedisCache::new(redis_client.clone()));

    // ── WebSocket broadcaster ──
    let broadcaster = Arc::new(EventBroadcaster::new(256));

    // ── Services applicatifs ──
    let analyze_uc = Arc::new(AnalyzeMessageService::new(
        rule_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
    ));
    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(ticket_repo.clone(), cache.clone()));
    let security_uc = Arc::new(ManageSecurityService::new(security_repo.clone(), cache.clone()));
    let moderation_uc = Arc::new(ManageModerationService::new(moderation_repo.clone(), cache.clone()));
    let stats_uc = Arc::new(ManageStatsService::new(stats_repo.clone(), infraction_repo.clone(), cache.clone()));

    // ── State & Router ──
    let state = AppState {
        analyze_uc,
        rules_uc,
        infractions_uc,
        tickets_uc,
        security_uc,
        moderation_uc,
        stats_uc,
        broadcaster,
        api_key: config.api_key.clone(),
        pg_pool: pg_pool.clone(),
        redis_client: redis_client.clone(),
    };

    let app = router::build(state, config.max_body_size, config.rate_limit_per_sec, &config.allowed_origins);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

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

    // Fermer le pool PostgreSQL proprement
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
