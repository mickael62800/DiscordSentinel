mod adapters;
mod application;
mod config;
mod domain;
mod ports;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::adapters::inbound::http::{router, state::AppState};
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::postgres::{
    PgInfractionRepository, PgModerationRepository, PgRuleRepository, PgSecurityEventRepository,
    PgTicketRepository,
};
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::{
    AnalyzeMessageService, ManageInfractionsService, ManageModerationService,
    ManageRulesService, ManageSecurityService, ManageTicketsService,
};
use crate::config::AppConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sentinel_api=info,tower_http=debug".into()),
        )
        .init();

    let config = AppConfig::from_env();

    info!(addr = %config.bind_addr(), "Démarrage de Sentinel API");

    // ── Connexions infrastructure ──
    let pg_pool = PgPoolOptions::new()
        .max_connections(20)
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

    // ── Adapters sortants ──
    let rule_repo = Arc::new(PgRuleRepository::new(pg_pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pg_pool.clone()));
    let ticket_repo = Arc::new(PgTicketRepository::new(pg_pool.clone()));
    let security_repo = Arc::new(PgSecurityEventRepository::new(pg_pool.clone()));
    let moderation_repo = Arc::new(PgModerationRepository::new(pg_pool.clone()));
    let cache = Arc::new(RedisCache::new(redis_client));

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
    let tickets_uc = Arc::new(ManageTicketsService::new(ticket_repo.clone()));
    let security_uc = Arc::new(ManageSecurityService::new(security_repo.clone()));
    let moderation_uc = Arc::new(ManageModerationService::new(moderation_repo.clone()));

    // ── State & Router ──
    let state = AppState {
        analyze_uc,
        rules_uc,
        infractions_uc,
        tickets_uc,
        security_uc,
        moderation_uc,
        broadcaster,
        api_key: config.api_key.clone(),
    };

    let app = router::build(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

    info!("Sentinel API prêt (WebSocket sur /ws)");

    axum::serve(listener, app).await.expect("Erreur serveur");
}
