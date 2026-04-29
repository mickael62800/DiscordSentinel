//! MOD #4 — Detection de cibles a risque pour les commandes destructives.

use std::time::Instant;

use dashmap::DashMap;
use serenity::all::{Context, GuildId, User};
use serenity::prelude::TypeMapKey;
use tracing::warn;
use crate::domain::entities::system::discord_ids::ChannelId;

/// Age minimum d'un compte avant de considerer qu'il n'est plus "recent".
const RECENT_ACCOUNT_DAYS: i64 = 7;

/// Custom ID prefix pour les boutons "Confirmer" (suivi du pending_id).
pub const CONFIRM_PREFIX: &str = "sentinel_mod_risky_confirm_";

/// Custom ID prefix pour les boutons "Annuler".
pub const CANCEL_PREFIX: &str = "sentinel_mod_risky_cancel_";

/// TTL d'une confirmation en memoire (au dela, on purge).
pub const PENDING_TTL_SECS: u64 = 300;

/// Type d'action en attente de confirmation.
#[derive(Clone)]
pub enum PendingKind {
    Ban {
        delete_message_days: u8,
        is_permanent: bool,
    },
    Mute {
        /// Timeout Discord a appliquer en secondes (pre-capped a 28j).
        timeout_secs: u64,
    },
}

#[derive(Clone)]
pub struct RiskyPending {
    pub kind: PendingKind,
    pub guild_id: String,
    pub channel_id: ChannelId,
    pub target_id: String,
    pub target_name: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub reason: String,
    /// Duree en secondes (None = permanent)
    pub duration_secs: Option<u64>,
    pub duration_label: String,
    pub created_at: Instant,
}

pub struct RiskyPendingKey;
impl TypeMapKey for RiskyPendingKey {
    type Value = DashMap<String, RiskyPending>;
}

/// Inspecte une cible et retourne un libelle du risque si applicable.
pub async fn check_target_risk(ctx: &Context, guild_id: GuildId, target: &User) -> Option<String> {
    // 1. Compte Discord recent (cree il y a moins de N jours)
    let created_days_ago = account_age_days(target);
    if created_days_ago < RECENT_ACCOUNT_DAYS {
        return Some(format!(
            "compte Discord cree il y a seulement {} jour(s)",
            created_days_ago.max(0)
        ));
    }

    // 2. Bot — sanctionner un bot est souvent une erreur
    if target.bot {
        return Some("cible est un bot".to_string());
    }

    // 3. Membre de l'equipe de moderation (role avec MODERATE_MEMBERS)
    match guild_id.member(&ctx.http, target.id).await {
        Ok(member) => {
            match guild_id.to_guild_cached(&ctx.cache).map(|g| g.clone()) {
                Some(guild) => {
                    let is_moderator = member.roles.iter().any(|role_id| {
                        guild
                            .roles
                            .get(role_id)
                            .map(|r| {
                                r.permissions.moderate_members()
                                    || r.permissions.ban_members()
                                    || r.permissions.kick_members()
                                    || r.permissions.administrator()
                            })
                            .unwrap_or(false)
                    });
                    if is_moderator {
                        return Some(
                            "cible fait partie de l'equipe de moderation".to_string(),
                        );
                    }
                }
                None => {
                    warn!(
                        guild_id = %guild_id,
                        target_id = %target.id,
                        "risk check: guild cache miss, forcing confirmation (fail-safe)"
                    );
                    return Some(
                        "impossible de verifier les permissions (cache manquant)".to_string(),
                    );
                }
            }
        }
        Err(e) => {
            warn!(error = %e, target_id = %target.id, "risk check: member fetch failed");
        }
    }

    None
}

fn account_age_days(user: &User) -> i64 {
    account_age_days_from_ts(
        user.created_at().unix_timestamp(),
        chrono::Utc::now().timestamp(),
    )
}

fn account_age_days_from_ts(created_ts: i64, now_ts: i64) -> i64 {
    ((now_ts - created_ts) / 86_400).max(0)
}

/// Purge les pending confirmations expirees.
pub fn purge_expired(store: &DashMap<String, RiskyPending>) {
    let now = Instant::now();
    store.retain(|_, p| {
        now.duration_since(p.created_at).as_secs() < PENDING_TTL_SECS
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_removes_expired() {
        let store: DashMap<String, RiskyPending> = DashMap::new();
        store.insert(
            "old".to_string(),
            RiskyPending {
                kind: PendingKind::Mute { timeout_secs: 60 },
                guild_id: "g".into(),
                channel_id: "c".into(),
                target_id: "t".into(),
                target_name: "t".into(),
                moderator_id: "m".into(),
                moderator_name: "m".into(),
                reason: "r".into(),
                duration_secs: None,
                duration_label: "permanent".into(),
                created_at: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(PENDING_TTL_SECS + 10))
                    .unwrap_or_else(Instant::now),
            },
        );
        store.insert(
            "fresh".to_string(),
            RiskyPending {
                kind: PendingKind::Mute { timeout_secs: 60 },
                guild_id: "g".into(),
                channel_id: "c".into(),
                target_id: "t".into(),
                target_name: "t".into(),
                moderator_id: "m".into(),
                moderator_name: "m".into(),
                reason: "r".into(),
                duration_secs: None,
                duration_label: "permanent".into(),
                created_at: Instant::now(),
            },
        );
        purge_expired(&store);
        assert!(store.get("old").is_none() || store.len() == 1);
        assert!(store.get("fresh").is_some());
    }

    const DAY_SECS: i64 = 86_400;

    #[test]
    fn account_age_zero_if_created_now() {
        let now = 1_700_000_000_i64;
        assert_eq!(account_age_days_from_ts(now, now), 0);
    }

    #[test]
    fn account_age_one_day() {
        let now = 1_700_000_000_i64;
        let created = now - DAY_SECS;
        assert_eq!(account_age_days_from_ts(created, now), 1);
    }

    #[test]
    fn account_age_exactly_recent_threshold() {
        let now = 1_700_000_000_i64;
        let created = now - (RECENT_ACCOUNT_DAYS * DAY_SECS);
        assert_eq!(account_age_days_from_ts(created, now), RECENT_ACCOUNT_DAYS);
    }

    #[test]
    fn account_age_fractional_day_truncates_down() {
        let now = 1_700_000_000_i64;
        let created = now - (DAY_SECS + DAY_SECS / 2);
        assert_eq!(account_age_days_from_ts(created, now), 1);
    }

    #[test]
    fn account_age_future_timestamp_returns_zero() {
        let now = 1_700_000_000_i64;
        let created = now + DAY_SECS;
        assert_eq!(account_age_days_from_ts(created, now), 0);
    }

    #[test]
    fn account_age_very_old_account() {
        let now = 1_700_000_000_i64;
        let created = now - (5 * 365 * DAY_SECS);
        assert_eq!(account_age_days_from_ts(created, now), 5 * 365);
    }
}
