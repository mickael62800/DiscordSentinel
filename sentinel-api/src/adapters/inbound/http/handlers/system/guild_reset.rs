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
    // Signature HMAC (secret = API_KEY partage bot<->api) : le bot rejette un
    // event guild_reset non signe ou mal signe -> impossible de forcer un reset
    // destructif (unban-all + strip-roles) en publiant sur Redis sans le secret.
    let sig = sign_guild_reset(
        &state.api_key,
        &guild_id,
        body.unban,
        body.unmute,
        body.remove_roles,
    );
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
            "sig": sig,
        }),
    );

    Ok(Json(ResetGuildResponse {
        tables_wiped: outcome.tables_wiped.len(),
        total_rows: outcome.total_rows,
    }))
}

/// Signature HMAC-SHA256 d'un event `guild_reset`. Le format canonique du
/// message est partage a l'identique avec le consumer bot. Secret vide (mode
/// dev sans API_KEY) -> signature vide (le bot n'exige alors pas de signature).
pub fn sign_guild_reset(
    secret: &str,
    guild_id: &str,
    unban: bool,
    unmute: bool,
    remove_roles: bool,
) -> String {
    if secret.is_empty() {
        return String::new();
    }
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let msg = format!("guild_reset:{guild_id}:{unban}:{unmute}:{remove_roles}");
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("cle HMAC");
    mac.update(msg.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
