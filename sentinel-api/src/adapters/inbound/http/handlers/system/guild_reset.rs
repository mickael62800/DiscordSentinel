//! Factory reset d'un serveur (DANGER, IRREVERSIBLE).
//!
//! `POST /api/system/guild-reset/{guild_id}` — reserve a l'OWNER (RBAC) avec
//! une confirmation forte (le nom exact du serveur). Efface toutes les donnees
//! du guild en base, puis publie un event Redis `guild_reset` pour que le bot
//! annule l'etat Discord (deban / unmute / retrait des roles temp+quarantaine).

use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ResetGuildBody {
    /// Doit etre EXACTEMENT le nom du serveur (garde-fou anti-clic accidentel).
    pub confirmation: String,
    /// Actions Discord a executer par le bot (toutes activees par defaut).
    #[serde(default = "default_true")]
    pub unban: bool,
    #[serde(default = "default_true")]
    pub unmute: bool,
    #[serde(default = "default_true")]
    pub remove_roles: bool,
}

#[derive(Debug, Serialize)]
pub struct ResetGuildResponse {
    pub tables_wiped: usize,
    pub total_rows: u64,
}

/// POST /api/system/guild-reset/{guild_id}
pub async fn reset_guild(
    State(state): State<AppState>,
    ctx: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<ResetGuildBody>,
) -> Result<Json<ResetGuildResponse>, ApiError> {
    // ── Garde-fou 1 : OWNER uniquement ──
    let ctx = ctx
        .map(|e| e.0)
        .ok_or_else(|| ApiError::from(DomainError::Forbidden("authentification requise".into())))?;
    require_role(&ctx, Role::Owner).map_err(|_| {
        ApiError::from(DomainError::Forbidden(
            "Seul le proprietaire du serveur peut le reinitialiser.".into(),
        ))
    })?;

    // ── Garde-fou 2 : confirmation forte (nom du serveur), verifiee cote use case ──
    let outcome = state
        .reset_guild_uc
        .reset(&guild_id, &body.confirmation)
        .await?;

    tracing::warn!(
        guild_id,
        actor = %ctx.discord_user_id,
        total_rows = outcome.total_rows,
        tables = outcome.tables_wiped.len(),
        "FACTORY RESET execute (donnees du serveur effacees)"
    );

    // ── Event vers le bot : annule l'etat Discord ──
    state.broadcaster.broadcast(
        "guild_reset",
        serde_json::json!({
            "guild_id": guild_id,
            "unban": body.unban,
            "unmute": body.unmute,
            "remove_roles": body.remove_roles,
            "quarantine_role_id": outcome.discord_context.quarantine_role_id,
            "temp_role_ids": outcome.discord_context.temp_role_ids,
            "actor": { "source": "web", "user_id": ctx.discord_user_id },
        }),
    );

    Ok(Json(ResetGuildResponse {
        tables_wiped: outcome.tables_wiped.len(),
        total_rows: outcome.total_rows,
    }))
}
