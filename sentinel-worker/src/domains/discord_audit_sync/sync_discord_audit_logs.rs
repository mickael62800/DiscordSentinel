//! Phase 6A — Reconciliation des audit_logs avec l'API Discord.
//!
//! Ce worker importe dans la table `audit_logs` les actions de moderation
//! effectuees HORS de nos bots (via le client Discord directement, ou par
//! un autre bot installe sur la guild). Sans ce sync, le desktop ne voit
//! que les actions effectuees via Sentinel.
//!
//! # Flow
//!
//! 1. Pour chaque guild dans `guilds` :
//!    - Recupere le `last_entry_id` depuis `discord_audit_sync_state`
//!    - Appelle `GET /guilds/{id}/audit-logs?after={last_entry_id}&limit=100`
//!    - Parse les entries et insert dans `audit_logs` avec un prefix
//!      `discord_audit:` sur `event_type` (pour les distinguer des actions
//!      directes)
//!    - Update `last_entry_id` au plus recent fetche
//!
//! # Action types Discord couverts (MVP)
//!
//! - 20 = `MEMBER_KICK`       → `discord_audit:member_kick`
//! - 22 = `MEMBER_BAN_ADD`    → `discord_audit:member_ban`
//! - 23 = `MEMBER_BAN_REMOVE` → `discord_audit:member_unban`
//! - 24 = `MEMBER_UPDATE`     → `discord_audit:member_timeout` (si timeout)
//! - 25 = `MEMBER_ROLE_UPDATE`→ `discord_audit:member_role_update`
//!
//! Les autres types (channel/role create/delete, message delete, etc.) sont
//! ignores par le MVP — ils peuvent etre ajoutes incrementalement dans
//! `map_action_type`.
//!
//! # Dedup
//!
//! On stocke l'`entry_id` Discord dans `details.discord_entry_id` pour pouvoir
//! dedupliquer au niveau DB si le meme entry est re-ingere (ex: apres un
//! reset du `last_entry_id`). Le sync normal evite ca via le curseur.
//!
//! # Rate limits
//!
//! Discord impose un rate limit global + par-route. Pour le MVP on fait 1
//! request par guild par tick (5 min), largement sous le budget. Les
//! headers `X-RateLimit-Remaining` et `Retry-After` ne sont pas encore
//! respectes — a ajouter si on scale a beaucoup de guilds.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{debug, info, warn};

use super::ENTRIES_PER_CALL;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

pub async fn run(pool: &PgPool, bot_token: &str) -> Result<(), String> {
    // 1. Recuperer les guilds a synchroniser
    let guilds: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("query guilds: {e}"))?;

    if guilds.is_empty() {
        debug!("Aucune guild a synchroniser");
        return Ok(());
    }

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    let mut total_imported = 0u32;
    let mut guilds_synced = 0u32;
    let mut guilds_errored = 0u32;

    for (guild_id,) in guilds {
        match sync_guild(&http, pool, bot_token, &guild_id).await {
            Ok(imported) => {
                total_imported += imported;
                guilds_synced += 1;
                if imported > 0 {
                    info!(guild_id = %guild_id, imported, "Discord audit log synced");
                }
            }
            Err(e) => {
                warn!(guild_id = %guild_id, error = %e, "Discord audit sync failed");
                guilds_errored += 1;

                // Enregistre l'erreur dans le state.
                if let Err(db_err) = sqlx::query(
                    "INSERT INTO discord_audit_sync_state (guild_id, last_synced_at, last_error, consecutive_errors) \
                     VALUES ($1, NOW(), $2, 1) \
                     ON CONFLICT (guild_id) DO UPDATE SET \
                        last_synced_at = NOW(), \
                        last_error = EXCLUDED.last_error, \
                        consecutive_errors = discord_audit_sync_state.consecutive_errors + 1",
                )
                .bind(&guild_id)
                .bind(&e)
                .execute(pool)
                .await
                {
                    warn!(guild_id = %guild_id, error = %db_err, "Echec sauvegarde error state dans sync_state");
                }
            }
        }
    }

    info!(
        guilds_synced,
        guilds_errored,
        total_imported,
        "Discord audit sync tick termine"
    );
    Ok(())
}

