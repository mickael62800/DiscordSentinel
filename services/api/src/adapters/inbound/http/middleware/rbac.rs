//! Phase 7 B — RBAC fin par guild.
//!
//! Complete `guild_auth_middleware` (Phase 2 B) avec la notion de role
//! applicatif. Doit tourner APRES guild_auth car il reutilise le meme flow
//! d'extraction du token Discord.
//!
//! # Flow
//!
//! 1. Si `X-Discord-Token` absent → pass-through (bot/internal, deja valide
//!    par `auth_middleware`).
//! 2. Sinon, fetch l'identite du user via cache Redis `user_id:<token_hash>`
//!    ou fallback `GET /users/@me`.
//! 3. Extrait le `guild_id` depuis l'URI (meme heuristique que guild_auth).
//!    Si absent → stocke juste le user dans les extensions (role = None,
//!    endpoint global).
//! 4. Lookup `api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2`.
//!    - **Row trouvee** → role = celui de la DB
//!    - **Row absente** → role par defaut = `viewer` (read-only, POLA).
//! 5. Stocke `RoleContext { user_id, role }` dans les extensions de la
//!    requete. Les handlers peuvent l'extraire pour gater des operations
//!    sensibles via `require_role(Role::Admin)`.
//!
//! # Bootstrap
//!
//! Les premiers `owner` doivent etre seedes en SQL direct au deploiement
//! initial. Pas d'auto-promote pour eviter la prise de contr le.
//!
//! ```sql
//! INSERT INTO api_users (discord_user_id, display_name) VALUES ('1234...', 'Alice');
//! INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ('1234...', '9876...', 'owner');
//! ```

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::adapters::inbound::http::state::AppState;

const USER_ID_CACHE_TTL_SECS: u64 = 600;
const DISCORD_TOKEN_HEADER: &str = "x-discord-token";

use crate::domain::enums::system::role::Role;

/// Contexte injecte dans les extensions de la requete pour les handlers.
#[derive(Debug, Clone)]
pub struct RoleContext {
    pub discord_user_id: String,
    /// `None` si l'URI ne contient pas de guild (endpoint global).
    pub role: Option<Role>,
    pub guild_id: Option<String>,
}

pub async fn rbac_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Recuperer le token Discord. Absent ⇒ pass-through (appel bot/internal).
    let discord_token = match request
        .headers()
        .get(DISCORD_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return Ok(next.run(request).await),
    };

    // 2. Fetch user_id (cache Redis + fallback Discord API)
    let user_id = match get_or_fetch_user_id(&state, &discord_token).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "rbac: impossible de recuperer user_id");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    // 3. Extraire guild_id depuis l'URI (heuristique identique a guild_auth)
    let path = request.uri().path().to_string();
    let guild_id = extract_guild_id_from_path(&path);

    // 4. Role pour cette (user, guild). Si pas de guild, on passe sans role.
    //
    // **Fail-CLOSED** sur erreur DB : on retourne 503 au caller au lieu de
    // degrader silencieusement a `Viewer`. Un fail-open serait dangereux
    // car un user qui devrait etre `Owner` pourrait soudain lire des data
    // auxquelles il n'a pas droit (le fallback `Viewer` ignore `api_user_guilds`
    // completement, donnant un acces read-only a TOUS les users Discord
    // authentifies sur la guild, meme ceux pas dans `api_user_guilds`).
    //
    // Note : `lookup_role` retourne deja Ok(Role::Viewer) quand la row
    // n'existe pas (principe du moindre privilege pour un user legitime
    // de la guild Discord). L'Err ici est reserve aux VRAIES erreurs DB
    // (pool sature, connexion timeout, query malformee).
    let role = if let Some(ref gid) = guild_id {
        match lookup_role(&state, &user_id, gid).await {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    user_id = %user_id,
                    guild_id = %gid,
                    "rbac: lookup role DB error, returning 503 (fail-closed)"
                );
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    } else {
        None
    };

    // 5. Injection dans les extensions
    request.extensions_mut().insert(RoleContext {
        discord_user_id: user_id,
        role,
        guild_id,
    });

    Ok(next.run(request).await)
}

