//! # nexus-api — API HTTP de la plateforme jeux Nexus
//!
//! Binaire axum. Architecture hexagonale calquee sur `sentinel-api` :
//! les adapters (`src/adapters/{inbound,outbound}`) implementent les ports
//! de `nexus-core`, `src/bootstrap` cable le tout.
//!
//! Env :
//!   - NEXUS_DATABASE_URL (obligatoire) : base Postgres `nexus`
//!   - NEXUS_API_KEY (recommande) : Bearer exige sur /api/*
//!   - NEXUS_API_PORT (defaut 3100)
//!   - NEXUS_METRICS_TOKEN (optionnel) : protege /metrics
//!   - NEXUS_ALLOWED_ORIGINS, NEXUS_MAX_BODY_SIZE,
//!     NEXUS_RATE_LIMIT_PER_SEC, NEXUS_HEAVY_RATE_LIMIT_PER_SEC
//!     (cf. `adapters::inbound::http::HttpConfig`)
//!
//! Routes :
//!   - POST /api/wheel/{guild_id}/{user_id}/spin
//!   - GET  /api/wallet/{guild_id}/{user_id}
//!   - GET  /health, GET /metrics

mod adapters;
mod bootstrap;

use tokio::signal;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Avant toute chose : une metrique emise avant l'installation du recorder
    // est perdue definitivement.
    adapters::inbound::http::metrics::init_prometheus();
    adapters::inbound::http::metrics::spawn_tokio_runtime_sampler();

    let state = match bootstrap::build_state().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("bootstrap nexus-api impossible: {e}");
            std::process::exit(1);
        }
    };

    let port: u16 = std::env::var("NEXUS_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3100);

    let app = adapters::inbound::http::build_router(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("nexus-api en ecoute sur {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind nexus-api");

    // `into_make_service_with_connect_info` est REQUIS par le rate limit :
    // sans lui, l'extracteur `ConnectInfo` echoue et toutes les requetes
    // seraient rejetees.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("serve nexus-api");

    tracing::info!("nexus-api arrete proprement");
}

/// Ecoute SIGTERM (Docker) et Ctrl+C (dev local).
///
/// Sans ca, un `docker compose down` coupe net les requetes en vol — y compris
/// une creation de serveur de jeu a mi-chemin, qui laisse alors un conteneur
/// orphelin et un port reserve.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("ecoute Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("ecoute SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl+C recu"),
        _ = terminate => tracing::info!("SIGTERM recu"),
    }
}
