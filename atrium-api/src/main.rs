use atrium_api::{router, AppConfig};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    platform_common_api::metrics::init_prometheus();

    let config = AppConfig::from_env().expect("Configuration Atrium API invalide");
    atrium_api::run_migrations(&config)
        .await
        .expect("Erreur lors des migrations Atrium");
    let rag = atrium_api::rag::service(&config).expect("Configuration RAG Atrium invalide");
    let budget = std::sync::Arc::new(
        atrium_api::budget::BudgetGuard::new(&config)
            .expect("Configuration du budget Atrium invalide"),
    );
    rag.index_knowledge()
        .await
        .expect("Erreur lors de l'indexation RAG Atrium");
    let addr = config.bind_addr;
    tokio::spawn(atrium_api::grpc::serve(
        config.clone(),
        rag.clone(),
        budget.clone(),
    ));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Impossible de binder Atrium API");
    tracing::info!(%addr, "Atrium API demarree");
    axum::serve(listener, router(config, rag, budget))
        .await
        .expect("Erreur serveur Atrium API");
}