/// Helper a appeler depuis les handlers pour gater des operations sensibles.
/// Retourne `Err(StatusCode::FORBIDDEN)` si le user n'a pas le role requis,
/// `Err(StatusCode::UNAUTHORIZED)` si aucun RoleContext n'est present (= pas
/// d'appel desktop authentifie), `Ok(())` sinon.
///
/// Exemple d'usage dans un handler :
/// ```ignore
/// use axum::Extension;
/// use crate::adapters::inbound::http::middleware::rbac::{RoleContext, require_role};
/// use crate::domain::enums::system::role::Role;
///
/// pub async fn delete_config(
///     Extension(ctx): Extension<RoleContext>,
///     // ...
/// ) -> Result<Json<...>, ApiError> {
///     require_role(&ctx, Role::Admin)?;
///     // ... logique admin-only
/// }
/// ```
#[allow(dead_code)]
pub fn require_role(ctx: &RoleContext, required: Role) -> Result<(), StatusCode> {
    match ctx.role {
        Some(role) if role.satisfies(required) => Ok(()),
        Some(_) => Err(StatusCode::FORBIDDEN),
        None => Err(StatusCode::FORBIDDEN), // endpoint non scope par guild
    }
}

/// Phase 7 B — Helper pour les endpoints **globaux** (non scoped par guild).
///
/// Exemple : `/purge/logs` purge les logs systeme de TOUTES les guilds, donc
/// `require_role_for_guild` n'a aucun sens ici. On check contre une liste
/// statique de "superadmin" definis par env var `SUPERADMIN_USER_IDS`.
///
/// Retourne `Err(FORBIDDEN)` si :
/// - le caller n'a pas de `RoleContext` (pas d'auth desktop), OU
/// - son `discord_user_id` n'est pas dans la liste superadmin.
///
/// Bootstrap : definir `SUPERADMIN_USER_IDS=123,456,789` au deploiement.
/// Si la liste est vide (pas configuree), TOUS les appels sont refuses par
/// securite — c'est volontaire : mieux vaut bloquer que laisser passer.
#[allow(dead_code)]
pub fn require_superadmin(
    state: &AppState,
    ctx: &RoleContext,
) -> Result<(), StatusCode> {
    if state.superadmin_user_ids.iter().any(|id| id == &ctx.discord_user_id) {
        Ok(())
    } else {
        tracing::warn!(
            user_id = %ctx.discord_user_id,
            "rbac: acces superadmin refuse (user non liste dans SUPERADMIN_USER_IDS)"
        );
        Err(StatusCode::FORBIDDEN)
    }
}

/// Variante pour les handlers dont le `guild_id` n'est PAS dans le path
/// (body-based comme `bot_config`, `purge`, ou ressource-id-based comme
/// `/infractions/{id}`). Le middleware n'a pas pu resoudre le role car
/// l'heuristique d'extraction snowflake ne trouve rien dans l'URL — on
/// fait un lookup DB explicite ici.
///
/// Semantique identique a `require_role` :
/// - Si le caller n'a pas de `RoleContext` → `Err(FORBIDDEN)` (pas d'auth desktop)
/// - Si le role effectif est suffisant → `Ok(())`
/// - Sinon → `Err(FORBIDDEN)`
///
/// Note : si la row `api_user_guilds` n'existe pas pour ce (user, guild),
/// on retombe sur `Role::Viewer` (principe du moindre privilege) — identique
/// au comportement du middleware.
#[allow(dead_code)]
pub async fn require_role_for_guild(
    state: &AppState,
    ctx: &RoleContext,
    guild_id: &str,
    required: Role,
) -> Result<(), StatusCode> {
    // **Fail-CLOSED** sur erreur DB : cf. rbac_middleware. Retourne 503
    // SERVICE_UNAVAILABLE plutot que degrader silencieusement a `Viewer`
    // (fail-open dangereux). Le handler remontera l'erreur au caller.
    let role = match lookup_role(state, &ctx.discord_user_id, guild_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                error = %e,
                user_id = %ctx.discord_user_id,
                guild_id,
                "require_role_for_guild: DB error, returning 503 (fail-closed)"
            );
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    if role.satisfies(required) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Phase 7 B — Helper handler-friendly : wrap `require_role` pour les handlers
/// path-based qui utilisent `Option<Extension<RoleContext>>`.
///
/// Elimine la duplication du pattern :
/// ```ignore
/// if let Some(Extension(ctx)) = rbac {
///     require_role(&ctx, Role::Admin)
///         .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour X".into())))?;
/// }
/// ```
///
/// Devient :
/// ```ignore
/// check_role(&rbac, Role::Admin, "admin+ requis pour X")?;
/// ```
///
/// Sémantique :
/// - `rbac` absent → pass-through (appel bot/internal non gated)
/// - `rbac` présent et `role.satisfies(required)` → `Ok(())`
/// - Sinon → `Err(ApiError(DomainError::Forbidden(...)))`
#[allow(dead_code)]
pub fn check_role(
    rbac: &Option<axum::Extension<RoleContext>>,
    required: Role,
    forbidden_msg: &str,
) -> Result<(), crate::adapters::inbound::http::errors::ApiError> {
    use crate::adapters::inbound::http::errors::ApiError;
    use crate::domain::errors::DomainError;

    let Some(axum::Extension(ctx)) = rbac else {
        return Ok(());
    };

    // NOTE : pas d'acces a AppState ici (helper sync). Pour le bypass
    // superadmin, utiliser la variante async `check_role_for_guild` ou
    // appeler directement `require_superadmin` depuis le handler.
    // Alternative : exposer `check_role_with_state` si besoin.
    match require_role(ctx, required) {
        Ok(()) => Ok(()),
        Err(_) => Err(ApiError(DomainError::Forbidden(forbidden_msg.to_string()))),
    }
}

/// Phase 7 B — Helper handler-friendly pour les endpoints body-based ou
/// resource-id-based (guild_id pas dans le path).
///
/// Wrap `require_role_for_guild` avec le même pattern de pass-through.
/// Async car il doit faire un lookup DB (contrairement à `check_role`).
///
/// **Distingue** les 2 cas d'erreur de `require_role_for_guild` :
/// - `FORBIDDEN` (role insuffisant) -> `DomainError::Forbidden` = HTTP 403
/// - `SERVICE_UNAVAILABLE` (erreur DB, fail-closed) -> `DomainError::Internal`
///   = HTTP 500 (le handler remonte l'erreur au caller, qui retry)
#[allow(dead_code)]
pub async fn check_role_for_guild(
    state: &AppState,
    rbac: &Option<axum::Extension<RoleContext>>,
    guild_id: &str,
    required: Role,
    forbidden_msg: &str,
) -> Result<(), crate::adapters::inbound::http::errors::ApiError> {
    use crate::adapters::inbound::http::errors::ApiError;
    use crate::domain::errors::DomainError;

    let Some(axum::Extension(ctx)) = rbac else {
        return Ok(());
    };

    // Bypass global : un superadmin (defini via SUPERADMIN_USER_IDS) passe
    // toutes les gates RBAC par guild, quel que soit son role en DB.
    if state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id)
    {
        return Ok(());
    }

    match require_role_for_guild(state, ctx, guild_id, required).await {
        Ok(()) => Ok(()),
        Err(StatusCode::SERVICE_UNAVAILABLE) => Err(ApiError(DomainError::Internal(
            "RBAC lookup DB error (fail-closed)".to_string(),
        ))),
        Err(_) => Err(ApiError(DomainError::Forbidden(forbidden_msg.to_string()))),
    }
}

