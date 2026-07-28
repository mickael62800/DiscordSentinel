//! # nexus-gateway — gateway de la plateforme jeux Nexus
//!
//! Scaffold minimal, calque sur le role de `sentinel-gateway` : point
//! d'entree reseau/routage entre les composants Nexus. Logique a venir.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("nexus-gateway scaffold — rien a router pour l'instant");
}
