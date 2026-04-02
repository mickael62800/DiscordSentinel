use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::{
    BanEntryDto, LogActionDto, ModerationActionResponseDto, UserHistoryDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::{AddStrikeCommand, CreateReminderCommand};

#[derive(Debug, Deserialize)]
pub struct BansQuery {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// POST /api/moderation/actions — enregistrer une action de modération
pub async fn log_action(
    State(state): State<AppState>,
    Json(dto): Json<LogActionDto>,
) -> Result<Json<ModerationActionResponseDto>, ApiError> {
    let action_type = dto.action_type.clone();
    let target_name = dto.target_name.clone();
    let moderator_name = dto.moderator_name.clone();
    let reason = dto.reason.clone();

    let guild_id = dto.guild_id.clone();
    let target_id = dto.target_id.clone();
    let strike_reason = dto.reason.clone();
    let moderator_id = dto.moderator_id.clone();
    let duration = dto.duration;

    let command = dto.into();
    let action = state.moderation_uc.log_action(command).await?;

    // Add a strike and check for escalation
    let strike_result = state
        .strikes_uc
        .add_strike(AddStrikeCommand {
            guild_id: guild_id.clone(),
            user_id: target_id.clone(),
            reason: strike_reason,
            source: "moderator".into(),
            infraction_id: None,
        })
        .await
        .ok();

    let mut dto = ModerationActionResponseDto::from(action);
    if let Some(ref sr) = strike_result {
        dto.strikes_count = Some(sr.active_count);
        dto.escalation_action = sr.escalation_action.clone();
        dto.escalation_duration = sr.escalation_duration;
    }

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": action_type,
            "target_id": target_id,
            "target_name": target_name,
            "moderator_name": moderator_name,
            "reason": reason,
            "guild_id": guild_id,
        }),
    );

    if let Some(ref sr) = strike_result {
        if sr.escalation_action.is_some() {
            state.broadcaster.broadcast(
                "strike_added",
                serde_json::json!({
                    "guild_id": guild_id,
                    "user_id": target_id,
                    "active_count": sr.active_count,
                    "escalation_action": sr.escalation_action,
                    "escalation_duration": sr.escalation_duration,
                }),
            );
        }
    }

    // Auto-create reminder for temporary sanctions (mute_temp, ban_temp)
    // Auto-create reminder for temporary sanctions
    if action_type == "mute_temp" || action_type == "ban_temp" {
        if let Some(dur) = duration {
            let action_uuid = dto.id.parse().unwrap_or_default();
            let _ = state.reminders_uc.create_reminder(CreateReminderCommand {
                guild_id: guild_id.clone(),
                moderator_id,
                moderator_name: moderator_name.clone(),
                target_id: target_id.clone(),
                target_name: target_name.clone(),
                action_type: action_type.clone(),
                reason: reason.clone(),
                action_id: action_uuid,
                duration_secs: dur,
                remind_before_secs: 3600,
            }).await;
        }
    }

    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteBanDto {
    pub guild_id: String,
    pub user_id: String,
    pub reason: String,
}

/// POST /api/moderation/execute-ban — execute un ban Discord + log l'action
pub async fn execute_ban(
    State(state): State<AppState>,
    Json(dto): Json<ExecuteBanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .discord_api
        .ban_user(&dto.guild_id, &dto.user_id, &dto.reason)
        .await
        .map_err(ApiError)?;

    let reason = dto.reason.clone();

    let command = crate::ports::inbound::LogModerationCommand {
        guild_id: dto.guild_id.clone(),
        channel_id: String::new(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: dto.user_id.clone(),
        target_name: dto.user_id.clone(),
        action_type: "ban_permanent".into(),
        reason: dto.reason,
        gravity: None,
        duration: None,
    };
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "ban_permanent",
            "target_id": &dto.user_id,
            "target_name": &dto.user_id,
            "moderator_name": "Desktop App",
            "guild_id": &dto.guild_id,
            "reason": &reason,
        }),
    );

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUnbanDto {
    pub guild_id: String,
    pub user_id: String,
}

/// POST /api/moderation/execute-unban — debannir un utilisateur Discord
pub async fn execute_unban(
    State(state): State<AppState>,
    Json(dto): Json<ExecuteUnbanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .discord_api
        .unban_user(&dto.guild_id, &dto.user_id)
        .await
        .map_err(ApiError)?;

    let target_id = dto.user_id.clone();
    let guild_id = dto.guild_id.clone();

    let command = crate::ports::inbound::LogModerationCommand {
        guild_id: dto.guild_id,
        channel_id: String::new(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: target_id.clone(),
        target_name: target_id.clone(),
        action_type: "unban".into(),
        reason: "Deban depuis le desktop".into(),
        gravity: None,
        duration: None,
    };
    state
        .moderation_uc
        .delete_bans_for_user(&guild_id, &target_id)
        .await?;
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "unban",
            "target_id": &target_id,
            "moderator_name": "Desktop App",
            "guild_id": &guild_id,
        }),
    );

    Ok(ok_response())
}

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<AppState>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500);
    let offset = params.offset.unwrap_or(0).max(0);
    let bans = state
        .moderation_uc
        .list_bans(params.guild_id.as_deref(), limit, offset)
        .await?;
    Ok(map_to_dtos(bans))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserHistoryDto>, ApiError> {
    let history = state
        .moderation_uc
        .get_history(&guild_id, &user_id)
        .await?;
    Ok(single_dto(history))
}
