//! Scheduler central : enregistre tous les jobs periodiques avec leur
//! intervalle et delegue l'execution a `spawn_periodic` (impl commune
//! qui gere shutdown, panic catch, log lifecycle, metrics).
//!
//! Lecture de ce fichier = inventaire complet de ce que fait le worker.
//! Ajouter un job = ajouter une section ici + creer le module dans
//! `domains/{domain}/{job}.rs`.

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use sentinel_worker_common::spawn_periodic;

use crate::config::{CleanupConfig, WorkerConfig};
use crate::domains;

const WORKER_NAME: &str = "sentinel-worker";

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();

    // ─────────────────────────────────────────────────────────────
    // Domaine : cleanup (porte de l'ancien cleanup-worker)
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = CleanupConfig::from(config);
        spawn_periodic(
            "cleanup_old_data",
            config.cleanup_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            WORKER_NAME,
            move |pool| {
                let cfg = cfg.clone();
                Box::pin(async move { domains::cleanup::cleanup_old_data::run(&pool, &cfg).await })
            },
        );

        if config.vacuum_enabled {
            spawn_periodic(
                "vacuum_tables",
                config.vacuum_interval_secs,
                pool.clone(),
                shutdown.clone(),
                api_url.clone(),
                WORKER_NAME,
                |pool| Box::pin(async move { domains::cleanup::vacuum_tables::run(&pool).await }),
            );
        } else {
            info!("VACUUM desactive par configuration");
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Phases suivantes : ajouter ici les autres domaines au fur et a
    // mesure de leur migration depuis les anciens workers.
    // ─────────────────────────────────────────────────────────────
}
