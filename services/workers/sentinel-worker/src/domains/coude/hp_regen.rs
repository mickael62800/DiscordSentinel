//! Tick de regen passive des HP.
//!
//! Phase 4 refacto : thin. La logique SQL vit dans l'API
//! (`PlayerRepository::regen_hp_tick` + RPC `HpRegenTick`).
//! Le worker ne fait que lire les taux depuis l'env et appeler le RPC.

use sqlx::PgPool;
use tonic::Request;
use tracing::{debug, error, warn};

use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;
use sentinel_proto::coude::v1::HpRegenTickRequest;
use sentinel_worker_common::grpc;

const DEFAULT_RATE_0_25: f64 = 100.0;
const DEFAULT_RATE_25_50: f64 = 50.0;
const DEFAULT_RATE_50_75: f64 = 30.0;
const DEFAULT_RATE_75_100: f64 = 10.0;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let rate_0_25 = env_rate("HP_REGEN_RATE_0_25", DEFAULT_RATE_0_25);
    let rate_25_50 = env_rate("HP_REGEN_RATE_25_50", DEFAULT_RATE_25_50);
    let rate_50_75 = env_rate("HP_REGEN_RATE_50_75", DEFAULT_RATE_50_75);
    let rate_75_100 = env_rate("HP_REGEN_RATE_75_100", DEFAULT_RATE_75_100);

    let channel = grpc::connect().await.map_err(|e| {
        error!(error = %e, "hp_regen: echec connect gRPC");
        e
    })?;
    let interceptor = grpc::bearer_interceptor()?;
    let mut client = CoudePlayerServiceClient::with_interceptor(channel, interceptor);

    match client
        .hp_regen_tick(Request::new(HpRegenTickRequest {
            rate_0_25,
            rate_25_50,
            rate_50_75,
            rate_75_100,
        }))
        .await
    {
        Ok(resp) => {
            let updated = resp.into_inner().updated;
            if updated > 0 {
                debug!(updated, "hp_regen tick OK");
            }
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "hp_regen: echec RPC");
            Err(format!("hp_regen_tick RPC: {e}"))
        }
    }
}

fn env_rate(key: &str, default: f64) -> f64 {
    match std::env::var(key) {
        Ok(v) => v.parse::<f64>().unwrap_or(default),
        Err(_) => default,
    }
}
