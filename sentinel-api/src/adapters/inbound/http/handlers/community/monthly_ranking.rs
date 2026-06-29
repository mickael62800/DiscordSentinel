//! Publication du classement mensuel d'activite (texte / vocal / global) sur
//! Discord. Declenche par le sentinel-worker (tick), l'API decide quoi faire
//! en lisant la config par guild (`progression-bot`).
//!
//! Modele "baseline" : `user_levels` ne stocke que l'XP CUMULEE. On capture au
//! debut de chaque mois une baseline (snapshot) de l'XP cumulee. Le classement
//! du mois ecoule = XP actuelle - baseline du debut de ce mois.
//!
//! Idempotence : tant que la baseline du mois courant existe, on ne refait
//! rien. Au passage de mois, on (1) publie le mois precedent s'il a une
//! baseline COMPLETE (non `partial`), puis (2) pose la baseline du mois courant.
//!
//! Option A : la toute premiere baseline (posee en cours de mois) est `partial`
//! et ne sera jamais publiee ; seul le premier mois entierement couvert l'est.

use axum::extract::State;
use axum::Json;
use chrono::Datelike;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

const PROGRESSION_BOT: &str = "progression-bot";
const LAST_PERIOD_KEY: &str = "monthly_ranking_last_period";

#[derive(Serialize)]
pub struct MonthlyRankingReport {
    pub guilds_published: usize,
    pub guilds_baselined: usize,
    pub guilds_skipped: usize,
    pub status: &'static str,
}

async fn read_cfg(state: &AppState, guild_id: &str, key: &str) -> Option<String> {
    let configs = state
        .bot_config_repo
        .get_config(guild_id, PROGRESSION_BOT)
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

fn period_string(year: i32, month: u32) -> String {
    format!("{:04}-{:02}", year, month)
}

fn month_label_fr(period: &str) -> String {
    const MOIS: [&str; 12] = [
        "Janvier",
        "Fevrier",
        "Mars",
        "Avril",
        "Mai",
        "Juin",
        "Juillet",
        "Aout",
        "Septembre",
        "Octobre",
        "Novembre",
        "Decembre",
    ];
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(y), Ok(m)) = (parts[0].parse::<i32>(), parts[1].parse::<usize>()) {
            if (1..=12).contains(&m) {
                return format!("{} {}", MOIS[m - 1], y);
            }
        }
    }
    period.to_string()
}

/// Construit un bloc de classement (top N, deltas > 0 uniquement).
fn build_ranking_block(mut rows: Vec<(String, i64)>, top: usize) -> String {
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    let lines: Vec<String> = rows
        .into_iter()
        .filter(|(_, xp)| *xp > 0)
        .take(top)
        .enumerate()
        .map(|(i, (uid, xp))| format!("**{}.** <@{}> — {} XP", i + 1, uid, xp))
        .collect();
    if lines.is_empty() {
        "_Aucune activite ce mois-ci._".to_string()
    } else {
        lines.join("\n")
    }
}

