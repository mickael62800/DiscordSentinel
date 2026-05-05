//! Job "Roue du Destin" — chaos journalier aleatoire.
//!
//! Le worker appelle `TriggerDailyChaos` pour chaque guild. L'API
//! decide de tout (cap journalier, joueurs, montant, transfert,
//! detection taunts faillite/jackpot).
//!
//! Le worker :
//! 1. poste l'embed dans Discord si un chaos a ete declenche ;
//! 2. Migration #5 : XADD `daily_chaos_taunts` sur `sentinel:events`
//!    avec les TauntEvents renvoyes par l'API, que le bot consomme via
//!    `taunts_dispatch` (meme pattern que `tournament_resolved`).

use sqlx::PgPool;
use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{info, warn};

use sentinel_proto::coude::v1::coude_social_service_client::CoudeSocialServiceClient;
use sentinel_proto::coude::v1::TriggerDailyChaosRequest;

const STREAM_KEY: &str = "sentinel:events";
const PAYLOAD_FIELD: &str = "payload";
const STREAM_MAXLEN: i64 = 10_000;

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

    // Redis client pour publier les taunts sur `sentinel:events`
    // (Migration #5). Best-effort : si Redis est indispo, on skip
    // juste le XADD (l'embed Discord reste poste).
    let redis_url = std::env::var("REDIS_URL").unwrap_or_default();
    let redis_client = if redis_url.is_empty() {
        None
    } else {
        redis::Client::open(redis_url).ok()
    };

    for guild_id in &guild_ids {
        // Sub-feature gate : chaos_enabled (top-level) +
        // daily_chaos_enabled (sub-toggle). Default true pour les deux.
        if !crate::common::is_feature_enabled(_pool, guild_id, "coude-bot", "chaos_enabled", true).await {
            continue;
        }
        if !crate::common::is_feature_enabled(_pool, guild_id, "coude-bot", "daily_chaos_enabled", true).await {
            continue;
        }

        let mut client = CoudeSocialServiceClient::new(channel.clone());
        let mut req = Request::new(TriggerDailyChaosRequest {
            guild_id: guild_id.clone(),
        });
        // API_middleware attend Authorization: Bearer <key> (pattern utilise
        // par tous les autres workers gRPC : resolve_betting, hp_regen, etc.)
        if let Ok(v) = format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
            req.metadata_mut().insert("authorization", v);
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
            taunts = resp.taunt_events.len(),
            "Daily chaos triggered — posting to Discord"
        );

        post_chaos_embed(bot_token, &resp.channel_id, &resp).await;

        // Publie les taunts (faillite loser, jackpot winner) sur la
        // stream `sentinel:events`. Le bot consomme via taunts_dispatch.
        if !resp.taunt_events.is_empty() {
            if let Some(client) = &redis_client {
                publish_taunts(client, guild_id, &resp.taunt_events).await;
            } else {
                warn!(
                    guild_id,
                    "Redis indispo, daily_chaos_taunts non publies"
                );
            }
        }
    }

    Ok(())
}

async fn publish_taunts(
    client: &redis::Client,
    guild_id: &str,
    taunts: &[sentinel_proto::coude::v1::TauntEvent],
) {
    let taunts_json: Vec<serde_json::Value> = taunts
        .iter()
        .map(|t| {
            serde_json::json!({
                "channel_id": t.channel_id,
                "target_user_id": t.target_user_id,
                "message": t.message,
                "nickname_suffix": t.nickname_suffix,
                "streak_kind": t.streak_kind,
                "streak_value": t.streak_value,
            })
        })
        .collect();
    let event_payload = serde_json::json!({
        "event": "daily_chaos_taunts",
        "data": {
            "guild_id": guild_id,
            "taunts": taunts_json,
        }
    });

    match client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let res: redis::RedisResult<String> = redis::cmd("XADD")
                .arg(STREAM_KEY)
                .arg("MAXLEN")
                .arg("~")
                .arg(STREAM_MAXLEN)
                .arg("*")
                .arg(PAYLOAD_FIELD)
                .arg(event_payload.to_string())
                .query_async(&mut conn)
                .await;
            if let Err(e) = res {
                warn!(error = %e, guild_id, "XADD daily_chaos_taunts failed");
            } else {
                info!(guild_id, "daily_chaos_taunts event publie");
            }
        }
        Err(e) => warn!(error = %e, guild_id, "Redis connect failed, XADD skip"),
    }
}

async fn connect_grpc(_api_url: &str) -> Result<Channel, String> {
    // Delegue a crate::common::grpc::connect() pour beneficier du
    // mTLS optionnel (GRPC_TLS_DIR) en coherence avec les autres callers.
    crate::common::grpc::connect().await
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
