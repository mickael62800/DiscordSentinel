//! Handlers HTTP de sauvegarde / restauration de serveur (`guild_backup`).
//!
//! Action PUISSANTE (capture/restauration de toute la structure d'un serveur)
//! -> reservee au role **Owner**. Le bot (appels internes sans
//! `X-Discord-Token`) contourne la gate RBAC via `check_role_for_guild`
//! (rbac = None => pass-through), ce qui lui permet de capturer et restaurer.
//!
//! Phase 1 (backbone) : STOCKAGE seul. La capture Discord (production du
//! `GuildSnapshot`) et la restauration effective sont cote bot (phase 2).

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use axum::http::StatusCode;
use sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot;
use sentinel_core::ports::inbound::guild_backup::manage_snapshots::{SnapshotId, SnapshotSummary};
use sentinel_core::domain::enums::system::role::Role;

const OWNER_REQUIRED: &str = "owner requis (sauvegarde/restauration de serveur)";

#[derive(Debug, Serialize)]
pub struct StoredSnapshotDto {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct SnapshotSummaryDto {
    pub id: String,
    pub guild_id: String,
    pub label: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub schema_version: u32,
    pub role_count: u32,
    pub channel_count: u32,
}

impl From<SnapshotSummary> for SnapshotSummaryDto {
    fn from(s: SnapshotSummary) -> Self {
        SnapshotSummaryDto {
            id: s.id.to_string(),
            guild_id: s.guild_id,
            label: s.label,
            created_at: s.created_at,
            created_by: s.created_by,
            schema_version: s.schema_version,
            role_count: s.role_count,
            channel_count: s.channel_count,
        }
    }
}

/// POST /api/guild-backup/{guild_id}/snapshots — stocke une nouvelle capture.
/// Body = `GuildSnapshot`. Owner requis (bypass interne pour le bot).
pub async fn store_snapshot(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(mut snapshot): Json<GuildSnapshot>,
) -> Result<(StatusCode, Json<StoredSnapshotDto>), ApiError> {
    check_role_for_guild(&state, &rbac, &guild_id, Role::Owner, OWNER_REQUIRED).await?;
    // Le guild_id autoritaire est celui du path (evite un mismatch body/URL).
    snapshot.guild_id = guild_id.clone();
    // Quota de retention configurable (guild-backup-bot / snapshot_quota).
    // Absent => defaut historique (20). Le service borne a [1, 100].
    let quota = state
        .bot_config_repo
        .get_config(&guild_id, "guild-backup-bot")
        .await
        .ok()
        .and_then(|cfg| {
            cfg.iter()
                .find(|c| c.config_key == "snapshot_quota")
                .and_then(|c| c.config_value.parse::<u32>().ok())
        })
        .unwrap_or(
            sentinel_core::application::guild_backup::manage_snapshots_service::MAX_SNAPSHOTS_PER_GUILD,
        );
    let id = state
        .guild_snapshots_uc
        .store_snapshot_with_quota(snapshot, quota)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(StoredSnapshotDto { id: id.to_string() }),
    ))
}

/// GET /api/guild-backup/{guild_id}/snapshots — liste les captures (resumes).
pub async fn list_snapshots(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<SnapshotSummaryDto>>, ApiError> {
    check_role_for_guild(&state, &rbac, &guild_id, Role::Owner, OWNER_REQUIRED).await?;
    let summaries = state.guild_snapshots_uc.list_snapshots(&guild_id).await?;
    Ok(Json(summaries.into_iter().map(Into::into).collect()))
}

/// GET /api/guild-backup/snapshots/{snapshot_id} — capture complete (pour la
/// restauration). Owner de la guild concernee requis (bypass interne bot).
pub async fn get_snapshot(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(snapshot_id): Path<String>,
) -> Result<Json<GuildSnapshot>, ApiError> {
    let id = parse_id(&snapshot_id)?;
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;
    // Le guild_id vient de la ressource chargee (pas du path) : la gate protege
    // contre une lecture cross-serveur.
    check_role_for_guild(&state, &rbac, &snapshot.guild_id, Role::Owner, OWNER_REQUIRED).await?;
    Ok(Json(snapshot))
}

/// DELETE /api/guild-backup/snapshots/{snapshot_id} — supprime une capture.
pub async fn delete_snapshot(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(snapshot_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge d'abord pour connaitre le guild_id (RBAC) et distinguer 404.
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;
    check_role_for_guild(&state, &rbac, &snapshot.guild_id, Role::Owner, OWNER_REQUIRED).await?;
    state.guild_snapshots_uc.delete_snapshot(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RenameSnapshotBody {
    pub label: String,
}

/// PATCH /api/guild-backup/snapshots/{snapshot_id} — renomme une capture.
/// Owner de la guild concernee requis (bypass interne bot).
pub async fn rename_snapshot(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(snapshot_id): Path<String>,
    Json(body): Json<RenameSnapshotBody>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge d'abord pour connaitre le guild_id (RBAC) et distinguer 404.
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;
    check_role_for_guild(&state, &rbac, &snapshot.guild_id, Role::Owner, OWNER_REQUIRED).await?;
    state.guild_snapshots_uc.rename_snapshot(id, &body.label).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Corps de `POST /{guild_id}/capture` — demande de capture (executee par le bot).
#[derive(Debug, Deserialize)]
pub struct CaptureRequestBody {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

/// POST /api/guild-backup/{guild_id}/capture — publie un event Redis pour que
/// le bot capture le serveur. Le web ne peut pas agir sur Discord : l'API se
/// contente de publier `guild_backup:capture_requested`. Owner requis.
pub async fn request_capture(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<CaptureRequestBody>,
) -> Result<StatusCode, ApiError> {
    check_role_for_guild(&state, &rbac, &guild_id, Role::Owner, OWNER_REQUIRED).await?;
    state.broadcaster.broadcast(
        "guild_backup:capture_requested",
        serde_json::json!({
            "guild_id": guild_id,
            "label": body.label,
            "requested_by": body.requested_by,
        }),
    );
    Ok(StatusCode::ACCEPTED)
}

/// Corps de `POST /snapshots/{snapshot_id}/restore` — demande de restauration.
#[derive(Debug, Deserialize)]
pub struct RestoreRequestBody {
    #[serde(default)]
    pub wipe: bool,
    #[serde(default)]
    pub requested_by: Option<String>,
}

/// POST /api/guild-backup/snapshots/{snapshot_id}/restore — publie un event
/// Redis pour que le bot restaure le serveur depuis la capture. Owner requis.
pub async fn request_restore(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(snapshot_id): Path<String>,
    Json(body): Json<RestoreRequestBody>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge la capture pour resoudre le guild_id (RBAC + payload event).
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;
    check_role_for_guild(&state, &rbac, &snapshot.guild_id, Role::Owner, OWNER_REQUIRED).await?;
    state.broadcaster.broadcast(
        "guild_backup:restore_requested",
        serde_json::json!({
            "guild_id": snapshot.guild_id,
            "snapshot_id": id.to_string(),
            "wipe": body.wipe,
            "requested_by": body.requested_by,
        }),
    );
    Ok(StatusCode::ACCEPTED)
}

fn parse_id(raw: &str) -> Result<SnapshotId, ApiError> {
    validation::parse_uuid("snapshot_id", raw).map_err(ApiError)
}