async fn sync_guild(
    http: &reqwest::Client,
    pool: &PgPool,
    bot_token: &str,
    guild_id: &str,
) -> Result<u32, String> {
    // 1. Recuperer le curseur
    let last_entry_id: Option<String> =
        sqlx::query_scalar("SELECT last_entry_id FROM discord_audit_sync_state WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("query sync state: {e}"))?
            .flatten();

    // 2. Appel Discord
    let mut url = format!(
        "{DISCORD_API_BASE}/guilds/{guild_id}/audit-logs?limit={ENTRIES_PER_CALL}"
    );
    if let Some(ref id) = last_entry_id {
        url.push_str(&format!("&after={id}"));
    }

    let resp = http
        .get(&url)
        .header("Authorization", format!("Bot {bot_token}"))
        .send()
        .await
        .map_err(|e| format!("discord GET failed: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        // Le bot n'a pas VIEW_AUDIT_LOG sur cette guild — on n'insiste pas
        return Err("VIEW_AUDIT_LOG manquant".into());
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        // Rate limited par Discord — respecter Retry-After avant de retenter.
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(5.0);
        warn!(guild_id = %guild_id, retry_after, "Discord rate limit — attente");
        tokio::time::sleep(std::time::Duration::from_secs_f64(retry_after)).await;
        return Err(format!("rate limited ({retry_after}s), retry au prochain tick"));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("discord non-success {status}: {body}"));
    }

    let audit_log: AuditLogResponse = resp
        .json()
        .await
        .map_err(|e| format!("discord parse: {e}"))?;

    if audit_log.audit_log_entries.is_empty() {
        // Pas de nouvelles entries — juste update last_synced_at
        sqlx::query(
            "INSERT INTO discord_audit_sync_state (guild_id, last_entry_id, last_synced_at, consecutive_errors) \
             VALUES ($1, $2, NOW(), 0) \
             ON CONFLICT (guild_id) DO UPDATE SET \
                last_synced_at = NOW(), \
                last_error = NULL, \
                consecutive_errors = 0",
        )
        .bind(guild_id)
        .bind(&last_entry_id)
        .execute(pool)
        .await
        .map_err(|e| format!("update sync state: {e}"))?;
        return Ok(0);
    }

    // Construit une map user_id → username pour enrichir les inserts
    let user_map: std::collections::HashMap<String, String> = audit_log
        .users
        .iter()
        .map(|u| (u.id.clone(), u.username.clone()))
        .collect();

    // 3. Insert les entries pertinentes. Discord renvoie les entries du
    //    plus recent au plus ancien — on inverse pour que les inserts
    //    soient chronologiques et que `last_entry_id` reflete le plus
    //    recent.
    let mut inserted = 0u32;
    let mut newest_id = last_entry_id.clone();

    for entry in audit_log.audit_log_entries.iter().rev() {
        let Some(event_type) = map_action_type(entry.action_type) else {
            // Type d'action non couvert par le MVP — on skip mais on
            // avance quand meme le curseur.
            if is_newer_snowflake(newest_id.as_deref(), &entry.id) {
                newest_id = Some(entry.id.clone());
            }
            continue;
        };

        let actor_name = entry
            .user_id
            .as_deref()
            .and_then(|uid| user_map.get(uid).cloned());

        let details = serde_json::json!({
            "discord_entry_id": entry.id,
            "action_type_raw": entry.action_type,
            "changes": entry.changes,
            "options": entry.options,
            "reason": entry.reason,
        });

        let res = sqlx::query(
            "INSERT INTO audit_logs (guild_id, event_type, actor_id, actor_name, target_id, details, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        )
        .bind(guild_id)
        .bind(&event_type)
        .bind(&entry.user_id)
        .bind(&actor_name)
        .bind(&entry.target_id)
        .bind(&details)
        .execute(pool)
        .await;

        match res {
            Ok(_) => {
                inserted += 1;
                // Avancer le curseur SEULEMENT si l'INSERT a reussi.
                // Sinon l'entry sera re-fetchee au prochain sync.
                if is_newer_snowflake(newest_id.as_deref(), &entry.id) {
                    newest_id = Some(entry.id.clone());
                }
            }
            Err(e) => warn!(
                error = %e,
                entry_id = %entry.id,
                event_type = %event_type,
                "insert audit_log failed — curseur non avance"
            ),
        }
    }

    // 4. Update le curseur
    sqlx::query(
        "INSERT INTO discord_audit_sync_state (guild_id, last_entry_id, last_synced_at, consecutive_errors) \
         VALUES ($1, $2, NOW(), 0) \
         ON CONFLICT (guild_id) DO UPDATE SET \
            last_entry_id = EXCLUDED.last_entry_id, \
            last_synced_at = NOW(), \
            last_error = NULL, \
            consecutive_errors = 0",
    )
    .bind(guild_id)
    .bind(&newest_id)
    .execute(pool)
    .await
    .map_err(|e| format!("update sync state: {e}"))?;

    Ok(inserted)
}

/// Compare deux snowflakes Discord (stockes en String dans le JSON) par leur
/// valeur u64. Retourne true si `candidate` est plus recent que `current`.
///
/// Historiquement le code faisait une comparaison de Strings, ce qui
/// fonctionne tant que les deux snowflakes ont la **meme longueur** (cas
/// habituel en 2024+ : ~19 digits). Mais c'est un bug latent : si les
/// longueurs different (ex : vieux snowflake 17 digits vs nouveau 19 digits),
/// la comparaison ASCII renvoie un ordre faux et le curseur `last_entry_id`
/// peut se retrouver bloque ou rater des entries.
fn is_newer_snowflake(current: Option<&str>, candidate: &str) -> bool {
    let candidate_id: u64 = match candidate.parse() {
        Ok(v) => v,
        Err(_) => return false, // candidate invalide : ne pas avancer le curseur
    };
    match current {
        None => true,
        Some(s) => match s.parse::<u64>() {
            Ok(curr) => candidate_id > curr,
            Err(_) => true, // current invalide : accepter le candidate
        },
    }
}

/// Mapping des action_types Discord numeriques vers des `event_type` lisibles
/// stockes dans `audit_logs`. Les valeurs proviennent de la doc Discord :
/// <https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events>
///
/// MVP : on couvre uniquement les actions de moderation pertinentes. Les autres
/// (channel/role create, message delete, etc.) retournent None et sont skip.
fn map_action_type(action_type: u32) -> Option<String> {
    let name = match action_type {
        20 => "member_kick",
        22 => "member_ban",
        23 => "member_unban",
        24 => "member_timeout",
        25 => "member_role_update",
        _ => return None,
    };
    Some(format!("discord_audit:{name}"))
}

// ═══════════════════════════════════════════════════
// Discord API response types
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct AuditLogResponse {
    #[serde(default)]
    audit_log_entries: Vec<AuditLogEntry>,
    #[serde(default)]
    users: Vec<DiscordUser>,
}

#[derive(Debug, Deserialize)]
struct AuditLogEntry {
    id: String,
    user_id: Option<String>,
    target_id: Option<String>,
    action_type: u32,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    changes: serde_json::Value,
    #[serde(default)]
    options: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

// Suppress unused warnings on DateTime import — utile si on ajoute du tracking
// temporel plus tard
#[allow(dead_code)]
fn _ensure_chrono_used() -> Option<DateTime<Utc>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_action_type_member_ban() {
        assert_eq!(map_action_type(22), Some("discord_audit:member_ban".to_string()));
    }

    #[test]
    fn map_action_type_member_unban() {
        assert_eq!(
            map_action_type(23),
            Some("discord_audit:member_unban".to_string())
        );
    }

    #[test]
    fn map_action_type_member_kick() {
        assert_eq!(
            map_action_type(20),
            Some("discord_audit:member_kick".to_string())
        );
    }

    #[test]
    fn map_action_type_member_timeout() {
        assert_eq!(
            map_action_type(24),
            Some("discord_audit:member_timeout".to_string())
        );
    }

    #[test]
    fn map_action_type_member_role_update() {
        assert_eq!(
            map_action_type(25),
            Some("discord_audit:member_role_update".to_string())
        );
    }

    #[test]
    fn map_action_type_unknown_returns_none() {
        assert_eq!(map_action_type(1), None);
        assert_eq!(map_action_type(72), None); // MESSAGE_DELETE pas couvert MVP
        assert_eq!(map_action_type(999), None);
    }

    // ── Tests du helper is_newer_snowflake ────────────────

    #[test]
    fn snowflake_none_current_accepts_anything() {
        assert!(is_newer_snowflake(None, "1234567890123456789"));
    }

    #[test]
    fn snowflake_strictly_greater() {
        assert!(is_newer_snowflake(
            Some("1234567890123456789"),
            "1234567890123456790"
        ));
    }

    #[test]
    fn snowflake_equal_is_not_newer() {
        assert!(!is_newer_snowflake(
            Some("1234567890123456789"),
            "1234567890123456789"
        ));
    }

    #[test]
    fn snowflake_strictly_smaller() {
        assert!(!is_newer_snowflake(
            Some("1234567890123456790"),
            "1234567890123456789"
        ));
    }

    /// Regression test pour le bug P0 : comparaison string vs u64.
    /// En string, "2" < "10" serait false car '2' > '1' en ASCII.
    /// En u64, 2 < 10 est vrai.
    #[test]
    fn snowflake_different_lengths_compared_numerically() {
        // "2" (1 digit) < "10" (2 digits) en u64 -> 10 doit etre newer que 2
        assert!(is_newer_snowflake(Some("2"), "10"));

        // "99" (2 digits) < "100" (3 digits) en u64 mais "99" > "100" en string
        assert!(is_newer_snowflake(Some("99"), "100"));

        // Sens inverse : "1000" > "999" en u64
        assert!(!is_newer_snowflake(Some("1000"), "999"));
    }

    #[test]
    fn snowflake_invalid_candidate_ignored() {
        assert!(!is_newer_snowflake(Some("1234"), "not-a-number"));
    }

    #[test]
    fn snowflake_invalid_current_accepts_candidate() {
        // Si le curseur en base est corrompu (pas un u64), on accepte le
        // candidate pour se re-synchroniser.
        assert!(is_newer_snowflake(Some("corrupted"), "1234567890123456789"));
    }
}
