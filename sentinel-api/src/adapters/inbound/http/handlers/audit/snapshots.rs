//! Jobs analytics declenches par le sentinel-worker (snapshots, cleanup,
//! publication Top users). Le metier vit dans le use case `ManageSnapshotsUseCase`
//! (archi hexagonale) : le worker se contente de tick et POST, ces handlers ne
//! font que RBAC/validation, appeler le use case et — pour Top users — poster
//! l'embed Discord.
//!
//! Endpoints :
//!   POST /api/analytics/snapshot/daily          → snapshot quotidien
//!   POST /api/analytics/snapshot/hourly         → snapshot horaire
//!   POST /api/analytics/retention-cleanup       → purge donnees > X jours
//!   POST /api/analytics/publish-top-users       → publie embed Top users
//!   GET  /api/analytics/export                  → export daily_activity

use axum::extract::{Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::audit::snapshot::JobReport;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

const ANALYTICS_BOT: &str = "analytics";

// ── Jobs ────────────────────────────────────────────────────────────────

/// POST /api/analytics/snapshot/daily
pub async fn snapshot_daily_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.snapshot_daily_all().await?))
}

/// POST /api/analytics/snapshot/hourly
pub async fn snapshot_hourly_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.snapshot_hourly_all().await?))
}

/// POST /api/analytics/retention-cleanup
pub async fn retention_cleanup_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.retention_cleanup_all().await?))
}

/// POST /api/analytics/publish-top-users
///
/// Le use case calcule les publications dues (config par guild, intervalle,
/// top infracteurs). Ce handler poste l'embed Discord (concern inbound) et
/// persiste l'horodatage via le use case apres un post reussi.
pub async fn publish_top_users_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    if state.discord_bot_token.is_empty() {
        return Err(ApiError(DomainError::Internal(
            "SENTINEL_DISCORD_TOKEN non configure — publication impossible".into(),
        )));
    }

    let plan = state.snapshots_uc.plan_top_publications().await?;
    let client = reqwest::Client::new();
    let mut processed = 0;

    for pub_ in &plan.publications {
        // Securite : channel_id vient de la config guild (DB). On le valide comme
        // snowflake numerique avant de l'interpoler dans l'URL Discord, pour
        // qu'un id malforme (`../`, %2F...) ne puisse pas atteindre un autre
        // endpoint de l'API Discord avec le bot token.
        if crate::adapters::inbound::http::validation::validate_discord_id(
            "channel_id",
            &pub_.channel_id,
        )
        .is_err()
        {
            continue;
        }
        let embed = serde_json::json!({
            "title": pub_.title,
            "description": pub_.description,
            "color": pub_.color,
            "timestamp": pub_.published_at,
        });
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            pub_.channel_id
        );
        let resp = match client
            .post(&url)
            .header("Authorization", format!("Bot {}", state.discord_bot_token))
            .json(&serde_json::json!({ "embeds": [embed] }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, guild = %pub_.guild_id, "publish_top_users: send echec");
                continue;
            }
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(guild = %pub_.guild_id, status = %status, body = %body, "publish_top_users: Discord refus");
            continue;
        }

        if let Err(e) = state
            .snapshots_uc
            .mark_top_published(&pub_.guild_id, &pub_.published_at)
            .await
        {
            tracing::warn!(error = %e, guild = %pub_.guild_id, "publish_top_users: persist last echec");
        }
        processed += 1;
    }

    Ok(Json(JobReport::ok(processed, plan.skipped)))
}

// ── Export ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExportQuery {
    pub guild_id: String,
    pub days: Option<i32>,
    /// "json" | "csv". Si absent, fallback sur la cle `export_format` du guild.
    pub format: Option<String>,
}

/// GET /api/analytics/export?guild_id=...&days=N&format=json|csv
pub async fn export_analytics(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    if params.guild_id.is_empty() {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id requis".into(),
        )));
    }
    // IDOR : export cross-serveur de l'activite (messages/vocal/infractions).
    check_role_for_guild(
        &state,
        &rbac,
        &params.guild_id,
        Role::Moderator,
        "moderator+ requis pour exporter les analytics",
    )
    .await?;
    let days = crate::adapters::inbound::http::helpers::normalize_in(params.days, 30, 1, 365);

    let format = match params.format {
        Some(f) if !f.is_empty() => f,
        _ => read_export_format(&state, &params.guild_id).await,
    };
    let format = format.to_lowercase();

    let activities = state
        .daily_activity_repo
        .get_activity(Some(&params.guild_id), days)
        .await?;

    match format.as_str() {
        "csv" => {
            let mut out = String::from("day,messages,voice_minutes,active_members,new_members,leaves,infractions,warns,mutes,bans\n");
            for a in &activities {
                out.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{}\n",
                    a.day,
                    a.messages,
                    a.voice_minutes,
                    a.active_members,
                    a.new_members,
                    a.leaves,
                    a.infractions,
                    a.warns,
                    a.mutes,
                    a.bans
                ));
            }
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"analytics.csv\"",
                    ),
                ],
                out,
            )
                .into_response())
        }
        _ => {
            #[derive(Serialize)]
            struct Row {
                day: String,
                messages: i64,
                voice_minutes: i64,
                active_members: i32,
                new_members: i32,
                leaves: i32,
                infractions: i32,
                warns: i32,
                mutes: i32,
                bans: i32,
            }
            let rows: Vec<Row> = activities
                .into_iter()
                .map(|a| Row {
                    day: a.day.to_string(),
                    messages: a.messages,
                    voice_minutes: a.voice_minutes,
                    active_members: a.active_members,
                    new_members: a.new_members,
                    leaves: a.leaves,
                    infractions: a.infractions,
                    warns: a.warns,
                    mutes: a.mutes,
                    bans: a.bans,
                })
                .collect();
            Ok(Json(rows).into_response())
        }
    }
}

/// Lit la cle `export_format` du guild (fallback "json"). Concern handler : ne
/// touche pas au SQL (passe par le repo de config).
async fn read_export_format(state: &AppState, guild_id: &str) -> String {
    state
        .bot_config_repo
        .get_config(guild_id, ANALYTICS_BOT)
        .await
        .ok()
        .and_then(|cfgs| {
            cfgs.into_iter()
                .find(|c| c.config_key == "export_format")
                .map(|c| c.config_value)
        })
        .unwrap_or_else(|| "json".into())
}
