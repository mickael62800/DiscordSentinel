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
use axum::{Extension, Json};
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
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

/// Recupere les deltas d'XP (texte / vocal) par membre pour une periode donnee.
///
/// Delta = XP cumulee actuelle - baseline du snapshot `baseline_period_ym`.
/// Si aucune baseline n'existe pour cette periode, le LEFT JOIN + COALESCE(0)
/// fait retomber le calcul sur l'XP cumulee totale (fallback "cumul total").
///
/// Partagee entre le job auto (`publish_monthly_ranking_all`) et la publication
/// forcee (`force_publish_monthly_ranking`) : la SQL n'est pas dupliquee.
async fn fetch_ranking_deltas(
    state: &AppState,
    guild_id: &str,
    baseline_period_ym: &str,
) -> Result<Vec<(String, i64, i64)>, ApiError> {
    // Roles exclus du classement (CSV d'IDs). Les membres portant un de ces
    // roles (colonne guild_members.roles, JSONB array d'IDs) sont ecartes.
    // Un array vide n'exclut personne (l'operateur `?|` renvoie false).
    let excluded_roles: Vec<String> = read_cfg(state, guild_id, "monthly_ranking_excluded_roles")
        .await
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    sqlx::query_as(
        r#"SELECT ul.user_id,
                  (ul.xp_text  - COALESCE(s.xp_text, 0))  AS d_text,
                  (ul.xp_voice - COALESCE(s.xp_voice, 0)) AS d_voice
           FROM user_levels ul
           LEFT JOIN user_levels_monthly_snapshot s
             ON s.guild_id = ul.guild_id AND s.user_id = ul.user_id AND s.period_ym = $2
           WHERE ul.guild_id = $1
             AND NOT EXISTS (
               SELECT 1 FROM guild_members gm
               WHERE gm.guild_id = ul.guild_id
                 AND gm.user_id = ul.user_id
                 AND gm.roles ?| $3::text[]
             )"#,
    )
    .bind(guild_id)
    .bind(baseline_period_ym)
    .bind(&excluded_roles)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(e.to_string())))
}

// ── Publication forcee (commande admin `/classement forcer`) ──

#[derive(Deserialize)]
pub struct ForceRankingRequest {
    pub guild_id: String,
    /// "actuel" (mois en cours, defaut) | "precedent" (mois complet ecoule).
    #[serde(default)]
    pub mois: Option<String>,
}

#[derive(Serialize)]
pub struct RankingEntry {
    pub user_id: String,
    pub xp: i64,
}

#[derive(Serialize)]
pub struct ForceRankingResponse {
    pub period_label: String,
    pub note: Option<String>,
    pub text: Vec<RankingEntry>,
    pub voice: Vec<RankingEntry>,
    pub global: Vec<RankingEntry>,
}

/// Top N (deltas > 0), trie decroissant, selon la cle donnee (texte / vocal /
/// global). Renvoie une liste structuree pour que le bot fasse le rendu.
fn top_entries(
    rows: &[(String, i64, i64)],
    top: usize,
    key: impl Fn(i64, i64) -> i64,
) -> Vec<RankingEntry> {
    let mut entries: Vec<RankingEntry> = rows
        .iter()
        .map(|(uid, t, v)| RankingEntry {
            user_id: uid.clone(),
            xp: key(*t, *v),
        })
        .filter(|e| e.xp > 0)
        .collect();
    entries.sort_by(|a, b| b.xp.cmp(&a.xp));
    entries.truncate(top);
    entries
}

