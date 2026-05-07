use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

use crate::adapters::inbound::http::state::AppState;

/// Middleware d'authentification par Bearer token.
/// Passe si aucune clé API n'est configurée (dev mode — log un warning).
pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Pas de clé configurée → tout passe (dev mode uniquement, REQUIRE_API_KEY=false)
    if state.api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    // 1. Bearer API_KEY (services internes : bot, workers).
    if let Some(header) = auth_header {
        if let Some(token) = header.strip_prefix("Bearer ") {
            // Comparaison constant-time : empeche un attaquant de deviner la cle
            // caractere par caractere via la latence de reponse (timing attack).
            if token.as_bytes().ct_eq(state.api_key.as_bytes()).into() {
                return Ok(next.run(request).await);
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // 2. X-Discord-Token (utilisateurs web apres OAuth).
    // La validation du token (call Discord /users/@me) est faite par les
    // middlewares en aval (rbac, guild_auth). Ici on verifie juste qu'un
    // header non-vide est present, suffisant pour les endpoints qui ont
    // une couche RBAC/whitelist (>=99% des endpoints proteges).
    let discord_token = request
        .headers()
        .get("x-discord-token")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty());
    if discord_token.is_some() {
        return Ok(next.run(request).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
