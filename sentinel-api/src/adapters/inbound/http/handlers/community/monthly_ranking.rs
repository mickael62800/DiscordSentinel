//! Publication du classement mensuel d'activite (texte / vocal / global) sur
//! Discord. Adaptateur ENTRANT mince : RBAC + parse + envoi Discord. Toute la
//! regle metier (gates, deltas, assemblage des tops, baselines) vit dans
//! `ManageMonthlyRankingUseCase` ; le SQL dans `MonthlyRankingRepository`.

use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;

#[derive(Serialize)]
pub struct MonthlyRankingReport {
    pub guilds_published: usize,
    pub guilds_baselined: usize,
    pub guilds_skipped: usize,
    pub status: &'static str,
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

fn map_entries(
    entries: Vec<sentinel_core::domain::entities::community::monthly_ranking::RankingEntry>,
) -> Vec<RankingEntry> {
    entries
        .into_iter()
        .map(|e| RankingEntry {
            user_id: e.user_id,
            xp: e.xp,
        })
        .collect()
}

/// POST /api/analytics/force-monthly-ranking
///
/// Publication FORCEE a la demande : bypass les gates. Ne poste PAS sur Discord
/// (contrairement au job auto) : renvoie les donnees au bot qui rend l'embed.
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

    let data = state
        .monthly_ranking_uc
        .force_ranking(&req.guild_id, req.mois)
        .await?;

    Ok(Json(ForceRankingResponse {
        period_label: data.period_label,
        note: data.note,
        text: map_entries(data.text),
        voice: map_entries(data.voice),
        global: map_entries(data.global),
    }))
}

/// POST /api/analytics/publish-monthly-ranking
///
/// Job auto (tick worker) : le use case applique les gates + pose les baselines
/// et renvoie le plan des classements a poster ; le handler poste sur Discord et
/// notifie le use case (memorisation de la periode publiee).
pub async fn publish_monthly_ranking_all(
    State(state): State<AppState>,
) -> Result<Json<MonthlyRankingReport>, ApiError> {
    let plan = state.monthly_ranking_uc.plan_and_baseline().await?;

    let now = chrono::Utc::now();
    let client = reqwest::Client::new();
    let mut published = 0usize;

    for item in &plan.publications {
        if state.discord_bot_token.is_empty() {
            tracing::warn!(guild = %item.guild_id, "publish_monthly_ranking: SENTINEL_DISCORD_TOKEN absent");
            continue;
        }

        let embed = serde_json::json!({
            "title": format!("\u{1f3c6} Classement de {}", item.period_label),
            "description": "Les membres les plus actifs du mois \u{2014} bravo \u{1f44f}",
            "color": 0xF1C40Fu32,
            "fields": [
                { "name": "\u{1f4dd} Top Texte", "value": item.text_block, "inline": false },
                { "name": "\u{1f399}\u{fe0f} Top Vocal", "value": item.voice_block, "inline": false },
                { "name": "\u{1f3c5} Top Global", "value": item.global_block, "inline": false }
            ],
            "timestamp": now.to_rfc3339(),
        });

        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            item.channel_id
        );
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
                    .monthly_ranking_uc
                    .mark_published(&item.guild_id, &item.period)
                    .await;
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                tracing::warn!(guild = %item.guild_id, %status, body = %body, "publish_monthly_ranking: Discord refus");
            }
            Err(e) => {
                tracing::warn!(error = %e, guild = %item.guild_id, "publish_monthly_ranking: send echec");
            }
        }
    }

    Ok(Json(MonthlyRankingReport {
        guilds_published: published,
        guilds_baselined: plan.baselined,
        guilds_skipped: plan.skipped,
        status: "ok",
    }))
}