/// POST /api/analytics/publish-monthly-ranking
pub async fn publish_monthly_ranking_all(
    State(state): State<AppState>,
) -> Result<Json<MonthlyRankingReport>, ApiError> {
    let now = chrono::Utc::now();
    let this_period = period_string(now.year(), now.month());
    let (py, pm) = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };
    let prev_period = period_string(py, pm);
    let baseline_partial = now.day() != 1;

    let guilds: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let mut published = 0usize;
    let mut baselined = 0usize;
    let mut skipped = 0usize;

    let client = reqwest::Client::new();

    for (guild_id,) in &guilds {
        // Module + feature actives ?
        if !parse_bool(read_cfg(&state, guild_id, "enabled").await, true) {
            skipped += 1;
            continue;
        }
        if !parse_bool(
            read_cfg(&state, guild_id, "monthly_ranking_enabled").await,
            false,
        ) {
            skipped += 1;
            continue;
        }

        // Baseline du mois courant deja posee -> rien a faire ce mois-ci.
        let has_this: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym = $2)",
        )
        .bind(guild_id)
        .bind(&this_period)
        .fetch_one(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
        if has_this {
            skipped += 1;
            continue;
        }

        // Le mois precedent a-t-il une baseline COMPLETE (publiable) ?
        let prev_partial: Option<bool> = sqlx::query_scalar(
            "SELECT bool_or(partial) FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym = $2",
        )
        .bind(guild_id)
        .bind(&prev_period)
        .fetch_one(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

        if matches!(prev_partial, Some(false)) {
            // Publication du classement du mois precedent.
            if let Some(channel_id) = read_cfg(&state, guild_id, "monthly_ranking_channel_id")
                .await
                .filter(|s| !s.is_empty())
            {
                let top = parse_i64(
                    read_cfg(&state, guild_id, "monthly_ranking_top_count").await,
                    10,
                )
                .clamp(1, 25) as usize;

                let rows: Vec<(String, i64, i64)> = sqlx::query_as(
                    r#"SELECT ul.user_id,
                              (ul.xp_text  - COALESCE(s.xp_text, 0))  AS d_text,
                              (ul.xp_voice - COALESCE(s.xp_voice, 0)) AS d_voice
                       FROM user_levels ul
                       LEFT JOIN user_levels_monthly_snapshot s
                         ON s.guild_id = ul.guild_id AND s.user_id = ul.user_id AND s.period_ym = $2
                       WHERE ul.guild_id = $1"#,
                )
                .bind(guild_id)
                .bind(&prev_period)
                .fetch_all(&state.pg_pool)
                .await
                .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

                let text_block = build_ranking_block(
                    rows.iter().map(|(u, t, _)| (u.clone(), *t)).collect(),
                    top,
                );
                let voice_block = build_ranking_block(
                    rows.iter().map(|(u, _, v)| (u.clone(), *v)).collect(),
                    top,
                );
                let global_block = build_ranking_block(
                    rows.iter().map(|(u, t, v)| (u.clone(), t + v)).collect(),
                    top,
                );

                let embed = serde_json::json!({
                    "title": format!("\u{1f3c6} Classement de {}", month_label_fr(&prev_period)),
                    "description": "Les membres les plus actifs du mois \u{2014} bravo \u{1f44f}",
                    "color": 0xF1C40Fu32,
                    "fields": [
                        { "name": "\u{1f4dd} Top Texte", "value": text_block, "inline": false },
                        { "name": "\u{1f399}\u{fe0f} Top Vocal", "value": voice_block, "inline": false },
                        { "name": "\u{1f3c5} Top Global", "value": global_block, "inline": false }
                    ],
                    "timestamp": now.to_rfc3339(),
                });

                if state.discord_bot_token.is_empty() {
                    tracing::warn!(guild = %guild_id, "publish_monthly_ranking: SENTINEL_DISCORD_TOKEN absent");
                } else {
                    let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");
                    match client
                        .post(&url)
                        .header("Authorization", format!("Bot {}", state.discord_bot_token))
                        .json(&serde_json::json!({ "embeds": [embed] }))
                        .send()
                        .await
                    {
                        Ok(r) if r.status().is_success() => {
                            published += 1;
                            let _ = state
                                .bot_config_repo
                                .set_config(
                                    guild_id,
                                    PROGRESSION_BOT,
                                    LAST_PERIOD_KEY,
                                    &prev_period,
                                )
                                .await;
                        }
                        Ok(r) => {
                            let status = r.status();
                            let body = r.text().await.unwrap_or_default();
                            tracing::warn!(guild = %guild_id, %status, body = %body, "publish_monthly_ranking: Discord refus");
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, guild = %guild_id, "publish_monthly_ranking: send echec");
                        }
                    }
                }
            }
        }

        // Pose la baseline du mois courant (idempotent).
        sqlx::query(
            r#"INSERT INTO user_levels_monthly_snapshot (guild_id, user_id, period_ym, xp_text, xp_voice, partial)
               SELECT guild_id, user_id, $2, xp_text, xp_voice, $3
               FROM user_levels WHERE guild_id = $1
               ON CONFLICT (guild_id, user_id, period_ym) DO NOTHING"#,
        )
        .bind(guild_id)
        .bind(&this_period)
        .bind(baseline_partial)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
        baselined += 1;
    }

    Ok(Json(MonthlyRankingReport {
        guilds_published: published,
        guilds_baselined: baselined,
        guilds_skipped: skipped,
        status: "ok",
    }))
}
