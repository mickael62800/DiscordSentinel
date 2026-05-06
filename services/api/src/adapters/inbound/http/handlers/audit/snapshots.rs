//! Jobs analytics declenches par le sentinel-worker (snapshots, cleanup,
//! publication Top users). Le metier vit ici pour respecter l'archi
//! hexagonale : le worker se contente de tick et POST, l'API decide quoi
//! faire en lisant la config par guild.
//!
//! Endpoints :
//!   POST /api/analytics/snapshot/daily          → snapshot quotidien
//!   POST /api/analytics/snapshot/hourly         → snapshot horaire
//!   POST /api/analytics/retention-cleanup       → purge donnees > X jours
//!   POST /api/analytics/publish-top-users       → publie embed Top users
//!   GET  /api/analytics/export                  → export daily_activity
//!
//! Lecture de config :
//! Toutes les cles sont stockees sous bot_name='analytics' dans
//! `bot_guild_config`. Les flags (track_voice_stats, track_message_stats,
//! data_retention_days, top_users_count, export_format,
//! top_users_publish_*) sont lus a chaque tick — pas de cache.

use axum::extract::{Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

const ANALYTICS_BOT: &str = "analytics";
/// Sentinel cle utilisee par publish_top_users pour memoriser le dernier
/// post — pas dans le schema UI (pure cle de state).
const LAST_PUBLISH_KEY: &str = "top_users_last_published_at";

#[derive(Serialize)]
pub struct JobReport {
    pub guilds_processed: usize,
    pub guilds_skipped: usize,
    pub status: &'static str,
}

// ── Helpers config ──────────────────────────────────────────────────────

async fn read_cfg(state: &AppState, guild_id: &str, key: &str) -> Option<String> {
    let configs = state
        .bot_config_repo
        .get_config(guild_id, ANALYTICS_BOT)
        .await
        .ok()?;
    configs
        .into_iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value)
}

fn parse_bool(value: Option<String>, default: bool) -> bool {
    match value.as_deref() {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        _ => default,
    }
}

fn parse_i64(value: Option<String>, default: i64) -> i64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

async fn module_enabled(state: &AppState, guild_id: &str) -> bool {
    parse_bool(read_cfg(state, guild_id, "enabled").await, true)
}

async fn list_guild_ids(state: &AppState) -> Result<Vec<String>, ApiError> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(&state.pg_pool)
            .await
            .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
    Ok(rows.into_iter().map(|(g,)| g).collect())
}

// ── Jobs ────────────────────────────────────────────────────────────────

/// POST /api/analytics/snapshot/daily
pub async fn snapshot_daily_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let guilds = list_guild_ids(&state).await?;
    let mut processed = 0;
    let mut skipped = 0;

    for guild_id in &guilds {
        if !module_enabled(&state, guild_id).await {
            skipped += 1;
            continue;
        }
        let track_voice =
            parse_bool(read_cfg(&state, guild_id, "track_voice_stats").await, true);
        let track_msg =
            parse_bool(read_cfg(&state, guild_id, "track_message_stats").await, true);

        let msg_expr = if track_msg {
            "GREATEST(COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0) - COALESCE((SELECT messages FROM daily_activity WHERE guild_id = $1 AND day = CURRENT_DATE - 1), 0), 0)"
        } else {
            "0"
        };
        let voice_expr = if track_voice {
            "GREATEST(COALESCE((SELECT SUM(voice_seconds) / 60 FROM user_stats WHERE guild_id = $1), 0) - COALESCE((SELECT voice_minutes FROM daily_activity WHERE guild_id = $1 AND day = CURRENT_DATE - 1), 0), 0)"
        } else {
            "0"
        };

        let sql = format!(
            "INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, leaves, infractions, warns, mutes, bans) \
             SELECT $1, CURRENT_DATE, \
               {msg_expr}, \
               {voice_expr}, \
               COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= CURRENT_DATE), 0)::integer, \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_join' AND created_at >= CURRENT_DATE)::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_leave' AND created_at >= CURRENT_DATE)::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE)::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'warn')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'mute')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'ban')::integer, 0) \
             ON CONFLICT (guild_id, day) DO UPDATE SET \
               messages = EXCLUDED.messages, voice_minutes = EXCLUDED.voice_minutes, \
               active_members = EXCLUDED.active_members, new_members = EXCLUDED.new_members, \
               leaves = EXCLUDED.leaves, infractions = EXCLUDED.infractions, \
               warns = EXCLUDED.warns, mutes = EXCLUDED.mutes, bans = EXCLUDED.bans"
        );

        if let Err(e) = sqlx::query(&sql).bind(guild_id).execute(&state.pg_pool).await {
            tracing::warn!(error = %e, guild = %guild_id, "snapshot_daily echec");
            continue;
        }
        processed += 1;
    }

    Ok(Json(JobReport { guilds_processed: processed, guilds_skipped: skipped, status: "ok" }))
}