/// POST /api/analytics/force-monthly-ranking
///
/// Publication FORCEE a la demande : bypass les gates
/// `monthly_ranking_enabled` / baseline presente / `partial`. Ne poste PAS sur
/// Discord (contrairement au job auto) : renvoie les donnees au bot qui rend
/// l'embed et le poste dans le salon configure OU le salon d'invocation.
///
/// RBAC : `Admin` sur la guild (pass-through pour les appels bot/internes).
pub async fn force_publish_monthly_ranking(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(req): Json<ForceRankingRequest>,
) -> Result<Json<ForceRankingResponse>, ApiError> {
    check_role_for_guild(
        &state,
        &rbac,
        &req.guild_id,
        Role::Admin,
        "admin+ requis pour forcer la publication du classement mensuel",
    )
    .await?;

    let now = chrono::Utc::now();
    let this_period = period_string(now.year(), now.month());
    let (py, pm) = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };
    let prev_period = period_string(py, pm);

    let mois = req.mois.as_deref().unwrap_or("actuel");
    // period_ym = mois affiche ; baseline_period_ym = snapshot de reference.
    let (period_ym, baseline_period_ym) = match mois {
        "precedent" => (prev_period.clone(), prev_period.clone()),
        _ => (this_period.clone(), this_period.clone()),
    };

    // Fallback cumul total : si pas de baseline pour la periode, la SQL retombe
    // deja sur l'XP cumulee (COALESCE 0). On signale juste via `note`.
    let has_baseline: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym = $2)",
    )
    .bind(&req.guild_id)
    .bind(&baseline_period_ym)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let note = if has_baseline {
        None
    } else {
        Some("(cumul total \u{2014} pas de baseline ce mois)".to_string())
    };

    let rows = fetch_ranking_deltas(&state, &req.guild_id, &baseline_period_ym).await?;

    let top = parse_i64(
        read_cfg(&state, &req.guild_id, "monthly_ranking_top_count").await,
        10,
    )
    .clamp(1, 25) as usize;

    Ok(Json(ForceRankingResponse {
        period_label: month_label_fr(&period_ym),
        note,
        text: top_entries(&rows, top, |t, _| t),
        voice: top_entries(&rows, top, |_, v| v),
        global: top_entries(&rows, top, |t, v| t + v),
    }))
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

                let rows = fetch_ranking_deltas(&state, guild_id, &prev_period).await?;

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

        // `partial` fonde sur la CONTINUITE, pas sur le jour du mois : une
        // baseline n'est "partielle" (donc jamais publiee) que si c'est la
        // toute premiere de ce serveur (aucune periode anterieure). Ainsi un
        // tick en retard (worker down le 1er du mois, activation en cours de
        // mois deja couverte) ne fait plus rater la publication : un mois se
        // publie des lors qu'il a un predecesseur baseline.
        let has_prior: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym < $2)",
        )
        .bind(guild_id)
        .bind(&this_period)
        .fetch_one(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
        let baseline_partial = !has_prior;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_label_fr_formats_period() {
        assert_eq!(month_label_fr("2026-07"), "Juillet 2026");
        assert_eq!(month_label_fr("2026-01"), "Janvier 2026");
        // Periode invalide -> renvoyee telle quelle.
        assert_eq!(month_label_fr("bogus"), "bogus");
    }

    #[test]
    fn period_string_zero_pads() {
        assert_eq!(period_string(2026, 7), "2026-07");
        assert_eq!(period_string(2026, 12), "2026-12");
    }

    #[test]
    fn top_entries_sorts_filters_and_truncates() {
        let rows = vec![
            ("a".to_string(), 10, 5),
            ("b".to_string(), 30, 0),
            ("c".to_string(), 0, 40),
            ("d".to_string(), -5, 0), // delta negatif ignore sur texte
        ];

        let text = top_entries(&rows, 2, |t, _| t);
        assert_eq!(text.len(), 2);
        assert_eq!(text[0].user_id, "b");
        assert_eq!(text[0].xp, 30);
        assert_eq!(text[1].user_id, "a");

        let global = top_entries(&rows, 10, |t, v| t + v);
        // c = 40, b = 30, a = 15 ; d = -5 filtre.
        assert_eq!(global.len(), 3);
        assert_eq!(global[0].user_id, "c");
        assert_eq!(global[0].xp, 40);
    }

    #[test]
    fn top_entries_empty_when_no_positive() {
        let rows = vec![("a".to_string(), 0, 0), ("b".to_string(), -1, -1)];
        assert!(top_entries(&rows, 10, |t, v| t + v).is_empty());
    }
}
