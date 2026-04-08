use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::dto::notes::{AddNoteDto, UserNoteDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;

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
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.notes_uc.delete_note(&id).await?;
    Ok(ok_response())
}
