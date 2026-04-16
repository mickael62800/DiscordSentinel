use rand::Rng;
use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use sentinel_worker_common::spawn_periodic;

use crate::config::WorkerConfig;
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();
    let bot_token = config.discord_bot_token.clone();

    // Job 1 : expiration des combats pending
    spawn_periodic(
        "expire_combats",
        config.combat_expiry_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "coude-worker",
        |pool| Box::pin(async move { jobs::expire_combats::run(&pool).await }),
    );

    // Job 2 : resolution des combats betting
    {
        let api = api_url.clone();
        let token = bot_token.clone();
        spawn_periodic(
            "resolve_betting",
            config.betting_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "coude-worker",
            move |pool| {
                let api = api.clone();
                let token = token.clone();
                Box::pin(async move { jobs::resolve_betting::run(&pool, &api, &token).await })
            },
        );
    }

    // Job 3 : regen passive des HP (taux degressif par palier)
    {
        let api = api_url.clone();
        spawn_periodic(
            "hp_regen",
            config.hp_regen_tick_secs,
            pool.clone(),
            shutdown.clone(),
            api,
            "coude-worker",
            |pool| Box::pin(async move { jobs::hp_regen::run(&pool).await }),
        );
    }

    // Job 4 : redistribution hebdo des caisses communautaires (Phase 9).
    let min_days = config.cashbox_min_days as i64;
    spawn_periodic(
        "redistribute_cashbox",
        config.cashbox_tick_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "coude-worker",
        move |pool| {
            Box::pin(async move { jobs::redistribute_cashbox::run(&pool, min_days).await })
        },
    );

    // Job 5 : "Roue du Destin" — chaos journalier avec timer aleatoire.
    // Le worker dort un temps aleatoire entre 2h et 6h, puis appelle l'API
    // qui decide seule (cap 5/jour, joueurs, montant, transfert).
    {
        let api = api_url;
        let token = bot_token;
        let mut rx = shutdown;
        tokio::spawn(async move {
            loop {
                // Delai aleatoire 2-6h (7200..21600 secondes).
                let delay_secs = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(7200..=21600)
                };
                info!(delay_secs, "daily_chaos: prochain tick dans {}h{}",
                    delay_secs / 3600, (delay_secs % 3600) / 60);

                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)) => {}
                    _ = rx.changed() => {
                        if *rx.borrow() { return; }
                    }
                }

                if let Err(e) = jobs::daily_chaos::run(&pool, &api, &token).await {
                    tracing::error!(error = %e, "daily_chaos job failed");
                }
            }
        });
    }
}
