use std::time::Instant;

use dashmap::DashMap;
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::GuildId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use tracing::{error, info};

/// Gere le lockdown automatique pendant un raid.
/// Desactive SEND_MESSAGES pour @everyone sur tous les salons texte,
/// puis restaure les permissions apres expiration.
pub struct LockdownManager {
    /// guild_id -> (activation_time, Vec<(channel_id, original_everyone_overwrite)>)
    active: DashMap<GuildId, (Instant, Vec<(u64, Option<PermissionOverwrite>)>)>,
}

impl LockdownManager {
    pub fn new() -> Self {
        Self {
            active: DashMap::new(),
        }
    }

    /// Active le lockdown sur tous les salons texte d'un serveur.
    pub async fn activate(&self, ctx: &Context, guild_id: GuildId) {
        if self.active.contains_key(&guild_id) {
            return; // Deja actif
        }

        let channels = match guild_id.channels(&ctx.http).await {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Impossible de recuperer les channels pour lockdown");
                return;
            }
        };

        let everyone_role = serenity::model::id::RoleId::new(guild_id.get());
        let mut saved_states = Vec::new();

        for (channel_id, channel) in &channels {
            if channel.kind != ChannelType::Text {
                continue;
            }

            // Sauvegarder le permission overwrite actuel pour @everyone
            let existing = channel
                .permission_overwrites
                .iter()
                .find(|ow| ow.kind == PermissionOverwriteType::Role(everyone_role))
                .cloned();

            saved_states.push((channel_id.get(), existing.clone()));

            // Merger SEND_MESSAGES dans les deny existants (ne pas ecraser)
            let overwrite = match &existing {
                Some(ow) => PermissionOverwrite {
                    allow: ow.allow - Permissions::SEND_MESSAGES,
                    deny: ow.deny | Permissions::SEND_MESSAGES,
                    kind: PermissionOverwriteType::Role(everyone_role),
                },
                None => PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::SEND_MESSAGES,
                    kind: PermissionOverwriteType::Role(everyone_role),
                },
            };

            if let Err(e) = channel_id.create_permission(&ctx.http, overwrite).await {
                error!(
                    error = %e,
                    channel = %channel.name,
                    "Impossible d'appliquer le lockdown"
                );
            }
        }

        let count = saved_states.len();
        self.active.insert(guild_id, (Instant::now(), saved_states));

        info!(
            guild_id = %guild_id,
            channels = count,
            "Lockdown active — SEND_MESSAGES desactive pour @everyone"
        );
    }

    /// Desactive le lockdown en restaurant les permissions.
    #[allow(dead_code)]
    pub async fn deactivate(&self, ctx: &Context, guild_id: GuildId) {
        self.deactivate_with_http(&ctx.http, guild_id).await;
    }

    /// Desactive via un Http (pour les background tasks sans Context).
    pub async fn deactivate_with_http(&self, http: &serenity::http::Http, guild_id: GuildId) {
        let entry = match self.active.remove(&guild_id) {
            Some((_, data)) => data,
            None => return,
        };

        let (_, saved_states) = entry;
        let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

        for (channel_id_raw, original) in &saved_states {
            let channel_id = serenity::model::id::ChannelId::new(*channel_id_raw);

            match original {
                Some(ow) => {
                    // Restaurer l'overwrite original
                    if let Err(e) = channel_id.create_permission(http, ow.clone()).await {
                        error!(
                            error = %e,
                            channel_id = %channel_id,
                            "Impossible de restaurer la permission lockdown"
                        );
                    }
                }
                None => {
                    // Pas d'overwrite original — supprimer celui qu'on a cree
                    if let Err(e) = channel_id
                        .delete_permission(http, PermissionOverwriteType::Role(everyone_role))
                        .await
                    {
                        error!(
                            error = %e,
                            channel_id = %channel_id,
                            "Impossible de supprimer la permission lockdown"
                        );
                    }
                }
            }
        }

        info!(
            guild_id = %guild_id,
            channels = saved_states.len(),
            "Lockdown desactive — permissions restaurees"
        );
    }

    /// Verifie si le lockdown est actif pour un serveur.
    #[allow(dead_code)]
    pub fn is_active(&self, guild_id: GuildId) -> bool {
        self.active.contains_key(&guild_id)
    }

    /// Retourne les guilds dont le lockdown a depasse la duree maximale.
    pub fn expired_guilds(&self, duration_secs: u64) -> Vec<GuildId> {
        let max_duration = std::time::Duration::from_secs(duration_secs);
        let now = Instant::now();

        self.active
            .iter()
            .filter(|entry| now.duration_since(entry.value().0) >= max_duration)
            .map(|entry| *entry.key())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_active_initially() {
        let manager = LockdownManager::new();
        assert!(!manager.is_active(GuildId::new(1)));
    }

    #[test]
    fn expired_guilds_returns_expired() {
        let manager = LockdownManager::new();
        let guild = GuildId::new(1);

        manager.active.insert(
            guild,
            (
                Instant::now() - std::time::Duration::from_secs(600),
                vec![],
            ),
        );

        let expired = manager.expired_guilds(300);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], guild);
    }

    #[test]
    fn expired_guilds_ignores_recent() {
        let manager = LockdownManager::new();
        let guild = GuildId::new(1);

        manager.active.insert(
            guild,
            (
                Instant::now() - std::time::Duration::from_secs(100),
                vec![],
            ),
        );

        let expired = manager.expired_guilds(300);
        assert!(expired.is_empty());
    }

    #[test]
    fn multiple_guilds_mixed_expiry() {
        let manager = LockdownManager::new();
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);

        manager.active.insert(
            guild_a,
            (
                Instant::now() - std::time::Duration::from_secs(600),
                vec![],
            ),
        );
        manager.active.insert(
            guild_b,
            (
                Instant::now() - std::time::Duration::from_secs(100),
                vec![],
            ),
        );

        let expired = manager.expired_guilds(300);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], guild_a);
    }
}