async fn get_or_fetch_user_id(state: &AppState, access_token: &str) -> Result<String, String> {
    let cache_key = format!("user_id:{}", short_hash(access_token));

    // Cache Redis
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
            if !cached.is_empty() {
                return Ok(cached);
            }
        }
    }

    // Fallback Discord API
    let user = state
        .discord_api
        .get_user_me(access_token)
        .await
        .map_err(|e| format!("Discord API: {e}"))?;

    // Upsert api_users (best-effort — on ne bloque pas sur une erreur DB)
    let _ = sqlx::query(
        "INSERT INTO api_users (discord_user_id, display_name) \
         VALUES ($1, $2) \
         ON CONFLICT (discord_user_id) \
         DO UPDATE SET display_name = EXCLUDED.display_name, last_seen_at = NOW()",
    )
    .bind(&user.id)
    .bind(&user.username)
    .execute(&state.pg_pool)
    .await;

    // Cache (best-effort)
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _: Result<(), _> = conn.set_ex(&cache_key, &user.id, USER_ID_CACHE_TTL_SECS).await;
    }

    Ok(user.id)
}

async fn lookup_role(state: &AppState, user_id: &str, guild_id: &str) -> Result<Role, String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM api_user_guilds \
         WHERE discord_user_id = $1 AND guild_id = $2",
    )
    .bind(user_id)
    .bind(guild_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| format!("lookup role: {e}"))?;

    match row {
        Some((role_str,)) => Role::from_str(&role_str)
            .ok_or_else(|| format!("role DB invalide: {role_str}")),
        // Fallback : si pas de row mais user dans la guild (guild_auth l'a deja valide),
        // on lui donne viewer par defaut. Principe du moindre privilege.
        None => Ok(Role::Viewer),
    }
}

/// Heuristique : on prend le premier segment du path qui ressemble a un
/// snowflake Discord (17-20 chiffres). Duplique de guild_auth pour eviter
/// une dependance inter-middleware.
fn extract_guild_id_from_path(path: &str) -> Option<String> {
    // Doit rester synchronise avec guild_auth.rs (idem fonction).
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

fn short_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}


#[cfg(test)]
#[path = "tests/rbac.rs"]
mod tests;
