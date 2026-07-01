use std::time::Instant;

use dashmap::DashMap;
use serenity::builder::EditChannel;
use serenity::model::channel::ChannelType;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tracing::{error, info};

use crate::shared::heartbeat::ApiClientKey;

/// Gere l'activation automatique du slowmode pendant un raid.
pub struct SlowmodeManager {
    /// guild_id -> (timestamp d'activation, ancien slowmode par channel_id)
    active: DashMap<GuildId, (Instant, Vec<(u64, u16)>)>,
}

impl SlowmodeManager {
    pub fn new() -> Self {
        Self {
            active: DashMap::new(),
        }
    }

    /// Active le slowmode sur tous les salons texte d'un serveur.
    ///
    /// `slowmode_secs` : rate-limit applique cote Discord (delai entre messages).
    /// `persist_duration_secs` : duree de vie persistee cote API pour que le
    /// worker `expire_slowmode` sache quand restaurer. Doit refleter la duree
    /// utilisee par la boucle de revert locale (`SecurityConfig.slowmode_duration_secs`,
    /// reglable via l'env `SLOWMODE_DURATION_SECS`, defaut 300).
    pub async fn activate(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        slowmode_secs: u16,
        persist_duration_secs: u64,
    ) {
        if self.active.contains_key(&guild_id) {
            return; // Deja actif
        }

        let channels = match guild_id.channels(&ctx.http).await {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Impossible de recuperer les channels");
                return;
            }
        };

        let mut previous_states = Vec::new();

        for (channel_id, channel) in &channels {
            if channel.kind != ChannelType::Text {
                continue;
            }

            // Sauvegarder l'ancien slowmode
            let old_rate = channel.rate_limit_per_user.unwrap_or(0);
            previous_states.push((channel_id.get(), old_rate));

            // Appliquer le nouveau slowmode
            let edit = EditChannel::new().rate_limit_per_user(slowmode_secs);
            if let Err(e) = channel_id.edit(&ctx.http, edit).await {
                error!(
                    error = %e,
                    channel = %channel.name,
                    "Impossible d'activer le slowmode"
                );
            }
        }

        let count = previous_states.len();

        // Phase 5H — persiste les previous_states en DB pour resilience
        // au restart bot. Le worker `expire_slowmode` (sentinel-worker)
        // detectera l'expiration et publiera l'event de restauration.
        let states_json: Vec<serde_json::Value> = previous_states
            .iter()
            .map(|(ch, rate)| {
                serde_json::json!({
                    "channel_id": ch.to_string(),
                    "rate": *rate as u64,
                })
            })
            .collect();
        if let Some(base) = ctx.data.read().await.get::<ApiClientKey>() {
            let body = serde_json::json!({
                "guild_id": guild_id.to_string(),
                "previous_states": states_json,
                "duration_secs": persist_duration_secs,
            });
            base.post_fire_and_forget("/api/security/slowmode", &body)
                .await;
        }

        self.active
            .insert(guild_id, (Instant::now(), previous_states));

        info!(
            guild_id = %guild_id,
            channels = count,
            slowmode_secs,
            "Slowmode anti-raid active"
        );
    }

    /// Desactive le slowmode en restaurant les valeurs precedentes.
    #[allow(dead_code)]
    pub async fn deactivate(&self, ctx: &Context, guild_id: GuildId) {
        self.deactivate_with_http(&ctx.http, guild_id).await;
    }

    /// Desactive le slowmode via un Arc<Http> (pour les background tasks sans Context).
    pub async fn deactivate_with_http(&self, http: &serenity::http::Http, guild_id: GuildId) {
        let entry = match self.active.remove(&guild_id) {
            Some((_, data)) => data,
            None => return,
        };

        let (_, previous_states) = entry;

        for (channel_id_raw, old_rate) in &previous_states {
            let channel_id = serenity::model::id::ChannelId::new(*channel_id_raw);
            let edit = EditChannel::new().rate_limit_per_user(*old_rate);
            if let Err(e) = channel_id.edit(http, edit).await {
                error!(
                    error = %e,
                    channel_id = %channel_id,
                    "Impossible de restaurer le slowmode"
                );
            }
        }

        info!(
            guild_id = %guild_id,
            channels = previous_states.len(),
            "Slowmode anti-raid desactive"
        );
    }

    /// Retourne les guilds dont le slowmode a depasse la duree maximale.
    pub fn expired_guilds(&self, duration_secs: u64) -> Vec<GuildId> {
        let max_duration = std::time::Duration::from_secs(duration_secs);
        let now = Instant::now();

        self.active
            .iter()
            .filter(|entry| now.duration_since(entry.value().0) >= max_duration)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Verifie si le slowmode est actif pour un serveur.
    #[allow(dead_code)]
    pub fn is_active(&self, guild_id: GuildId) -> bool {
        self.active.contains_key(&guild_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_active_initially() {
        let manager = SlowmodeManager::new();
        assert!(!manager.is_active(GuildId::new(1)));
    }

    #[test]
    fn test_expired_guilds() {
        let manager = SlowmodeManager::new();
        let guild = GuildId::new(1);

        manager.active.insert(
            guild,
            (Instant::now() - std::time::Duration::from_secs(600), vec![]),
        );

        let expired = manager.expired_guilds(300);
        assert_eq!(expired.len(), 1);

        let not_expired = manager.expired_guilds(900);
        assert!(not_expired.is_empty());
    }
}
