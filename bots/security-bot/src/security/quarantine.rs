use std::time::Instant;

use dashmap::DashMap;
use serenity::all::EditMember;
use serenity::model::id::{GuildId, RoleId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

/// Gère la quarantaine des utilisateurs suspects.
/// Les utilisateurs en quarantaine reçoivent un rôle restrictif
/// et doivent passer un captcha pour être libérés.
pub struct QuarantineManager {
    /// (guild_id, user_id) → timestamp de mise en quarantaine
    quarantined: DashMap<(GuildId, UserId), Instant>,
}

impl QuarantineManager {
    pub fn new() -> Self {
        Self {
            quarantined: DashMap::new(),
        }
    }

    /// Met un utilisateur en quarantaine en lui assignant le rôle.
    pub async fn quarantine_user(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
    ) -> bool {
        // Assigner le rôle quarantaine
        let edit = EditMember::new().roles(vec![role_id]);
        match guild_id.edit_member(&ctx.http, user_id, edit).await {
            Ok(_) => {
                self.quarantined.insert((guild_id, user_id), Instant::now());
                info!(
                    guild_id = %guild_id,
                    user_id = %user_id,
                    "Utilisateur mis en quarantaine"
                );
                true
            }
            Err(e) => {
                error!(
                    error = %e,
                    guild_id = %guild_id,
                    user_id = %user_id,
                    "Impossible d'assigner le rôle quarantaine"
                );
                false
            }
        }
    }

    /// Libère un utilisateur de la quarantaine.
    pub async fn release_user(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
    ) -> bool {
        if let Err(e) = ctx.http.remove_member_role(guild_id, user_id, role_id, Some("Captcha vérifié")).await {
            warn!(error = %e, "Impossible de retirer le rôle quarantaine");
            return false;
        }

        self.quarantined.remove(&(guild_id, user_id));
        info!(
            guild_id = %guild_id,
            user_id = %user_id,
            "Utilisateur libéré de quarantaine"
        );
        true
    }

    /// Vérifie si un utilisateur est en quarantaine.
    pub fn is_quarantined(&self, guild_id: GuildId, user_id: UserId) -> bool {
        self.quarantined.contains_key(&(guild_id, user_id))
    }

    /// Retourne les utilisateurs dont le timeout de quarantaine a expiré.
    pub fn expired_users(&self, timeout_secs: u64) -> Vec<(GuildId, UserId)> {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();

        self.quarantined
            .iter()
            .filter(|entry| now.duration_since(*entry.value()) >= timeout)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Retourne le nombre total d'utilisateurs en quarantaine.
    pub fn quarantined_count(&self) -> usize {
        self.quarantined.len()
    }

    /// Supprime un utilisateur du tracking (après kick par ex).
    pub fn remove_tracking(&self, guild_id: GuildId, user_id: UserId) {
        self.quarantined.remove(&(guild_id, user_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking() {
        let manager = QuarantineManager::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);

        assert!(!manager.is_quarantined(guild, user));

        manager.quarantined.insert((guild, user), Instant::now());
        assert!(manager.is_quarantined(guild, user));

        manager.remove_tracking(guild, user);
        assert!(!manager.is_quarantined(guild, user));
    }

    #[test]
    fn test_expired_users() {
        let manager = QuarantineManager::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);

        // Insérer avec un timestamp dans le passé
        manager
            .quarantined
            .insert((guild, user), Instant::now() - std::time::Duration::from_secs(600));

        let expired = manager.expired_users(300); // timeout 5min
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], (guild, user));

        // Pas expiré si timeout plus long
        let expired = manager.expired_users(900);
        assert!(expired.is_empty());
    }
}
