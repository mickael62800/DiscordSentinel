use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::{GuildId, RoleId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

/// Gere la quarantaine des utilisateurs suspects.
/// Les utilisateurs en quarantaine recoivent un role restrictif
/// et doivent passer un captcha pour etre liberes.
/// Les roles originaux sont sauvegardes pour etre restaures a la liberation.
pub struct QuarantineManager {
    /// (guild_id, user_id) -> (timestamp, roles originaux)
    quarantined: DashMap<(GuildId, UserId), (Instant, Vec<RoleId>)>,
}

impl QuarantineManager {
    pub fn new() -> Self {
        Self {
            quarantined: DashMap::new(),
        }
    }

    /// Met un utilisateur en quarantaine en lui assignant le role.
    /// Sauvegarde les roles originaux pour les restaurer a la liberation.
    pub async fn quarantine_user(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
    ) -> bool {
        // Sauvegarder les roles actuels du membre
        let original_roles = match guild_id.member(&ctx.http, user_id).await {
            Ok(member) => member.roles.clone(),
            Err(e) => {
                warn!(error = %e, "Impossible de lire les roles du membre, quarantaine sans sauvegarde");
                Vec::new()
            }
        };

        // Ajouter le role quarantaine sans supprimer les autres
        match ctx
            .http
            .add_member_role(guild_id, user_id, role_id, Some("Quarantaine Sentinel"))
            .await
        {
            Ok(_) => {
                self.quarantined
                    .insert((guild_id, user_id), (Instant::now(), original_roles));
                info!(
                    guild_id = %guild_id,
                    user_id = %user_id,
                    "Utilisateur mis en quarantaine (roles sauvegardes)"
                );
                true
            }
            Err(e) => {
                error!(
                    error = %e,
                    guild_id = %guild_id,
                    user_id = %user_id,
                    "Impossible d'assigner le role quarantaine"
                );
                false
            }
        }
    }

    /// Libere un utilisateur de la quarantaine.
    pub async fn release_user(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
    ) -> bool {
        if let Err(e) = ctx
            .http
            .remove_member_role(guild_id, user_id, role_id, Some("Captcha verifie"))
            .await
        {
            warn!(error = %e, "Impossible de retirer le role quarantaine");
            return false;
        }

        self.quarantined.remove(&(guild_id, user_id));
        info!(
            guild_id = %guild_id,
            user_id = %user_id,
            "Utilisateur libere de quarantaine"
        );
        true
    }

    /// Verifie si un utilisateur est en quarantaine.
    /// Rehydrate une entree de quarantaine au demarrage (depuis la DB). Les
    /// roles originaux ne sont pas persistes mais `release_user` ne les utilise
    /// pas (il retire seulement le role de quarantaine).
    pub fn rehydrate(&self, guild_id: GuildId, user_id: UserId) {
        self.quarantined
            .entry((guild_id, user_id))
            .or_insert_with(|| (Instant::now(), Vec::new()));
    }

    pub fn is_quarantined(&self, guild_id: GuildId, user_id: UserId) -> bool {
        self.quarantined.contains_key(&(guild_id, user_id))
    }

    /// Retourne le nombre total d'utilisateurs en quarantaine.
    pub fn quarantined_count(&self) -> usize {
        self.quarantined.len()
    }

    /// Supprime un utilisateur du tracking (apres kick par ex).
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

        manager
            .quarantined
            .insert((guild, user), (Instant::now(), Vec::new()));
        assert!(manager.is_quarantined(guild, user));

        manager.remove_tracking(guild, user);
        assert!(!manager.is_quarantined(guild, user));
    }
}
