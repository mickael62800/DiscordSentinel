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
use sentinel_core::domain::errors::DomainError;

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
    let rows: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
    Ok(rows.into_iter().map(|(g,)| g).collect())
}

// ── Jobs ────────────────────────────────────────────────────────────────

/// POST /api/analytics/snapshot/daily
///
/// Calcule l'activité journaliere via une **baseline** : on snapshot le
/// `total_user_stats` au début de chaque "jour analytics" (configurable via
/// `analytics.baseline_anchor_hour`, défaut 0 = minuit UTC). Le delta devient
/// `daily_activity[D].messages = total_now - baseline[D].total_messages`.
///
/// Avantage vs ancienne version : daily_activity[D] reste correct meme si
/// le job rate un tick. La baseline d'un jour donne reste figee une fois
/// captee, donc le calcul ne diverge plus jour apres jour.
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
        let track_voice = parse_bool(read_cfg(&state, guild_id, "track_voice_stats").await, true);
        let track_msg = parse_bool(
            read_cfg(&state, guild_id, "track_message_stats").await,
            true,
        );
        // Heure UTC à laquelle la baseline change (0-23). Permet aux admins
        // qui veulent un "jour" qui finit ailleurs qu'à minuit UTC.
        let anchor_hour =
            parse_i64(read_cfg(&state, guild_id, "baseline_anchor_hour").await, 0).clamp(0, 23);

        // Le "stat day" courant : si l'heure UTC >= anchor, on est dans le
        // jour courant ; sinon on est encore dans le jour précédent (la
        // baseline n'a pas encore tourne).
        let stat_day_expr = format!(
            "CASE WHEN EXTRACT(HOUR FROM NOW())::int >= {anchor_hour} \
             THEN CURRENT_DATE ELSE CURRENT_DATE - 1 END"
        );

        // Step 1 : ON CONFLICT DO NOTHING -> on n'écrase JAMAIS une baseline
        // déjà capturée pour ce jour. Premier tick du jour -> baseline figée
        // avec le total cumulatif courant. Ticks suivants -> no-op.
        let baseline_sql = format!(
            "INSERT INTO analytics_daily_baseline (guild_id, day, total_messages, total_voice_seconds) \
             SELECT $1, ({stat_day_expr}), \
                    COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0), \
                    COALESCE((SELECT SUM(voice_seconds) FROM user_stats WHERE guild_id = $1), 0) \
             ON CONFLICT (guild_id, day) DO NOTHING"
        );
        if let Err(e) = sqlx::query(&baseline_sql)
            .bind(guild_id)
            .execute(&state.pg_pool)
            .await
        {
            tracing::warn!(error = %e, guild = %guild_id, "snapshot_daily baseline insert echec");
            continue;
        }

        // Step 2 : delta = total_now - baseline[stat_day].total_*
        let msg_expr = if track_msg {
            format!(
                "GREATEST(COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0) \
                 - COALESCE((SELECT total_messages FROM analytics_daily_baseline WHERE guild_id = $1 AND day = ({stat_day_expr})), 0), 0)"
            )
        } else {
            "0".to_string()
        };
        let voice_expr = if track_voice {
            format!(
                "GREATEST(COALESCE((SELECT SUM(voice_seconds) / 60 FROM user_stats WHERE guild_id = $1), 0) \
                 - COALESCE((SELECT total_voice_seconds / 60 FROM analytics_daily_baseline WHERE guild_id = $1 AND day = ({stat_day_expr})), 0), 0)"
            )
        } else {
            "0".to_string()
        };

        // Les colonnes "active_members / new_members / leaves / infractions /
        // warns / mutes / bans" sont calculees telles quelles — elles sont
        // basées sur des compteurs absolus du jour, pas sur user_stats cumulatif.
        let sql = format!(
            "INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, leaves, infractions, warns, mutes, bans) \
             SELECT $1, ({stat_day_expr}), \
               {msg_expr}, \
               {voice_expr}, \
               COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= ({stat_day_expr})), 0)::integer, \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_join' AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_leave' AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'warn')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'mute')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'ban')::integer, 0) \
             ON CONFLICT (guild_id, day) DO UPDATE SET \
               messages = EXCLUDED.messages, voice_minutes = EXCLUDED.voice_minutes, \
               active_members = EXCLUDED.active_members, new_members = EXCLUDED.new_members, \
               leaves = EXCLUDED.leaves, infractions = EXCLUDED.infractions, \
               warns = EXCLUDED.warns, mutes = EXCLUDED.mutes, bans = EXCLUDED.bans"
        );

        if let Err(e) = sqlx::query(&sql)
            .bind(guild_id)
            .execute(&state.pg_pool)
            .await
        {
            tracing::warn!(error = %e, guild = %guild_id, "snapshot_daily echec");
            continue;
        }
        processed += 1;
    }

    Ok(Json(JobReport {
        guilds_processed: processed,
        guilds_skipped: skipped,
        status: "ok",
    }))
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
        let track_msg = parse_bool(
            read_cfg(&state, guild_id, "track_message_stats").await,
            true,
        );
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
        if let Err(e) = sqlx::query(&sql)
            .bind(guild_id)
            .execute(&state.pg_pool)
            .await
        {
            tracing::warn!(error = %e, guild = %guild_id, "snapshot_hourly echec");
            continue;
        }
        processed += 1;
    }

    Ok(Json(JobReport {
        guilds_processed: processed,
        guilds_skipped: skipped,
        status: "ok",
    }))
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
        // Retention configurable :
        // - data_retention_days  : daily_activity + analytics_daily_baseline (defaut 90j)
        // - hourly_retention_days : hourly_activity (defaut 30j car volumineuse — 24 lignes/jour/guild)
        // 0 ou negatif = illimite, on ne purge pas cette dimension.
        let daily_retention =
            parse_i64(read_cfg(&state, guild_id, "data_retention_days").await, 90);
        let hourly_retention = parse_i64(
            read_cfg(&state, guild_id, "hourly_retention_days").await,
            30,
        );

        if daily_retention <= 0 && hourly_retention <= 0 {
            // tout illimite : on skip
            skipped += 1;
            continue;
        }

        if daily_retention > 0 {
            let r = daily_retention as i32;
            let _ = sqlx::query(
                "DELETE FROM daily_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int",
            )
            .bind(guild_id)
            .bind(r)
            .execute(&state.pg_pool)
            .await
            .map_err(|e| tracing::warn!(error = %e, guild = %guild_id, "retention daily echec"));
            let _ = sqlx::query("DELETE FROM analytics_daily_baseline WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int")
                .bind(guild_id)
                .bind(r)
                .execute(&state.pg_pool)
                .await
                .map_err(|e| tracing::warn!(error = %e, guild = %guild_id, "retention baseline echec"));
        }
        if hourly_retention > 0 {
            let r = hourly_retention as i32;
            let _ = sqlx::query(
                "DELETE FROM hourly_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int",
            )
            .bind(guild_id)
            .bind(r)
            .execute(&state.pg_pool)
            .await
            .map_err(|e| tracing::warn!(error = %e, guild = %guild_id, "retention hourly echec"));
        }
        processed += 1;
    }

    Ok(Json(JobReport {
        guilds_processed: processed,
        guilds_skipped: skipped,
        status: "ok",
    }))
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
        let enabled = parse_bool(
            read_cfg(&state, guild_id, "top_users_publish_enabled").await,
            false,
        );
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
        let interval_days = parse_i64(
            read_cfg(&state, guild_id, "top_users_publish_interval_days").await,
            7,
        );
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
        let min_total =
            parse_i64(read_cfg(&state, guild_id, "low_activity_filter").await, 0).max(0);
        let top = match state
            .analytics_repo
            .get_top_infractors(Some(guild_id), 30, count, min_total)
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

    Ok(Json(JobReport {
        guilds_processed: processed,
        guilds_skipped: skipped,
        status: "ok",
    }))
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
