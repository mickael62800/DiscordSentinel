//! Codes d'invitation a usage unique pour onboarder de nouveaux users.
//!
//! Endpoints :
//!   POST /api/invitations              (owner+, scope guild) — genere un code
//!   GET  /api/invitations/{guild_id}   (owner+) — liste les codes
//!   DELETE /api/invitations/{code}     (owner+) — revoque un code non utilise
//!   POST /api/auth/redeem-invitation   (auth Discord token requis) — consomme code

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::enums::system::role::Role;
use crate::domain::errors::DomainError;

const VALID_ROLES: &[&str] = &["viewer", "moderator", "admin", "owner"];

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// Genere un code aleatoire format XXXX-XXXX-XXXX (12 chars + 2 tirets, 36^12 = 4.7e18 entropy).
fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // sans 0/O/1/I/L pour lisibilite
    let mut rng = rand::thread_rng();
    let mut parts = Vec::with_capacity(3);
    for _ in 0..3 {
        let s: String = (0..4)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect();
        parts.push(s);
    }
    parts.join("-")
}

// ── Generate ──

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

pub async fn create_invitation(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Json(dto): Json<CreateInvitationDto>,
) -> Result<Json<InvitationDto>, ApiError> {
    // Owner+ requis pour generer un code (endpoint scope par guild via body).
    // Le RBAC middleware ne resout pas le role car guild_id est dans le body.
    // On fait le check explicit via require_role_for_guild ou superadmin.
    let is_super = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);
    if !is_super {
        // Sinon resoudre le role pour la guild cible
        validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2",
        )
        .bind(&ctx.discord_user_id)
        .bind(&dto.guild_id)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query role: {e}"))))?;
        let role_str = row.map(|r| r.0).unwrap_or_default();
        let role = Role::from_str(&role_str).unwrap_or(Role::Viewer);
        require_role(&RoleContext { discord_user_id: ctx.discord_user_id.clone(), role: Some(role), guild_id: None }, Role::Owner)
            .map_err(|s| forbid(s, "owner+ requis pour generer une invitation"))?;
    }

    if !VALID_ROLES.contains(&dto.role.as_str()) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "role invalide : {}",
            dto.role
        ))));
    }

    let mut code = generate_code();
    // Retry si collision (extremement improbable avec 4.7e18)
    for _ in 0..5 {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT code FROM invitation_codes WHERE code = $1")
                .bind(&code)
                .fetch_optional(&state.pg_pool)
                .await
                .ok()
                .flatten();
        if exists.is_none() {
            break;
        }
        code = generate_code();
    }

    let expires_at: Option<chrono::DateTime<chrono::Utc>> = match dto.expires_in_hours {
        Some(0) => None, // 0 = pas d'expiration
        Some(h) => Some(chrono::Utc::now() + chrono::Duration::hours(h)),
        None => Some(chrono::Utc::now() + chrono::Duration::hours(168)), // defaut 7j
    };

    sqlx::query(
        "INSERT INTO invitation_codes \
         (code, guild_id, role, created_by, expires_at, notes) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&code)
    .bind(&dto.guild_id)
    .bind(&dto.role)
    .bind(&ctx.discord_user_id)
    .bind(expires_at)
    .bind(&dto.notes)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("insert: {e}"))))?;

    Ok(Json(InvitationDto {
        code: code.clone(),
        guild_id: dto.guild_id,
        role: dto.role,
        created_by: ctx.discord_user_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: expires_at.map(|t| t.to_rfc3339()),
        used_at: None,
        used_by_discord_id: None,
        notes: dto.notes,
        status: "active".to_string(),
    }))
}

// ── List ──

pub async fn list_invitations(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<InvitationDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    // Owner+ ou superadmin
    let is_super = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);
    if !is_super {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2",
        )
        .bind(&ctx.discord_user_id)
        .bind(&guild_id)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query role: {e}"))))?;
        let role = Role::from_str(&row.map(|r| r.0).unwrap_or_default()).unwrap_or(Role::Viewer);
        require_role(&RoleContext { discord_user_id: ctx.discord_user_id.clone(), role: Some(role), guild_id: None }, Role::Owner)
            .map_err(|s| forbid(s, "owner+ requis"))?;
    }

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT code, guild_id, role, created_by, created_at, expires_at, used_at, used_by_discord_id, notes \
         FROM invitation_codes \
         WHERE guild_id = $1 \
         ORDER BY created_at DESC",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("query list: {e}"))))?;

    let now = chrono::Utc::now();
    let out: Vec<InvitationDto> = rows
        .into_iter()
        .map(|(code, guild_id, role, created_by, created_at, expires_at, used_at, used_by, notes)| {
            let status = if used_at.is_some() {
                "used".to_string()
            } else if expires_at.map(|e| e < now).unwrap_or(false) {
                "expired".to_string()
            } else {
                "active".to_string()
            };
            InvitationDto {
                code,
                guild_id,
                role,
                created_by,
                created_at: created_at.to_rfc3339(),
                expires_at: expires_at.map(|t| t.to_rfc3339()),
                used_at: used_at.map(|t| t.to_rfc3339()),
                used_by_discord_id: used_by,
                notes,
                status,
            }
        })
        .collect();
    Ok(Json(out))
}

