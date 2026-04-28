use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use crate::adapters::inbound::http::dto::moderation::notes::AddNoteDto;
use crate::adapters::inbound::http::dto::moderation::notes::UserNoteDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;

/// POST /api/notes
pub async fn add_note(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<AddNoteDto>,
) -> Result<Json<UserNoteDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_content(&dto.content).map_err(ApiError)?;

    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour ajouter une note",
    )
    .await?;

    let command = dto.into();
    let note = state.notes_uc.add_note(command).await?;
    Ok(single_dto(note))
}

/// GET /api/notes/{guild_id}/{user_id}
pub async fn get_notes(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<UserNoteDto>>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    // Moderator+ requis : les notes sont sensibles (contexte interne de modo).
    use crate::adapters::inbound::http::middleware::rbac::check_role;
    check_role(&rbac, Role::Moderator, "moderator+ requis pour lire les notes")?;

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
    if rbac.is_some() {
        let note_uuid = uuid::Uuid::parse_str(&id).map_err(|_| {
            ApiError(DomainError::ValidationError("id note invalide".into()))
        })?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM user_notes WHERE id = $1",
        )
        .bind(note_uuid)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(sqlx_internal("fetch note guild_id"))?;

        if let Some((guild_id,)) = row {
            check_role_for_guild(
                &state,
                &rbac,
                &guild_id,
                Role::Moderator,
                "moderator+ requis pour supprimer une note",
            )
            .await?;
        }
        // Si la note n'existe pas, on laisse `delete_note` retourner sa propre
        // 404/NotFound plutot que de masquer avec un 403.
    }

    state.notes_uc.delete_note(&id).await?;
    Ok(ok_response())
}
