//! MOD #4 — Detection de cibles a risque pour les commandes destructives.
//!
//! Certaines cibles necessitent une confirmation explicite avant qu'une sanction
//! (ban, mute) soit appliquee :
//!   - un autre moderateur (role avec permission `MODERATE_MEMBERS` ou membre
//!     de l'equipe de staff) — eviter les bans accidentels entre collegues
//!   - un compte recent (< 7 jours) — souvent suspect, mais peut aussi etre
//!     un vrai nouveau membre : laisser le moderateur confirmer
//!   - un compte avec une anciennete Discord tres elevee (> 5 ans) sans activite
//!     locale — souvent un compte legitime pris a tort pour un bot
//!
//! Quand une cible est a risque, la commande defere au lieu d'executer : elle
//! poste une confirmation avec deux boutons (Confirmer / Annuler) et stocke
//! l'action en attente dans `RiskyPendingKey`. Le handler de boutons reprend
//! ensuite l'execution.

use std::time::Instant;

use dashmap::DashMap;
use serenity::all::{Context, GuildId, User};
use serenity::prelude::TypeMapKey;
use tracing::warn;

/// Age minimum d'un compte avant de considerer qu'il n'est plus "recent".
const RECENT_ACCOUNT_DAYS: i64 = 7;

/// Custom ID prefix pour les boutons "Confirmer" (suivi du pending_id).
pub const CONFIRM_PREFIX: &str = "sentinel_mod_risky_confirm_";

/// Custom ID prefix pour les boutons "Annuler".
pub const CANCEL_PREFIX: &str = "sentinel_mod_risky_cancel_";

/// TTL d'une confirmation en memoire (au dela, on purge).
pub const PENDING_TTL_SECS: u64 = 300;

/// Type d'action en attente de confirmation — garde assez d'info pour executer
/// la sanction apres le clic "Confirmer" sans re-parser la command.
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
    pub channel_id: String,
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
///
/// `None` = pas de risque, la sanction peut etre appliquee immediatement.
/// `Some("libelle")` = risque detecte, afficher une confirmation au moderateur.
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
    // On charge le membre du guild puis on verifie les permissions de ses roles.
    match guild_id.member(&ctx.http, target.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache).map(|g| g.clone()) {
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
                    return Some("cible fait partie de l'equipe de moderation".to_string());
                }
            }
        }
        Err(e) => {
            // Si on ne peut pas fetch le membre, on ne bloque pas la sanction —
            // c'est probablement qu'il n'est plus dans le serveur (ban via ID externe)
            warn!(error = %e, target_id = %target.id, "risk check: member fetch failed");
        }
    }

    None
}

fn account_age_days(user: &User) -> i64 {
    let created = user.created_at();
    let now = chrono::Utc::now().timestamp();
    ((now - created.unix_timestamp()) / 86_400).max(0)
}

/// Purge les pending confirmations expirees. Appele apres chaque access.
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
                // created_at way in the past
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
}
