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

const USER_GUILDS_CACHE_TTL_SECS: u64 = 300;
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
fn extract_guild_id_from_path(path: &str) -> Option<String> {
    path.split('/').find_map(|seg| {
        if seg.len() >= 17 && seg.len() <= 20 && seg.chars().all(|c| c.is_ascii_digit()) {
            Some(seg.to_string())
        } else {
            None
        }
    })
}

async fn get_or_fetch_user_guilds(
    state: &AppState,
    access_token: &str,
) -> Result<HashSet<String>, String> {
    // Cle de cache : hash du token (eviter de stocker le token en clair).
    let cache_key = format!("user_guilds:{}", short_hash(access_token));

    // Tenter le cache Redis
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
            if let Ok(set) = serde_json::from_str::<HashSet<String>>(&cached) {
                return Ok(set);
            }
        }
    }

    // Fallback Discord API
    let guilds = state
        .discord_api
        .get_user_guilds(access_token)
        .await
        .map_err(|e| format!("Discord API: {e}"))?;
    let set: HashSet<String> = guilds.into_iter().map(|g| g.id).collect();

    // Cacher le resultat (best-effort)
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(serialized) = serde_json::to_string(&set) {
            let _: Result<(), _> = conn
                .set_ex(&cache_key, serialized, USER_GUILDS_CACHE_TTL_SECS)
                .await;
        }
    }

    Ok(set)
}

/// Hash court non-cryptographique pour deriver une cle de cache du token.
/// On veut juste eviter de stocker le token complet en cle Redis.
fn short_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_guild_id_finds_snowflake_in_path() {
        assert_eq!(
            extract_guild_id_from_path("/api/coude/123456789012345678/players"),
            Some("123456789012345678".to_string())
        );
    }

    #[test]
    fn extract_guild_id_handles_no_guild() {
        assert_eq!(extract_guild_id_from_path("/api/health"), None);
        assert_eq!(extract_guild_id_from_path("/api/coude/guilds"), None);
    }

    #[test]
    fn extract_guild_id_ignores_short_segments() {
        // /api/v10/foo : aucun segment de 17-20 chiffres
        assert_eq!(extract_guild_id_from_path("/api/v10/foo"), None);
    }

    #[test]
    fn extract_guild_id_ignores_uuid_segments() {
        // UUID = 36 chars avec tirets, ne match pas le filtre
        assert_eq!(
            extract_guild_id_from_path("/api/coude/abcd-1234-ef56/x"),
            None
        );
    }

    #[test]
    fn short_hash_is_stable() {
        assert_eq!(short_hash("token123"), short_hash("token123"));
        assert_ne!(short_hash("token123"), short_hash("token124"));
    }
}