// ── Revoke (delete unused) ──

pub async fn revoke_invitation(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Recupere la guild associee pour le check role
    let row: Option<(String, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as("SELECT guild_id, used_at FROM invitation_codes WHERE code = $1")
            .bind(&code)
            .fetch_optional(&state.pg_pool)
            .await
            .map_err(|e| ApiError(DomainError::Internal(format!("query: {e}"))))?;
    let Some((guild_id, used_at)) = row else {
        return Err(ApiError(DomainError::NotFound("code introuvable".into())));
    };
    if used_at.is_some() {
        return Err(ApiError(DomainError::Conflict(
            "code deja utilise, ne peut pas etre revoque".into(),
        )));
    }

    let is_super = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);
    if !is_super {
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2",
        )
        .bind(&ctx.discord_user_id)
        .bind(&guild_id)
        .fetch_optional(&state.pg_pool)
        .await
        .ok()
        .flatten();
        let role = Role::from_str(&r.map(|x| x.0).unwrap_or_default()).unwrap_or(Role::Viewer);
        require_role(&RoleContext { discord_user_id: ctx.discord_user_id.clone(), role: Some(role), guild_id: None }, Role::Owner)
            .map_err(|s| forbid(s, "owner+ requis"))?;
    }

    sqlx::query("DELETE FROM invitation_codes WHERE code = $1 AND used_at IS NULL")
        .bind(&code)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("delete: {e}"))))?;

    Ok(Json(serde_json::json!({ "ok": true })))
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
    let code = dto.code.trim().to_uppercase();
    if code.is_empty() {
        return Err(ApiError(DomainError::ValidationError("code vide".into())));
    }

    // Recupere le code
    let row: Option<(String, String, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as(
            "SELECT guild_id, role, expires_at, used_at FROM invitation_codes WHERE code = $1",
        )
        .bind(&code)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("query: {e}"))))?;

    let Some((guild_id, role, expires_at, used_at)) = row else {
        return Err(ApiError(DomainError::NotFound(
            "code invalide ou inexistant".into(),
        )));
    };

    if used_at.is_some() {
        return Err(ApiError(DomainError::Conflict(
            "code deja utilise par un autre utilisateur".into(),
        )));
    }
    if let Some(exp) = expires_at {
        if exp < chrono::Utc::now() {
            return Err(ApiError(DomainError::Conflict(
                "code expire".into(),
            )));
        }
    }

    // Transaction : ajouter au RBAC + marquer code consomme atomique
    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("tx: {e}"))))?;

    // Insert ou update api_user_guilds
    sqlx::query(
        "INSERT INTO api_user_guilds (discord_user_id, guild_id, role, granted_by, granted_at) \
         VALUES ($1, $2, $3, 'invitation', NOW()) \
         ON CONFLICT (discord_user_id, guild_id) DO UPDATE SET \
             role = EXCLUDED.role, \
             granted_by = 'invitation', \
             granted_at = NOW()",
    )
    .bind(&ctx.discord_user_id)
    .bind(&guild_id)
    .bind(&role)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("grant role: {e}"))))?;

    // Marquer le code consomme (atomic check-and-set)
    let updated = sqlx::query(
        "UPDATE invitation_codes SET used_at = NOW(), used_by_discord_id = $2 \
         WHERE code = $1 AND used_at IS NULL",
    )
    .bind(&code)
    .bind(&ctx.discord_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("consume: {e}"))))?;

    if updated.rows_affected() == 0 {
        // Race condition : un autre user a consomme le code entre-temps.
        return Err(ApiError(DomainError::Conflict(
            "race : code consomme par un autre utilisateur".into(),
        )));
    }

    tx.commit()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("commit: {e}"))))?;

    tracing::info!(
        target: "audit::invitation",
        actor = %ctx.discord_user_id,
        guild_id = %guild_id,
        role = %role,
        code = %code,
        "invitation redeemed"
    );

    Ok(Json(RedeemResponse {
        guild_id,
        role,
        message: "Invitation acceptée".to_string(),
    }))
}
