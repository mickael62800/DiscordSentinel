//! Expiration batch des combats pending.
//!
//! Phase 4 refacto : thin. Toute la logique metier (claim atomique, debit
//! penalite, increment cowardice, refund paris) vit dans l'API via
//! `ExpireCombatsBatchUseCase` exposee par le RPC
//! `CoudeCombatsService.ExpireCombatsBatch`.

use sqlx::PgPool;
use tonic::transport::{Channel, Endpoint};
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{error, info, warn};

use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::Empty as ProtoEmpty;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let url = std::env::var("GRPC_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    let channel: Channel = Endpoint::from_shared(url.clone())
        .map_err(|e| format!("invalid GRPC_API_URL {url}: {e}"))?
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30))
        .connect()
        .await
        .map_err(|e| format!("connect gRPC {url}: {e}"))?;

    let auth: MetadataValue<_> = format!("Bearer {api_key}")
        .parse()
        .map_err(|e| format!("invalid api_key: {e}"))?;

    let mut client = CoudeCombatsServiceClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", auth.clone());
            Ok(req)
        },
    );

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
