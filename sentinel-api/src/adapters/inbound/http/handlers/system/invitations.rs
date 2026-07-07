//! Codes d'invitation a usage unique pour onboarder de nouveaux users.
//!
//! Adaptateur ENTRANT mince : parse + RBAC + map. Toute la regle metier
//! (generation de code unique, expiration, octroi de role atomique) vit dans
//! `ManageInvitationsUseCase` ; le SQL dans `InvitationRepository`.
//!
//! Endpoints :
//!   POST /api/invitations              (owner+, scope guild) — genere un code
//!   GET  /api/invitations/{guild_id}   (owner+) — liste les codes
//!   DELETE /api/invitations/code/{code}(owner+) — revoque un code non utilise
//!   POST /api/auth/redeem-invitation   (auth Discord token requis) — consomme code
//!   GET  /api/auth/check-access        (auth Discord token requis)

use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::ports::inbound::system::manage_invitations::CreateInvitationCommand;
use sentinel_core::domain::entities::system::invitation::Invitation;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

/// Message RBAC commun aux endpoints owner+.
const OWNER_REQUIRED: &str = "owner+ requis pour gerer les invitations";

// ── DTO ──

#[derive(Debug, Deserialize)]
pub struct CreateInvitationDto {
    pub guild_id: String,
    pub role: String,
    /// Heures avant expiration (defaut 168 = 7 jours, 0 = pas d'expiration)
    #[serde(default)]
    pub expires_in_hours: Option<i64>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvitationDto {
    pub code: String,
    pub guild_id: String,
    pub role: String,
    pub created_by: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub used_at: Option<String>,
    pub used_by_discord_id: Option<String>,
    pub notes: Option<String>,
    pub status: String, // "active" | "expired" | "used"
}

impl InvitationDto {
    fn from_domain(inv: Invitation, now: chrono::DateTime<chrono::Utc>) -> Self {
        let status = inv.status(now).to_string();
        InvitationDto {
            code: inv.code,
            guild_id: inv.guild_id,
            role: inv.role,
            created_by: inv.created_by,
            created_at: inv.created_at.to_rfc3339(),
            expires_at: inv.expires_at.map(|t| t.to_rfc3339()),
            used_at: inv.used_at.map(|t| t.to_rfc3339()),
            used_by_discord_id: inv.used_by_discord_id,
            notes: inv.notes,
            status,
        }
    }
}

// ── Generate ──

pub async fn create_invitation(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Json(dto): Json<CreateInvitationDto>,
) -> Result<Json<InvitationDto>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;

    // Owner+ (ou superadmin) requis. Le guild_id est dans le body, donc lookup
    // RBAC explicite via le helper (superadmin bypass inclus).
    let rbac = Some(Extension(ctx.clone()));
    check_role_for_guild(&state, &rbac, &dto.guild_id, Role::Owner, OWNER_REQUIRED).await?;

    let inv = state
        .invitations_uc
        .create_invitation(CreateInvitationCommand {
            guild_id: dto.guild_id,
            role: dto.role,
            expires_in_hours: dto.expires_in_hours,
            notes: dto.notes,
            created_by: ctx.discord_user_id,
        })
        .await?;

    Ok(Json(InvitationDto::from_domain(inv, chrono::Utc::now())))
}

// ── List ──

pub async fn list_invitations(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<InvitationDto>>, ApiError> {
    let rbac = Some(Extension(ctx));
    check_role_for_guild(&state, &rbac, &guild_id, Role::Owner, OWNER_REQUIRED).await?;

    let now = chrono::Utc::now();
    let out = state
        .invitations_uc
        .list_invitations(&guild_id)
        .await?
        .into_iter()
        .map(|inv| InvitationDto::from_domain(inv, now))
        .collect();
    Ok(Json(out))
}

// ── Revoke (delete unused) ──

pub async fn revoke_invitation(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Recupere la guild associee pour le check role.
    let Some(inv) = state.invitations_uc.find_invitation(&code).await? else {
        return Err(ApiError(DomainError::NotFound("code introuvable".into())));
    };
    if inv.used_at.is_some() {
        return Err(ApiError(DomainError::Conflict(
            "code deja utilise, ne peut pas etre revoque".into(),
        )));
    }

    let rbac = Some(Extension(ctx));
    check_role_for_guild(&state, &rbac, &inv.guild_id, Role::Owner, OWNER_REQUIRED).await?;

    state.invitations_uc.revoke_invitation(&code).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Check access (apres OAuth, avant dashboard) ──

#[derive(Debug, Serialize)]
pub struct CheckAccessResponse {
    pub is_authorized: bool,
    pub is_superadmin: bool,
    /// Nombre de guilds pour lesquelles l'utilisateur a un role
    pub guild_count: i64,
    pub message: String,
}

/// GET /api/auth/check-access
/// Auth requis : X-Discord-Token. Le middleware injecte RoleContext avec
/// au minimum discord_user_id. Pas besoin d'avoir un role pour appeler.
pub async fn check_access(
    State(state): State<AppState>,
    rbac: Option<axum::Extension<RoleContext>>,
) -> Result<Json<CheckAccessResponse>, ApiError> {
    let Some(axum::Extension(ctx)) = rbac else {
        return Err(ApiError(DomainError::Forbidden(
            "auth Discord requise (X-Discord-Token manquant ou invalide)".into(),
        )));
    };

    let is_superadmin = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);

    let access = state
        .invitations_uc
        .check_access(&ctx.discord_user_id, is_superadmin)
        .await?;

    Ok(Json(CheckAccessResponse {
        is_authorized: access.is_authorized,
        is_superadmin: access.is_superadmin,
        guild_count: access.guild_count,
        message: access.message,
    }))
}

// ── Redeem (par l'utilisateur invite) ──

#[derive(Debug, Deserialize)]
pub struct RedeemDto {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RedeemResponse {
    pub guild_id: String,
    pub role: String,
    pub message: String,
}

/// POST /api/auth/redeem-invitation
/// Auth requis : X-Discord-Token (le middleware injecte RoleContext avec
/// discord_user_id, peu importe le role). Pas besoin d'etre dans la whitelist.
pub async fn redeem_invitation(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Json(dto): Json<RedeemDto>,
) -> Result<Json<RedeemResponse>, ApiError> {
    let redeemed = state
        .invitations_uc
        .redeem_invitation(&ctx.discord_user_id, &dto.code)
        .await?;

    Ok(Json(RedeemResponse {
        guild_id: redeemed.guild_id,
        role: redeemed.role,
        message: "Invitation acceptée".to_string(),
    }))
}
