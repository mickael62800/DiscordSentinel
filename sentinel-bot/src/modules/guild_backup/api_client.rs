//! Client HTTP du module guild_backup vers l'API interne.
//!
//! Le bot appelle l'API SANS `X-Discord-Token` : la gate RBAC Owner de l'API
//! est alors en pass-through (bypass interne), ce qui autorise capture et
//! restauration. L'authentification reste la cle API interne (Bearer) posee
//! par [`BaseApiClient::auth`].

use serde::{Deserialize, Serialize};

use sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot;

use crate::shared::api_client::BaseApiClient;

/// Identifiant d'une sauvegarde stockee (UUID renvoye par l'API).
pub type SnapshotId = String;

/// Resume leger d'une sauvegarde (liste sans le payload complet).
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    #[allow(dead_code)]
    pub guild_id: String,
    pub label: String,
    pub created_at: String,
    #[allow(dead_code)]
    pub created_by: Option<String>,
    #[allow(dead_code)]
    pub schema_version: u32,
    pub role_count: u32,
    pub channel_count: u32,
}

#[derive(Debug, Deserialize)]
struct StoredSnapshotDto {
    id: String,
}

/// POST /api/guild-backup/{guild_id}/snapshots — stocke une capture.
pub async fn store_snapshot(
    api: &BaseApiClient,
    guild_id: &str,
    snapshot: &GuildSnapshot,
) -> Result<SnapshotId, String> {
    let dto: StoredSnapshotDto = api
        .post_json(&format!("/api/guild-backup/{guild_id}/snapshots"), snapshot)
        .await?;
    Ok(dto.id)
}

/// GET /api/guild-backup/{guild_id}/snapshots — liste les captures (resumes).
pub async fn list_snapshots(
    api: &BaseApiClient,
    guild_id: &str,
) -> Result<Vec<SnapshotSummary>, String> {
    api.get_json(&format!("/api/guild-backup/{guild_id}/snapshots"))
        .await
}

/// GET /api/guild-backup/snapshots/{snapshot_id} — capture complete.
pub async fn get_snapshot(api: &BaseApiClient, snapshot_id: &str) -> Result<GuildSnapshot, String> {
    api.get_json(&format!("/api/guild-backup/snapshots/{snapshot_id}"))
        .await
}

/// Une re-attribution de roles en attente (envoyee au restore).
#[derive(Debug, Serialize)]
pub struct PendingRoleGrant {
    pub user_id: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SavedGrantsDto {
    saved: u64,
}

#[derive(Debug, Deserialize)]
struct ConsumedGrantDto {
    role_ids: Vec<String>,
}

/// POST /api/guild-backup/{guild_id}/pending-roles — enregistre les grants a
/// re-attribuer aux membres a leur retour. Renvoie le nombre d'entrees ecrites.
pub async fn save_pending_roles(
    api: &BaseApiClient,
    guild_id: &str,
    grants: &[PendingRoleGrant],
) -> Result<u64, String> {
    let dto: SavedGrantsDto = api
        .post_json(
            &format!("/api/guild-backup/{guild_id}/pending-roles"),
            &grants,
        )
        .await?;
    Ok(dto.saved)
}

/// POST /api/guild-backup/{guild_id}/pending-roles/{user_id}/consume — lit ET
/// supprime (atomique) les roles en attente d'un membre. Vecteur vide si aucun.
pub async fn consume_pending_roles(
    api: &BaseApiClient,
    guild_id: &str,
    user_id: &str,
) -> Result<Vec<String>, String> {
    let dto: ConsumedGrantDto = api
        .post_json(
            &format!("/api/guild-backup/{guild_id}/pending-roles/{user_id}/consume"),
            &serde_json::json!({}),
        )
        .await?;
    Ok(dto.role_ids)
}

/// DELETE /api/guild-backup/{guild_id}/pending-roles — purge les grants d'une
/// guild (repartir propre avant un nouveau restore). Best-effort.
pub async fn clear_pending_roles(api: &BaseApiClient, guild_id: &str) -> Result<(), String> {
    let path = format!("/api/guild-backup/{guild_id}/pending-roles");
    let req = api.client().delete(format!("{}{}", api.base_url(), path));
    let resp = api
        .auth(req)
        .send()
        .await
        .map_err(|e| format!("Purge impossible : {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Purge refusee ({status}) : {body}"));
    }
    Ok(())
}

/// DELETE /api/guild-backup/snapshots/{snapshot_id} — supprime une capture.
///
/// L'API repond 204 No Content (corps vide) : `delete_json` attend un JSON,
/// donc on utilise directement le client bas-niveau pour ne rien deserialiser.
pub async fn delete_snapshot(api: &BaseApiClient, snapshot_id: &str) -> Result<(), String> {
    let path = format!("/api/guild-backup/snapshots/{snapshot_id}");
    let req = api.client().delete(format!("{}{}", api.base_url(), path));
    let resp = api
        .auth(req)
        .send()
        .await
        .map_err(|e| format!("Suppression impossible : {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Suppression refusee ({status}) : {body}"));
    }
    Ok(())
}
