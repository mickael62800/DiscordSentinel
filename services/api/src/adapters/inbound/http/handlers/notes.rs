use axum::extract::{Path, State};
use axum::{Extension, Json};

use crate::adapters::inbound::http::dto::notes::{AddNoteDto, UserNoteDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{require_role_for_guild, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;

/// POST /api/notes
pub async fn add_note(
    State(state): State<AppState>,
    Json(dto): Json<AddNoteDto>,
) -> Result<Json<UserNoteDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_content(&dto.content).map_err(ApiError)?;

    let command = dto.into();
    let note = state.notes_uc.add_note(command).await?;
    Ok(single_dto(note))
}

/// GET /api/notes/{guild_id}/{user_id}
pub async fn get_notes(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<UserNoteDto>>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    let notes = state.notes_uc.get_notes(&guild_id, &user_id).await?;
    Ok(map_to_dtos(notes))
}

/// DELETE /api/notes/{id}
pub async fn delete_note(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate RBAC : moderator+ requis. L'`id` de la note ne contient
    // pas le guild_id, donc on fetch d'abord en direct sqlx (pattern
    // "ressource-id-based" — plus simple qu'ajouter une methode au repo).
    if let Some(Extension(ctx)) = rbac {
        let note_uuid = uuid::Uuid::parse_str(&id).map_err(|_| {
            ApiError(DomainError::ValidationError("id note invalide".into()))
        })?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM user_notes WHERE id = $1",
        )
        .bind(note_uuid)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("fetch note guild_id: {e}"))))?;

        if let Some((guild_id,)) = row {
            require_role_for_guild(&state, &ctx, &guild_id, Role::Moderator)
                .await
                .map_err(|_| ApiError(DomainError::Forbidden("moderator+ requis pour supprimer une note".into())))?;
        }
        // Si la note n'existe pas, on laisse `delete_note` retourner sa propre
        // 404/NotFound plutot que de masquer avec un 403.
    }

    state.notes_uc.delete_note(&id).await?;
    Ok(ok_response())
}
