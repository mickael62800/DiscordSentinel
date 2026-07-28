//! # nexus-worker — worker de fond de la plateforme jeux Nexus
//!
//! Scaffold tokio minimal, calque sur `sentinel-worker` : executera les jobs
//! asynchrones de Nexus via les ports de `nexus-core`. Pour l'instant :
//! attend le signal d'arret (Ctrl+C) puis se termine proprement.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("nexus-worker scaffold demarre — en attente du signal d'arret");

    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("attente du signal impossible: {e}");
        return;
    }
    tracing::info!("signal recu — arret de nexus-worker");
}