/// POST /api/analytics/snapshot/hourly
pub async fn snapshot_hourly_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let guilds = list_guild_ids(&state).await?;
    let mut processed = 0;
    let mut skipped = 0;

    for guild_id in &guilds {
        if !module_enabled(&state, guild_id).await {
            skipped += 1;
            continue;
        }
        let track_msg =
            parse_bool(read_cfg(&state, guild_id, "track_message_stats").await, true);
        let msg_expr = if track_msg {
            "COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= date_trunc('hour', NOW())), 0)::bigint"
        } else {
            "0::bigint"
        };
        let sql = format!(
            "INSERT INTO hourly_activity (guild_id, day, hour, messages, infractions) \
             SELECT $1, CURRENT_DATE, EXTRACT(HOUR FROM NOW())::smallint, \
               {msg_expr}, \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= date_trunc('hour', NOW()))::integer, 0) \
             ON CONFLICT (guild_id, day, hour) DO UPDATE SET \
               messages = EXCLUDED.messages, infractions = EXCLUDED.infractions"
        );
        if let Err(e) = sqlx::query(&sql).bind(guild_id).execute(&state.pg_pool).await {
            tracing::warn!(error = %e, guild = %guild_id, "snapshot_hourly echec");
            continue;
        }
        processed += 1;
    }

    Ok(Json(JobReport { guilds_processed: processed, guilds_skipped: skipped, status: "ok" }))
}

/// POST /api/analytics/retention-cleanup
pub async fn retention_cleanup_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let guilds = list_guild_ids(&state).await?;
    let mut processed = 0;
    let mut skipped = 0;

    for guild_id in &guilds {
        if !module_enabled(&state, guild_id).await {
            skipped += 1;
            continue;
        }
        let retention =
            parse_i64(read_cfg(&state, guild_id, "data_retention_days").await, 90);
        if retention <= 0 {
            // 0 = illimite : ne purge rien
            skipped += 1;
            continue;
        }

        let r = retention as i32;
        let _ = sqlx::query("DELETE FROM daily_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int")
            .bind(guild_id)
            .bind(r)
            .execute(&state.pg_pool)
            .await
            .map_err(|e| tracing::warn!(error = %e, guild = %guild_id, "retention daily echec"));
        let _ = sqlx::query("DELETE FROM hourly_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int")
            .bind(guild_id)
            .bind(r)
            .execute(&state.pg_pool)
            .await
            .map_err(|e| tracing::warn!(error = %e, guild = %guild_id, "retention hourly echec"));
        processed += 1;
    }

    Ok(Json(JobReport { guilds_processed: processed, guilds_skipped: skipped, status: "ok" }))
}

