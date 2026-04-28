use axum::extract::Path;
use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::dto::reminders::CreateReminderDto;
use crate::adapters::inbound::http::dto::reminders::SanctionReminderDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::state::AppState;

/// POST /api/reminders
pub async fn create_reminder(
    State(state): State<AppState>,
    Json(dto): Json<CreateReminderDto>,
) -> Result<Json<SanctionReminderDto>, ApiError> {
    let command = dto.into();
    let reminder = state.reminders_uc.create_reminder(command).await?;
    Ok(single_dto(reminder))
}

/// GET /api/reminders/pending
pub async fn get_pending(
    State(state): State<AppState>,
) -> Result<Json<Vec<SanctionReminderDto>>, ApiError> {
    let reminders = state.reminders_uc.get_pending_reminders().await?;
    Ok(map_to_dtos(reminders))
}

/// GET /api/reminders/{guild_id}
pub async fn list_by_guild(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<SanctionReminderDto>>, ApiError> {
    let reminders = state.reminders_uc.list_by_guild(&guild_id).await?;
    Ok(map_to_dtos(reminders))
}
