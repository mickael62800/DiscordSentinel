//! Handlers HTTP pour les coalitions (cf. COUPE_AMELIORATIONS 5.3).

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::coude::coalition::ActiveCoalition;
use sentinel_core::domain::entities::coude::coalition::COALITION_DURATION_HOURS;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
#[derive(Debug, Serialize)]
pub struct CoalitionMemberDto {
    pub member_id: String,
    pub member_name: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ActiveCoalitionDto {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub opened_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub broken_by: Option<String>,
    pub broken_at: Option<DateTime<Utc>>,
    pub members: Vec<CoalitionMemberDto>,
}

impl From<ActiveCoalition> for ActiveCoalitionDto {
    fn from(c: ActiveCoalition) -> Self {
        Self {
            id: c.id.to_string(),
            guild_id: c.guild_id,
            target_id: c.target_id,
            opened_at: c.opened_at,
            expires_at: c.expires_at,
            status: c.status.as_db_str().into(),
            broken_by: c.broken_by,
            broken_at: c.broken_at,
            members: c
                .members
                .into_iter()
                .map(|m| CoalitionMemberDto {
                    member_id: m.member_id,
                    member_name: m.member_name,
                    joined_at: m.joined_at,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinCoalitionDto {
    pub target_id: String,
    pub member_id: String,
    pub member_name: String,
}

/// POST /api/coude/{guild_id}/coalitions/join
///
/// Cree la coalition si necessaire, ajoute le membre. Retourne l etat
/// actualise. Le bot debit le wallet du membre AVANT cet appel.
pub async fn join_coalition(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<JoinCoalitionDto>,
) -> Result<Json<ActiveCoalitionDto>, ApiError> {
    let existing = state
        .coude_coalition_repo
        .get_active(&guild_id, &dto.target_id)
        .await?;
    let coalition = match existing {
        Some(c) => state
            .coude_coalition_repo
            .add_member(c.id, &dto.member_id, &dto.member_name)
            .await?,
        None => {
            let id = state
                .coude_coalition_repo
                .create_with_first_member(
                    &guild_id,
                    &dto.target_id,
                    &dto.member_id,
                    &dto.member_name,
                    COALITION_DURATION_HOURS,
                )
                .await?;
            state
                .coude_coalition_repo
                .add_member(id, &dto.member_id, &dto.member_name)
                .await?
        }
    };
    Ok(Json(coalition.into()))
}

/// GET /api/coude/{guild_id}/coalitions/by-target/{target_id}
pub async fn get_coalition_by_target(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
) -> Result<Json<Option<ActiveCoalitionDto>>, ApiError> {
    let c = state
        .coude_coalition_repo
        .get_active(&guild_id, &target_id)
        .await?;
    Ok(Json(c.map(Into::into)))
}
