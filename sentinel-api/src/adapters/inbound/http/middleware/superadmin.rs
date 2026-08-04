//! Gate d'acces unique du back-office : **superadmin uniquement**.
//!
//! Remplace l'ancienne pile RBAC multi-roles (`rbac` + `whitelist` +
//! `guild_auth` + `global_rbac`) par une regle unique :
//!
//!   1. Dev mode (`API_KEY` vide)                    → pass-through.
//!   2. `AuthKind::Internal` (bot/workers, Bearer)   → pass-through.
//!   3. Utilisateur web : son identite Discord doit figurer dans
//!      `SUPERADMIN_USER_IDS` (.env)                 → sinon **403**.
//!
//! Il n'y a plus de roles applicatifs, plus de table `api_user_guilds`, plus
//! d'invitations, plus de gating par guild : le back-office a exactement un
//! utilisateur humain autorise (ou plusieurs si l'env en liste plusieurs).
//!
//! # Fail-closed
//!
//! Si `SUPERADMIN_USER_IDS` est vide, AUCUN utilisateur web ne passe. C'est
//! volontaire : mieux vaut un back-office inaccessible qu'un back-office
//! ouvert. Les services internes continuent de fonctionner via l'`API_KEY`.
//!
//! # Identite
//!
//! Le `discord_user_id` resolu est injecte en extension `WebUser`. Les
//! handlers qui attribuent une action a son auteur (audit, `deleted_by`,
//! `granted_by`...) le lisent via `Option<Extension<WebUser>>` — `None`
//! signifiant « appel interne bot/worker ».

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::adapters::inbound::http::middleware::auth::AuthKind;
use crate::adapters::inbound::http::state::AppState;

const USER_ID_CACHE_TTL_SECS: u64 = 600;
const DISCORD_TOKEN_HEADER: &str = "x-discord-token";

/// Identite Discord du caller web, injectee en extension de requete.
/// Absente pour les appels internes (bot/workers) et en dev mode.
#[derive(Debug, Clone)]
pub struct WebUser {
    pub discord_user_id: String,
}

pub async fn superadmin_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Dev mode : pas d'API_KEY configuree, on ne casse pas le local.
    if state.api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    // 2. Service interne de confiance (bot/workers via Bearer API_KEY).
    if request.extensions().get::<AuthKind>() == Some(&AuthKind::Internal) {
        return Ok(next.run(request).await);
    }

    // 3. Utilisateur web : on exige un token Discord exploitable.
    let (mut parts, body) = request.into_parts();
    let discord_token = match parts
        .headers
        .get(DISCORD_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        Some(t) if !t.is_empty() => t.to_string(),
        // Ni Bearer interne ni token web : `auth_middleware` a deja filtre ce
        // cas, on reste fail-closed par defense en profondeur.
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let user_id = match resolve_discord_user_id(&state, &discord_token).await {
        Ok(id) => id,
        Err(e) => {
            // Discord injoignable : on refuse plutot que de laisser passer.
            tracing::warn!(error = %e, "superadmin: resolution identite Discord impossible");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    if !state.superadmin_user_ids.iter().any(|id| id == &user_id) {
        tracing::warn!(
            user_id = %user_id,
            path = %parts.uri.path(),
            "superadmin: acces refuse (absent de SUPERADMIN_USER_IDS)"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    parts.extensions.insert(WebUser {
        discord_user_id: user_id,
    });

    Ok(next.run(Request::from_parts(parts, body)).await)
}

/// Resout le `discord_user_id` d'un access_token (cache Redis + fallback
/// `GET /users/@me`).
async fn resolve_discord_user_id(state: &AppState, access_token: &str) -> Result<String, String> {
    let cache_key = format!("user_id:{}", token_cache_key(access_token));

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
            if !cached.is_empty() {
                return Ok(cached);
            }
        }
    }

    let user = state
        .discord_api
        .get_user_me(access_token)
        .await
        .map_err(|e| format!("Discord API: {e}"))?;

    // Cache best-effort : une panne Redis ne doit pas bloquer l'acces.
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _: Result<(), _> = conn
            .set_ex(&cache_key, &user.id, USER_ID_CACHE_TTL_SECS)
            .await;
    }

    Ok(user.id)
}

/// Derive une cle de cache opaque a partir d'un access_token (jamais stocke en
/// clair). SHA-256 tronque a 128 bits : contrairement a un hash non
/// cryptographique, une collision choisie n'est pas calculable, ce qui ecarte
/// l'usurpation d'identite par collision de cle de cache (resolution du token
/// A vers le user_id de B).
fn token_cache_key(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
#[path = "tests/superadmin.rs"]
mod tests;
