//! Phase 2 B — Middleware multi-tenant : filtre les requetes API par
//! appartenance Discord du user appelant.
//!
//! Principe :
//!   1. Le desktop fait OAuth2 Discord (scope `identify` + `guilds`) et
//!      obtient un `access_token`. Il l'envoie dans le header `X-Discord-Token`
//!      en plus du `Authorization: Bearer <api_key>` deja gere par auth_middleware.
//!   2. Ce middleware extrait le `guild_id` depuis l'URI (paths `/.../{guild_id}/...`).
//!   3. Il verifie que le user a bien acces a cette guild :
//!        - cache Redis `user_guilds:<token_hash>` (TTL 5 min)
//!        - sinon appel Discord `GET /users/@me/guilds` avec l'access_token
//!   4. Refuse avec 403 si le guild n'est pas dans la liste autorisee.
//!
//! Comportements speciaux :
//!   - Si `X-Discord-Token` est absent → on passe a travers (appel bot/internal).
//!     L'auth_middleware a deja valide la cle API, donc c'est safe.
//!   - Si l'URI ne contient pas de guild_id → on passe a travers (endpoint global).
//!   - Si Discord API echoue → 503 (fail closed pour les utilisateurs).

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::collections::HashSet;

use crate::adapters::inbound::http::state::AppState;

const USER_GUILDS_CACHE_TTL_SECS: u64 = 3600; // 1 h : cache "live"
const USER_GUILDS_STALE_TTL_SECS: u64 = 86_400; // 24 h : fallback stale en cas de 429 Discord
const DISCORD_TOKEN_HEADER: &str = "x-discord-token";

pub async fn guild_auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Recuperer le token Discord. Absent ⇒ pas un appel desktop ⇒ pass-through.
    let discord_token = match request
        .headers()
        .get(DISCORD_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return Ok(next.run(request).await),
    };

    // 2. Extraire le guild_id depuis l'URI. Si absent ⇒ endpoint global ⇒ pass.
    let path = request.uri().path();
    let guild_id = match extract_guild_id_from_path(path) {
        Some(g) => g,
        None => return Ok(next.run(request).await),
    };

    // 3. Recuperer la liste des guilds autorisees pour ce token (cache + fallback).
    let allowed = match get_or_fetch_user_guilds(&state, &discord_token).await {
        Ok(set) => set,
        Err(e) => {
            tracing::warn!(error = %e, "guild_auth: impossible de recuperer les guilds");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    if !allowed.contains(&guild_id) {
        tracing::info!(
            guild_id = %guild_id,
            path = %path,
            "guild_auth: acces refuse (guild non autorisee)"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Cherche `{guild_id}` dans le path. Heuristique : on prend le premier
/// segment qui ressemble a un Discord snowflake (uniquement des chiffres,
/// 17-20 caracteres).
///
/// Skip les segments precedes d'un mot-cle qui designe un autre type de
/// snowflake (channel_id, user_id, message_id...). Sans ca, une URL comme
/// `/api/voice-channels/by-channel/{channel_id}` faisait passer le
/// channel_id pour un guild_id et faisait echouer le guild_auth en 403.
fn extract_guild_id_from_path(path: &str) -> Option<String> {
    // Mots-cles qui designent un autre type de snowflake que guild_id
    // (channel_id, user_id, message_id, link_id...). Le snowflake suivant
    // doit etre ignore. Liste minimale et conservative : on n'inclut que
    // les prefixes qui ne sont JAMAIS suivis d'un guild_id dans nos routes.
    const NON_GUILD_PREFIXES: &[&str] = &[
        "by-channel",
        "by-message",
        "by-user",
        "by-role",
        "co-admins",
        "bans",
        "user",
        "users",
    ];
    let segments: Vec<&str> = path.split('/').collect();
    for (i, seg) in segments.iter().enumerate() {
        if seg.len() >= 17 && seg.len() <= 20 && seg.chars().all(|c| c.is_ascii_digit()) {
            let prev = if i > 0 { segments[i - 1] } else { "" };
            if NON_GUILD_PREFIXES.contains(&prev) {
                continue;
            }
            return Some(seg.to_string());
        }
    }
    None
}

async fn get_or_fetch_user_guilds(
    state: &AppState,
    access_token: &str,
) -> Result<HashSet<String>, String> {
    // Cles de cache : hash du token (eviter de stocker le token en clair).
    // Deux entrees : live (1h, autoritative) + stale (24h, fallback en cas de
    // panne/rate-limit Discord).
    let key_hash = short_hash(access_token);
    let live_key = format!("user_guilds:{}", key_hash);
    let stale_key = format!("user_guilds_stale:{}", key_hash);

    // Tenter le cache live Redis
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, String>(&live_key).await {
            if let Ok(set) = serde_json::from_str::<HashSet<String>>(&cached) {
                return Ok(set);
            }
        }
    }

    // Fallback Discord API
    match state.discord_api.get_user_guilds(access_token).await {
        Ok(guilds) => {
            let set: HashSet<String> = guilds.into_iter().map(|g| g.id).collect();

            // Cacher live + stale (best-effort)
            if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
                if let Ok(serialized) = serde_json::to_string(&set) {
                    let _: Result<(), _> = conn
                        .set_ex(&live_key, serialized.clone(), USER_GUILDS_CACHE_TTL_SECS)
                        .await;
                    let _: Result<(), _> = conn
                        .set_ex(&stale_key, serialized, USER_GUILDS_STALE_TTL_SECS)
                        .await;
                }
            }

            Ok(set)
        }
        Err(e) => {
            // Discord en panne / rate-limit : tenter le cache stale.
            if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
                if let Ok(cached) = conn.get::<_, String>(&stale_key).await {
                    if let Ok(set) = serde_json::from_str::<HashSet<String>>(&cached) {
                        tracing::warn!(
                            error = %e,
                            "guild_auth: Discord indisponible, fallback sur cache stale"
                        );
                        return Ok(set);
                    }
                }
            }
            Err(format!("Discord API: {e}"))
        }
    }
}

/// Hash court non-cryptographique pour deriver une cle de cache du token.
/// On veut juste eviter de stocker le token complet en cle Redis.
fn short_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}


#[cfg(test)]
#[path = "tests/guild_auth.rs"]
mod tests;
