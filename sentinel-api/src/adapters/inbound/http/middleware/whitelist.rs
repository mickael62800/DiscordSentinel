//! Phase securite — Defense en profondeur sur l'auth des users Discord.
//!
//! Probleme : `auth_middleware` accepte n'importe quel `X-Discord-Token`
//! non vide. Du coup, un user Discord random (jamais invite, pas dans
//! `api_user_guilds`, pas superadmin) peut hit tous les endpoints qui n'ont
//! PAS de `guild_id` dans leur path : `/api/guilds`, `/api/docker/*`,
//! `/api/security/*`, `/api/system/info`, etc.
//!
//! `guild_auth_middleware` ne couvre que les endpoints ayant un guild_id
//! dans le path. Et `rbac_middleware` n'echoue jamais sur l'absence de
//! whitelist (il default a `Viewer` par principe du moindre privilege).
//!
//! Ce middleware comble ce trou : si l'appel vient d'un user Discord
//! (`X-Discord-Token` present), on exige qu'il soit :
//!   - dans `SUPERADMIN_USER_IDS` (env var), OU
//!   - present dans au moins une row de `api_user_guilds`.
//!
//! Sinon -> 403 Forbidden, peu importe le path demande.
//!
//! # Exemptions
//!
//! Les endpoints suivants sont exemptes (un user pas encore whitelist
//! doit pouvoir les appeler pour S'AUTORISER lui-meme via un code) :
//!   - `/api/auth/check-access` — sondage cote front pour decider du UX
//!   - `/api/auth/redeem-invitation` — consomme un code invitation
//!
//! # Bots/internes
//!
//! Si l'appel utilise une `Authorization: Bearer <api_key>` (sans
//! `X-Discord-Token`), `auth_middleware` a deja valide. Ce middleware
//! pass-through dans ce cas (pas de token Discord -> rien a verifier).

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;

const WHITELIST_CACHE_TTL_SECS: u64 = 300; // 5 min : compromis fraicheur / DB load
const WHITELIST_CACHE_PREFIX: &str = "user_whitelisted:";

const EXEMPT_PATHS: &[&str] = &["/api/auth/check-access", "/api/auth/redeem-invitation"];

pub async fn whitelist_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Pas de RoleContext = pas d'appel desktop (auth via API key Bearer
    //    deja validee par auth_middleware) -> pass-through.
    let Some(ctx) = request.extensions().get::<RoleContext>().cloned() else {
        return Ok(next.run(request).await);
    };

    // 2. Endpoint exempt ? (le user doit pouvoir s'auto-autoriser).
    let path = request.uri().path();
    if EXEMPT_PATHS.iter().any(|p| path == *p) {
        return Ok(next.run(request).await);
    }

    // 3. Superadmin -> autorise sans question.
    if state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id)
    {
        return Ok(next.run(request).await);
    }

    // 4. Cache Redis : evite un round-trip DB sur chaque requete.
    let cache_key = format!("{}{}", WHITELIST_CACHE_PREFIX, ctx.discord_user_id);
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
            if cached == "1" {
                return Ok(next.run(request).await);
            }
            if cached == "0" {
                tracing::warn!(
                    user_id = %ctx.discord_user_id,
                    path = %path,
                    "whitelist: acces refuse (cache: pas dans api_user_guilds)"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // 5. Lookup DB : EXISTS au moins une row dans api_user_guilds.
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM api_user_guilds WHERE discord_user_id = $1)",
    )
    .bind(&ctx.discord_user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "whitelist: lookup DB error -> 503");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Cache du resultat (best-effort).
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let val = if exists { "1" } else { "0" };
        let _: Result<(), _> = conn.set_ex(&cache_key, val, WHITELIST_CACHE_TTL_SECS).await;
    }

    if !exists {
        tracing::warn!(
            user_id = %ctx.discord_user_id,
            path = %path,
            "whitelist: acces refuse (pas dans api_user_guilds, pas superadmin)"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
