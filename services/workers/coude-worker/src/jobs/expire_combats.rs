//! Expiration batch des combats pending.
//!
//! Phase 4 refacto : thin. Toute la logique metier (claim atomique, debit
//! penalite, increment cowardice, refund paris) vit dans l'API via
//! `ExpireCombatsBatchUseCase` exposee par le RPC
//! `CoudeCombatsService.ExpireCombatsBatch`.

use sqlx::PgPool;
use tonic::Request;
use tracing::{error, info, warn};

use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::Empty as ProtoEmpty;
use sentinel_worker_common::grpc;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let channel = grpc::connect().await?;
    let interceptor = grpc::bearer_interceptor()?;
    let mut client = CoudeCombatsServiceClient::with_interceptor(channel, interceptor);

    match client.expire_combats_batch(Request::new(ProtoEmpty {})).await {
        Ok(resp) => {
            let combats = resp.into_inner().combats;
            if !combats.is_empty() {
                info!(count = combats.len(), "Combats expires resolus par l'API");
                for c in &combats {
                    warn!(
                        combat_id = %c.combat_id,
                        defender = %c.defender_name,
                        penalty = c.penalty,
                        "Combat expire (timeout defenseur)"
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Echec appel ExpireCombatsBatch gRPC");
            Err(format!("expire_combats_batch RPC: {e}"))
        }
    }
}
