//! Resolution batch des combats Coup de Coude en phase betting.
//!
//! Phase 3 refacto : ce job est devenu ~thin. Toute la logique metier
//! (claim atomique, moteur de combat, wallet, stats, paris, assurance,
//! vol chaos, explosion) vit maintenant dans l'API via le use case
//! `ResolveBettingBatchUseCase` exposee par le RPC gRPC
//! `CoudeCombatsService.ResolveBettingBatch`.
//!
//! Le worker ne fait plus que :
//!   1. Un appel gRPC `resolve_betting_batch()` a l'API
//!   2. Un loop pour poster les resultats sur Discord
//!
//! Avant Phase 3 : 683 lignes de SQL + combat_engine duplique.
//! Apres Phase 3 : ~130 lignes de IO pur (gRPC client + Discord API).

use sqlx::PgPool;
use tonic::transport::{Channel, Endpoint};
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{error, info, warn};

use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::{Empty as ProtoEmpty, ResolvedBettingCombat};

/// Signature conservee pour ne pas casser scheduler.rs. `pool` n'est plus
/// utilise mais on le garde pour compatibilite avec le trait periodique.
pub async fn run(_pool: &PgPool, _api_url: &str, bot_token: &str) -> Result<(), String> {
    if bot_token.is_empty() {
        warn!("Pas de token Discord configure, skip resolve_betting tick");
        return Ok(());
    }

    // Appel gRPC a l'API pour resoudre le batch.
    let combats = match call_resolve_batch().await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Echec appel ResolveBettingBatch gRPC");
            return Err(format!("gRPC ResolveBettingBatch: {e}"));
        }
    };

    if combats.is_empty() {
        return Ok(());
    }

    info!(count = combats.len(), "Combats resolus par l'API, post sur Discord");

    // Post des resultats sur Discord (seul IO conserve cote worker).
    for combat in combats {
        let channel_id = match combat.channel_id.as_deref() {
            Some(c) if !c.is_empty() => c,
            _ => {
                warn!(combat_id = %combat.combat_id, "Pas de channel_id, skip Discord post");
                continue;
            }
        };
        post_result_to_discord(
            bot_token,
            channel_id,
            combat.message_id.as_deref(),
            &combat.result_message,
            combat.is_draw,
        )
        .await;
    }

    Ok(())
}

async fn call_resolve_batch() -> Result<Vec<ResolvedBettingCombat>, String> {
    let url = std::env::var("GRPC_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    let channel: Channel = Endpoint::from_shared(url.clone())
        .map_err(|e| format!("invalid GRPC_API_URL {url}: {e}"))?
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(60))
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

    let resp = client
        .resolve_betting_batch(Request::new(ProtoEmpty {}))
        .await
        .map_err(|e| format!("resolve_betting_batch RPC: {e}"))?;

    Ok(resp.into_inner().combats)
}

async fn post_result_to_discord(
    bot_token: &str,
    channel_id: &str,
    message_id: Option<&str>,
    content: &str,
    is_draw: bool,
) {
    let color = if is_draw { 0x9B59B6 } else { 0x57F287 };
    let title = "⚔️ Résultat du Coup de Coude !";

    let client = reqwest::Client::new();

    // Editer le message existant si on a le message_id (challenge original).
    if let Some(mid) = message_id {
        let url = format!("https://discord.com/api/v10/channels/{}/messages/{}", channel_id, mid);
        let resp = client
            .patch(&url)
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&serde_json::json!({
                "embeds": [{
                    "title": title,
                    "description": content,
                    "color": color
                }],
                "components": []
            }))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => return,
            Ok(r) => warn!("Discord edit message failed: {}", r.status()),
            Err(e) => warn!("Discord edit request failed: {e}"),
        }
    }

    // Fallback : poster un nouveau message
    let url = format!("https://discord.com/api/v10/channels/{}/messages", channel_id);
    if let Err(e) = client
        .post(&url)
        .header("Authorization", format!("Bot {}", bot_token))
        .json(&serde_json::json!({
            "embeds": [{
                "title": title,
                "description": content,
                "color": color
            }]
        }))
        .send()
        .await
    {
        warn!(error = %e, channel_id, "Echec post resultat combat Discord (fallback)");
    }
}
