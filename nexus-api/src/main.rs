//! # nexus-api — API HTTP de la plateforme jeux Nexus
//!
//! Binaire axum minimal (scaffold). Architecture hexagonale calquee sur
//! `sentinel-api` : les adapters (`src/adapters/{inbound,outbound}`)
//! implementent les ports de `nexus-core`, `src/bootstrap` cable le tout.
//! Port d'ecoute : `NEXUS_API_PORT` (defaut 3100).

mod adapters;
mod bootstrap;

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("NEXUS_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3100);

    let app = Router::new().route("/", get(|| async { "nexus-api: hello" }));

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("nexus-api scaffold en ecoute sur {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind nexus-api");
    axum::serve(listener, app).await.expect("serve nexus-api");
}
