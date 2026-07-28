//! Bootstrap : cablage des services nexus-core avec les adapters Postgres.

use std::sync::Arc;

use nexus_core::application::play_wheel_service::PlayWheelService;
use nexus_core::ports::inbound::get_wallet::GetWalletUseCase;
use nexus_core::ports::inbound::play_wheel::PlayWheelUseCase;
use sqlx::postgres::PgPoolOptions;

use crate::adapters::outbound::postgres::wallet_repository::PgWalletRepository;
use crate::adapters::outbound::postgres::wheel_repository::PgWheelRepository;

#[derive(Clone)]
pub struct AppState {
    pub play_wheel: Arc<dyn PlayWheelUseCase>,
    pub get_wallet: Arc<dyn GetWalletUseCase>,
    /// Si Some, toutes les routes /api exigent `Authorization: Bearer <key>`.
    pub api_key: Option<String>,
}

/// Connecte le pool Postgres (NEXUS_DATABASE_URL), applique les migrations
/// `nexus-api/migrations/`, et construit l'AppState.
pub async fn build_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let db_url = std::env::var("NEXUS_DATABASE_URL")
        .map_err(|_| "NEXUS_DATABASE_URL manquante (ex: postgres://user:pass@host/nexus)")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let wheel_repo = Arc::new(PgWheelRepository::new(pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pool));
    let service = Arc::new(PlayWheelService::new(wheel_repo, wallet_repo));

    let api_key = std::env::var("NEXUS_API_KEY").ok().filter(|k| !k.is_empty());
    if api_key.is_none() {
        tracing::warn!("NEXUS_API_KEY absente — API SANS auth (dev uniquement)");
    }

    Ok(AppState {
        play_wheel: service.clone(),
        get_wallet: service,
        api_key,
    })
}
