//! Tick de regen passive des HP.
//!
//! Phase 4 refacto : thin. La logique SQL vit dans l'API
//! (`CoudePlayerRepository::regen_hp_tick` + RPC `HpRegenTick`).
//! Le worker ne fait que lire les taux depuis l'env et appeler le RPC.

use sqlx::PgPool;
use tonic::transport::{Channel, Endpoint};
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, error, warn};

use sentinel_proto::coude::v1::coude_player_service_client::CoudePlayerServiceClient;
use sentinel_proto::coude::v1::HpRegenTickRequest;

const DEFAULT_RATE_0_25: f64 = 100.0;
const DEFAULT_RATE_25_50: f64 = 50.0;
const DEFAULT_RATE_50_75: f64 = 30.0;
const DEFAULT_RATE_75_100: f64 = 10.0;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let rate_0_25 = env_rate("HP_REGEN_RATE_0_25", DEFAULT_RATE_0_25);
    let rate_25_50 = env_rate("HP_REGEN_RATE_25_50", DEFAULT_RATE_25_50);
    let rate_50_75 = env_rate("HP_REGEN_RATE_50_75", DEFAULT_RATE_50_75);
    let rate_75_100 = env_rate("HP_REGEN_RATE_75_100", DEFAULT_RATE_75_100);

    let url = std::env::var("GRPC_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    let channel: Channel = match Endpoint::from_shared(url.clone())
        .and_then(|e| Ok(e.connect_timeout(std::time::Duration::from_secs(5))))
    {
        Ok(e) => match e.connect().await {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "hp_regen: echec connect gRPC");
                return Err(format!("connect gRPC: {e}"));
            }
        },
        Err(e) => {
            error!(error = %e, "hp_regen: invalid GRPC_API_URL");
            return Err(format!("invalid GRPC_API_URL: {e}"));
        }
    };

    let auth: MetadataValue<_> = format!("Bearer {api_key}")
        .parse()
        .map_err(|e| format!("invalid api_key: {e}"))?;

    let mut client = CoudePlayerServiceClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", auth.clone());
            Ok(req)
        },
    );

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
