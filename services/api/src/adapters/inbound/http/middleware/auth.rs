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

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            // Comparaison constant-time : empeche un attaquant de deviner la cle
            // caractere par caractere via la latence de reponse (timing attack).
            // Si les longueurs different, ct_eq retourne 0 sans short-circuit.
            if token.as_bytes().ct_eq(state.api_key.as_bytes()).into() {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
