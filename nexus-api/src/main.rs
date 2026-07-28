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
//!
//! Routes :
//!   - POST /api/wheel/{guild_id}/{user_id}/spin
//!   - GET  /api/wallet/{guild_id}/{user_id}
//!   - GET  /health

mod adapters;
mod bootstrap;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

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
    axum::serve(listener, app).await.expect("serve nexus-api");
}
