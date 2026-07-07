use crate::adapters::inbound::http::dto::moderation::notes::AddNoteDto;
use crate::adapters::inbound::http::dto::moderation::notes::UserNoteDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuildUser;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use sentinel_core::domain::enums::system::role::Role;

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
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Vec<UserNoteDto>>, ApiError> {
    // Validation

    // Moderator+ requis : les notes sont sensibles (contexte interne de modo).
    use crate::adapters::inbound::http::middleware::rbac::check_role;
    check_role(
        &rbac,
        Role::Moderator,
        "moderator+ requis pour lire les notes",
    )?;

    let notes = state.notes_uc.get_notes(&guild_id, &user_id).await?;
    Ok(map_to_dtos(notes))
}

/// DELETE /api/notes/{id}
pub async fn delete_note(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Gate RBAC : moderator+ requis. L'`id` de la note ne porte pas le guild_id,
    // on le recupere via le USE CASE (plus de SQL inline dans le handler ->
    // respect ports/adapters).
    if rbac.is_some() {
        // Validation de format (422 si l'id n'est pas un UUID) reste un concern
        // du handler ; le lookup guild_id passe par le use case.
        validation::parse_uuid("id", &id).map_err(ApiError)?;
        if let Some(guild_id) = state.notes_uc.note_guild_id(&id).await? {
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