/// POST /api/analytics/publish-top-users
///
/// Pour chaque guild ou le module + la publication sont actifs :
///   - skip si pas de salon configure
///   - skip si l intervalle minimal n'est pas ecoule depuis le dernier post
///   - sinon : calcule top via analytics_repo + post embed Discord
///   - met a jour `top_users_last_published_at` (cle de state, hors schema UI)
pub async fn publish_top_users_all(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let guilds = list_guild_ids(&state).await?;
    let mut processed = 0;
    let mut skipped = 0;
    let now = chrono::Utc::now();

    if state.discord_bot_token.is_empty() {
        return Err(ApiError(DomainError::Internal(
            "SENTINEL_DISCORD_TOKEN non configure — publication impossible".into(),
        )));
    }

    let client = reqwest::Client::new();

    for guild_id in &guilds {
        if !module_enabled(&state, guild_id).await {
            skipped += 1;
            continue;
        }
        let enabled =
            parse_bool(read_cfg(&state, guild_id, "top_users_publish_enabled").await, false);
        if !enabled {
            skipped += 1;
            continue;
        }
        let channel_id = match read_cfg(&state, guild_id, "top_users_publish_channel_id").await {
            Some(s) if !s.is_empty() => s,
            _ => {
                skipped += 1;
                continue;
            }
        };
        let interval_days =
            parse_i64(read_cfg(&state, guild_id, "top_users_publish_interval_days").await, 7);
        if let Some(s) = read_cfg(&state, guild_id, LAST_PUBLISH_KEY).await {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&s) {
                let elapsed = now.signed_duration_since(last.with_timezone(&chrono::Utc));
                if elapsed < chrono::Duration::days(interval_days) {
                    skipped += 1;
                    continue;
                }
            }
        }

        let count = parse_i64(read_cfg(&state, guild_id, "top_users_count").await, 10);
        let top = match state
            .analytics_repo
            .get_top_infractors(Some(guild_id), 30, count)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, guild = %guild_id, "publish_top_users: get_top_infractors echec");
                continue;
            }
        };

        let mut description = String::new();
        for (i, t) in top.iter().enumerate() {
            description.push_str(&format!(
                "**{}.** <@{}> — {} infractions ({}w / {}m / {}b)\n",
                i + 1,
                t.user_id,
                t.total_infractions,
                t.warns,
                t.mutes,
                t.bans
            ));
        }
        if description.is_empty() {
            description.push_str("_Aucune infraction sur les 30 derniers jours._");
        }

        let embed = serde_json::json!({
            "title": format!("Top {} infracteurs (30j)", count),
            "description": description,
            "color": 0xED4245u32,
            "timestamp": now.to_rfc3339(),
        });

        let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");
        let resp = match client
            .post(&url)
            .header("Authorization", format!("Bot {}", state.discord_bot_token))
            .json(&serde_json::json!({ "embeds": [embed] }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, guild = %guild_id, "publish_top_users: send echec");
                continue;
            }
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(guild = %guild_id, status = %status, body = %body, "publish_top_users: Discord refus");
            continue;
        }

        if let Err(e) = state
            .bot_config_repo
            .set_config(guild_id, ANALYTICS_BOT, LAST_PUBLISH_KEY, &now.to_rfc3339())
            .await
        {
            tracing::warn!(error = %e, guild = %guild_id, "publish_top_users: persist last echec");
        }
        processed += 1;
    }

    Ok(Json(JobReport { guilds_processed: processed, guilds_skipped: skipped, status: "ok" }))
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
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    if params.guild_id.is_empty() {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id requis".into(),
        )));
    }
    let days = params.days.unwrap_or(30).clamp(1, 365);

    let format = match params.format {
        Some(f) if !f.is_empty() => f,
        _ => read_cfg(&state, &params.guild_id, "export_format")
            .await
            .unwrap_or_else(|| "json".into()),
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
                    a.day, a.messages, a.voice_minutes, a.active_members,
                    a.new_members, a.leaves, a.infractions, a.warns, a.mutes, a.bans
                ));
            }
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (header::CONTENT_DISPOSITION, "attachment; filename=\"analytics.csv\""),
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
