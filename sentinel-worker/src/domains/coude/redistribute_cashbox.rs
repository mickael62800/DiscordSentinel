//! Job worker : redistribue les caisses communautaires eligibles.
//!
//! Phase 9. Thin : toute la logique (list guilds dues, claim atomique, tirage
//! aleatoire, credit des gagnants, audit) vit cote API via
//! `CoudeSocialService.RedistributeDueCashboxes`. Le worker ne fait que tick
//! regulierement (1h par defaut) et l'API filtre elle-meme les guilds
//! dont la derniere redistribution date de plus de `min_days_since_last`
//! jours — un tick frequent ne cause aucune sur-redistribution.

use sqlx::PgPool;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info};

use sentinel_proto::coude::v1::coude_social_service_client::CoudeSocialServiceClient;
use sentinel_proto::coude::v1::RedistributeDueRequest;

// Intercepteur tonic : Result<_, tonic::Status> impose un Err volumineux.
#[allow(clippy::result_large_err)]
pub async fn run(_pool: &PgPool, min_days: i64) -> Result<(), String> {
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    // Delegue a crate::common::grpc::connect() pour beneficier
    // du mTLS optionnel (GRPC_TLS_DIR) en coherence avec les autres callers.
    let channel: Channel = crate::common::grpc::connect().await?;

    let auth: MetadataValue<_> = format!("Bearer {api_key}")
        .parse()
        .map_err(|e| format!("invalid api_key: {e}"))?;

    let mut client =
        CoudeSocialServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", auth.clone());
            Ok(req)
        });

    let req = RedistributeDueRequest {
        min_days_since_last: min_days,
    };

    match client.redistribute_due_cashboxes(Request::new(req)).await {
        Ok(resp) => {
            let results = resp.into_inner().redistributed;
            if results.is_empty() {
                // Pas de bruit si aucune guild n'etait due.
                return Ok(());
            }
            for r in &results {
                info!(
                    guild_id = %r.guild_id,
                    total = r.total_amount,
                    winners = r.winners.len(),
                    "Cashbox redistributee"
                );
                for w in &r.winners {
                    info!(
                        guild_id = %r.guild_id,
                        user_id = %w.user_id,
                        username = %w.username,
                        amount = w.amount_won,
                        "Cashbox winner"
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Echec appel RedistributeDueCashboxes gRPC");
            Err(format!("redistribute_due_cashboxes RPC: {e}"))
        }
    }
}
