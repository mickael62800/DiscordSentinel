//! Cree des propositions de ban pour les users a 0 points de conduite.
//!
//! Toute la regle (find users + filter + insert infraction) vit cote API
//! (`ManageConductUseCase::sync_ban_proposals`). Le worker ne fait que
//! planifier l'appel HTTP.

use sqlx::PgPool;
use tracing::{debug, info, warn};

use sentinel_worker_common::api;

#[derive(serde::Deserialize)]
struct SyncResp {
    created: u64,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    match api::post_empty::<SyncResp>("/api/conduct/sync-ban-proposals").await {
        Ok(r) if r.created > 0 => {
            info!(
                count = r.created,
                "Propositions de ban creees pour utilisateurs a 0 points"
            );
            Ok(())
        }
        Ok(_) => {
            debug!("Aucun utilisateur a 0 points sans proposition de ban");
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "sync_ban_proposals: appel API echoue");
            Err(e)
        }
    }
}
