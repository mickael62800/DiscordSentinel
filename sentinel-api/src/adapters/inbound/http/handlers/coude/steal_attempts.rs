//! Phase 5 — Endpoints pour la table `coude_steal_attempts`.
//!
//! Le bot Discord persiste chaque /voler ici (au lieu de lancer un
//! `tokio::spawn(sleep 60s)` qui mourrait avec le process). Le worker
//! `expire_steals` (sentinel-worker, domaine coude) scanne les pending
//! expires et publie un event Redis pour declencher la resolution AFK
//! cote bot.
//!
//! Adaptateur ENTRANT mince : parse + map. Le calcul de la fenetre de
//! defense et les transitions de statut vivent dans
//! `ManageStealAttemptsUseCase` ; le SQL dans `StealAttemptRepository`.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::auth::AuthKind;
use crate::adapters::inbound::http::middleware::rbac::require_internal;
use crate::adapters::inbound::http::state::AppState;
use axum::Extension;

use sentinel_core::ports::inbound::coude::manage_steal_attempts::CreateStealAttempt;

#[derive(Deserialize)]
pub struct CreateStealAttemptDto {
    pub guild_id: String,
    pub thief_id: String,
    pub target_id: String,
    pub message_id: String,
    pub channel_id: String,
    /// Duree de la fenetre de defense en secondes. Le bot envoie 60.
    pub window_secs: i64,
}

#[derive(Serialize)]
pub struct StealAttemptDto {
    pub id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// POST /api/coude/steals — bot cree une tentative quand /voler est lance.
pub async fn create_steal_attempt(
    State(state): State<AppState>,
    auth: Option<Extension<AuthKind>>,
    Json(dto): Json<CreateStealAttemptDto>,
) -> Result<Json<StealAttemptDto>, ApiError> {
    // Endpoint bot-only : le guild_id vient du body, aucune notion de role web
    // ne s'applique. On refuse tout appel non-interne (defense anti-IDOR).
    require_internal(&state, auth.as_deref())?;
    let created = state
        .coude_steal_attempts_uc
        .create_attempt(CreateStealAttempt {
            guild_id: dto.guild_id,
            thief_id: dto.thief_id,
            target_id: dto.target_id,
            message_id: dto.message_id,
            channel_id: dto.channel_id,
            window_secs: dto.window_secs,
        })
        .await?;

    Ok(Json(StealAttemptDto {
        id: created.id,
        expires_at: created.expires_at,
    }))
}

/// PATCH /api/coude/steals/{id}/defend — la victime a clique le bouton.
/// Marque pending -> defended (atomique, idempotent).
pub async fn mark_defended(
    State(state): State<AppState>,
    auth: Option<Extension<AuthKind>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // Bot-only : mutation par UUID opaque, pas de guild resolu. On refuse tout
    // appel web (sinon un utilisateur web pourrait piloter des vols d'autres
    // guildes par id).
    require_internal(&state, auth.as_deref())?;
    // Idempotent cote bot : qu'il y ait eu transition ou non (deja
    // defended/expired ou id inconnu), on renvoie 204.
    state.coude_steal_attempts_uc.mark_defended(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/coude/steals/{id}/resolved — CLAIM atomique de la resolution.
/// Renvoie `{claimed: true}` uniquement si CET appel a fait la transition vers
/// 'resolved' (etat final). Les appelants (clic "Se defendre" ET worker AFK)
/// s'en servent pour n'appliquer le transfert de coins QU'UNE fois : le 2e
/// resolveur recoit claimed=false et n'effectue pas le vol -> plus de
/// double-resolution (victime sur-drainee).
pub async fn mark_resolved(
    State(state): State<AppState>,
    auth: Option<Extension<AuthKind>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Bot/worker-only : ce claim declenche le transfert de coins. Interdit au web.
    require_internal(&state, auth.as_deref())?;
    let claimed = state.coude_steal_attempts_uc.claim_resolved(id).await?;
    Ok(Json(serde_json::json!({ "claimed": claimed })))
}
