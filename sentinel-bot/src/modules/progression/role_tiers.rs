//! Attribution des roles de palier — ADAPTATEUR Discord.
//!
//! La decision (quels roles ajouter, lesquels retirer) vit dans le core
//! (`sentinel_core::domain::services::progression::role_tiers`) avec ses tests.
//! Ce module ne fait que l'orchestration : lire la config, lire les roles
//! actuels du membre, et n'appeler Discord que pour les differences.

use serenity::all::{Context, GuildId, RoleId, UserId};
use tracing::{info, warn};

use sentinel_core::domain::services::progression::role_tiers::{
    analyser_paliers, roles_pour_niveau, ModePalier,
};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::MODULE_BOT_NAME;

/// Applique les paliers de roles pour un membre a son niveau courant.
///
/// Best-effort de bout en bout : un echec ne remonte pas et n'interrompt
/// jamais le gain d'XP. Perdre un role est genant, perdre la progression du
/// membre le serait davantage.
pub async fn appliquer_paliers(ctx: &Context, guild_id: GuildId, user_id: UserId, niveau: i32) {
    let config = {
        let data = ctx.data.read().await;
        let Some(base) = data.get::<ApiClientKey>() else {
            return;
        };
        match base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, %guild_id, "paliers: config illisible");
                return;
            }
        }
    };

    let paliers = analyser_paliers(&BaseApiClient::config_or(&config, "level_role_rewards", ""));
    if paliers.is_empty() {
        return;
    }
    let mode = ModePalier::depuis_config(&BaseApiClient::config_or(
        &config,
        "level_role_mode",
        "cumulatif",
    ));

    let (a_ajouter, a_retirer) = roles_pour_niveau(&paliers, niveau, mode);

    // Les roles actuels du membre : sans eux, on redemanderait a Discord
    // d'ajouter un role deja porte a chaque message, et le journal d'audit du
    // serveur se remplirait de mouvements sans objet.
    let membre = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, %guild_id, %user_id, "paliers: membre introuvable");
            return;
        }
    };
    let porte: Vec<u64> = membre.roles.iter().map(|r| r.get()).collect();

    for role in a_ajouter.into_iter().filter(|r| !porte.contains(r)) {
        match membre.add_role(&ctx.http, RoleId::new(role)).await {
            Ok(()) => info!(%guild_id, %user_id, role, niveau, "palier: role attribue"),
            Err(e) => warn!(error = %e, %guild_id, %user_id, role, "palier: echec attribution"),
        }
    }

    for role in a_retirer.into_iter().filter(|r| porte.contains(r)) {
        match membre.remove_role(&ctx.http, RoleId::new(role)).await {
            Ok(()) => info!(%guild_id, %user_id, role, niveau, "palier: role retire"),
            Err(e) => warn!(error = %e, %guild_id, %user_id, role, "palier: echec retrait"),
        }
    }
}
