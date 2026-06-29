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
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info, warn};

use sentinel_proto::coude::v1::coude_combats_service_client::CoudeCombatsServiceClient;
use sentinel_proto::coude::v1::{Empty as ProtoEmpty, ResolvedBettingCombat, TauntEvent};

/// Limite Discord sur le nickname (32 chars).
const DISCORD_NICKNAME_MAX: usize = 32;

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

    info!(
        count = combats.len(),
        "Combats resolus par l'API, post sur Discord"
    );

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

        // Phase 9 Part D — dispatch des taunts emis par l'API. Le worker
        // ne contient toujours aucune logique metier : l'API a deja cuisine
        // le channel_id, le message et le nickname_suffix. On se contente
        // de poster + renommer via l'API REST Discord brute.
        for ev in &combat.taunt_events {
            dispatch_taunt_event(bot_token, &combat.guild_id, ev).await;
        }
    }

    Ok(())
}

async fn call_resolve_batch() -> Result<Vec<ResolvedBettingCombat>, String> {
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    // Delegue a crate::common::grpc::connect() pour beneficier
    // du mTLS optionnel (GRPC_TLS_DIR) en coherence avec les autres callers.
    let channel: Channel = crate::common::grpc::connect().await?;

    let auth: MetadataValue<_> = format!("Bearer {api_key}")
        .parse()
        .map_err(|e| format!("invalid api_key: {e}"))?;

    let mut client =
        CoudeCombatsServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", auth.clone());
            Ok(req)
        });

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
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages/{}",
            channel_id, mid
        );
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
    let url = format!(
        "https://discord.com/api/v10/channels/{}/messages",
        channel_id
    );
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

/// Phase 9 Part D — Dispatche un TauntEvent : post du message + rename
/// du pseudo. Zero logique metier : tout (channel_id, message, suffixe)
/// est deja cuisine par l'API cote resolve_betting_batch_service.
async fn dispatch_taunt_event(bot_token: &str, guild_id: &str, ev: &TauntEvent) {
    let client = reqwest::Client::new();

    // 1) Post du message de raillerie dans le salon dedie.
    let color: u32 = match ev.streak_kind.as_str() {
        "win" => 0xF1C40F,
        "loss" => 0xE74C3C,
        "steal_victim" => 0x9B59B6,
        _ => 0x95A5A6,
    };
    // Si messages_enabled=false cote API, ev.message est vide : skip le post.
    if !ev.message.is_empty() {
        let post_url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            ev.channel_id
        );
        if let Err(e) = client
            .post(&post_url)
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&serde_json::json!({
                "embeds": [{
                    "title": "🔥 Raillerie automatique",
                    "description": ev.message,
                    "color": color,
                    "footer": { "text": format!("Serie : {} × {}", ev.streak_kind, ev.streak_value) },
                }]
            }))
            .send()
            .await
        {
            warn!(error = %e, channel_id = %ev.channel_id, "Echec post taunt message (worker)");
        }
    }

    // Si rename_enabled=false cote API, nickname_suffix est vide : skip
    // toute la sequence fetch-member + patch-nick.
    if ev.nickname_suffix.is_empty() {
        return;
    }

    // 2) Rename : recupere le nickname courant, applique le suffixe en
    //    tronquant le base pour rester sous 32 chars. On lit d'abord le
    //    member pour connaitre son display_name actuel.
    let member_url = format!(
        "https://discord.com/api/v10/guilds/{}/members/{}",
        guild_id, ev.target_user_id
    );
    let current = match client
        .get(&member_url)
        .header("Authorization", format!("Bot {}", bot_token))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(json) => json
                .get("nick")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    json.get("user")
                        .and_then(|u| u.get("global_name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    json.get("user")
                        .and_then(|u| u.get("username"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }),
            Err(e) => {
                warn!(error = %e, "parse member JSON taunt");
                return;
            }
        },
        Ok(r) => {
            warn!(status = %r.status(), "GET member non-ok (taunt)");
            return;
        }
        Err(e) => {
            warn!(error = %e, "GET member failed (taunt)");
            return;
        }
    };
    let Some(current_name) = current else {
        warn!(user_id = %ev.target_user_id, "Pas de nom de base pour rename taunt");
        return;
    };

    // Idempotence : si deja suffixe, on ne refait rien.
    if current_name.ends_with(&ev.nickname_suffix) {
        return;
    }
    let suffix_len = ev.nickname_suffix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(suffix_len);
    let base: String = current_name.chars().take(max_base).collect();
    let new_nick = format!("{}{}", base, ev.nickname_suffix);

    let patch_url = format!(
        "https://discord.com/api/v10/guilds/{}/members/{}",
        guild_id, ev.target_user_id
    );
    if let Err(e) = client
        .patch(&patch_url)
        .header("Authorization", format!("Bot {}", bot_token))
        .json(&serde_json::json!({ "nick": new_nick }))
        .send()
        .await
    {
        warn!(error = %e, user_id = %ev.target_user_id, new_nick, "Echec rename member (taunt worker)");
    }
}
