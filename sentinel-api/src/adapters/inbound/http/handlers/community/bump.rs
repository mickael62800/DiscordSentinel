//! Bump rewards : recompense graduee (selon le nombre de bumps de la semaine)
//! quand un membre fait /bump (Disboard), + pilotage du rappel apres cooldown.
//!
//! Localise (raw sqlx + wallet_uc) : la table bump_events est un simple journal
//! et la recompense est un calcul pur ; pas de regle metier transverse a isoler.

use axum::extract::{Path, State};
use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

/// Recompense graduee : 1er bump = base ; chaque bump suppl. de la semaine
/// ajoute `step` ; plafonnee a `max`. `n` = Nieme bump de la semaine (>=1).
fn bump_reward(n: i64, base: i64, step: i64, max: i64) -> i64 {
    let raw = base + (n - 1).max(0) * step;
    raw.clamp(0, max.max(base))
}

fn cfg_str<'a>(
    entries: &'a [sentinel_core::domain::entities::system::bot_config::BotGuildConfig],
    key: &str,
) -> Option<&'a str> {
    entries.iter().find(|e| e.config_key == key).map(|e| e.config_value.as_str())
}
fn cfg_bool(entries: &[sentinel_core::domain::entities::system::bot_config::BotGuildConfig], key: &str, d: bool) -> bool {
    cfg_str(entries, key).map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")).unwrap_or(d)
}
fn cfg_i64(entries: &[sentinel_core::domain::entities::system::bot_config::BotGuildConfig], key: &str, d: i64) -> i64 {
    cfg_str(entries, key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(d)
}

#[derive(Debug, Deserialize)]
pub struct RecordBumpBody {
    #[serde(default)]
    pub username: String,
    /// Salon ou Disboard a poste (fallback si bump_channel_id non configure).
    #[serde(default)]
    pub channel_id: String,
}

#[derive(Debug, Serialize)]
pub struct BumpRewardDto {
    pub rewarded: bool,
    pub reward: i64,
    pub weekly_count: i64,
    pub new_balance: Option<i64>,
    /// Role VIP a attribuer (le bot fait l'ajout Discord, idempotent). `None`
    /// si la feature est desactivee ou le seuil de bumps pas encore atteint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vip_role_id: Option<String>,
    /// `true` uniquement au bump qui fait FRANCHIR le seuil (pour annoncer le
    /// passage VIP une seule fois). `false` ensuite (role deja debloque).
    #[serde(default)]
    pub vip_just_unlocked: bool,
}

/// POST /api/bump/{guild_id}/{user_id} Ã¢â‚¬â€ enregistre un bump, calcule la
/// recompense graduee de la semaine et credite le wallet.
pub async fn record_bump(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(body): Json<RecordBumpBody>,
) -> Result<Json<BumpRewardDto>, ApiError> {
    let cfg = state.bot_config_repo.get_config(&guild_id, "bump-bot").await.unwrap_or_default();
    if !cfg_bool(&cfg, "enabled", false) {
        return Ok(Json(BumpRewardDto { rewarded: false, reward: 0, weekly_count: 0, new_balance: None, vip_role_id: None, vip_just_unlocked: false }));
    }
    let base = cfg_i64(&cfg, "bump_reward_base", 100).max(0);
    let step = cfg_i64(&cfg, "bump_reward_step", 50).max(0);
    let max = cfg_i64(&cfg, "bump_reward_max", 500).max(0);
    let cooldown = cfg_i64(&cfg, "bump_cooldown_minutes", 120).clamp(1, 1440);
    let reminder_enabled = cfg_bool(&cfg, "bump_reminder_enabled", true);
    let channel = {
        let c = cfg_str(&cfg, "bump_channel_id").unwrap_or("").trim().to_string();
        if c.is_empty() { body.channel_id.clone() } else { c }
    };

    // Nieme bump de la semaine (fenetre glissante 7 jours).
    let week_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bump_events \
         WHERE guild_id = $1 AND user_id = $2 AND bumped_at >= NOW() - INTERVAL '7 days'",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(sqlx_internal("bump count"))?;
    let n = week_count + 1;
    let reward = bump_reward(n, base, step, max);

    sqlx::query(
        "INSERT INTO bump_events (guild_id, user_id, username, reward_coins, weekly_index) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&body.username)
    .bind(reward as i32)
    .bind(n as i32)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("bump insert"))?;

    // Credit du wallet partage.
    let new_balance = if reward > 0 {
        match state
            .wallet_uc
            .credit(&guild_id, &user_id, reward, "disboard-bump", &format!("Bump #{n} de la semaine"))
            .await
        {
            Ok(m) => Some(m.new_balance),
            Err(e) => {
                tracing::warn!(error = %e, guild_id, user_id, "Echec credit recompense bump");
                None
            }
        }
    } else {
        None
    };

    // Etat pour le rappel apres cooldown (snapshot de la config).
    let _ = sqlx::query(
        "INSERT INTO bump_guild_state (guild_id, channel_id, last_bump_at, cooldown_minutes, reminder_enabled, reminder_sent, updated_at) \
         VALUES ($1,$2,NOW(),$3,$4,FALSE,NOW()) \
         ON CONFLICT (guild_id) DO UPDATE SET \
            channel_id = EXCLUDED.channel_id, last_bump_at = NOW(), \
            cooldown_minutes = EXCLUDED.cooldown_minutes, reminder_enabled = EXCLUDED.reminder_enabled, \
            reminder_sent = FALSE, updated_at = NOW()",
    )
    .bind(&guild_id)
    .bind(&channel)
    .bind(cooldown as i32)
    .bind(reminder_enabled)
    .execute(&state.pg_pool)
    .await;

    // Role VIP : attribue a partir d'un seuil de bumps CUMULES (all-time).
    // Le bot fait l'ajout Discord (idempotent) ; on lui renvoie juste le role
    // a poser + un flag "vient de debloquer" pour annoncer une seule fois.
    let mut vip_role_id: Option<String> = None;
    let mut vip_just_unlocked = false;
    if cfg_bool(&cfg, "vip_enabled", false) {
        let vip_role = cfg_str(&cfg, "vip_role_id").unwrap_or("").trim().to_string();
        let vip_threshold = cfg_i64(&cfg, "vip_bump_threshold", 10).max(1);
        if !vip_role.is_empty() {
            let total_bumps: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM bump_events WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&guild_id)
            .bind(&user_id)
            .fetch_one(&state.pg_pool)
            .await
            .unwrap_or(0);
            if total_bumps >= vip_threshold {
                vip_role_id = Some(vip_role);
                // Le COUNT inclut le bump qu'on vient d'inserer : le passage
                // se fait exactement quand le total atteint le seuil.
                vip_just_unlocked = total_bumps == vip_threshold;
            }
        }
    }

    Ok(Json(BumpRewardDto {
        rewarded: true,
        reward,
        weekly_count: n,
        new_balance,
        vip_role_id,
        vip_just_unlocked,
    }))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DueReminderDto {
    pub guild_id: String,
    pub channel_id: String,
}

/// GET /api/bump/due-reminders Ã¢â‚¬â€ guilds dont le cooldown est ecoule et dont le
/// rappel n'a pas encore ete envoye (poll par le bot).
pub async fn due_reminders(
    State(state): State<AppState>,
) -> Result<Json<Vec<DueReminderDto>>, ApiError> {
    let rows: Vec<DueReminderDto> = sqlx::query_as(
        "SELECT guild_id, channel_id FROM bump_guild_state \
         WHERE reminder_enabled AND NOT reminder_sent AND channel_id <> '' \
           AND NOW() >= last_bump_at + make_interval(mins => cooldown_minutes)",
    )
    .fetch_all(&state.pg_pool)
    .await
    .map_err(sqlx_internal("due reminders"))?;
    Ok(Json(rows))
}

/// POST /api/bump/{guild_id}/reminder-sent Ã¢â‚¬â€ marque le rappel comme envoye.
pub async fn mark_reminder_sent(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE bump_guild_state SET reminder_sent = TRUE, updated_at = NOW() WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        .map_err(sqlx_internal("mark reminder"))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
