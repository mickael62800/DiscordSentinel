//! Bump rewards (HTTP) : adaptateur ENTRANT mince. Toute la regle metier
//! (recompense graduee, cooldown atomique, seuil VIP) vit dans
//! `ManageBumpUseCase` ; le SQL dans `BumpRepository`. Ici : parse + RBAC + map.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::community::manage_bump::RecordBumpCommand;

/// Provider par defaut si un vieux client n'envoie pas le champ (retrocompat).
fn default_provider() -> String {
    "disboard".to_string()
}

#[derive(Debug, Deserialize)]
pub struct RecordBumpBody {
    #[serde(default)]
    pub username: String,
    /// Salon ou le bot a poste (fallback si bump_channel_id non configure).
    #[serde(default)]
    pub channel_id: String,
    /// Plateforme de bump ("disboard" | "discordl" | ...). Defaut retrocompat.
    #[serde(default = "default_provider")]
    pub provider: String,
}

#[derive(Debug, Serialize)]
pub struct BumpRewardDto {
    pub rewarded: bool,
    pub reward: i64,
    pub weekly_count: i64,
    pub new_balance: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vip_role_id: Option<String>,
    #[serde(default)]
    pub vip_just_unlocked: bool,
}

/// POST /api/bump/{guild_id}/{user_id} — enregistre un bump.
pub async fn record_bump(
    State(state): State<AppState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(body): Json<RecordBumpBody>,
) -> Result<Json<BumpRewardDto>, ApiError> {
    // Constater un bump et crediter est une operation du BOT (Bearer API_KEY ->
    // Internal, bypass). Sans cette garde, tout appelant web creditait n'importe
    // quel user de n'importe quel serveur (IDOR + creation de monnaie).

    let reward = state
        .bump_uc
        .record_bump(RecordBumpCommand {
            guild_id,
            user_id,
            username: body.username,
            channel_id: body.channel_id,
            provider: body.provider,
        })
        .await?;

    Ok(Json(BumpRewardDto {
        rewarded: reward.rewarded,
        reward: reward.reward,
        weekly_count: reward.weekly_count,
        new_balance: reward.new_balance,
        vip_role_id: reward.vip_role_id,
        vip_just_unlocked: reward.vip_just_unlocked,
    }))
}

#[derive(Debug, Serialize)]
pub struct DueReminderDto {
    pub guild_id: String,
    pub channel_id: String,
    pub provider: String,
}

/// GET /api/bump/due-reminders — rappels dus (poll par le bot).
pub async fn due_reminders(
    State(state): State<AppState>,
) -> Result<Json<Vec<DueReminderDto>>, ApiError> {
    let rows = state.bump_uc.due_reminders().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| DueReminderDto {
                guild_id: r.guild_id,
                channel_id: r.channel_id,
                provider: r.provider,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct MarkReminderBody {
    /// Provider a marquer. Absent (vieux client) => tous les providers.
    #[serde(default)]
    pub provider: Option<String>,
}

/// POST /api/bump/{guild_id}/reminder-sent — marque le rappel envoye.
pub async fn mark_reminder_sent(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    body: Option<Json<MarkReminderBody>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let provider = body.and_then(|Json(b)| b.provider);
    state
        .bump_uc
        .mark_reminder_sent(&guild_id, provider)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
