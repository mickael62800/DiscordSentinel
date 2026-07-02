//! Tick periodique du Tamagotchi.
//!
//! Delegue tout le travail a l'API (`POST /api/tamagotchi/tick`) qui applique
//! la decroissance des jauges + les transitions maladie/mort (logique core)
//! avec la config de chaque guild. Le worker ne fait que declencher.

use sqlx::PgPool;
use tracing::info;

use crate::common::api;

#[derive(serde::Deserialize)]
struct TickSummary {
    processed: usize,
    sick: usize,
    died: usize,
    recovered: usize,
    #[serde(default)]
    channels_reclaimed: usize,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let summary: TickSummary = api::post_empty("/api/tamagotchi/tick").await?;
    if summary.processed > 0 || summary.channels_reclaimed > 0 {
        info!(
            processed = summary.processed,
            sick = summary.sick,
            died = summary.died,
            recovered = summary.recovered,
            channels_reclaimed = summary.channels_reclaimed,
            "Tick tamagotchi"
        );
    }
    Ok(())
}
