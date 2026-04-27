//! Tick periodique de regen des points de conduite.
//!
//! Toute la regle metier vit cote API (`apply_conduct_regen` du domain
//! + `ManageConductUseCase::run_regen`). Le worker ne fait que planifier.

use sqlx::PgPool;
use tracing::{debug, info, warn};

use sentinel_worker_common::api;

#[derive(serde::Deserialize)]
struct RegenTickResp {
    regenerated: u64,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    match api::post_empty::<RegenTickResp>("/api/conduct/regen-tick").await {
        Ok(r) if r.regenerated > 0 => {
            info!(count = r.regenerated, "Points de conduite regeneres");
            Ok(())
        }
        Ok(_) => {
            debug!("Aucun point de conduite a regenerer");
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "conduct_regen: appel API echoue");
            Err(e)
        }
    }
}
