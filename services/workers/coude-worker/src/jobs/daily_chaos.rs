//! Job "Roue du Destin" — chaos journalier aleatoire.
//!
//! Le worker appelle `TriggerDailyChaos` pour chaque guild. L'API
//! decide de tout (cap journalier, joueurs, montant, transfert).
//! Le worker ne fait que poster l'embed dans Discord si un chaos
//! a ete declenche.

use sqlx::PgPool;
use tonic::transport::{Channel, Endpoint};
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{info, warn};

use sentinel_proto::coude::v1::coude_social_service_client::CoudeSocialServiceClient;
use sentinel_proto::coude::v1::TriggerDailyChaosRequest;

/// Appele par le scheduler. Itere sur toutes les guilds avec joueurs
/// coude actifs et tente un trigger chaos pour chacune.
pub async fn run(_pool: &PgPool, api_url: &str, bot_token: &str) -> Result<(), String> {
    if bot_token.is_empty() {
        warn!("Pas de token Discord configure, skip daily_chaos tick");
        return Ok(());
    }

    let channel = connect_grpc(api_url).await?;
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    let guild_ids = fetch_guild_ids(_pool).await?;

    for guild_id in &guild_ids {
        let mut client = CoudeSocialServiceClient::new(channel.clone());
        let mut req = Request::new(TriggerDailyChaosRequest {
            guild_id: guild_id.clone(),
        });
        if let Ok(v) = api_key.parse::<MetadataValue<_>>() {
            req.metadata_mut().insert("x-api-key", v);
        }

        let resp = match client.trigger_daily_chaos(req).await {
            Ok(r) => r.into_inner(),
            Err(e) => {
                warn!(guild_id, error = %e, "Erreur gRPC TriggerDailyChaos");
                continue;
            }
        };

        if !resp.triggered {
            continue;
        }

        info!(
            guild_id,
            loser = %resp.loser_id,
            winner = %resp.winner_id,
            amount = resp.amount,
            "Daily chaos triggered — posting to Discord"
        );

        post_chaos_embed(bot_token, &resp.channel_id, &resp).await;
    }

    Ok(())
}

async fn connect_grpc(api_url: &str) -> Result<Channel, String> {
    let grpc_url = std::env::var("GRPC_API_URL")
        .unwrap_or_else(|_| api_url.replace("http://", "http://").to_string());
    let endpoint = Endpoint::from_shared(grpc_url.clone())
        .map_err(|e| format!("bad grpc endpoint: {e}"))?;
    endpoint
        .connect()
        .await
        .map_err(|e| format!("gRPC connect failed ({}): {e}", grpc_url))
}

async fn fetch_guild_ids(pool: &PgPool) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT guild_id FROM coude_players")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("fetch guild_ids: {e}"))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn post_chaos_embed(
    bot_token: &str,
    channel_id: &str,
    resp: &sentinel_proto::coude::v1::DailyChaosResponse,
) {
    let client = reqwest::Client::new();
    let embed = serde_json::json!({
        "title": "\u{1f32a}\u{fe0f} LA ROUE DU DESTIN A TOURNE !",
        "description": format!(
            "\u{1f480} <@{}> perd **{} coins** (-20%)\n\u{1f381} <@{}> gagne **{} coins** gratuitement !\n\nLa vie est injuste. Coup de Coude aussi.",
            resp.loser_id, resp.amount, resp.winner_id, resp.amount
        ),
        "color": 0x9B59B6,
        "footer": { "text": "Coup de Coude | Sentinel" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let body = serde_json::json!({ "embeds": [embed] });
    let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");

    if let Err(e) = client
        .post(&url)
        .header("Authorization", format!("Bot {bot_token}"))
        .json(&body)
        .send()
        .await
    {
        warn!(error = %e, channel_id, "Echec post chaos embed Discord");
    }
}
